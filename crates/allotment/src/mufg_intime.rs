//! MUFG Intime India provider — session/token registrar adapter.
//!
//! Gate 2 verified the public flow: identifier-free discovery
//! (`IPO.aspx/GetDetails`), a session cookie plus server-generated request
//! token (`IPO.aspx/generateToken`, stored as `hidToken`), conditional
//! CAPTCHA (markup present but dormant during research), and a JSON-wrapped
//! XML result contract. No real investor lookup was submitted.
//!
//! The live transport is implemented but separately authorization-gated:
//! token failures, session expiry, and CAPTCHA activation map to operational
//! states — never a guessed
//! financial result. `MufgEphemeralSession` holds cookie/token values only
//! in memory with a redacted `Debug` impl; they are never serialized,
//! logged, or persisted. The parser reads only `company_id`/`companyname`
//! (discovery) and `ALLOT`/`SHARES` (results); member fields (PEMNDG,
//! NAME1) are required for structure but never copied.

use aes::Aes128;
use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use sanket_identity_security::Pan;

use crate::http::{HttpPolicyError, SharedHttpClient, TIMEOUT_SECS};
use crate::provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, HumanVerificationRequirement,
    IssueDiscoveryMode, LookupKeyKind, NegativeResultProof, PositiveResultProof,
    ProviderAllotmentResult, ProviderCapabilities, ProviderError, ProviderHealth,
    ProviderTransportKind, RegistrarIssue, SessionRequirement,
};

const DISCOVERY_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/public-issues.html";
const ISSUES_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/GetDetails";
const TOKEN_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/generateToken";
const LOOKUP_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/SearchOnPan";
const ALLOWED_HOSTS: &[&str] = &["in.mpms.mufg.com"];
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;
const TOKEN_KEY: &[u8; 16] = b"0123456789abcdef";
const TOKEN_IV: &[u8; 16] = b"abcdef9876543210";
const ISSUE_FINGERPRINT: &str = "mufg-issues-companyid-xml-v1";
const RESULT_FINGERPRINT: &str = "mufg-result-d-xml-table-v1";

/// CAPTCHA activation is conditional: markup can exist while dormant.
/// Only an explicitly active container yields `Required`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MufgCaptchaState {
    Absent,
    Dormant,
    Required,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MufgIssue {
    pub provider_issue_id: String,
    pub display_name: String,
    pub discovered_at: String,
    pub source_url: String,
    pub structural_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MufgPublicPrecheck {
    pub issue: MufgIssue,
    pub captcha_state: MufgCaptchaState,
    pub tls_http_healthy: bool,
    pub session_bootstrap_succeeded: bool,
    pub request_token_obtained: bool,
    pub lookup_endpoint: &'static str,
    pub observed_at: String,
}

/// Ephemeral, provider-private session state. Values live only in memory.
/// The `Debug` implementation never renders cookie/token bytes.
pub struct MufgEphemeralSession {
    id: String,
    cookie_jar: String,
    request_token: String,
    created_at: String,
    expires_at: Option<String>,
}

impl std::fmt::Debug for MufgEphemeralSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MufgEphemeralSession")
            .field("id", &self.id)
            .field("cookie_jar", &"[REDACTED]")
            .field("request_token", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl MufgEphemeralSession {
    pub fn new(
        id: impl Into<String>,
        cookie_jar: impl Into<String>,
        request_token: impl Into<String>,
        created_at: impl Into<String>,
        expires_at: Option<String>,
    ) -> Result<Self, ProviderError> {
        let session = Self {
            id: id.into(),
            cookie_jar: cookie_jar.into(),
            request_token: request_token.into(),
            created_at: created_at.into(),
            expires_at,
        };
        if session.id.is_empty()
            || session.cookie_jar.is_empty()
            || session.request_token.is_empty()
        {
            return Err(ProviderError::Unknown(
                "mufg session must carry id, cookie, and token".into(),
            ));
        }
        Ok(session)
    }

    /// Lexical RFC 3339 UTC comparison under the fixed-shape timestamp
    /// contract; fails safe (anything at/past expiry reads expired).
    pub fn is_expired(&self, now: &str) -> bool {
        match self.expires_at.as_deref() {
            Some(expires_at) => now >= expires_at,
            None => false,
        }
    }
}

pub struct MufgIntimeProvider {
    network_enabled: bool,
}

struct MufgLiveSession {
    http: SharedHttpClient,
    issue: MufgIssue,
    captcha_state: MufgCaptchaState,
    encrypted_token: Zeroizing<String>,
}

impl MufgIntimeProvider {
    pub fn new() -> Self {
        Self {
            network_enabled: true,
        }
    }

    pub fn offline_for_tests() -> Self {
        Self {
            network_enabled: false,
        }
    }

    /// Parse the JSON-wrapped XML discovery payload. Only unique, all-digit
    /// `company_id`s with a non-empty `companyname` are recognized; any
    /// placeholder, duplicate, or drifted shape fails closed.
    pub fn parse_issue_bundle(
        bundle: &str,
        discovered_at: &str,
    ) -> Result<Vec<MufgIssue>, ProviderError> {
        let mut issues = Vec::new();
        let mut rest = xml_payload(bundle)?;
        while let Some((provider_issue_id, after_id)) = tagged_value(&rest, "company_id") {
            let Some((display_name, after_name)) = tagged_value(after_id, "companyname") else {
                return Err(ProviderError::Unknown(
                    "mufg issue record is incomplete".into(),
                ));
            };
            rest = after_name.to_owned();

            if provider_issue_id.is_empty()
                || provider_issue_id.len() > 32
                || !provider_issue_id.bytes().all(|byte| byte.is_ascii_digit())
                || display_name.is_empty()
                || display_name.len() > 200
                || issues
                    .iter()
                    .any(|issue: &MufgIssue| issue.provider_issue_id == provider_issue_id)
            {
                return Err(ProviderError::Unknown(
                    "mufg issue bundle failed structural validation".into(),
                ));
            }
            issues.push(MufgIssue {
                provider_issue_id,
                display_name,
                discovered_at: discovered_at.into(),
                source_url: DISCOVERY_URL.into(),
                structural_fingerprint: ISSUE_FINGERPRINT.into(),
            });
        }
        if issues.is_empty() {
            return Err(ProviderError::Unknown(
                "mufg issue bundle contained no recognized records".into(),
            ));
        }
        Ok(issues)
    }

    /// Extract the server-generated request token from the bootstrap
    /// response. Exactly one non-empty `hidToken` value is accepted;
    /// missing, empty, or duplicated tokens are structural drift.
    pub fn parse_token_response(body: &str) -> Result<String, ProviderError> {
        let normalized = body.to_ascii_lowercase();
        let mut token = None;
        let mut offset = 0;
        while let Some(relative_start) = normalized[offset..].find("<input") {
            let start = offset + relative_start;
            let after_name = normalized.as_bytes().get(start + "<input".len()).copied();
            if !after_name
                .is_some_and(|byte| byte.is_ascii_whitespace() || byte == b'>' || byte == b'/')
            {
                offset = start + "<input".len();
                continue;
            }
            let end = start
                + normalized[start..]
                    .find('>')
                    .ok_or_else(|| ProviderError::Unknown("mufg token field changed".into()))?;
            let element = &body[start..=end];
            let id = html_attribute(element, "id")?;
            let name = html_attribute(element, "name")?;
            if id.is_some_and(|value| value.eq_ignore_ascii_case("hidToken"))
                || name.is_some_and(|value| value.eq_ignore_ascii_case("hidToken"))
            {
                let extracted = html_attribute(element, "value")?
                    .ok_or_else(|| ProviderError::Unknown("mufg token field changed".into()))?;
                if token.is_some() {
                    return Err(ProviderError::Unknown(
                        "mufg token response contains multiple tokens".into(),
                    ));
                }
                if extracted.is_empty() {
                    return Err(ProviderError::Unknown("mufg token is empty".into()));
                }
                token = Some(extracted.to_owned());
            }
            offset = end + 1;
        }
        token.ok_or_else(|| ProviderError::Unknown("mufg token missing".into()))
    }

    /// Current MUFG token endpoint contract: ASP.NET JSON wrapper with one
    /// short server token in `d`. No HTML token field is involved.
    pub fn parse_generated_token(body: &str) -> Result<String, ProviderError> {
        let token = xml_payload(body)?;
        if token.is_empty()
            || token.len() > 128
            || !token.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(ProviderError::Unknown(
                "mufg generated token failed structural validation".into(),
            ));
        }
        Ok(token)
    }

    /// Match the public page's CryptoJS AES-128-CBC/PKCS#7 token transform.
    pub fn encrypt_request_token(token: &str) -> Result<String, ProviderError> {
        if token.is_empty() || token.len() > 128 {
            return Err(ProviderError::Unknown(
                "mufg generated token failed structural validation".into(),
            ));
        }
        let mut buffer = vec![0_u8; token.len() + 16];
        buffer[..token.len()].copy_from_slice(token.as_bytes());
        let encrypted = cbc::Encryptor::<Aes128>::new(TOKEN_KEY.into(), TOKEN_IV.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buffer, token.len())
            .map_err(|_| ProviderError::Unknown("mufg token encryption failed".into()))?;
        Ok(BASE64.encode(encrypted))
    }

    /// Build the synthetic lookup request envelope. The body carries only
    /// synthetic placeholder values — the real PAN enters exclusively
    /// through the future transport slice via `with_pan`, never here.
    pub fn build_lookup_request(
        session: &MufgEphemeralSession,
        issue_code: &str,
        now: &str,
    ) -> Result<MufgLookupRequest, ProviderError> {
        if session.is_expired(now) {
            return Err(ProviderError::Retryable(
                "mufg session must be refreshed before lookup".into(),
            ));
        }
        if issue_code.is_empty() || !issue_code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ProviderError::Unknown(
                "mufg lookup issue code is invalid".into(),
            ));
        }
        Ok(MufgLookupRequest {
            endpoint: LOOKUP_URL.to_owned(),
            method: "POST".to_owned(),
            content_type: "application/json; charset=utf-8".to_owned(),
            headers: vec![
                "Cookie: [REDACTED]".to_owned(),
                "Content-Type: application/json; charset=utf-8".to_owned(),
                "Accept: application/json".to_owned(),
            ],
            body: serde_json::json!({
                "clientid": issue_code,
                "PAN": "[SYNTHETIC_LOOKUP]",
                "IFSC": "",
                "CHKVAL": "1",
                "token": session.request_token.as_str(),
            })
            .to_string(),
        })
    }
}

/// Narrow lookup-request DTO. It deliberately does not implement serde:
/// outbound encoding belongs at the transport boundary, not persisted state.
#[derive(Clone, PartialEq, Eq)]
pub struct MufgLookupRequest {
    pub endpoint: String,
    pub method: String,
    pub content_type: String,
    pub headers: Vec<String>,
    pub body: String,
}

impl MufgLookupRequest {
    /// Redacted debug rendering for logs/tests. Never includes the raw
    /// cookie, token, or PAN value.
    pub fn debug(&self) -> String {
        format!(
            "MufgLookupRequest {{ endpoint: {}, method: {}, body: [SYNTHETIC_LOOKUP] }}",
            self.endpoint, self.method
        )
    }
}

impl MufgIntimeProvider {
    /// Classify CAPTCHA activation from the rendered page markup. Markup
    /// hidden with `display:none` is `Dormant`; a visible CAPTCHA container
    /// is `Required`; no CAPTCHA references at all is `Absent`; markup
    /// present but visibility undecidable is `Unknown` (fail safe).
    pub fn captcha_state(page: &str) -> Result<MufgCaptchaState, ProviderError> {
        let document;
        let page = if page.trim_start().starts_with('{') {
            document = xml_payload(page)?;
            document.as_str()
        } else {
            page
        };
        let normalized = page.to_ascii_lowercase();
        let evidence = captcha_element_evidence(&normalized)?;
        if evidence.is_empty() {
            // A textual reference without the recognized container is
            // ambiguous: fail safe rather than silently Absent.
            return Ok(if normalized.contains("captch") {
                MufgCaptchaState::Unknown
            } else {
                MufgCaptchaState::Absent
            });
        }

        if evidence.contains(&CaptchaEvidence::Unknown) {
            return Ok(MufgCaptchaState::Unknown);
        }
        Ok(if evidence.contains(&CaptchaEvidence::Visible) {
            MufgCaptchaState::Required
        } else {
            MufgCaptchaState::Dormant
        })
    }

    /// Normalize a JSON-wrapped XML result body. Reads only `ALLOT`/`SHARES`
    /// from the record row and `Message` from the operational row. Every
    /// unrecognized shape fails closed to `Unknown` — the financial outcome
    /// is never guessed. Token/session/captcha/ratelimit message rows map
    /// to typed operational errors, never financial statuses.
    pub fn parse_result_body(
        body: &str,
        issue_confirmed: bool,
        checked_at: &str,
        contract_fingerprint: &str,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if contract_fingerprint != RESULT_FINGERPRINT {
            return Err(ProviderError::Unknown(
                "mufg result fingerprint is not accepted".into(),
            ));
        }

        let payload = xml_payload(body)?;

        // Operational message rows (Table1) take precedence when present.
        if let Some((message, _)) = tagged_value(&payload, "Message") {
            return Err(match message.to_ascii_uppercase().as_str() {
                "SESSION EXPIRED" | "INVALID REQUEST TOKEN" | "TOKEN EXPIRED" => {
                    ProviderError::Retryable("mufg session or token must be refreshed".into())
                }
                m if m.contains("CAPTCHA") => {
                    ProviderError::NeedsHuman("mufg captcha activation".into())
                }
                m if m.contains("NO RECORD") => {
                    // Structurally recognized provider no-record phrase takes
                    // precedence over generic retry wording in the same row.
                    return Ok(ProviderAllotmentResult::not_found(
                        checked_at,
                        contract_fingerprint,
                    ));
                }
                m if m.contains("RATE") || m.contains("TRY LATER") => ProviderError::RateLimited,
                m if m.contains("UNAVAILABLE") || m.contains("TRY AGAIN") => {
                    ProviderError::Unavailable("mufg service reported unavailable".into())
                }
                _ => ProviderError::Unknown("mufg message row is not recognized".into()),
            });
        }

        // Result record row: require the full structural field set.
        for field in [
            "PEMNDG",
            "NAME1",
            "SHARES",
            "offer_price",
            "ALLOT",
            "AMTADJ",
            "RFNDAMT",
        ] {
            let Some(_) = tagged_value(&payload, field) else {
                return Err(ProviderError::Unknown(
                    "mufg result record is incomplete".into(),
                ));
            };
        }

        let (_, after_shares) = tagged_value(&payload, "SHARES")
            .ok_or_else(|| ProviderError::Unknown("mufg result record is incomplete".into()))?;
        let (allot_raw, _) = tagged_value(after_shares, "ALLOT")
            .ok_or_else(|| ProviderError::Unknown("mufg result record is incomplete".into()))?;
        if allot_raw.trim().is_empty() {
            return Ok(ProviderAllotmentResult::pending(
                checked_at,
                contract_fingerprint,
            ));
        }
        let allotted_shares = allot_raw
            .trim()
            .parse::<u64>()
            .map_err(|_| ProviderError::Unknown("mufg allotted shares were not numeric".into()))?;

        if allotted_shares == 0 {
            let proof = NegativeResultProof::new(true, issue_confirmed, true, true, true)
                .map_err(|_| ProviderError::Unknown("mufg negative proof failed".into()))?;
            Ok(ProviderAllotmentResult::confirmed_not_allotted(
                proof,
                checked_at,
                contract_fingerprint,
            ))
        } else {
            let proof = PositiveResultProof::new(true, true, true, true)?;
            ProviderAllotmentResult::confirmed_allotted(
                proof,
                allotted_shares,
                None,
                None,
                checked_at,
                contract_fingerprint,
            )
        }
    }

    /// Public, identifier-free contract/session precheck. The fresh cookie jar
    /// and both token forms are dropped in memory before this returns.
    pub fn identifier_free_precheck(
        &self,
        issue_code: &str,
        expected_name: &str,
    ) -> Result<MufgPublicPrecheck, ProviderError> {
        let session = self.bootstrap_live_session(Some(issue_code), expected_name)?;
        let observed_at = session.issue.discovered_at.clone();
        Ok(MufgPublicPrecheck {
            issue: session.issue,
            captcha_state: session.captcha_state,
            tls_http_healthy: true,
            session_bootstrap_succeeded: true,
            request_token_obtained: !session.encrypted_token.is_empty(),
            lookup_endpoint: LOOKUP_URL,
            observed_at,
        })
    }

    fn bootstrap_live_session(
        &self,
        issue_code: Option<&str>,
        expected_name: &str,
    ) -> Result<MufgLiveSession, ProviderError> {
        if !self.network_enabled {
            return Err(ProviderError::Retryable(
                "mufg network is disabled for this provider".into(),
            ));
        }
        let observed_at = now_rfc3339()?;
        let http = SharedHttpClient::isolated_no_redirects();
        let page = http
            .get(
                DISCOVERY_URL,
                ALLOWED_HOSTS,
                MAX_RESPONSE_BYTES,
                TIMEOUT_SECS,
            )
            .map_err(map_http_error)?;
        let captcha_state = Self::captcha_state(&page)?;

        wait_http_backstop();
        let token_response = http
            .post_json(
                TOKEN_URL,
                ALLOWED_HOSTS,
                "{}",
                MAX_RESPONSE_BYTES,
                TIMEOUT_SECS,
            )
            .map_err(map_http_error)?;
        if !http.has_cookie("in.mpms.mufg.com", "/", "ASP.NET_SessionId") {
            return Err(ProviderError::Unknown(
                "mufg session bootstrap did not establish the required cookie".into(),
            ));
        }
        let raw_token = Zeroizing::new(Self::parse_generated_token(&token_response)?);
        let encrypted_token = Zeroizing::new(Self::encrypt_request_token(&raw_token)?);

        wait_http_backstop();
        let issue_bundle = http
            .post_json(
                ISSUES_URL,
                ALLOWED_HOSTS,
                "{}",
                MAX_RESPONSE_BYTES,
                TIMEOUT_SECS,
            )
            .map_err(map_http_error)?;
        let issues = Self::parse_issue_bundle(&issue_bundle, &observed_at)?;
        let issue = resolve_issue(issues, issue_code, expected_name)?;

        Ok(MufgLiveSession {
            http,
            issue,
            captcha_state,
            encrypted_token,
        })
    }

    fn execute_live_lookup(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        let session = self
            .bootstrap_live_session(context.issue.issue_code.as_deref(), &context.issue.ipo_name)?;
        match session.captcha_state {
            MufgCaptchaState::Absent | MufgCaptchaState::Dormant => {}
            MufgCaptchaState::Required => {
                return Err(ProviderError::NeedsHuman(
                    "mufg requires owner-completed human verification".into(),
                ));
            }
            MufgCaptchaState::Unknown => {
                return Err(ProviderError::Unknown(
                    "mufg captcha state is ambiguous".into(),
                ));
            }
        }

        let payload = LiveLookupPayload {
            client_id: &session.issue.provider_issue_id,
            lookup: pan.as_normalized(),
            ifsc: "",
            check_value: "1",
            token: &session.encrypted_token,
        };
        let body = Zeroizing::new(
            serde_json::to_string(&payload)
                .map_err(|_| ProviderError::Unknown("mufg request encoding failed".into()))?,
        );
        wait_http_backstop();
        let response = session
            .http
            .post_json(
                LOOKUP_URL,
                ALLOWED_HOSTS,
                &body,
                MAX_RESPONSE_BYTES,
                TIMEOUT_SECS,
            )
            .map_err(map_http_error)?;
        Self::parse_result_body(&response, true, &now_rfc3339()?, RESULT_FINGERPRINT)
    }
}

#[derive(Serialize)]
struct LiveLookupPayload<'a> {
    #[serde(rename = "clientid")]
    client_id: &'a str,
    #[serde(rename = "PAN")]
    lookup: &'a str,
    #[serde(rename = "IFSC")]
    ifsc: &'a str,
    #[serde(rename = "CHKVAL")]
    check_value: &'a str,
    token: &'a str,
}

fn resolve_issue(
    issues: Vec<MufgIssue>,
    issue_code: Option<&str>,
    expected_name: &str,
) -> Result<MufgIssue, ProviderError> {
    if expected_name.trim().is_empty()
        || issue_code
            .is_some_and(|code| code.is_empty() || !code.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(ProviderError::Unknown(
            "mufg issue selector is invalid".into(),
        ));
    }
    let expected_name = normalized_issue_name(expected_name);
    let mut matches = issues.into_iter().filter(|issue| {
        issue_code.is_none_or(|code| issue.provider_issue_id == code)
            && normalized_issue_name(&issue.display_name) == expected_name
    });
    let Some(issue) = matches.next() else {
        return Err(ProviderError::Unavailable(
            "mufg requested issue is not available".into(),
        ));
    };
    if matches.next().is_some() {
        return Err(ProviderError::Unknown(
            "mufg issue selector was ambiguous".into(),
        ));
    }
    Ok(issue)
}

fn normalized_issue_name(name: &str) -> String {
    let normalized = name.trim().to_ascii_lowercase();
    normalized
        .strip_suffix(" - ipo")
        .unwrap_or(&normalized)
        .trim()
        .to_owned()
}

fn now_rfc3339() -> Result<String, ProviderError> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| ProviderError::Unknown("mufg observation time failed".into()))
}

fn wait_http_backstop() {
    std::thread::sleep(std::time::Duration::from_millis(510));
}

fn map_http_error(error: HttpPolicyError) -> ProviderError {
    match error {
        HttpPolicyError::RateLimited | HttpPolicyError::Status(429) => ProviderError::RateLimited,
        HttpPolicyError::Status(408 | 500..=599) | HttpPolicyError::Transport(_) => {
            ProviderError::Unavailable("mufg transport is unavailable".into())
        }
        HttpPolicyError::InvalidUrl
        | HttpPolicyError::HostNotAllowed { .. }
        | HttpPolicyError::RedirectNotAllowed { .. }
        | HttpPolicyError::TooManyRedirects
        | HttpPolicyError::SizeCapExceeded(_)
        | HttpPolicyError::Status(_)
        | HttpPolicyError::UnexpectedContentType(_) => {
            ProviderError::Unknown("mufg transport policy rejected the response".into())
        }
    }
}

impl Default for MufgIntimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AllotmentProvider for MufgIntimeProvider {
    fn provider_id(&self) -> &'static str {
        "mufg-intime-live"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            issue_discovery: IssueDiscoveryMode::PublicHttp,
            lookup_keys: vec![
                LookupKeyKind::Pan,
                LookupKeyKind::ApplicationNumber,
                LookupKeyKind::DematAccount,
                LookupKeyKind::BankAccountAndIfsc,
            ],
            session: SessionRequirement::CookieAndRequestToken,
            human_verification: HumanVerificationRequirement::Conditional,
            transport: ProviderTransportKind::Hybrid,
            background: BackgroundExecution::PrepareOnly,
        }
    }

    fn health(&self) -> ProviderHealth {
        if self.network_enabled && crate::LIVE_TRANSPORT_IMPLEMENTED {
            ProviderHealth::Available
        } else {
            ProviderHealth::Degraded
        }
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        issue.registrar_id.to_ascii_lowercase().contains("mufg")
            || issue.registrar_id.to_ascii_lowercase().contains("intime")
    }

    fn prepare_lookup(
        &self,
        context: &AllotmentLookupContext,
    ) -> Result<Option<ProviderAllotmentResult>, ProviderError> {
        let session = self
            .bootstrap_live_session(context.issue.issue_code.as_deref(), &context.issue.ipo_name)?;
        match session.captcha_state {
            MufgCaptchaState::Absent | MufgCaptchaState::Dormant => Ok(None),
            MufgCaptchaState::Required => Err(ProviderError::NeedsHuman(
                "mufg requires owner-completed human verification".into(),
            )),
            MufgCaptchaState::Unknown => Err(ProviderError::Unknown(
                "mufg captcha state is ambiguous".into(),
            )),
        }
    }

    fn check_allotment(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if !crate::REAL_INVESTOR_LOOKUP_AUTHORIZED {
            return Err(ProviderError::Retryable(
                "mufg real investor lookup is not authorized".into(),
            ));
        }
        self.execute_live_lookup(context, pan)
    }
}

/// Extract the inner XML payload from the ASP.NET JSON wrapper `{"d": "..."}`.
/// Any wrapper change fails closed.
fn xml_payload(body: &str) -> Result<String, ProviderError> {
    let document: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| ProviderError::Unknown("mufg result was not valid JSON".into()))?;
    let payload = document
        .as_object()
        .and_then(|object| object.get("d"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ProviderError::Unknown("mufg result wrapper changed".into()))?;
    Ok(payload.to_owned())
}

/// Read one quoted HTML attribute from a single opening tag. Attribute names
/// are ASCII-case-insensitive; malformed quoting fails closed.
fn html_attribute<'a>(tag: &'a str, name: &str) -> Result<Option<&'a str>, ProviderError> {
    let normalized = tag.to_ascii_lowercase();
    let mut offset = 0;
    while let Some(relative_start) = normalized[offset..].find(name) {
        let start = offset + relative_start;
        let boundary = start == 0
            || normalized.as_bytes()[start - 1].is_ascii_whitespace()
            || normalized.as_bytes()[start - 1] == b'<';
        if !boundary {
            offset = start + name.len();
            continue;
        }

        let mut cursor = start + name.len();
        while normalized
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        if normalized.as_bytes().get(cursor) != Some(&b'=') {
            return Err(ProviderError::Unknown(
                "mufg token attribute changed".into(),
            ));
        }
        cursor += 1;
        while normalized
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        let quote = *normalized
            .as_bytes()
            .get(cursor)
            .filter(|quote| **quote == b'\'' || **quote == b'"')
            .ok_or_else(|| ProviderError::Unknown("mufg token attribute changed".into()))?;
        let value_start = cursor + 1;
        let value_end = normalized[value_start..]
            .find(char::from(quote))
            .map(|relative_end| value_start + relative_end)
            .ok_or_else(|| ProviderError::Unknown("mufg token attribute changed".into()))?;
        return Ok(Some(&tag[value_start..value_end]));
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptchaEvidence {
    Hidden,
    Visible,
    Unknown,
}

/// Classify actual CAPTCHA elements only; script-string references are not DOM
/// elements. The current MUFG contract puts visibility on a bounded ancestor
/// `.paddingcnd` div rather than the marker's immediate child container.
fn captcha_element_evidence(page: &str) -> Result<Vec<CaptchaEvidence>, ProviderError> {
    let mut stack: Vec<(&str, &str)> = Vec::new();
    let mut evidence = Vec::new();
    let mut cursor = 0;

    while let Some(relative_start) = page[cursor..].find('<') {
        let start = cursor + relative_start;
        if page[start..].starts_with("<!--") {
            let end = page[start + 4..]
                .find("-->")
                .map(|relative| start + 4 + relative + 3)
                .ok_or_else(|| ProviderError::Unknown("mufg captcha markup changed".into()))?;
            cursor = end;
            continue;
        }
        let end = start
            + page[start..]
                .find('>')
                .ok_or_else(|| ProviderError::Unknown("mufg captcha markup changed".into()))?;
        let tag = &page[start..=end];
        let Some((name, closing)) = html_tag_name(tag) else {
            cursor = end + 1;
            continue;
        };

        if name == "script" && !closing {
            let script_end = page[end + 1..]
                .find("</script>")
                .map(|relative| end + 1 + relative + "</script>".len())
                .ok_or_else(|| ProviderError::Unknown("mufg captcha script changed".into()))?;
            cursor = script_end;
            continue;
        }

        if closing {
            if matches!(name, "div" | "section") {
                match stack.pop() {
                    Some((open_name, _)) if open_name == name => {}
                    _ => return Ok(vec![CaptchaEvidence::Unknown]),
                }
            }
        } else {
            if is_captcha_element(name, tag)? {
                evidence.push(captcha_ancestor_evidence(&stack)?);
            }
            if matches!(name, "div" | "section") && !tag.ends_with("/>") {
                stack.push((name, tag));
            }
        }
        cursor = end + 1;
    }

    if !evidence.is_empty() && !stack.is_empty() {
        return Ok(vec![CaptchaEvidence::Unknown]);
    }
    Ok(evidence)
}

fn html_tag_name(tag: &str) -> Option<(&str, bool)> {
    let bytes = tag.as_bytes();
    if bytes.first() != Some(&b'<') {
        return None;
    }
    let closing = bytes.get(1) == Some(&b'/');
    let start = if closing { 2 } else { 1 };
    let end = tag[start..].find(|character: char| {
        character.is_ascii_whitespace() || matches!(character, '/' | '>')
    })? + start;
    (end > start).then_some((&tag[start..end], closing))
}

fn is_captcha_element(name: &str, tag: &str) -> Result<bool, ProviderError> {
    let id = html_attribute(tag, "id")?;
    let src = html_attribute(tag, "src")?;
    Ok((name == "input" && id == Some("txtcaptch"))
        || (name == "img"
            && (matches!(id, Some("img_cap" | "cimage"))
                || src.is_some_and(|value| value.starts_with("cimage.aspx")))))
}

fn captcha_ancestor_evidence(stack: &[(&str, &str)]) -> Result<CaptchaEvidence, ProviderError> {
    for (name, tag) in stack.iter().rev() {
        let id = html_attribute(tag, "id")?.unwrap_or("");
        let class = html_attribute(tag, "class")?.unwrap_or("");
        if class
            .split_ascii_whitespace()
            .any(|value| value == "paddingcnd")
            || id.contains("captcha")
            || class.contains("captcha")
        {
            if *name != "div" {
                return Ok(CaptchaEvidence::Unknown);
            }
            return visibility_evidence(tag, class.contains("paddingcnd"));
        }
    }

    let Some((name, tag)) = stack.iter().rev().find(|(name, _)| *name == "div") else {
        return Ok(CaptchaEvidence::Unknown);
    };
    debug_assert_eq!(*name, "div");
    visibility_evidence(tag, false)
}

fn visibility_evidence(tag: &str, current_wrapper: bool) -> Result<CaptchaEvidence, ProviderError> {
    let style = html_attribute(tag, "style")?.unwrap_or("");
    let compact: String = style
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if compact.contains("display:none") {
        Ok(CaptchaEvidence::Hidden)
    } else if compact.contains("display:block")
        || compact.contains("display:inline")
        || compact.contains("display:flex")
        || compact.contains("display:grid")
        || (!current_wrapper && compact.is_empty())
    {
        Ok(CaptchaEvidence::Visible)
    } else {
        Ok(CaptchaEvidence::Unknown)
    }
}

/// Extract the value inside `<tag>value</tag>` starting at the front of
/// `text`; returns the remainder after the closing tag.
fn tagged_value<'a>(text: &'a str, tag: &str) -> Option<(String, &'a str)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text.find(&close)?;
    if end < start {
        return None;
    }
    let value = &text[start..end];
    if value.contains('<') {
        // Nested markup is not a recognized value shape.
        return None;
    }
    Some((value.trim().to_owned(), &text[end + close.len()..]))
}

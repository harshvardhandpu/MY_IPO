//! MUFG Intime India provider — session/token registrar adapter.
//!
//! Gate 2 verified the public flow: identifier-free discovery
//! (`IPO.aspx/GetDetails`), a session cookie plus server-generated request
//! token (`IPO.aspx/generateToken`, stored as `hidToken`), conditional
//! CAPTCHA (markup present but dormant during research), and a JSON-wrapped
//! XML result contract. No real investor lookup was submitted.
//!
//! The unattended `check_allotment` path fails closed until the live
//! session/token transport slice wires it: token failures, session expiry,
//! and CAPTCHA activation map to operational states — never a guessed
//! financial result. `MufgEphemeralSession` holds cookie/token values only
//! in memory with a redacted `Debug` impl; they are never serialized,
//! logged, or persisted. The parser reads only `company_id`/`companyname`
//! (discovery) and `ALLOT`/`SHARES` (results); member fields (PEMNDG,
//! NAME1) are required for structure but never copied.

use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, HumanVerificationRequirement,
    IssueDiscoveryMode, LookupKeyKind, NegativeResultProof, PositiveResultProof,
    ProviderAllotmentResult, ProviderCapabilities, ProviderError, ProviderHealth,
    ProviderTransportKind, RegistrarIssue, SessionRequirement,
};

const DISCOVERY_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/public-issues.html";
const LOOKUP_URL: &str = "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/SearchOnPan";
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

pub struct MufgIntimeProvider;

impl MufgIntimeProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn offline_for_tests() -> Self {
        Self
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
                break;
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
        let mut token = None;
        let mut rest = body;
        while let Some(at) = rest.find("hidToken") {
            rest = &rest[at..];
            // Scan to the closing quote of the value attribute.
            let value_open = rest
                .find("value=\"")
                .ok_or_else(|| ProviderError::Unknown("mufg token field changed".into()))?;
            let value = &rest[value_open + "value=\"".len()..];
            let value_close = value
                .find('"')
                .ok_or_else(|| ProviderError::Unknown("mufg token field changed".into()))?;
            let extracted = &value[..value_close];
            if token.is_some() {
                return Err(ProviderError::Unknown(
                    "mufg token response contains multiple tokens".into(),
                ));
            }
            if extracted.is_empty() {
                return Err(ProviderError::Unknown("mufg token is empty".into()));
            }
            token = Some(extracted.to_owned());
            rest = &value[value_close..];
        }
        token.ok_or_else(|| ProviderError::Unknown("mufg token missing".into()))
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
            content_type: "application/json".to_owned(),
            headers: vec![
                "__RequestVerificationToken: [REDACTED]".to_owned(),
                "Content-Type: application/json".to_owned(),
            ],
            body: serde_json::json!({
                "companyId": issue_code,
                "searchText": "[SYNTHETIC_LOOKUP]",
                "searchMode": "PAN",
                "requestToken": "[REDACTED]",
            })
            .to_string(),
        })
    }
}

/// Narrow lookup-request DTO. No member identity fields are persisted; the
/// token header value is redacted in Debug and never logged.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        let has_captcha_container = page.contains("CImage") || page.contains("txtCaptch");
        if !has_captcha_container {
            // A textual reference without the recognized container is
            // ambiguous: fail safe rather than silently Absent.
            let mentions_captcha = page.contains("captch");
            return Ok(if mentions_captcha {
                MufgCaptchaState::Unknown
            } else {
                MufgCaptchaState::Absent
            });
        }
        let has_hidden = page.contains("display:none") || page.contains("display: none");
        if has_hidden {
            Ok(MufgCaptchaState::Dormant)
        } else {
            Ok(MufgCaptchaState::Required)
        }
        // ponytail: visibility classification covers the observed markup
        // shapes; richer detection (class-based hiding) upgrades if MUFG
        // changes its markup.
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
                m if m.contains("RATE") || m.contains("TRY LATER") => ProviderError::RateLimited,
                m if m.contains("UNAVAILABLE") || m.contains("TRY AGAIN") => {
                    ProviderError::Unavailable("mufg service reported unavailable".into())
                }
                m if m.contains("NO RECORD") => {
                    // Structurally recognized no-record phrase.
                    return Ok(ProviderAllotmentResult::not_found(
                        checked_at,
                        contract_fingerprint,
                    ));
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
        // The deterministic session/token transport is not yet proven
        // unattended; capability-honest reporting, not HTTP-200 optimism.
        ProviderHealth::Degraded
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        issue.registrar_id.to_ascii_lowercase().contains("mufg")
            || issue.registrar_id.to_ascii_lowercase().contains("intime")
    }

    fn check_allotment(
        &self,
        _context: &AllotmentLookupContext,
        _pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        // Fail closed: session/token transport is a later slice.
        Err(ProviderError::Retryable(
            "mufg session/token transport is not prepared".into(),
        ))
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

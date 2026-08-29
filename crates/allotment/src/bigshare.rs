//! Bigshare Services provider — human-verification-first registrar adapter.
//!
//! Gate 2 established that every new Bigshare search requires a
//! server-verified CAPTCHA. The unattended path therefore ALWAYS fails closed
//! to `NeedsHuman` — never an error afterthought, never a guessed result.
//! Structured `{"d": {...}}` results reached through the legitimate
//! user-completed challenge are normalized with the Gate 1 invariant:
//! UNKNOWN/NOTFOUND/CAPTCHA/RATELIMIT/WARMING can never become NOT_ALLOTTED.
//! The parser reads only `Status` and the numeric `ALOTED` share count; it
//! never copies member fields (APPLICATION_NO, DPID, Name) or PAN.

use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, HumanVerificationRequirement,
    IssueDiscoveryMode, LookupKeyKind, NegativeResultProof, PositiveResultProof,
    ProviderAllotmentResult, ProviderCapabilities, ProviderError, ProviderHealth,
    ProviderTransportKind, RegistrarIssue, SessionRequirement,
};
use crate::status::NormalizedAllotmentStatus;

const DISCOVERY_URL: &str = "https://ipo.bigshareonline.com/ipo_status.html";
const ISSUE_FINGERPRINT: &str = "bigshare-issues-ddlCompany-options-v1";
const RESULT_FINGERPRINT: &str = "bigshare-result-d-status-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BigshareIssue {
    pub provider_issue_id: String,
    pub display_name: String,
    pub discovered_at: String,
    pub source_url: String,
    pub structural_fingerprint: String,
}

/// CAPTCHA is mandatory for every new Bigshare search, so the adapter has no
/// unattended network mode to switch — every lookup fails closed to
/// `NeedsHuman` until the member completes the official challenge.
pub struct BigshareProvider;

impl BigshareProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn offline_for_tests() -> Self {
        Self
    }

    /// Parse the host-rendered company select (`ddlCompany`) option list.
    /// Only static `value="digits"` options with a non-empty label are
    /// recognized; anything else fails closed to Unknown.
    pub fn parse_issue_bundle(
        bundle: &str,
        discovered_at: &str,
    ) -> Result<Vec<BigshareIssue>, ProviderError> {
        let mut issues: Vec<BigshareIssue> = Vec::new();
        let mut rest = bundle;
        while let Some(start) = rest.find("<option") {
            rest = &rest[start + "<option".len()..];
            let Some((provider_issue_id, label_and_tail)) = option_value(rest) else {
                continue;
            };
            let Some((display_name, tail)) = option_label(label_and_tail) else {
                continue;
            };
            rest = tail;

            if provider_issue_id.len() > 32
                || !provider_issue_id.bytes().all(|byte| byte.is_ascii_digit())
                || display_name.is_empty()
                || display_name.len() > 200
                || issues
                    .iter()
                    .any(|issue: &BigshareIssue| issue.provider_issue_id == provider_issue_id)
            {
                return Err(ProviderError::Unknown(
                    "bigshare issue bundle failed structural validation".into(),
                ));
            }
            issues.push(BigshareIssue {
                provider_issue_id,
                display_name,
                discovered_at: discovered_at.into(),
                source_url: DISCOVERY_URL.into(),
                structural_fingerprint: ISSUE_FINGERPRINT.into(),
            });
        }
        if issues.is_empty() {
            return Err(ProviderError::Unknown(
                "bigshare issue bundle contained no recognized records".into(),
            ));
        }
        Ok(issues)
    }

    /// Normalize an ASP.NET-wrapped `{"d": {...}}` structured result body.
    /// Recognized `Status` values map to typed operational states; every
    /// unrecognized shape fails closed to Unknown — the financial outcome is
    /// never guessed.
    pub fn parse_result_body(
        body: &str,
        issue_confirmed: bool,
        checked_at: &str,
        contract_fingerprint: &str,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if contract_fingerprint != RESULT_FINGERPRINT {
            return Err(ProviderError::Unknown(
                "bigshare result fingerprint is not accepted".into(),
            ));
        }

        let document: serde_json::Value = serde_json::from_str(body)
            .map_err(|_| ProviderError::Unknown("bigshare result was not valid JSON".into()))?;
        let wrapper = document
            .as_object()
            .and_then(|object| object.get("d"))
            .ok_or_else(|| ProviderError::Unknown("bigshare result wrapper changed".into()))?;

        let status = wrapper
            .get("Status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ProviderError::Unknown("bigshare result status missing".into()))?;

        match status {
            "NOTFOUND" => Ok(ProviderAllotmentResult::not_found(
                checked_at,
                contract_fingerprint,
            )),
            "CAPTCHA" => ProviderAllotmentResult::operational(
                NormalizedAllotmentStatus::NeedsHumanVerification,
                checked_at,
                contract_fingerprint,
            )
            .ok_or_else(|| ProviderError::Unknown("bigshare captcha mapping failed".into())),
            "RATELIMIT" => ProviderAllotmentResult::operational(
                NormalizedAllotmentStatus::RateLimited,
                checked_at,
                contract_fingerprint,
            )
            .ok_or_else(|| ProviderError::Unknown("bigshare ratelimit mapping failed".into())),
            "WARMING" => ProviderAllotmentResult::operational(
                NormalizedAllotmentStatus::RetryableError,
                checked_at,
                contract_fingerprint,
            )
            .ok_or_else(|| ProviderError::Unknown("bigshare warming mapping failed".into())),
            "OK" => {
                Self::parse_ok_record(wrapper, issue_confirmed, checked_at, contract_fingerprint)
            }
            _ => Err(ProviderError::Unknown(
                "bigshare result status is not recognized".into(),
            )),
        }
    }

    fn parse_ok_record(
        wrapper: &serde_json::Value,
        issue_confirmed: bool,
        checked_at: &str,
        contract_fingerprint: &str,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        // Structure first: every required OK-record field must be present.
        let record = wrapper
            .as_object()
            .ok_or_else(|| ProviderError::Unknown("bigshare OK record changed".into()))?;
        for field in ["APPLICATION_NO", "DPID", "Name", "APPLIED", "ALOTED"] {
            if !record.contains_key(field) {
                return Err(ProviderError::Unknown(
                    "bigshare OK record is incomplete".into(),
                ));
            }
        }
        let match_count = record
            .get("MatchCount")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                ProviderError::Unknown("bigshare match count missing or non-numeric".into())
            })?;
        if match_count != 1 {
            return Err(ProviderError::Unknown(
                "bigshare result was empty or ambiguous".into(),
            ));
        }

        let allotted = record.get("ALOTED");
        if allotted.map(serde_json::Value::is_null).unwrap_or(false) {
            return Ok(ProviderAllotmentResult::pending(
                checked_at,
                contract_fingerprint,
            ));
        }
        let allotted_shares = allotted
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                ProviderError::Unknown("bigshare allotted shares were not numeric".into())
            })?;

        if allotted_shares == 0 {
            let proof = NegativeResultProof::new(true, issue_confirmed, true, true, true)
                .map_err(|_| ProviderError::Unknown("bigshare negative proof failed".into()))?;
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

impl Default for BigshareProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AllotmentProvider for BigshareProvider {
    fn provider_id(&self) -> &'static str {
        "bigshare-live"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            issue_discovery: IssueDiscoveryMode::PublicHttp,
            lookup_keys: vec![
                LookupKeyKind::Pan,
                LookupKeyKind::ApplicationNumber,
                LookupKeyKind::DematAccount,
            ],
            session: SessionRequirement::ChallengeToken,
            human_verification: HumanVerificationRequirement::Required,
            transport: ProviderTransportKind::Browser,
            background: BackgroundExecution::PrepareOnly,
        }
    }

    fn health(&self) -> ProviderHealth {
        // CAPTCHA is mandatory for every new search — the provider is always
        // in the human-verification-required state until a challenge is
        // legitimately completed through the isolated verification surface.
        ProviderHealth::HumanVerificationRequired
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        issue.registrar_id.to_ascii_lowercase().contains("bigshare")
    }

    fn prepare_lookup(
        &self,
        _context: &AllotmentLookupContext,
    ) -> Result<Option<ProviderAllotmentResult>, ProviderError> {
        Err(ProviderError::NeedsHuman(
            "bigshare portal requires official human verification".into(),
        ))
    }

    fn check_allotment(
        &self,
        _context: &AllotmentLookupContext,
        _pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        // First-class fail-closed state: a human must complete the official
        // challenge in the isolated verification surface. The adapter never
        // submits lookups, solves CAPTCHA, or guesses results unattended.
        Err(ProviderError::NeedsHuman(
            "bigshare portal requires the member to complete the CAPTCHA in the isolated verification surface".into(),
        ))
    }
}

/// Extract `value="..."` from the text following an `<option` tag. Empty
/// values (the placeholder option) are not recognized.
fn option_value(text: &str) -> Option<(String, &str)> {
    let start = text.find("value=\"")?;
    let tail = &text[start + "value=\"".len()..];
    let end = tail.find('"')?;
    if end == 0 {
        return None;
    }
    Some((tail[..end].to_owned(), &tail[end + 1..]))
}

/// Extract the `>Label</option>` text of an option tag.
fn option_label(text: &str) -> Option<(String, &str)> {
    let start = text.find('>')?;
    let tail = &text[start + 1..];
    let end = tail.find("</option>")?;
    Some((tail[..end].trim().to_owned(), &tail[end + 1..]))
}

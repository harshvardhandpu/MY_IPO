use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, HumanVerificationRequirement,
    IssueDiscoveryMode, LookupKeyKind, NegativeResultProof, PositiveResultProof,
    ProviderAllotmentResult, ProviderCapabilities, ProviderError, ProviderHealth,
    ProviderTransportKind, RegistrarIssue, SessionRequirement,
};

const OFFICIAL_STATUS_URL: &str = "https://ipostatus.kfintech.com";
const ISSUE_FINGERPRINT: &str = "kfin-issues-clientId-name-v1";
const RESULT_FINGERPRINT: &str = "kfin-result-data-array-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KfintechIssue {
    pub provider_issue_id: String,
    pub display_name: String,
    pub discovered_at: String,
    pub source_url: String,
    pub structural_fingerprint: String,
}

pub struct KfintechProvider {
    network_enabled: bool,
    force_human_verification: bool,
}

impl KfintechProvider {
    pub fn new() -> Self {
        Self {
            network_enabled: true,
            force_human_verification: false,
        }
    }

    pub fn offline_for_tests() -> Self {
        Self {
            network_enabled: false,
            force_human_verification: false,
        }
    }

    pub fn human_gate_for_tests() -> Self {
        Self {
            network_enabled: false,
            force_human_verification: true,
        }
    }

    pub fn parse_issue_bundle(
        bundle: &str,
        discovered_at: &str,
    ) -> Result<Vec<KfintechIssue>, ProviderError> {
        let mut issues = Vec::new();
        let mut rest = bundle;
        while let Some(start) = rest.find("clientId") {
            rest = &rest[start + "clientId".len()..];
            let Some((provider_issue_id, after_id)) = js_string_after_colon(rest) else {
                continue;
            };
            let Some(name_start) = after_id.find("name") else {
                continue;
            };
            let Some((display_name, after_name)) =
                js_string_after_colon(&after_id[name_start + "name".len()..])
            else {
                continue;
            };
            rest = after_name;

            if provider_issue_id.len() > 32
                || !provider_issue_id.bytes().all(|byte| byte.is_ascii_digit())
                || display_name.is_empty()
                || display_name.len() > 200
                || issues
                    .iter()
                    .any(|issue: &KfintechIssue| issue.provider_issue_id == provider_issue_id)
            {
                return Err(ProviderError::Unknown(
                    "kfintech issue bundle failed structural validation".into(),
                ));
            }
            issues.push(KfintechIssue {
                provider_issue_id,
                display_name,
                discovered_at: discovered_at.into(),
                source_url: OFFICIAL_STATUS_URL.into(),
                structural_fingerprint: ISSUE_FINGERPRINT.into(),
            });
        }
        if issues.is_empty() {
            return Err(ProviderError::Unknown(
                "kfintech issue bundle contained no recognized records".into(),
            ));
        }
        Ok(issues)
    }

    pub fn parse_result_body(
        body: &str,
        issue_confirmed: bool,
        checked_at: &str,
        contract_fingerprint: &str,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if contract_fingerprint != RESULT_FINGERPRINT {
            return Err(ProviderError::Unknown(
                "kfintech result fingerprint is not accepted".into(),
            ));
        }

        let document: serde_json::Value = serde_json::from_str(body)
            .map_err(|_| ProviderError::Unknown("kfintech result was not valid JSON".into()))?;
        let records = document
            .as_object()
            .and_then(|object| object.get("data"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ProviderError::Unknown("kfintech result wrapper changed".into()))?;
        if records.len() != 1 {
            return Err(ProviderError::Unknown(
                "kfintech result was empty or ambiguous".into(),
            ));
        }

        let record = records[0]
            .as_object()
            .ok_or_else(|| ProviderError::Unknown("kfintech result record changed".into()))?;
        for field in [
            "Appln_No",
            "Name",
            "DP_CLID",
            "Pan_No",
            "App_Shares",
            "All_Shares",
        ] {
            if !record.contains_key(field) {
                return Err(ProviderError::Unknown(
                    "kfintech result record is incomplete".into(),
                ));
            }
        }

        if record
            .get("All_Shares")
            .map(|value| value.is_null())
            .unwrap_or(false)
        {
            return Ok(ProviderAllotmentResult::pending(
                checked_at,
                contract_fingerprint,
            ));
        }

        let allotted_shares = record
            .get("All_Shares")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                ProviderError::Unknown("kfintech allotted shares were not numeric".into())
            })?;

        if allotted_shares == 0 {
            let proof = NegativeResultProof::new(true, issue_confirmed, true, true, true)
                .map_err(|_| ProviderError::Unknown("kfintech negative proof failed".into()))?;
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

impl Default for KfintechProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AllotmentProvider for KfintechProvider {
    fn provider_id(&self) -> &'static str {
        "kfintech-live"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            issue_discovery: IssueDiscoveryMode::PublicJavascript,
            lookup_keys: vec![
                LookupKeyKind::Pan,
                LookupKeyKind::ApplicationNumberAndPan,
                LookupKeyKind::DematAccount,
            ],
            session: SessionRequirement::None,
            human_verification: HumanVerificationRequirement::None,
            transport: ProviderTransportKind::Http,
            background: BackgroundExecution::Unattended,
        }
    }

    fn health(&self) -> ProviderHealth {
        if self.force_human_verification {
            ProviderHealth::HumanVerificationRequired
        } else if self.network_enabled {
            ProviderHealth::Degraded
        } else {
            ProviderHealth::Unknown
        }
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        issue.registrar_id.to_ascii_lowercase().contains("kfin")
    }

    fn check_allotment(
        &self,
        _context: &AllotmentLookupContext,
        _pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if self.force_human_verification {
            Err(ProviderError::NeedsHuman(
                "kfintech portal requires interactive verification".into(),
            ))
        } else {
            Err(ProviderError::Unknown(
                "kfintech result transport is not prepared".into(),
            ))
        }
    }
}

fn js_string_after_colon(value: &str) -> Option<(String, &str)> {
    let colon = value.find(':')?;
    let value = value[colon + 1..].trim_start();
    let quote = value.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let tail = &value[1..];
    let end = tail.find(quote as char)?;
    Some((tail[..end].trim().to_owned(), &tail[end + 1..]))
}

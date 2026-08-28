//! KFintech live provider — discovery + conservative check path.
//!
//! Live network path is best-effort. When the public portal requires JS/CAPTCHA
//! or returns unparseable content, results fail closed to UNKNOWN or
//! NEEDS_HUMAN_VERIFICATION — never silent NOT_ALLOTTED.

use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::provider::{
    AllotmentLookupContext, AllotmentProvider, ProviderAllotmentResult, ProviderError,
    ProviderHealth, RegistrarIssue,
};

const OFFICIAL_STATUS_URL: &str = "https://ipostatus.kfintech.com";
const DISCOVERY_URL: &str = "https://ipostatus.kfintech.com";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredIssue {
    pub issue_code: String,
    pub display_name: String,
    pub last_verified_at: String,
    pub source_url: String,
}

/// Live KFintech adapter. Network optional for unit tests via `network_enabled`.
pub struct LiveKfintechProvider {
    network_enabled: bool,
    /// When true, simulated live check always requests human verification
    /// (models captcha-gated portals without contacting them in CI).
    force_human_verification: bool,
}

impl LiveKfintechProvider {
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

    pub fn official_status_url() -> &'static str {
        OFFICIAL_STATUS_URL
    }

    /// Public issue discovery. Without network, returns empty list (caller may
    /// fall back to typed IPO name + manual registrar selection).
    pub fn discover_issues(&self) -> Result<Vec<DiscoveredIssue>, ProviderError> {
        if !self.network_enabled {
            return Ok(Vec::new());
        }
        // Best-effort GET of the public portal. The page is JS-heavy; we only
        // extract obvious issue tokens if present. Failure → empty, not panic.
        let body = http_get_text(DISCOVERY_URL).unwrap_or_default();
        Ok(parse_issue_candidates(&body))
    }

    pub fn availability(&self) -> ProviderHealth {
        if self.force_human_verification {
            return ProviderHealth::HumanVerificationRequired;
        }
        if !self.network_enabled {
            return ProviderHealth::Unknown;
        }
        match http_get_text(OFFICIAL_STATUS_URL) {
            Ok(body) if body_looks_like_challenge(&body) => {
                ProviderHealth::HumanVerificationRequired
            }
            Ok(_) => ProviderHealth::Available,
            Err(_) => ProviderHealth::Degraded,
        }
    }
}

impl Default for LiveKfintechProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AllotmentProvider for LiveKfintechProvider {
    fn provider_id(&self) -> &'static str {
        "kfintech-live"
    }

    fn health(&self) -> ProviderHealth {
        self.availability()
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        let id = issue.registrar_id.to_ascii_lowercase();
        id.contains("kfin")
    }

    fn check_allotment(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        // Never log pan.
        let _ = pan.as_normalized().len();

        if self.force_human_verification {
            return Err(ProviderError::NeedsHuman(
                "kfintech portal requires interactive verification".into(),
            ));
        }

        if !self.network_enabled {
            // Offline live provider cannot claim allotment outcomes.
            return Err(ProviderError::Unknown(
                "live network disabled; cannot classify allotment".into(),
            ));
        }

        // Attempt a lightweight reachability probe. Full status HTML is JS-rendered;
        // without a stable form POST contract we fail closed to human verification
        // rather than inventing NOT_ALLOTTED.
        let body = http_get_text(OFFICIAL_STATUS_URL).map_err(ProviderError::Unavailable)?;
        if body_looks_like_challenge(&body) {
            return Err(ProviderError::NeedsHuman(
                "captcha or challenge detected on kfintech status portal".into(),
            ));
        }

        // No deterministic machine-readable result without browser automation.
        Err(ProviderError::Unknown(format!(
            "kfintech live HTML for issue '{}' is not machine-parseable without browser session",
            context.issue.ipo_name
        )))
    }
}

fn body_looks_like_challenge(body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    b.contains("captcha") || b.contains("recaptcha") || b.contains("hcaptcha")
}

fn parse_issue_candidates(body: &str) -> Vec<DiscoveredIssue> {
    // Extremely conservative: look for option-like tokens; prefer empty over wrong.
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim();
        if t.len() > 8 && t.len() < 120 && t.contains("IPO") {
            out.push(DiscoveredIssue {
                issue_code: format!("guess-{}", out.len() + 1),
                display_name: t.chars().take(80).collect(),
                last_verified_at: "live-scan".into(),
                source_url: DISCOVERY_URL.into(),
            });
        }
        if out.len() >= 20 {
            break;
        }
    }
    out
}

fn http_get_text(url: &str) -> Result<String, String> {
    // ponytail: shell-out GET until we add ureq; upgrade if curl absent in CI.
    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "15",
            "-A",
            "SanketIPO/0.1 (+local; allotment-discovery)",
            url,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("curl exit {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

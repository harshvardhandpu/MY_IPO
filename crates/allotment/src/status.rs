//! Canonical allotment outcomes. Unknown never collapses to NOT_ALLOTTED.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizedAllotmentStatus {
    Allotted,
    NotAllotted,
    Pending,
    NotFound,
    Unknown,
    NeedsHumanVerification,
    RateLimited,
    ProviderUnavailable,
    RetryableError,
    ManualResult,
}

impl NormalizedAllotmentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allotted => "ALLOTTED",
            Self::NotAllotted => "NOT_ALLOTTED",
            Self::Pending => "PENDING",
            Self::NotFound => "NOT_FOUND",
            Self::Unknown => "UNKNOWN",
            Self::NeedsHumanVerification => "NEEDS_HUMAN_VERIFICATION",
            Self::RateLimited => "RATE_LIMITED",
            Self::ProviderUnavailable => "PROVIDER_UNAVAILABLE",
            Self::RetryableError => "RETRYABLE_ERROR",
            Self::ManualResult => "MANUAL_RESULT",
        }
    }

    /// Final account outcomes that should not auto-retry.
    pub const fn is_final(self) -> bool {
        matches!(
            self,
            Self::Allotted | Self::NotAllotted | Self::NotFound | Self::ManualResult
        )
    }

    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::ProviderUnavailable | Self::RetryableError | Self::Pending
        )
    }

    /// Map free-form provider text. Ambiguous text becomes Unknown — never NotAllotted.
    pub fn from_provider_text(text: &str) -> Self {
        let t = text.trim().to_ascii_uppercase();
        if t.is_empty() {
            return Self::Unknown;
        }
        if t.contains("CAPTCHA") || t.contains("OTP") || t.contains("HUMAN") {
            return Self::NeedsHumanVerification;
        }
        if t.contains("RATE") && t.contains("LIMIT") {
            return Self::RateLimited;
        }
        if t.contains("UNAVAILABLE") || t.contains("TIMEOUT") || t.contains("503") {
            return Self::ProviderUnavailable;
        }
        if t.contains("NOT ALLOT") || t.contains("NOT_ALLOT") || t.contains("NON-ALLOT") {
            return Self::NotAllotted;
        }
        if t.contains("ALLOT") && !t.contains("NOT") {
            return Self::Allotted;
        }
        if t.contains("PENDING") || t.contains("UNDER PROCESS") {
            return Self::Pending;
        }
        if t.contains("NOT FOUND") || t.contains("NO RECORD") {
            return Self::NotFound;
        }
        Self::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_text_never_becomes_not_allotted() {
        assert_eq!(
            NormalizedAllotmentStatus::from_provider_text("garbled xyz"),
            NormalizedAllotmentStatus::Unknown
        );
        assert_eq!(
            NormalizedAllotmentStatus::from_provider_text(""),
            NormalizedAllotmentStatus::Unknown
        );
    }

    #[test]
    fn allotted_and_not_allotted_parse() {
        assert_eq!(
            NormalizedAllotmentStatus::from_provider_text("Congratulations, shares allotted"),
            NormalizedAllotmentStatus::Allotted
        );
        assert_eq!(
            NormalizedAllotmentStatus::from_provider_text("Sorry, not allotted"),
            NormalizedAllotmentStatus::NotAllotted
        );
    }
}

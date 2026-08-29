use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderId {
    KfintechFixture,
    KfintechLive,
    BigshareLive,
    MufgIntimeLive,
}

impl ProviderId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KfintechFixture => "kfintech-fixture",
            Self::KfintechLive => "kfintech-live",
            Self::BigshareLive => "bigshare-live",
            Self::MufgIntimeLive => "mufg-intime-live",
        }
    }
}

pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn resolve(value: &str) -> Option<ProviderId> {
        match value.trim().to_ascii_lowercase().as_str() {
            "kfintech-fixture" => Some(ProviderId::KfintechFixture),
            "kfintech" | "kfin technologies" | "kfintech-live" => Some(ProviderId::KfintechLive),
            "bigshare" | "bigshare services" | "bigshare-live" => Some(ProviderId::BigshareLive),
            "link intime" | "linkintime" | "mufg" | "mufg intime" | "mufg-intime-live" => {
                Some(ProviderId::MufgIntimeLive)
            }
            _ => None,
        }
    }
}

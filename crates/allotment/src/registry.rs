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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub registrar_id: &'static str,
    pub registrar_name: &'static str,
    pub provider_id: ProviderId,
    pub official_status_url: &'static str,
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

    /// Resolve only explicit registrar ids/aliases. Product routing never uses
    /// company-name substring guesses.
    pub fn resolve_registrar(value: &str) -> Option<ProviderDescriptor> {
        match value.trim().to_ascii_lowercase().as_str() {
            "kfintech" | "kfin_technologies" => Some(ProviderDescriptor {
                registrar_id: "kfintech",
                registrar_name: "KFintech",
                provider_id: ProviderId::KfintechLive,
                official_status_url: "https://ipostatus.kfintech.com",
            }),
            "bigshare" | "bigshare_services" => Some(ProviderDescriptor {
                registrar_id: "bigshare",
                registrar_name: "Bigshare Services",
                provider_id: ProviderId::BigshareLive,
                official_status_url: "https://ipo.bigshareonline.com/ipo_status.html",
            }),
            "mufg_intime" | "mufg" | "link_intime" => Some(ProviderDescriptor {
                registrar_id: "mufg_intime",
                registrar_name: "MUFG Intime India",
                provider_id: ProviderId::MufgIntimeLive,
                official_status_url: "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx",
            }),
            _ => None,
        }
    }
}

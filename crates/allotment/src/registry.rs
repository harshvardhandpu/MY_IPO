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
    /// One canonical alias normalization (Gate 4F condition E): trim,
    /// lowercase, spaces fold to underscores. Every alias form —
    /// "link intime", "link_intime", "KFIN Technologies" — lands on one
    /// canonical key before lookup. Both `resolve` and `resolve_registrar`
    /// go through this policy; unknown keys FAIL CLOSED (None).
    fn canonical_key(value: &str) -> String {
        value.trim().to_ascii_lowercase().replace(' ', "_")
    }

    /// The single registrar alias table. Both resolvers share it — no
    /// divergent alias sets.
    fn descriptor_for_key(key: &str) -> Option<ProviderDescriptor> {
        match key {
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

    /// Resolve a registrar alias (or explicit provider id) to its provider.
    /// Never resolves by company-name substring guessing.
    pub fn resolve(value: &str) -> Option<ProviderId> {
        // Explicit provider ids (used in persisted jobs) resolve directly.
        let key = Self::canonical_key(value);
        match key.as_str() {
            "kfintech-fixture" => return Some(ProviderId::KfintechFixture),
            "kfintech-live" => return Some(ProviderId::KfintechLive),
            "bigshare-live" => return Some(ProviderId::BigshareLive),
            "mufg-intime-live" => return Some(ProviderId::MufgIntimeLive),
            _ => {}
        }
        Self::descriptor_for_key(&key).map(|d| d.provider_id)
    }

    /// Resolve only explicit registrar ids/aliases. Product routing never uses
    /// company-name substring guesses. Same canonical table as `resolve`.
    pub fn resolve_registrar(value: &str) -> Option<ProviderDescriptor> {
        Self::descriptor_for_key(&Self::canonical_key(value))
    }
}

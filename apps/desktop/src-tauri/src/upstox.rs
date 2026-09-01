use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sanket_allotment::http::{HttpPolicyError, SharedHttpClient, TIMEOUT_SECS};
use serde::{Deserialize, Serialize};

use crate::provider_credentials::{
    CredentialError, OsProviderCredentialStore, ProviderCredentialKey,
};

const UPSTOX_HOST: &str = "api.upstox.com";
const CACHE_TTL_SECS: u64 = 600;
const MAX_RESPONSE_BYTES: u64 = 256 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpoStatus {
    Open,
    Upcoming,
    Closed,
    Listed,
}

impl IpoStatus {
    fn wire(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Upcoming => "upcoming",
            Self::Closed => "closed",
            Self::Listed => "listed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueType {
    Regular,
    Sme,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceBasis {
    CutOff,
    UpperBandEstimate,
    Tba,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistrarMappingState {
    ConfirmedAlias,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IpoCatalogItemDto {
    pub source: String,
    pub source_ipo_id: String,
    pub isin: Option<String>,
    pub issue_size_crore: Option<String>,
    pub industry: Option<String>,
    pub symbol: String,
    pub name: String,
    pub issue_type: IssueType,
    pub status: IpoStatus,
    pub minimum_price_paise: Option<i64>,
    pub maximum_price_paise: Option<i64>,
    pub cut_off_price_paise: Option<i64>,
    pub planning_price_paise: Option<i64>,
    pub price_basis: PriceBasis,
    pub lot_size: Option<u64>,
    pub minimum_quantity: Option<u64>,
    pub minimum_lots: Option<u64>,
    pub cost_per_lot_paise: Option<i64>,
    pub minimum_application_amount_paise: Option<i64>,
    pub bidding_start_date: Option<String>,
    pub bidding_end_date: Option<String>,
    pub allotment_date: Option<String>,
    pub listing_date: Option<String>,
    pub pre_apply_start_date: Option<String>,
    pub allotment_start_date: Option<String>,
    pub refund_initiation_date: Option<String>,
    pub mandate_end_date: Option<String>,
    pub daily_start_time: Option<String>,
    pub daily_end_time: Option<String>,
    pub face_value_paise: Option<i64>,
    pub tick_size_paise: Option<i64>,
    pub listing_price_paise: Option<i64>,
    pub rhp_url: Option<String>,
    pub drhp_url: Option<String>,
    pub registrar_name: Option<String>,
    pub registrar_short_name: Option<String>,
    pub registrar_email: Option<String>,
    pub registrar_contact_name: Option<String>,
    pub registrar_contact_number: Option<String>,
    pub registrar_website: Option<String>,
    pub registrar_mapping_state: RegistrarMappingState,
    pub listing_exchange: Option<String>,
    pub total_subscription: Option<String>,
    pub fetched_at: String,
    pub stale: bool,
    pub safe_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct IpoCatalogueDto {
    pub status: IpoStatus,
    pub items: Vec<IpoCatalogItemDto>,
    pub stale: bool,
    pub fetched_at: String,
    pub safe_message: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IpoListQuery {
    pub status: IpoStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum UpstoxError {
    #[error("connect an Upstox Analytics Token first")]
    CredentialRequired,
    #[error("Upstox authentication failed")]
    TokenInvalid,
    #[error("Upstox access is forbidden")]
    Forbidden,
    #[error("Upstox rate limit reached")]
    RateLimited,
    #[error("IPO details were not found")]
    NotFound,
    #[error("Upstox is temporarily unavailable")]
    ProviderUnavailable,
    #[error("Upstox network request failed")]
    Network,
    #[error("Upstox returned invalid IPO data")]
    InvalidResponse,
    #[error("local network policy refused the request")]
    PolicyRefused,
    #[error("public IPO cache is unavailable")]
    CacheUnavailable,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn parse_status(value: &str) -> Result<IpoStatus, UpstoxError> {
    match value.to_ascii_lowercase().as_str() {
        "open" => Ok(IpoStatus::Open),
        "upcoming" => Ok(IpoStatus::Upcoming),
        "closed" => Ok(IpoStatus::Closed),
        "listed" => Ok(IpoStatus::Listed),
        _ => Err(UpstoxError::InvalidResponse),
    }
}

fn parse_issue_type(value: &str) -> Result<IssueType, UpstoxError> {
    match value.to_ascii_lowercase().as_str() {
        "regular" => Ok(IssueType::Regular),
        "sme" => Ok(IssueType::Sme),
        _ => Err(UpstoxError::InvalidResponse),
    }
}

#[derive(Deserialize)]
struct WireEnvelope<T> {
    status: String,
    data: T,
}

#[derive(Deserialize)]
struct WireRegistrar {
    name: Option<String>,
    email: Option<String>,
    contact_name: Option<String>,
    contact_number: Option<String>,
    website: Option<String>,
    registrar: Option<String>,
}

#[derive(Deserialize)]
struct WireTimeline {
    bidding_start_date: Option<String>,
    bidding_end_date: Option<String>,
    application_start_date: Option<String>,
    application_end_date: Option<String>,
    pre_apply_start_date: Option<String>,
    allotment_start_date: Option<String>,
    allotment_date: Option<String>,
    refund_initiation_date: Option<String>,
    listing_date: Option<String>,
    mandate_end_date: Option<String>,
}

#[derive(Deserialize)]
struct WireIpo {
    id: String,
    symbol: Option<String>,
    name: String,
    status: String,
    issue_type: String,
    isin: Option<String>,
    issue_size: Option<serde_json::Number>,
    industry: Option<String>,
    minimum_price: Option<serde_json::Number>,
    maximum_price: Option<serde_json::Number>,
    cut_off_price: Option<serde_json::Number>,
    lot_size: Option<u64>,
    minimum_quantity: Option<u64>,
    bidding_start_date: Option<String>,
    bidding_end_date: Option<String>,
    allotment_date: Option<String>,
    listing_date: Option<String>,
    timeline: Option<WireTimeline>,
    registrar_info: Option<WireRegistrar>,
    daily_start_time: Option<String>,
    daily_end_time: Option<String>,
    face_value: Option<serde_json::Number>,
    tick_size: Option<serde_json::Number>,
    listing_price: Option<serde_json::Number>,
    rhp_url: Option<String>,
    drhp_url: Option<String>,
    listing_exchange: Option<String>,
    total_subscription: Option<String>,
}

fn parse_paise(value: Option<&serde_json::Number>) -> Result<Option<i64>, UpstoxError> {
    let Some(value) = value else { return Ok(None) };
    let raw = value.to_string();
    if raw.starts_with('-') || raw.contains('e') || raw.contains('E') {
        return Err(UpstoxError::InvalidResponse);
    }
    let (whole, fraction) = raw.split_once('.').unwrap_or((&raw, ""));
    if whole.is_empty()
        || fraction.len() > 2
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(UpstoxError::InvalidResponse);
    }
    let whole = whole
        .parse::<i128>()
        .map_err(|_| UpstoxError::InvalidResponse)?;
    let cents = match fraction.len() {
        0 => 0,
        1 => {
            fraction
                .parse::<i128>()
                .map_err(|_| UpstoxError::InvalidResponse)?
                * 10
        }
        _ => fraction
            .parse::<i128>()
            .map_err(|_| UpstoxError::InvalidResponse)?,
    };
    let paise = whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(cents))
        .ok_or(UpstoxError::InvalidResponse)?;
    if paise <= 0 {
        return Ok(None);
    }
    i64::try_from(paise)
        .map(Some)
        .map_err(|_| UpstoxError::InvalidResponse)
}

fn parse_decimal_text(value: Option<&serde_json::Number>) -> Result<Option<String>, UpstoxError> {
    let Some(value) = value else { return Ok(None) };
    let raw = value.to_string();
    let (whole, fraction) = raw.split_once('.').unwrap_or((&raw, ""));
    if raw.starts_with('-')
        || raw.contains('e')
        || raw.contains('E')
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(UpstoxError::InvalidResponse);
    }
    Ok(Some(raw))
}

fn parse_positive_u64(value: Option<u64>) -> Option<u64> {
    value.filter(|value| *value > 0)
}

fn parse_date(value: Option<&str>) -> Result<Option<String>, UpstoxError> {
    let Some(value) = value else { return Ok(None) };
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err(UpstoxError::InvalidResponse);
    }
    Ok(Some(value.to_owned()))
}

fn planning_price(cutoff: Option<i64>, upper: Option<i64>) -> (Option<i64>, PriceBasis) {
    cutoff
        .map(|v| (Some(v), PriceBasis::CutOff))
        .or_else(|| upper.map(|v| (Some(v), PriceBasis::UpperBandEstimate)))
        .unwrap_or((None, PriceBasis::Tba))
}

fn registrar(
    value: Option<&WireRegistrar>,
) -> (Option<String>, Option<String>, RegistrarMappingState) {
    let Some(value) = value else {
        return (None, None, RegistrarMappingState::Unknown);
    };
    let name = value.name.clone().or_else(|| value.registrar.clone());
    let raw = value
        .registrar
        .clone()
        .or_else(|| name.clone())
        .unwrap_or_default();
    let compact: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let alias = if matches!(
        compact.as_str(),
        "kfintech" | "kfintechnologies" | "kfintechnologieslimited"
    ) {
        Some("KFintech")
    } else if matches!(
        compact.as_str(),
        "bigshare" | "bigshareservices" | "bigshareserviceslimited" | "bigshareservicespvtltd"
    ) {
        Some("Bigshare")
    } else if matches!(
        compact.as_str(),
        "mufgintime"
            | "mufgintimeindia"
            | "mufgintimeindialimited"
            | "mufgintimeindiaprivatelimited"
            | "linkintimeindia"
            | "linkintimeindialimited"
            | "linkintimeindiaprivatelimited"
    ) {
        Some("MUFG Intime")
    } else {
        None
    };
    (
        name,
        alias.map(ToOwned::to_owned),
        if alias.is_some() {
            RegistrarMappingState::ConfirmedAlias
        } else {
            RegistrarMappingState::Unknown
        },
    )
}

fn validate_source_ipo_id(source_ipo_id: &str) -> Result<(), UpstoxError> {
    if source_ipo_id.is_empty()
        || source_ipo_id == "."
        || source_ipo_id == ".."
        || source_ipo_id.len() > 128
        || !source_ipo_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return Err(UpstoxError::InvalidResponse);
    }
    Ok(())
}

fn normalize_item(value: &WireIpo, fetched_at: u64) -> Result<IpoCatalogItemDto, UpstoxError> {
    let source_ipo_id = value.id.clone();
    validate_source_ipo_id(&source_ipo_id)?;
    let symbol = value.symbol.clone().unwrap_or_default();
    let name = value.name.clone();
    if name.trim().is_empty() {
        return Err(UpstoxError::InvalidResponse);
    }
    let status = parse_status(&value.status)?;
    let issue_type = parse_issue_type(&value.issue_type)?;
    let minimum_price_paise = parse_paise(value.minimum_price.as_ref())?;
    let maximum_price_paise = parse_paise(value.maximum_price.as_ref())?;
    let cut_off_price_paise = parse_paise(value.cut_off_price.as_ref())?;
    let issue_size_crore = parse_decimal_text(value.issue_size.as_ref())?;
    let face_value_paise = parse_paise(value.face_value.as_ref())?;
    let tick_size_paise = parse_paise(value.tick_size.as_ref())?;
    let listing_price_paise = parse_paise(value.listing_price.as_ref())?;
    let (planning_price_paise, price_basis) =
        planning_price(cut_off_price_paise, maximum_price_paise);
    let lot_size = parse_positive_u64(value.lot_size);
    let minimum_quantity = parse_positive_u64(value.minimum_quantity);
    let minimum_lots = lot_size
        .zip(minimum_quantity)
        .and_then(|(lot, minimum)| (minimum % lot == 0).then_some(minimum / lot));
    let cost_per_lot_paise = lot_size
        .zip(planning_price_paise)
        .and_then(|(lot, price)| i64::try_from(lot).ok()?.checked_mul(price));
    let minimum_application_amount_paise = minimum_quantity
        .zip(planning_price_paise)
        .and_then(|(minimum, price)| i64::try_from(minimum).ok()?.checked_mul(price));
    let (registrar_name, registrar_short_name, registrar_mapping_state) =
        registrar(value.registrar_info.as_ref());
    let timeline = value.timeline.as_ref();
    let date = |direct: Option<&String>, fallback: Option<&String>| {
        parse_date(
            direct
                .map(String::as_str)
                .or_else(|| fallback.map(String::as_str)),
        )
    };
    Ok(IpoCatalogItemDto {
        source: "UPSTOX_IPO_API".to_owned(),
        source_ipo_id,
        isin: value.isin.clone(),
        issue_size_crore,
        industry: value.industry.clone(),
        symbol,
        name,
        issue_type,
        status,
        minimum_price_paise,
        maximum_price_paise,
        cut_off_price_paise,
        planning_price_paise,
        price_basis,
        lot_size,
        minimum_quantity,
        minimum_lots,
        cost_per_lot_paise,
        minimum_application_amount_paise,
        bidding_start_date: date(
            value.bidding_start_date.as_ref(),
            timeline.and_then(|v| {
                v.bidding_start_date
                    .as_ref()
                    .or(v.application_start_date.as_ref())
            }),
        )?,
        bidding_end_date: date(
            value.bidding_end_date.as_ref(),
            timeline.and_then(|v| {
                v.bidding_end_date
                    .as_ref()
                    .or(v.application_end_date.as_ref())
            }),
        )?,
        allotment_date: date(
            value.allotment_date.as_ref(),
            timeline.and_then(|v| v.allotment_date.as_ref()),
        )?,
        listing_date: date(
            value.listing_date.as_ref(),
            timeline.and_then(|v| v.listing_date.as_ref()),
        )?,
        pre_apply_start_date: date(timeline.and_then(|v| v.pre_apply_start_date.as_ref()), None)?,
        allotment_start_date: date(timeline.and_then(|v| v.allotment_start_date.as_ref()), None)?,
        refund_initiation_date: date(
            timeline.and_then(|v| v.refund_initiation_date.as_ref()),
            None,
        )?,
        mandate_end_date: date(timeline.and_then(|v| v.mandate_end_date.as_ref()), None)?,
        daily_start_time: value.daily_start_time.clone(),
        daily_end_time: value.daily_end_time.clone(),
        face_value_paise,
        tick_size_paise,
        listing_price_paise,
        rhp_url: value.rhp_url.clone(),
        drhp_url: value.drhp_url.clone(),
        registrar_name,
        registrar_short_name,
        registrar_email: value.registrar_info.as_ref().and_then(|v| v.email.clone()),
        registrar_contact_name: value
            .registrar_info
            .as_ref()
            .and_then(|v| v.contact_name.clone()),
        registrar_contact_number: value
            .registrar_info
            .as_ref()
            .and_then(|v| v.contact_number.clone()),
        registrar_website: value
            .registrar_info
            .as_ref()
            .and_then(|v| v.website.clone()),
        registrar_mapping_state,
        listing_exchange: value.listing_exchange.clone(),
        total_subscription: value.total_subscription.clone(),
        fetched_at: fetched_at.to_string(),
        stale: false,
        safe_message: None,
    })
}

pub fn parse_list_response(
    raw: &str,
    fetched_at: u64,
) -> Result<Vec<IpoCatalogItemDto>, UpstoxError> {
    let envelope: WireEnvelope<Vec<WireIpo>> =
        serde_json::from_str(raw).map_err(|_| UpstoxError::InvalidResponse)?;
    if envelope.status != "success" {
        return Err(UpstoxError::InvalidResponse);
    }
    let data = envelope.data;
    data.iter()
        .map(|item| normalize_item(item, fetched_at))
        .collect()
}

pub fn parse_details_response(
    raw: &str,
    fetched_at: u64,
) -> Result<IpoCatalogItemDto, UpstoxError> {
    let envelope: WireEnvelope<WireIpo> =
        serde_json::from_str(raw).map_err(|_| UpstoxError::InvalidResponse)?;
    if envelope.status != "success" {
        return Err(UpstoxError::InvalidResponse);
    }
    normalize_item(&envelope.data, fetched_at)
}

fn validate_catalogue_status(
    items: &[IpoCatalogItemDto],
    expected: IpoStatus,
) -> Result<(), UpstoxError> {
    if items.iter().any(|item| item.status != expected) {
        return Err(UpstoxError::InvalidResponse);
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct CacheFile {
    open: Vec<IpoCatalogItemDto>,
    upcoming: Vec<IpoCatalogItemDto>,
    details: BTreeMap<String, CachedDetail>,
    open_fetched_at: u64,
    open_expires_at: u64,
    upcoming_fetched_at: u64,
    upcoming_expires_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedDetail {
    item: IpoCatalogItemDto,
    expires_at: u64,
}

#[derive(Clone, Debug)]
pub struct PublicIpoCache {
    path: PathBuf,
}

impl PublicIpoCache {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn load(&self) -> Result<Option<CacheFile>, UpstoxError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path).map_err(|_| UpstoxError::CacheUnavailable)?;
        serde_json::from_str(&raw)
            .map(Some)
            .map_err(|_| UpstoxError::CacheUnavailable)
    }

    fn write(&self, value: &CacheFile) -> Result<(), UpstoxError> {
        let parent = self.path.parent().ok_or(UpstoxError::CacheUnavailable)?;
        fs::create_dir_all(parent).map_err(|_| UpstoxError::CacheUnavailable)?;
        let raw = serde_json::to_vec(value).map_err(|_| UpstoxError::CacheUnavailable)?;
        let temp = self.path.with_extension("tmp");
        fs::write(&temp, raw).map_err(|_| UpstoxError::CacheUnavailable)?;
        fs::rename(temp, &self.path).map_err(|_| UpstoxError::CacheUnavailable)
    }
}

#[derive(Clone)]
pub struct UpstoxService {
    credentials: OsProviderCredentialStore,
    cache: PublicIpoCache,
    http: &'static SharedHttpClient,
    last_request: Arc<Mutex<Option<Instant>>>,
    refresh_lock: Arc<Mutex<()>>,
}

impl UpstoxService {
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            credentials: OsProviderCredentialStore,
            cache: PublicIpoCache::new(cache_path),
            http: SharedHttpClient::global(),
            last_request: Arc::new(Mutex::new(None)),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    fn pace_request(&self) {
        let Ok(mut last) = self.last_request.lock() else {
            return;
        };
        if let Some(previous) = *last {
            let wait = Duration::from_secs(1).saturating_sub(previous.elapsed());
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
        }
        *last = Some(Instant::now());
    }

    pub fn list_available(
        &self,
        status: IpoStatus,
        force: bool,
    ) -> Result<IpoCatalogueDto, UpstoxError> {
        let now = now_secs();
        if !force {
            if let Some(cache) = self.cache.load()? {
                let (items, fetched_at, expires_at) = match status {
                    IpoStatus::Open => (&cache.open, cache.open_fetched_at, cache.open_expires_at),
                    IpoStatus::Upcoming => (
                        &cache.upcoming,
                        cache.upcoming_fetched_at,
                        cache.upcoming_expires_at,
                    ),
                    _ => return Err(UpstoxError::InvalidResponse),
                };
                if expires_at > now && validate_catalogue_status(items, status).is_ok() {
                    return Ok(IpoCatalogueDto {
                        status,
                        items: items.clone(),
                        stale: false,
                        fetched_at: fetched_at.to_string(),
                        safe_message: None,
                    });
                }
            }
        }
        let _refresh = self
            .refresh_lock
            .lock()
            .map_err(|_| UpstoxError::CacheUnavailable)?;
        if !force {
            if let Some(cache) = self.cache.load()? {
                let (items, fetched_at, expires_at) = match status {
                    IpoStatus::Open => (&cache.open, cache.open_fetched_at, cache.open_expires_at),
                    IpoStatus::Upcoming => (
                        &cache.upcoming,
                        cache.upcoming_fetched_at,
                        cache.upcoming_expires_at,
                    ),
                    _ => return Err(UpstoxError::InvalidResponse),
                };
                if expires_at > now && validate_catalogue_status(items, status).is_ok() {
                    return Ok(IpoCatalogueDto {
                        status,
                        items: items.clone(),
                        stale: false,
                        fetched_at: fetched_at.to_string(),
                        safe_message: None,
                    });
                }
            }
        }
        match self.fetch_list(status) {
            Ok(items) => {
                let mut cache = self.cache.load()?.unwrap_or_default();
                if status == IpoStatus::Open {
                    cache.open = items.clone();
                    cache.open_fetched_at = now;
                    cache.open_expires_at = now.saturating_add(CACHE_TTL_SECS);
                } else {
                    cache.upcoming = items.clone();
                    cache.upcoming_fetched_at = now;
                    cache.upcoming_expires_at = now.saturating_add(CACHE_TTL_SECS);
                }
                self.cache.write(&cache)?;
                Ok(IpoCatalogueDto {
                    status,
                    items,
                    stale: false,
                    fetched_at: now.to_string(),
                    safe_message: None,
                })
            }
            Err(error) => {
                let cache = match self.cache.load()? {
                    Some(cache) => cache,
                    None => return Err(error),
                };
                let (items, fetched_at) = match status {
                    IpoStatus::Open => (cache.open, cache.open_fetched_at),
                    IpoStatus::Upcoming => (cache.upcoming, cache.upcoming_fetched_at),
                    _ => return Err(UpstoxError::InvalidResponse),
                };
                if fetched_at == 0 || validate_catalogue_status(&items, status).is_err() {
                    return Err(error);
                }
                Ok(IpoCatalogueDto {
                    status,
                    items,
                    stale: true,
                    fetched_at: fetched_at.to_string(),
                    safe_message: Some(error.to_string()),
                })
            }
        }
    }

    pub fn details(&self, source_ipo_id: &str) -> Result<IpoCatalogItemDto, UpstoxError> {
        validate_source_ipo_id(source_ipo_id)?;
        let _refresh = self
            .refresh_lock
            .lock()
            .map_err(|_| UpstoxError::CacheUnavailable)?;
        let now = now_secs();
        let cached = self
            .cache
            .load()?
            .and_then(|cache| cache.details.get(source_ipo_id).cloned());
        if let Some(entry) = cached.as_ref().filter(|entry| entry.expires_at > now) {
            return Ok(entry.item.clone());
        }
        match self.fetch_details(source_ipo_id) {
            Ok(item) => {
                let mut cache = self.cache.load()?.unwrap_or_default();
                cache.details.insert(
                    source_ipo_id.to_owned(),
                    CachedDetail {
                        item: item.clone(),
                        expires_at: now.saturating_add(CACHE_TTL_SECS),
                    },
                );
                self.cache.write(&cache)?;
                Ok(item)
            }
            Err(error) => {
                let Some(entry) = cached else {
                    return Err(error);
                };
                let mut item = entry.item;
                item.stale = true;
                item.safe_message = Some(error.to_string());
                Ok(item)
            }
        }
    }

    fn fetch_list(&self, status: IpoStatus) -> Result<Vec<IpoCatalogItemDto>, UpstoxError> {
        let url = format!(
            "https://{UPSTOX_HOST}/v2/ipos?status={}&page_number=1&records=30",
            status.wire()
        );
        self.pace_request();
        self.credentials
            .with_secret(ProviderCredentialKey::UpstoxAnalyticsToken, |token| {
                let authorization = format!("Bearer {token}");
                let raw = self
                    .http
                    .get_json_with_header(
                        &url,
                        &[UPSTOX_HOST],
                        &authorization,
                        MAX_RESPONSE_BYTES,
                        TIMEOUT_SECS,
                    )
                    .map_err(map_http)?;
                let items = parse_list_response(&raw, now_secs())?;
                validate_catalogue_status(&items, status)?;
                Ok(items)
            })
            .map_err(map_credential)?
    }

    fn fetch_details(&self, source_ipo_id: &str) -> Result<IpoCatalogItemDto, UpstoxError> {
        let url = format!("https://{UPSTOX_HOST}/v2/ipos/{source_ipo_id}");
        self.pace_request();
        self.credentials
            .with_secret(ProviderCredentialKey::UpstoxAnalyticsToken, |token| {
                let authorization = format!("Bearer {token}");
                let raw = self
                    .http
                    .get_json_with_header(
                        &url,
                        &[UPSTOX_HOST],
                        &authorization,
                        MAX_RESPONSE_BYTES,
                        TIMEOUT_SECS,
                    )
                    .map_err(map_http)?;
                let item = parse_details_response(&raw, now_secs())?;
                if item.source_ipo_id != source_ipo_id {
                    return Err(UpstoxError::InvalidResponse);
                }
                Ok(item)
            })
            .map_err(map_credential)?
    }
}

fn map_credential(error: CredentialError) -> UpstoxError {
    match error {
        CredentialError::MissingCredential => UpstoxError::CredentialRequired,
        CredentialError::KeyringLocked
        | CredentialError::KeyringUnavailable
        | CredentialError::NoDurableBackend
        | CredentialError::StorageFailure => UpstoxError::ProviderUnavailable,
        CredentialError::EmptyCredential | CredentialError::CredentialTooLong => {
            UpstoxError::CredentialRequired
        }
    }
}

fn map_http(error: HttpPolicyError) -> UpstoxError {
    match error {
        HttpPolicyError::Status(401) => UpstoxError::TokenInvalid,
        HttpPolicyError::Status(403) => UpstoxError::Forbidden,
        HttpPolicyError::Status(404) => UpstoxError::NotFound,
        HttpPolicyError::Status(429) | HttpPolicyError::RateLimited => UpstoxError::RateLimited,
        HttpPolicyError::Status(500) | HttpPolicyError::Status(503) => {
            UpstoxError::ProviderUnavailable
        }
        HttpPolicyError::HostNotAllowed { .. }
        | HttpPolicyError::InvalidUrl
        | HttpPolicyError::RedirectNotAllowed { .. }
        | HttpPolicyError::TooManyRedirects
        | HttpPolicyError::UnexpectedContentType(_)
        | HttpPolicyError::SizeCapExceeded(_) => UpstoxError::PolicyRefused,
        HttpPolicyError::Transport(_) => UpstoxError::Network,
        HttpPolicyError::Status(_) => UpstoxError::ProviderUnavailable,
    }
}

pub fn list_available_ipos(
    state: &crate::AppState,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String> {
    state
        .upstox
        .list_available(query.status, false)
        .map_err(|e| e.to_string())
}
pub fn refresh_ipo_catalog(
    state: &crate::AppState,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String> {
    state
        .upstox
        .list_available(query.status, true)
        .map_err(|e| e.to_string())
}
pub fn get_ipo_details(
    state: &crate::AppState,
    source_ipo_id: String,
) -> Result<IpoCatalogItemDto, String> {
    state
        .upstox
        .details(&source_ipo_id)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_decimal_parser_has_no_zero_or_float_fallback() {
        let one = serde_json::json!(100.01).as_number().cloned();
        assert_eq!(parse_paise(one.as_ref()).unwrap(), Some(10001));
        let zero = serde_json::json!(0).as_number().cloned();
        assert_eq!(parse_paise(zero.as_ref()).unwrap(), None);
    }

    #[test]
    fn cache_round_trip_contains_only_normalized_public_values() {
        let directory = tempfile::tempdir().unwrap();
        let cache = PublicIpoCache::new(directory.path().join("upstox-ipo.json"));
        cache
            .write(&CacheFile {
                open_fetched_at: 1,
                open_expires_at: 2,
                ..CacheFile::default()
            })
            .unwrap();
        let loaded = cache.load().unwrap().unwrap();
        assert_eq!(loaded.open_fetched_at, 1);
        let body = fs::read_to_string(directory.path().join("upstox-ipo.json")).unwrap();
        assert!(!body.contains("Authorization"));
        assert!(!body.contains("Bearer"));
    }

    #[test]
    fn detail_id_validation_rejects_path_segments() {
        assert!(validate_source_ipo_id(".").is_err());
        assert!(validate_source_ipo_id("..").is_err());
        assert!(validate_source_ipo_id("safe-ipo").is_ok());
    }

    #[test]
    fn registrar_matching_is_exact_enough_to_fail_closed() {
        let wire = r#"{"status":"success","data":{"id":"x","name":"X","status":"open","issue_type":"regular","registrar_info":{"registrar":"notkfin"}}}"#;
        let item = parse_details_response(wire, 1).unwrap();
        assert_eq!(item.registrar_mapping_state, RegistrarMappingState::Unknown);
    }

    #[test]
    fn list_status_mismatch_is_rejected() {
        let items = parse_list_response(
            r#"{"status":"success","data":[{"id":"x","name":"X","status":"upcoming","issue_type":"regular"}]}"#,
            1,
        )
        .unwrap();
        assert!(validate_catalogue_status(&items, IpoStatus::Open).is_err());
    }

    #[test]
    fn detail_cache_expiry_is_per_source_id() {
        let first = parse_details_response(
            r#"{"status":"success","data":{"id":"first","name":"First","status":"open","issue_type":"regular"}}"#,
            1,
        )
        .unwrap();
        let second = parse_details_response(
            r#"{"status":"success","data":{"id":"second","name":"Second","status":"open","issue_type":"regular"}}"#,
            2,
        )
        .unwrap();
        let mut cache = CacheFile::default();
        cache.details.insert(
            "first".to_owned(),
            CachedDetail {
                item: first,
                expires_at: 10,
            },
        );
        cache.details.insert(
            "second".to_owned(),
            CachedDetail {
                item: second,
                expires_at: 20,
            },
        );
        assert_eq!(cache.details["first"].expires_at, 10);
        assert_eq!(cache.details["second"].expires_at, 20);
    }
}

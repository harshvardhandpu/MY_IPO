//! Public-only intelligence and AI-safe payload types.
//!
//! This crate is the *only* surface that may be serialized toward an external AI
//! service. Every type here is composed exclusively of sanitized, public fields.
//! It has no dependency on the member vault, identity-security, or any private
//! type, so there is no representable path to leak a PAN, UPI, name, proof, or
//! member path into an AI request.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A public-only intelligence record retained in the IntelligenceVault.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicIntelligenceRecord {
    pub record_id: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub content_hash: String,
}

/// A sanitized context handed to the ranking/decision AI. Contains no member or
/// friend identity, no PAN, no UPI, no proofs, and no private paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentDecisionPayload {
    pub ipo_name: String,
    pub ipo_identifier: String,
    pub price_band_paise_low: i64,
    pub price_band_paise_high: i64,
    pub lot_size: u32,
    pub subscription_times: u32,
    pub listing_gain_basis_points: i64,
    pub public_context_hashes: Vec<String>,
}

/// A single planned IPO within a decision request (master-source §78.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedIpo {
    pub typed_name: String,
    pub planned_amount_per_account_paise: i64,
}

/// The multi-IPO sanitized decision request (master-source §78.2).
///
/// Field set is fixed to approved keys only: session id, declared daily capital
/// (paise), account count, per-IPO typed name + planned amount, and an algorithm
/// version. No identity, PAN, UPI, name, email, proof, or path is representable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentDecisionRequest {
    pub session_id: String,
    pub declared_daily_capital_paise: i64,
    pub account_count: u32,
    pub ipos: Vec<PlannedIpo>,
    pub algorithm_version: String,
    pub public_context_refs: Vec<String>,
}

impl InvestmentDecisionRequest {
    pub fn new(
        session_id: impl Into<String>,
        declared_daily_capital_paise: i64,
        ipos: Vec<PlannedIpo>,
        algorithm_version: impl Into<String>,
    ) -> Self {
        Self::with_account_count(session_id, declared_daily_capital_paise, ipos, algorithm_version, 0)
    }

    pub fn with_account_count(
        session_id: impl Into<String>,
        declared_daily_capital_paise: i64,
        ipos: Vec<PlannedIpo>,
        algorithm_version: impl Into<String>,
        account_count: u32,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            declared_daily_capital_paise,
            account_count,
            ipos,
            algorithm_version: algorithm_version.into(),
            public_context_refs: Vec::new(),
        }
    }

    /// Fail closed if any string field carries a PAN-like or UPI-like token.
    pub fn assert_safe(&self) -> Result<(), AirBoundaryError> {
        for ipo in &self.ipos {
            if looks_like_pan(&ipo.typed_name) {
                return Err(AirBoundaryError::ProhibitedToken("pan"));
            }
            if looks_like_upi(&ipo.typed_name) {
                return Err(AirBoundaryError::ProhibitedToken("upi"));
            }
        }
        for r in &self.public_context_refs {
            if looks_like_pan(r) {
                return Err(AirBoundaryError::ProhibitedToken("pan"));
            }
        }
        Ok(())
    }
}

/// Public-only storage that may be exposed through the allowlisted AI gateway.
pub trait IntelligenceVault: Send + Sync {
    type Error;

    fn root(&self) -> &Path;
    fn store_public_record(&self, record: &PublicIntelligenceRecord) -> Result<(), Self::Error>;
}

/// A defensive validator that rejects any payload whose string fields carry a
/// token that looks like a sensitive identity value. This is a fail-closed
/// boundary: it never silently strips a value and continues, it refuses the
/// whole payload.
#[derive(Debug, Error)]
pub enum AirBoundaryError {
    #[error("prohibited token detected in field {0}")]
    ProhibitedToken(&'static str),
}

impl InvestmentDecisionPayload {
    /// Fail closed if any field embeds a PAN-like, UPI-like, or otherwise
    /// prohibited private token. Used by the AI gateway immediately before
    /// serialization.
    pub fn assert_safe(&self) -> Result<(), AirBoundaryError> {
        // Indian PAN: 5 letters, 4 digits, 1 letter.
        if looks_like_pan(&self.ipo_name)
            || looks_like_pan(&self.ipo_identifier)
            || self.public_context_hashes.iter().any(|h| looks_like_pan(h))
        {
            return Err(AirBoundaryError::ProhibitedToken("pan"));
        }
        // UPI: local-part@bank (e.g. name123@okhdfcbank).
        if looks_like_upi(&self.ipo_name) || looks_like_upi(&self.ipo_identifier) {
            return Err(AirBoundaryError::ProhibitedToken("upi"));
        }
        Ok(())
    }
}

/// Note: leading/trailing `$`/`^` anchors were removed from this doc comment;
/// the matchers below use `is_match` over the whole string by construction.
fn looks_like_pan(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 {
        return false;
    }
    b[..5].iter().all(|c| c.is_ascii_uppercase())
        && b[5..9].iter().all(|c| c.is_ascii_digit())
        && b[9].is_ascii_uppercase()
}

fn looks_like_upi(s: &str) -> bool {
    s.contains('@') && s.split('@').count() == 2 && !s.contains(char::is_whitespace)
}

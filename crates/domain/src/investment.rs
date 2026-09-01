//! Investment domain: sessions, IPO applications, and allocations.
//!
//! Amounts are integer paise (`Money`); shares are integer basis points
//! (`BasisPoints`). Allocations reference an account id — never a PAN.

use serde::{Deserialize, Serialize};

use crate::money::{BasisPoints, Money};

/// Public IPO facts captured with an application. It never carries credentials,
/// PAN, UPI, registrar provider issue ids, or broker-order state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpoMetadataSnapshot {
    pub metadata_source: String,
    pub source_ipo_id: String,
    pub source_status: String,
    pub source_name: String,
    pub source_symbol: String,
    pub source_isin: Option<String>,
    pub fetched_at: String,
    pub revalidated_at: Option<String>,
    pub minimum_price_paise: Option<i64>,
    pub maximum_price_paise: Option<i64>,
    pub cut_off_price_paise: Option<i64>,
    pub planning_price_paise: Option<i64>,
    pub price_basis: String,
    pub lot_size: Option<u64>,
    pub minimum_quantity: Option<u64>,
    pub minimum_lots: Option<u64>,
    pub lots: u64,
    pub quantity: u64,
    pub amount_per_account_paise: i64,
    pub total_capital_paise: i64,
    pub bidding_start_date: Option<String>,
    pub bidding_end_date: Option<String>,
    pub allotment_date: Option<String>,
    pub listing_date: Option<String>,
    pub registrar_name: Option<String>,
    pub registrar_short_name: Option<String>,
    pub registrar_website: Option<String>,
}

/// Errors for investment-domain construction.
#[derive(Debug, thiserror::Error)]
pub enum InvestmentError {
    #[error("declared capital must be greater than zero")]
    ZeroCapital,
    #[error("IPO name cannot be empty")]
    EmptyIpoName,
    #[error("planned amount must be greater than zero")]
    ZeroAmount,
    #[error("only a submitted session can be voided")]
    InvalidSessionTransition,
}

/// A declared plan to invest a daily capital across one or more IPOs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentSession {
    id: String,
    actor_member_id: String,
    declared_capital: Money,
    status: SessionStatus,
    recommendation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionStatus {
    Open,
    Submitted,
    /// Owner-corrected accidental/incorrect submission. Remains auditable; excluded from active totals.
    Voided,
}

impl InvestmentSession {
    pub fn open(
        id: impl Into<String>,
        actor_member_id: impl Into<String>,
        declared_capital: Money,
    ) -> Result<Self, InvestmentError> {
        if declared_capital.paise() <= 0 {
            return Err(InvestmentError::ZeroCapital);
        }
        Ok(Self {
            id: id.into(),
            actor_member_id: actor_member_id.into(),
            declared_capital,
            status: SessionStatus::Open,
            recommendation_id: None,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn actor_member_id(&self) -> &str {
        &self.actor_member_id
    }

    pub fn declared_capital(&self) -> Money {
        self.declared_capital
    }

    pub fn status(&self) -> &'static str {
        match self.status {
            SessionStatus::Open => "OPEN",
            SessionStatus::Submitted => "SUBMITTED",
            SessionStatus::Voided => "VOIDED",
        }
    }

    pub fn recommendation_id(&self) -> Option<&str> {
        self.recommendation_id.as_deref()
    }

    /// Record that a recommendation was produced for this session (draft only).
    pub fn attach_recommendation(&mut self, recommendation_id: impl Into<String>) {
        self.recommendation_id = Some(recommendation_id.into());
    }

    pub fn mark_submitted(&mut self) {
        self.status = SessionStatus::Submitted;
    }

    pub fn is_submitted(&self) -> bool {
        self.status == SessionStatus::Submitted
    }

    /// Owner correction: only a submitted session may be voided.
    pub fn mark_voided(&mut self) -> Result<(), InvestmentError> {
        if self.status != SessionStatus::Submitted {
            return Err(InvestmentError::InvalidSessionTransition);
        }
        self.status = SessionStatus::Voided;
        Ok(())
    }

    pub fn is_voided(&self) -> bool {
        self.status == SessionStatus::Voided
    }
}

/// An IPO application within a session: a planned amount per account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpoApplication {
    id: String,
    session_id: String,
    ipo_name: String,
    planned_amount_per_account: Money,
    notes: Option<String>,
}

impl IpoApplication {
    pub fn create(
        id: impl Into<String>,
        session_id: impl Into<String>,
        ipo_name: impl Into<String>,
        planned_amount_per_account: Money,
    ) -> Result<Self, InvestmentError> {
        let name = ipo_name.into();
        if name.trim().is_empty() {
            return Err(InvestmentError::EmptyIpoName);
        }
        if planned_amount_per_account.paise() <= 0 {
            return Err(InvestmentError::ZeroAmount);
        }
        Ok(Self {
            id: id.into(),
            session_id: session_id.into(),
            ipo_name: name,
            planned_amount_per_account,
            notes: None,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn ipo_name(&self) -> &str {
        &self.ipo_name
    }

    pub fn planned_amount_per_account(&self) -> Money {
        self.planned_amount_per_account
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn set_notes(&mut self, notes: Option<impl Into<String>>) {
        self.notes = notes.map(Into::into);
    }
}

/// A final (human-applied) allocation of an application to one account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestmentAllocation {
    id: String,
    application_id: String,
    account_id: String,
    amount: Money,
    share_basis_points: BasisPoints,
}

impl InvestmentAllocation {
    pub fn new(
        id: impl Into<String>,
        application_id: impl Into<String>,
        account_id: impl Into<String>,
        amount: Money,
        share_basis_points: BasisPoints,
    ) -> Self {
        Self {
            id: id.into(),
            application_id: application_id.into(),
            account_id: account_id.into(),
            amount,
            share_basis_points,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    /// The account this allocation targets. Never a PAN.
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn share_basis_points(&self) -> BasisPoints {
        self.share_basis_points
    }
}

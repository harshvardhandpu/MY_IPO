//! Explicit estimated-profit basis — never fabricated.

use serde::{Deserialize, Serialize};

use sanket_domain::Money;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfitPriceBasis {
    Unavailable,
    ActualListingPrice,
    CurrentMarketPrice,
    OwnerExpectedPrice,
    PublicEstimate,
}

impl ProfitPriceBasis {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "UNAVAILABLE",
            Self::ActualListingPrice => "ACTUAL_LISTING_PRICE",
            Self::CurrentMarketPrice => "CURRENT_MARKET_PRICE",
            Self::OwnerExpectedPrice => "OWNER_EXPECTED_PRICE",
            Self::PublicEstimate => "PUBLIC_ESTIMATE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimatedProfit {
    pub basis: ProfitPriceBasis,
    pub reference_price_paise: Option<i64>,
    pub issue_price_paise: Option<i64>,
    pub allotted_shares: u64,
    pub estimated_profit_paise: Option<i64>,
    pub note: Option<String>,
}

impl EstimatedProfit {
    pub fn unavailable(allotted_shares: u64) -> Self {
        Self {
            basis: ProfitPriceBasis::Unavailable,
            reference_price_paise: None,
            issue_price_paise: None,
            allotted_shares,
            estimated_profit_paise: None,
            note: Some("Estimated profit: Not available yet".into()),
        }
    }

    /// estimated_profit = allotted_shares * (reference_price - issue_price) in paise.
    pub fn compute(
        basis: ProfitPriceBasis,
        allotted_shares: u64,
        reference_price: Money,
        issue_price: Money,
        note: Option<String>,
    ) -> Self {
        if matches!(basis, ProfitPriceBasis::Unavailable) || allotted_shares == 0 {
            return Self::unavailable(allotted_shares);
        }
        let delta = reference_price.paise().saturating_sub(issue_price.paise());
        let profit = (allotted_shares as i64).saturating_mul(delta);
        Self {
            basis,
            reference_price_paise: Some(reference_price.paise()),
            issue_price_paise: Some(issue_price.paise()),
            allotted_shares,
            estimated_profit_paise: Some(profit),
            note,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sanket_domain::Money;

    #[test]
    fn compute_integer_paise() {
        let e = EstimatedProfit::compute(
            ProfitPriceBasis::OwnerExpectedPrice,
            35,
            Money::from_paise(15_000),
            Money::from_paise(10_000),
            None,
        );
        assert_eq!(e.estimated_profit_paise, Some(35 * 5_000));
    }

    #[test]
    fn unavailable_has_no_number() {
        let e = EstimatedProfit::unavailable(10);
        assert!(e.estimated_profit_paise.is_none());
    }
}

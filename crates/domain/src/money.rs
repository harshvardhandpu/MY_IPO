//! Deterministic integer accounting: paise and basis points.
//!
//! All money is integer paise (100 paise = ₹1). All percentages are integer
//! basis points (10_000 bp = 100%). No floating point anywhere on a money path.

use serde::{Deserialize, Serialize};
use std::fmt;

/// An amount of money in integer paise. Non-negative by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn from_paise(paise: i64) -> Self {
        // ponytail: no runtime check in const fn; checked_add/checked_sub guard
        // arithmetic, and callers build amounts from validated inputs.
        Money(paise)
    }

    pub const fn from_rupees(rupees: i64) -> Self {
        Money(rupees * 100)
    }

    pub const fn paise(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Money) -> Option<Money> {
        self.0.checked_add(other.0).map(Money)
    }

    /// Fails closed on negative results — accounting never goes below zero
    /// implicitly.
    pub fn checked_sub(self, other: Money) -> Option<Money> {
        self.0.checked_sub(other.0).filter(|v| *v >= 0).map(Money)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "₹{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, other: Money) -> Money {
        Money(self.0 + other.0)
    }
}

/// A percentage in integer basis points (1 bp = 0.01%, 10_000 bp = 100%).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BasisPoints(i64);

/// Errors for basis-point construction.
#[derive(Debug, thiserror::Error)]
#[error("basis points must be within 0..=10000, got {0}")]
pub struct BasisPointsError(pub i64);

impl BasisPoints {
    /// Unchecked constructor for constants known to be in range (e.g. defaults).
    pub const fn new(value: i64) -> Self {
        BasisPoints(value)
    }

    pub const fn try_new(value: i64) -> Result<Self, BasisPointsError> {
        if value < 0 || value > 10_000 {
            return Err(BasisPointsError(value));
        }
        Ok(BasisPoints(value))
    }

    /// The default friend share: 10% (1_000 bp).
    pub const fn friend_share_default() -> Self {
        BasisPoints(1_000)
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    /// Deterministic integer share: `amount * bp / 10_000`, truncated.
    /// Truncation (never rounding up) keeps splits conservative and exact:
    /// share + remainder always equals the source amount.
    pub fn share_of(self, amount: Money) -> Money {
        Money(amount.paise() * self.0 / 10_000)
    }
}

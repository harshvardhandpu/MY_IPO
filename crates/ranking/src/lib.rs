//! Versioned IPO ranking algorithm interface and a deterministic DEV algorithm.
//!
//! The owner's real ranking algorithm has not been supplied. This crate defines
//! the contract future algorithms implement and ships an explicitly labeled
//! deterministic DEVELOPMENT/TEST algorithm that exercises the ranking,
//! allocation, skip, and reasoning response structures. Its output is NOT
//! investment advice.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors the ranking layer can produce.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RecommendationError {
    #[error("at least one IPO is required")]
    NoIpos,
    #[error("no accounts are available")]
    NoAccounts,
}

/// A required public field an algorithm may declare it needs.
pub type PublicFieldRequirement = String;

/// The versioned algorithm contract. Future real algorithms implement this.
pub trait RankingAlgorithm: Send + Sync {
    /// Stable algorithm version, stored in every recommendation result.
    fn version(&self) -> &'static str;

    /// Public fields the algorithm requires in the sanitized decision payload.
    fn required_public_fields(&self) -> Vec<PublicFieldRequirement>;

    /// Produce a recommendation for a list of IPO names and an available
    /// account count. Receives only sanitized public inputs.
    fn evaluate(
        &self,
        ipos: Vec<String>,
        available_accounts: u32,
    ) -> Result<Recommendation, RecommendationError>;

    /// Human-readable explanation of how the recommendation was formed.
    fn explain(&self, recommendation: &Recommendation) -> String;
}

/// A per-IPO ranking result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedIpo {
    typed_name: String,
    ranking: u32,
    score: i64,
    recommended_account_count: u32,
    recommended_allocation_ratio_bp: i64,
    skip: bool,
    reason: String,
    missing_public_data: Vec<String>,
}

impl RankedIpo {
    pub fn typed_name(&self) -> &str {
        &self.typed_name
    }
    pub fn ranking(&self) -> u32 {
        self.ranking
    }
    pub fn score(&self) -> i64 {
        self.score
    }
    pub fn recommended_account_count(&self) -> u32 {
        self.recommended_account_count
    }
    pub fn recommended_allocation_ratio_bp(&self) -> i64 {
        self.recommended_allocation_ratio_bp
    }
    pub fn skip(&self) -> bool {
        self.skip
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn missing_public_data(&self) -> &[String] {
        &self.missing_public_data
    }
}

/// A whole-session recommendation (draft only; never auto-submits).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    algorithm_version: String,
    label: String,
    ipos: Vec<RankedIpo>,
}

impl Recommendation {
    pub fn algorithm_version(&self) -> &str {
        &self.algorithm_version
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn ipos(&self) -> impl Iterator<Item = &RankedIpo> {
        self.ipos.iter()
    }

    /// Convert the recommendation into a set of per-IPO allocation directives
    /// (typed name + recommended account count) for non-skipped IPOs.
    pub fn apply(&self) -> impl Iterator<Item = RecommendedAllocation> + '_ {
        self.ipos
            .iter()
            .filter(|i| !i.skip)
            .map(|i| RecommendedAllocation {
                typed_name: i.typed_name.clone(),
                recommended_account_count: i.recommended_account_count,
            })
    }
}

/// A single recommended allocation directive produced by applying a
/// recommendation (never an actual submitted allocation).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendedAllocation {
    typed_name: String,
    recommended_account_count: u32,
}

impl RecommendedAllocation {
    pub fn typed_name(&self) -> &str {
        &self.typed_name
    }
    pub fn recommended_account_count(&self) -> u32 {
        self.recommended_account_count
    }
}

/// A deterministic DEVELOPMENT/TEST algorithm.
///
/// It assigns a fixed per-IPO account count from a fixed distribution (5/3/1
/// then skip), giving a reproducible result useful only for exercising the
/// full pipeline. It is NOT a real investment strategy and its output must
/// never be presented as advice.
pub struct DevRankingAlgorithm;

impl DevRankingAlgorithm {
    pub fn new() -> Self {
        DevRankingAlgorithm
    }
}

impl Default for DevRankingAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingAlgorithm for DevRankingAlgorithm {
    fn version(&self) -> &'static str {
        "dev-ranking-v001"
    }

    fn required_public_fields(&self) -> Vec<PublicFieldRequirement> {
        Vec::new()
    }

    fn evaluate(
        &self,
        ipos: Vec<String>,
        available_accounts: u32,
    ) -> Result<Recommendation, RecommendationError> {
        if ipos.is_empty() {
            return Err(RecommendationError::NoIpos);
        }
        if available_accounts == 0 {
            return Err(RecommendationError::NoAccounts);
        }

        // Fixed deterministic account-share pattern: 50%, 30%, 10%, then skip.
        const SHARE_PATTERN_BP: [i64; 3] = [5_000, 3_000, 1_000];

        let mut ranked = Vec::with_capacity(ipos.len());
        for (index, name) in ipos.into_iter().enumerate() {
            if index >= SHARE_PATTERN_BP.len() {
                ranked.push(RankedIpo {
                    typed_name: name,
                    ranking: index as u32 + 1,
                    score: 0,
                    recommended_account_count: 0,
                    recommended_allocation_ratio_bp: 0,
                    skip: true,
                    reason: "dev algorithm ranks at most three IPOs; the rest are skipped"
                        .to_owned(),
                    missing_public_data: vec!["registrar data".to_owned()],
                });
                continue;
            }
            let allocation_bp = SHARE_PATTERN_BP[index];
            let count = (available_accounts as i64 * allocation_bp / 10_000).max(1) as u32;
            ranked.push(RankedIpo {
                typed_name: name,
                ranking: index as u32 + 1,
                score: (10_000 - allocation_bp) as i64,
                recommended_account_count: count,
                recommended_allocation_ratio_bp: allocation_bp,
                skip: false,
                reason: "deterministic dev ranking".to_owned(),
                missing_public_data: vec![],
            });
        }

        Ok(Recommendation {
            algorithm_version: self.version().to_owned(),
            label: "DEVELOPMENT/TEST ALGORITHM — NOT INVESTMENT ADVICE".to_owned(),
            ipos: ranked,
        })
    }

    fn explain(&self, recommendation: &Recommendation) -> String {
        format!(
            "{} evaluated {} IPO(s) with the deterministic development algorithm.",
            recommendation.label(),
            recommendation.ipos.len()
        )
    }
}
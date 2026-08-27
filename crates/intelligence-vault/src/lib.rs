use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicIntelligenceRecord {
    pub record_id: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub content_hash: String,
}

/// Public-only storage that may be exposed through the allowlisted AI gateway.
pub trait IntelligenceVault: Send + Sync {
    type Error;

    fn root(&self) -> &Path;
    fn store_public_record(&self, record: &PublicIntelligenceRecord) -> Result<(), Self::Error>;
}

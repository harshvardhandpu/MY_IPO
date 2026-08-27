use std::path::Path;

use sanket_domain::EventEnvelope;

/// Private operational storage. Implementations must never be given to AI services.
pub trait MemberVault: Send + Sync {
    type Error;

    fn root(&self) -> &Path;
    fn append_event(&self, event: &EventEnvelope) -> Result<(), Self::Error>;
}

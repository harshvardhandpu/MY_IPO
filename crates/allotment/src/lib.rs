//! Registrar allotment checking — provider contract, jobs, normalized results.
//!
//! PAN never lives on job/result types. Providers receive PAN only as a
//! temporary argument inside a purpose-scoped `with_pan` closure owned by the
//! application service — never via LLM/agent context.

mod job;
mod kfintech;
mod kfintech_live;
mod profit;
mod provider;
mod registry;
mod runtime;
mod status;

pub use job::{
    AllotmentCheckAttempt, AllotmentCheckJob, AllotmentJobError, AllotmentJobStatus,
    AllotmentResultSource, AttemptStatus, ManualReportedOutcome, ManualResultInput,
};
pub use kfintech::{KfintechIssue, KfintechProvider};
pub use kfintech_live::{DiscoveredIssue, LiveKfintechProvider};
pub use profit::{EstimatedProfit, ProfitPriceBasis};
pub use provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, FixtureKfintechProvider,
    HumanVerificationChallenge, HumanVerificationRequirement, HumanVerificationStatus,
    HumanVerificationType, IssueDiscoveryMode, LookupKeyKind, NegativeResultProof,
    NegativeResultProofError, ProviderAllotmentResult, ProviderCapabilities,
    ProviderContinuationReference, ProviderError, ProviderHealth, ProviderResultProvenance,
    ProviderTransportKind, RegistrarIssue, SafeProviderMetadataError, SanitizedFixtureProvenance,
    SessionRequirement,
};
pub use registry::{ProviderId, ProviderRegistry};
pub use runtime::{JobLease, ProviderRateLimiter, ProviderRatePolicy};
pub use status::NormalizedAllotmentStatus;

/// Build readiness and investor-data authorization are separate security gates.
pub const LIVE_ADAPTER_IMPLEMENTED: bool = false;
pub const REAL_INVESTOR_LOOKUP_AUTHORIZED: bool = false;

pub const fn real_investor_lookup_allowed() -> bool {
    LIVE_ADAPTER_IMPLEMENTED && REAL_INVESTOR_LOOKUP_AUTHORIZED
}

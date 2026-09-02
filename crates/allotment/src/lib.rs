//! Registrar allotment checking — provider contract, jobs, normalized results.
//!
//! PAN never lives on job/result types. Providers receive PAN only as a
//! temporary argument inside a purpose-scoped `with_pan` closure owned by the
//! application service — never via LLM/agent context.

mod bigshare;
pub mod http;
mod job;
mod kfintech;
mod kfintech_live;
mod mufg_intime;
mod profit;
mod provider;
mod registry;
mod runtime;
mod status;

pub use bigshare::{BigshareIssue, BigshareProvider};
pub use job::{
    AllotmentCheckAttempt, AllotmentCheckJob, AllotmentJobError, AllotmentJobStatus,
    AllotmentResultSource, AttemptStatus, ManualReportedOutcome, ManualResultInput,
};
pub use kfintech::{KfintechIssue, KfintechProvider};
pub use kfintech_live::{DiscoveredIssue, LiveKfintechProvider};
pub use mufg_intime::{
    MufgCaptchaState, MufgEphemeralSession, MufgIntimeProvider, MufgIssue, MufgPublicPrecheck,
};
pub use profit::{EstimatedProfit, ProfitPriceBasis};
pub use provider::{
    AllotmentLookupContext, AllotmentProvider, BackgroundExecution, FixtureKfintechProvider,
    HumanVerificationChallenge, HumanVerificationRequirement, HumanVerificationStatus,
    HumanVerificationType, IssueDiscoveryMode, LookupKeyKind, NegativeResultProof,
    NegativeResultProofError, ProviderAllotmentResult, ProviderCapabilities,
    ProviderContinuationReference, ProviderError, ProviderHealth, ProviderResultProvenance,
    ProviderTransportKind, RealInvestorLookupPermit, RegistrarIssue, SafeProviderMetadataError,
    SanitizedFixtureProvenance, SessionRequirement,
};
pub use registry::{ProviderDescriptor, ProviderId, ProviderRegistry};
pub use runtime::{JobLease, ProviderRateLimiter, ProviderRatePolicy};
pub use status::NormalizedAllotmentStatus;

/// Build readiness and investor-data authorization are separate security gates.
pub const LIVE_TRANSPORT_IMPLEMENTED: bool = true;
pub const LIVE_ADAPTER_IMPLEMENTED: bool = LIVE_TRANSPORT_IMPLEMENTED;

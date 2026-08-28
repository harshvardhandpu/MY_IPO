//! Registrar allotment checking — provider contract, jobs, normalized results.
//!
//! PAN never lives on job/result types. Providers receive PAN only as a
//! temporary argument inside a purpose-scoped `with_pan` closure owned by the
//! application service — never via LLM/agent context.

mod job;
mod provider;
mod status;

pub use job::{
    AllotmentCheckAttempt, AllotmentCheckJob, AllotmentJobError, AllotmentJobStatus,
    AllotmentResultSource, AttemptStatus, ManualResultInput,
};
pub use provider::{
    AllotmentLookupContext, AllotmentProvider, FixtureKfintechProvider, ProviderAllotmentResult,
    ProviderError, ProviderHealth, RegistrarIssue,
};
pub use status::NormalizedAllotmentStatus;

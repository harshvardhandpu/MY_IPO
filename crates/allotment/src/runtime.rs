//! Per-provider rate limiting and retry policy (sequential-friendly).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ProviderId;
use crate::status::NormalizedAllotmentStatus;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRatePolicy {
    pub min_interval_ms: u64,
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for ProviderRatePolicy {
    fn default() -> Self {
        Self {
            min_interval_ms: 1_500,
            max_attempts: 5,
            base_backoff_ms: 2_000,
            max_backoff_ms: 60_000,
        }
    }
}

impl ProviderRatePolicy {
    pub const fn for_provider(provider: ProviderId) -> Self {
        match provider {
            ProviderId::KfintechFixture => Self {
                min_interval_ms: 0,
                max_attempts: 1,
                base_backoff_ms: 0,
                max_backoff_ms: 0,
            },
            ProviderId::KfintechLive => Self {
                min_interval_ms: 1_500,
                max_attempts: 3,
                base_backoff_ms: 2_000,
                max_backoff_ms: 60_000,
            },
            ProviderId::BigshareLive => Self {
                min_interval_ms: 1_500,
                max_attempts: 1,
                base_backoff_ms: 0,
                max_backoff_ms: 0,
            },
            ProviderId::MufgIntimeLive => Self {
                min_interval_ms: 1_500,
                max_attempts: 2,
                base_backoff_ms: 2_000,
                max_backoff_ms: 60_000,
            },
        }
    }

    pub fn next_backoff_ms(&self, attempt_count: u32) -> u64 {
        let exp = attempt_count.saturating_sub(1).min(8);
        let raw = self.base_backoff_ms.saturating_mul(1u64 << exp);
        raw.min(self.max_backoff_ms)
    }

    pub fn should_retry(&self, status: NormalizedAllotmentStatus, attempt_count: u32) -> bool {
        status.is_retryable() && attempt_count < self.max_attempts
    }
}

/// Process-local limiter: one provider at a time with min spacing.
/// Policies are per provider id (Gate 4F condition B): `wait_turn` selects
/// the policy registered for that provider, never one global Default.
pub struct ProviderRateLimiter {
    inner: Mutex<HashMap<String, Instant>>,
    policy: Mutex<HashMap<String, ProviderRatePolicy>>,
}

impl ProviderRateLimiter {
    pub fn new(policy: ProviderRatePolicy) -> Self {
        let mut policies = HashMap::new();
        policies.insert(String::new(), policy);
        Self {
            inner: Mutex::new(HashMap::new()),
            policy: Mutex::new(policies),
        }
    }

    /// Register (or replace) the policy for a provider id.
    pub fn set_policy(&self, provider_id: &str, policy: ProviderRatePolicy) {
        let mut map = self.policy.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(provider_id.to_owned(), policy);
    }

    /// The policy that governs a provider id; falls back to the
    /// default-keyed policy (set at construction) if none registered.
    pub fn policy_for(&self, provider_id: &str) -> ProviderRatePolicy {
        let map = self.policy.lock().unwrap_or_else(|e| e.into_inner());
        map.get(provider_id)
            .or_else(|| map.get(""))
            .cloned()
            .unwrap_or_default()
    }

    /// Blocks until the provider may fire (simple sleep-based gate).
    pub fn wait_turn(&self, provider_id: &str) {
        let min = {
            let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let min_interval = self.policy_for(provider_id).min_interval_ms;
            match map.get(provider_id) {
                Some(last) => Duration::from_millis(min_interval)
                    .checked_sub(last.elapsed())
                    .unwrap_or_default(),
                None => Duration::ZERO,
            }
        };
        if !min.is_zero() {
            std::thread::sleep(min);
        }
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(provider_id.to_owned(), Instant::now());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobLease {
    pub job_id: String,
    pub owner_device_id: String,
    pub lease_token: String,
    pub expires_at_epoch_secs: u64,
}

impl JobLease {
    pub fn new(
        job_id: impl Into<String>,
        owner_device_id: impl Into<String>,
        ttl_secs: u64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            job_id: job_id.into(),
            owner_device_id: owner_device_id.into(),
            lease_token: uuid::Uuid::now_v7().to_string(),
            expires_at_epoch_secs: now.saturating_add(ttl_secs),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= self.expires_at_epoch_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_caps() {
        let p = ProviderRatePolicy::default();
        assert!(p.next_backoff_ms(1) <= p.max_backoff_ms);
        assert!(p.next_backoff_ms(20) == p.max_backoff_ms);
    }

    #[test]
    fn final_not_retried() {
        let p = ProviderRatePolicy::default();
        assert!(!p.should_retry(NormalizedAllotmentStatus::Allotted, 1));
        assert!(p.should_retry(NormalizedAllotmentStatus::RateLimited, 1));
    }
}

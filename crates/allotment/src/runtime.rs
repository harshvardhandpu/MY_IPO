//! Per-provider rate limiting and retry policy (sequential-friendly).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::status::NormalizedAllotmentStatus;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
pub struct ProviderRateLimiter {
    inner: Mutex<HashMap<String, Instant>>,
    policy: ProviderRatePolicy,
}

impl ProviderRateLimiter {
    pub fn new(policy: ProviderRatePolicy) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            policy,
        }
    }

    pub fn policy(&self) -> &ProviderRatePolicy {
        &self.policy
    }

    /// Blocks until the provider may fire (simple sleep-based gate).
    pub fn wait_turn(&self, provider_id: &str) {
        let wait = {
            let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(last) = map.get(provider_id) {
                let elapsed = last.elapsed();
                let min = Duration::from_millis(self.policy.min_interval_ms);
                min.checked_sub(elapsed).unwrap_or_default()
            } else {
                Duration::ZERO
            }
        };
        if !wait.is_zero() {
            std::thread::sleep(wait);
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

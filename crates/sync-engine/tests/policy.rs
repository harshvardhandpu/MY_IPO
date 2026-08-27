use std::time::Duration;

use sanket_sync_engine::SyncPolicy;

#[test]
fn default_sync_policy_stays_inside_product_latency_bounds() {
    let policy = SyncPolicy::default();

    assert!(policy.push_debounce >= Duration::from_secs(3));
    assert!(policy.push_debounce <= Duration::from_secs(10));
    assert!(policy.active_fetch_interval >= Duration::from_secs(15));
    assert!(policy.active_fetch_interval <= Duration::from_secs(30));
}

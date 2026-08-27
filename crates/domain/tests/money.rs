//! Phase 2B: deterministic integer accounting — paise and basis points.

use sanket_domain::{BasisPoints, Money};

#[test]
fn money_is_integer_paise() {
    let m = Money::from_paise(123_456);
    assert_eq!(m.paise(), 123_456);
    assert_eq!(m.to_string(), "₹1234.56");
}

#[test]
fn money_from_rupees_converts_exactly() {
    assert_eq!(Money::from_rupees(1_500).paise(), 150_000);
    assert_eq!(Money::from_rupees(0).paise(), 0);
}

#[test]
fn money_addition_is_exact() {
    let a = Money::from_paise(199);
    let b = Money::from_paise(1);
    assert_eq!((a + b).paise(), 200);
}

#[test]
fn money_checked_subtraction_fails_closed_on_negative() {
    let a = Money::from_paise(100);
    let b = Money::from_paise(101);
    assert!(a.checked_sub(b).is_none());
    assert_eq!(a.checked_sub(Money::from_paise(100)).unwrap().paise(), 0);
}

#[test]
fn money_checked_add_detects_overflow() {
    let a = Money::from_paise(i64::MAX);
    assert!(a.checked_add(Money::from_paise(1)).is_none());
}

#[test]
fn basis_points_share_is_deterministic_integer_math() {
    // 10% of ₹15,000.00 = ₹1,500.00 exactly.
    let amount = Money::from_rupees(15_000);
    let share = BasisPoints::new(1_000).share_of(amount);
    assert_eq!(share.paise(), 150_000);
}

#[test]
fn basis_points_share_truncates_remainder_deterministically() {
    // 10% of 101 paise = 10.1 → truncates to 10 (never rounds up, never floats).
    let amount = Money::from_paise(101);
    let share = BasisPoints::new(1_000).share_of(amount);
    assert_eq!(share.paise(), 10);
}

#[test]
fn basis_points_full_and_zero_shares() {
    let amount = Money::from_paise(12_345);
    assert_eq!(BasisPoints::new(10_000).share_of(amount).paise(), 12_345);
    assert_eq!(BasisPoints::new(0).share_of(amount).paise(), 0);
}

#[test]
fn basis_points_rejects_out_of_range() {
    assert!(BasisPoints::try_new(-1).is_err());
    assert!(BasisPoints::try_new(10_001).is_err());
    assert!(BasisPoints::try_new(10_000).is_ok());
    assert!(BasisPoints::try_new(0).is_ok());
}

#[test]
fn friend_share_default_is_ten_percent() {
    assert_eq!(BasisPoints::friend_share_default().value(), 1_000);
}

#[test]
fn share_split_never_exceeds_source_amount() {
    // Deterministic invariant: for any amount, share + remainder == amount.
    for paise in [0, 1, 99, 100, 101, 9_999, 12_345, 1_000_000_001] {
        let amount = Money::from_paise(paise);
        let share = BasisPoints::new(1_000).share_of(amount);
        let remainder = amount.checked_sub(share).expect("share <= amount");
        assert_eq!(share.paise() + remainder.paise(), paise);
    }
}

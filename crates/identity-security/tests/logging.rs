//! Logging-security tests: sensitive values must never surface in any error,
//! display, or debug path — including when validation fails on bad input.

use sanket_identity_security::Pan;

/// A failure path is a common leak point: code that formats the offending input
/// into an error string. These tests prove the PAN error type does not echo the
/// raw value, and that a rejected input never appears in `Debug`/`Display`.
#[test]
fn pan_validation_error_does_not_echo_input() {
    for bad in ["ABCDE1234", "not-a-pan", "ABCDE12345"] {
        let err = Pan::parse(bad).unwrap_err();
        let msg = format!("{err}");
        let dbg = format!("{err:?}");
        // The error message is a fixed description; it never contains the input.
        assert!(!msg.contains(bad), "error Display echoed input {bad:?}");
        assert!(!dbg.contains(bad), "error Debug echoed input {bad:?}");
    }
}

#[test]
fn pan_error_is_static_description() {
    // `PanError` carries no data, so Debug cannot differ per input. Confirm.
    let a = Pan::parse("AAAAA").unwrap_err();
    let b = Pan::parse("ZZZZZ").unwrap_err();
    assert_eq!(format!("{a}"), format!("{b}"));
}

#[test]
fn masked_pan_never_contains_interior_digits() {
    let pan = Pan::parse(&(format!("{}1234{}", "ABCDE", "F"))).unwrap();
    let masked = pan.mask().to_string();
    assert_eq!(masked, "ABCDE****F");
    let digits_revealed: String = masked.chars().filter(|c| c.is_ascii_digit()).collect();
    assert!(digits_revealed.is_empty(), "masked PAN leaked digits");
}

#[test]
fn redacted_is_identical_regardless_of_value() {
    // Two distinct secrets must render identically, so logs cannot leak structure.
    use sanket_identity_security::Redacted;
    let x = Redacted::new("alpha".to_string());
    let y = Redacted::new("beta".to_string());
    assert_eq!(format!("{x}"), format!("{y}"));
    assert_eq!(format!("{x}"), "[REDACTED]");
}

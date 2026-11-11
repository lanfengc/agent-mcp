use super::{DEFAULT_MAX_CAPTURE_AGE_MS, parse_max_capture_age};

#[test]
fn default_capture_age_is_ten_seconds() {
    assert_eq!(DEFAULT_MAX_CAPTURE_AGE_MS, 10_000);
}

#[test]
fn accepts_a_positive_millisecond_value() {
    assert_eq!(parse_max_capture_age("120000").unwrap(), 120_000);
    assert_eq!(parse_max_capture_age(" 250 ").unwrap(), 250);
}

#[test]
fn rejects_zero_and_values_above_the_limit() {
    assert!(parse_max_capture_age("0").is_err());
    assert!(parse_max_capture_age("86400001").is_err());
}

#[test]
fn rejects_a_non_numeric_value() {
    assert!(parse_max_capture_age("two minutes").is_err());
}

use super::*;

#[test]
fn versions_reject_a_fourth_part() {
    assert_eq!(version::parse_tag("v1.2.3.4"), Err(UpdateError::Invalid));
    assert_eq!(version::parse_current("1.2.3.4"), Err(UpdateError::Invalid));
}

#[test]
fn versions_reject_signs_spaces_and_other_non_digits() {
    for value in ["v1.2.+3", "v1.2.-3", "v1.2.x", "v1.+2.3"] {
        assert_eq!(version::parse_tag(value), Err(UpdateError::Invalid));
    }
    for value in ["1.2.+3", "1.2.-3", "1.2.x", "1.+2.3", " 1.2.3", "1.2.3 "] {
        assert_eq!(version::parse_current(value), Err(UpdateError::Invalid));
    }
}

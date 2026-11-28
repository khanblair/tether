use super::*;

#[test]
fn setup_reason_replaces_newlines_and_trims() {
    let reason = setup_reason(&tether_linux::LinuxError::Fetch(
        "  first line\nsecond line\r\n  ".into(),
    ));

    assert_eq!(
        reason,
        "Linux root filesystem download failed:   first line second line"
    );
}

#[test]
fn setup_reason_cuts_at_three_hundred_characters() {
    let reason = setup_reason(&tether_linux::LinuxError::Fetch(format!(
        "{}界{}",
        "a".repeat(260),
        "b".repeat(139),
    )));

    assert_eq!(reason.chars().count(), SETUP_REASON_LIMIT);
    assert!(reason.ends_with('界'));
}

#[test]
fn setup_reason_keeps_short_reasons_whole() {
    let reason = setup_reason(&tether_linux::LinuxError::Fetch("download failed".into()));

    assert_eq!(
        reason,
        "Linux root filesystem download failed: download failed"
    );
}

#[test]
fn setup_reason_packages_keeps_the_last_280_characters() {
    let output = format!("{}界{}", "h".repeat(320), "t".repeat(279));
    let reason = setup_reason(&tether_linux::LinuxError::Packages(output));

    assert!(reason.starts_with("base packages failed: 界"));
    assert!(reason.ends_with(&"t".repeat(279)));
    assert!(!reason.contains('h'));
    assert_eq!(
        reason.chars().count(),
        "base packages failed: ".chars().count() + 280
    );
}

#[test]
fn setup_fetch_failure_keeps_its_reason() {
    let result = map_setup_result(Err(tether_linux::LinuxError::Fetch("HTTP 503".into())));

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::SetupFailed { reason })
            if reason.contains("HTTP 503")
    ));
}

#[test]
fn setup_signature_failure_stays_verification_failure() {
    let result = map_setup_result(Err(tether_linux::LinuxError::Signature));

    assert!(matches!(
        result,
        Err(ExecutorRuntimeError::VerificationFailed)
    ));
}

#[test]
fn key_parser_decodes_lower_and_upper_hex() {
    let value = key_from_hex(Some(
        "00112233445566778899aabbccddeeff00112233445566778899AABBCCDDEEFF",
    ))
    .unwrap();
    assert_eq!(value[0], 0x00);
    assert_eq!(value[15], 0xff);
    assert_eq!(value[31], 0xff);
}

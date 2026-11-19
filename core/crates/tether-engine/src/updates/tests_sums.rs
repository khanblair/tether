use super::*;

#[test]
fn sums_reject_digests_that_are_not_exactly_64_hex_characters() {
    for digest in ["a".repeat(63), "a".repeat(65)] {
        let body = format!("{digest}  app.apk\n");
        assert_eq!(
            sums::digest_for(body.as_bytes(), "app.apk"),
            Err(UpdateError::Invalid)
        );
    }
}

#[test]
fn sums_decode_uppercase_and_mixed_case_hex_to_exact_bytes() {
    let expected = [
        0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
        0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
        0x67, 0x89,
    ];
    for digest in [
        "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
        "aBcDeF0123456789aBcDeF0123456789aBcDeF0123456789aBcDeF0123456789",
    ] {
        let body = format!("{digest}  app.apk\n");
        assert_eq!(sums::digest_for(body.as_bytes(), "app.apk"), Ok(expected));
    }
}

//! Webhook signature verification.
//!
//! A webhook endpoint that does not verify its payloads is an open door: anyone
//! who can reach the URL can tell Pulse that a build finished, and Pulse would
//! believe it.
//!
//! Two rules matter, and both are the kind that look like paranoia until they
//! are not:
//!
//! 1. **Verify the raw bytes, before parsing.** Any deserialise-then-reserialise
//!    round trip can change the bytes, and then a valid signature fails — or,
//!    worse, a re-encoded body passes for the wrong payload.
//! 2. **Compare in constant time.** A byte-by-byte comparison leaks how much of
//!    a forged signature was correct, which is enough to forge one.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Header GitHub sends, as `sha256=<hex>`.
pub const SIGNATURE_HEADER: &str = "x-hub-signature-256";

/// GitHub's own documented ceiling for a webhook payload.
pub const MAX_BODY_BYTES: usize = 25 * 1024 * 1024;

/// Length of a generated signing secret, in bytes.
const SECRET_BYTES: usize = 32;

/// Verifies `sha256=<hex>` against the raw body.
///
/// Returns `false` for anything malformed: a missing prefix, non-hex, a
/// truncated digest. There is no path that returns `true` without a full match.
#[must_use]
pub fn verify(secret: &[u8], body: &[u8], signature_header: &str) -> bool {
    let Some(encoded) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    let Ok(expected) = hex::decode(encoded) else {
        return false;
    };

    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(body);

    let computed = mac.finalize().into_bytes();

    // `ct_eq` returns a `Choice`; comparing lengths first is fine because the
    // length is not secret.
    computed.len() == expected.len() && bool::from(computed.as_slice().ct_eq(&expected))
}

/// A fresh signing secret, hex-encoded.
///
/// # Errors
///
/// Returns the OS error if no entropy source is available. That is worth
/// surfacing rather than substituting something predictable.
pub fn generate_secret() -> Result<String, crate::error::AppError> {
    let mut bytes = [0_u8; SECRET_BYTES];

    getrandom::fill(&mut bytes).map_err(|error| {
        crate::error::AppError::Internal(format!("cannot read from the OS entropy source: {error}"))
    })?;

    Ok(hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(secret: &[u8], body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret).expect("any key length");
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn accepts_a_correctly_signed_body() {
        let secret = b"hunter2";
        let body = br#"{"action":"completed"}"#;

        assert!(verify(secret, body, &sign(secret, body)));
    }

    #[test]
    fn rejects_a_tampered_body() {
        let secret = b"hunter2";
        let signature = sign(secret, br#"{"action":"completed"}"#);

        assert!(!verify(secret, br#"{"action":"cancelled"}"#, &signature));
    }

    #[test]
    fn rejects_the_wrong_secret() {
        let body = b"payload";

        assert!(!verify(b"correct", body, &sign(b"wrong", body)));
    }

    #[test]
    fn rejects_a_missing_prefix() {
        let secret = b"hunter2";
        let body = b"payload";
        let bare = sign(secret, body).trim_start_matches("sha256=").to_owned();

        assert!(!verify(secret, body, &bare));
    }

    #[test]
    fn rejects_non_hex_and_truncated_digests() {
        let secret = b"hunter2";
        let body = b"payload";
        let full = sign(secret, body);

        assert!(!verify(secret, body, "sha256=not hex at all"));
        assert!(!verify(secret, body, "sha256="));
        assert!(!verify(secret, body, &full[..20]));
        assert!(!verify(secret, body, ""));
    }

    #[test]
    fn a_single_flipped_character_is_rejected() {
        let secret = b"hunter2";
        let body = b"payload";
        let mut signature = sign(secret, body);

        // Flip the last hex digit.
        let last = signature.pop().expect("non-empty");
        signature.push(if last == '0' { '1' } else { '0' });

        assert!(!verify(secret, body, &signature));
    }

    #[test]
    fn generated_secrets_are_long_and_unique() {
        let first = generate_secret().expect("entropy");
        let second = generate_secret().expect("entropy");

        assert_eq!(first.len(), SECRET_BYTES * 2);
        assert_ne!(first, second, "two secrets must not collide");
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn the_body_cap_matches_github() {
        // GitHub refuses to deliver anything larger; if this drifts below that,
        // we silently drop valid deliveries.
        assert_eq!(MAX_BODY_BYTES, 25 * 1024 * 1024);
    }
}

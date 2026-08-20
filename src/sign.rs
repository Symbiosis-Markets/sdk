//! The API key request signing scheme.
//!
//! Exposed so callers building their own transport can produce valid
//! signatures. [`crate::Client`] signs automatically.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// `timestamp_ms "\n" METHOD "\n" PATH_AND_QUERY "\n" BODY`
///
/// `path_and_query` is the full request target including the query string,
/// and `body` is the exact bytes sent on the wire.
pub fn canonical_message(
    timestamp_ms: i64,
    method: &str,
    path_and_query: &str,
    body: &[u8],
) -> Vec<u8> {
    let ts = timestamp_ms.to_string();

    let mut msg =
        Vec::with_capacity(ts.len() + method.len() + path_and_query.len() + body.len() + 3);

    msg.extend_from_slice(ts.as_bytes());
    msg.push(b'\n');
    msg.extend_from_slice(method.as_bytes());
    msg.push(b'\n');
    msg.extend_from_slice(path_and_query.as_bytes());
    msg.push(b'\n');
    msg.extend_from_slice(body);

    msg
}

/// Base64-encoded HMAC-SHA256 of `message`, keyed with the API key secret.
pub fn sign(secret: &[u8], message: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts a key of any length");
    mac.update(message);

    BASE64.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_vector() {
        let msg = canonical_message(
            1_700_000_000_000,
            "POST",
            "/rfq/accept?x=1",
            b"{\"offer_id\":\"a\"}",
        );

        assert_eq!(
            sign(b"test-secret", &msg),
            "4Gym2IFbVBXsVJriS0hlRZS5dk2EKLPg0ZSsem7BDCI="
        );
    }

    #[test]
    fn canonical_message_layout() {
        let msg = canonical_message(1, "GET", "/a?b=c", b"body");

        assert_eq!(msg, b"1\nGET\n/a?b=c\nbody");
    }
}

//! PKCE (Proof Key for Code Exchange) verification.
//!
//! Implements S256 code challenge verification per RFC 7636.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// Verify a PKCE S256 code challenge.
///
/// Computes `BASE64URL(SHA256(code_verifier))` and compares to the stored challenge.
pub fn verify_s256(code_verifier: &str, code_challenge: &str) -> bool {
    let hash = Sha256::digest(code_verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(hash);
    computed == code_challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s256_valid() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_s256(verifier, challenge));
    }

    #[test]
    fn test_s256_invalid_verifier() {
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(!verify_s256("wrong-verifier", challenge));
    }

    #[test]
    fn test_s256_invalid_challenge() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert!(!verify_s256(verifier, "wrong-challenge"));
    }

    #[test]
    fn test_s256_roundtrip() {
        let verifier = "a]random/verifier_string.with";
        let hash = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hash);
        assert!(verify_s256(verifier, &challenge));
    }
}

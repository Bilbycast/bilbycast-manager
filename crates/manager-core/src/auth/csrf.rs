use rand::RngExt;

/// Generate a random CSRF token (32 hex characters).
pub fn generate_csrf_token() -> String {
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill(bytes.as_mut_slice());
    hex_encode(&bytes)
}

/// Constant-time comparison for CSRF tokens.
pub fn verify_csrf_token(expected: &str, provided: &str) -> bool {
    if expected.len() != provided.len() {
        return false;
    }
    let mut result = 0u8;
    for (a, b) in expected.bytes().zip(provided.bytes()) {
        result |= a ^ b;
    }
    result == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let token = generate_csrf_token();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_csrf_verification() {
        let token = generate_csrf_token();
        assert!(verify_csrf_token(&token, &token));
        assert!(!verify_csrf_token(&token, "different_token_value_12345678"));
    }
}

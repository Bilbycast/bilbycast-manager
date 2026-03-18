use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};

/// Hash a password using argon2id.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes())?;
    Ok(hash.to_string())
}

/// Verify a password against an argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

/// Validate password strength requirements.
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters");
    }
    if password.len() > 128 {
        return Err("Password must be at most 128 characters");
    }
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !(has_upper && has_lower && has_digit) {
        return Err("Password must contain uppercase, lowercase, and a digit");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "TestPass123!";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("WrongPass123!", &hash).unwrap());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("Abcdefg1").is_ok());
        assert!(validate_password("short").is_err());
        assert!(validate_password("alllowercase1").is_err());
        assert!(validate_password("ALLUPPERCASE1").is_err());
        assert!(validate_password("NoDigitsHere").is_err());
    }
}

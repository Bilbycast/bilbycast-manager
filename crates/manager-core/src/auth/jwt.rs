use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::UserRole;

/// JWT claims for user sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Subject: user ID.
    pub sub: String,
    /// User role.
    pub role: String,
    /// JWT ID: maps to session table for revocation.
    pub jti: String,
    /// Issued at (unix timestamp).
    pub iat: i64,
    /// Expiration (unix timestamp).
    pub exp: i64,
    /// Issuer.
    pub iss: String,
}

/// Create a session JWT for an authenticated user.
pub fn create_session_token(
    user_id: &str,
    role: UserRole,
    session_id: &str,
    secret: &[u8],
    lifetime_hours: u32,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + Duration::hours(lifetime_hours as i64);

    let claims = SessionClaims {
        sub: user_id.to_string(),
        role: role.as_str().to_string(),
        jti: session_id.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        iss: "bilbycast-manager".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

/// Validate and decode a session JWT.
pub fn validate_session_token(
    token: &str,
    secret: &[u8],
) -> Result<SessionClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&["bilbycast-manager"]);
    validation.set_required_spec_claims(&["sub", "role", "jti", "exp", "iss"]);

    let data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation,
    )?;

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_roundtrip() {
        let secret = b"test-secret-key-at-least-32-chars!!";
        let token = create_session_token(
            "user-123",
            UserRole::Admin,
            "session-456",
            secret,
            24,
        )
        .unwrap();

        let claims = validate_session_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.jti, "session-456");
    }

    #[test]
    fn test_jwt_invalid_secret() {
        let secret1 = b"test-secret-key-at-least-32-chars!!";
        let secret2 = b"different-secret-key-at-least-32-ch";
        let token = create_session_token(
            "user-123",
            UserRole::Admin,
            "session-456",
            secret1,
            24,
        )
        .unwrap();

        assert!(validate_session_token(&token, secret2).is_err());
    }
}

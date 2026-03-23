// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

// Copyright (c) 2026 Reza Rahimi. All rights reserved.
// SPDX-License-Identifier: Elastic-2.0

//! Input validation helpers for API request fields.
//! All functions return Ok(()) on success or Err(String) with a user-facing message.

/// Validate a username: 1-64 chars, alphanumeric + underscore + hyphen + dot, must start with alphanumeric.
pub fn validate_username(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > 64 {
        return Err("username must be 1–64 characters".into());
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphanumeric() {
        return Err("username must start with a letter or digit".into());
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
        return Err("username may only contain letters, digits, underscore, hyphen, or dot".into());
    }
    Ok(())
}

/// Validate a password: 8-128 characters.
pub fn validate_password(s: &str) -> Result<(), String> {
    if s.len() < 8 {
        return Err("password must be at least 8 characters".into());
    }
    if s.len() > 128 {
        return Err("password must be at most 128 characters".into());
    }
    Ok(())
}

/// Validate a display name: 1-128 chars, no control characters.
pub fn validate_display_name(s: &str) -> Result<(), String> {
    validate_name(s, "display name", 128)
}

/// Validate a generic name field: 1-max chars, no control characters.
pub fn validate_name(s: &str, field: &str, max: usize) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if s.len() > max {
        return Err(format!("{field} must be at most {max} characters"));
    }
    if s.chars().any(|c| c.is_control()) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(())
}

/// Validate an optional description: max N chars, no control characters (except newline/tab).
pub fn validate_description(s: &str, max: usize) -> Result<(), String> {
    if s.len() > max {
        return Err(format!("description must be at most {max} characters"));
    }
    if s.chars().any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r') {
        return Err("description must not contain control characters".into());
    }
    Ok(())
}

/// Validate an email address: basic format check, max 254 chars.
pub fn validate_email(s: &str) -> Result<(), String> {
    if s.len() > 254 {
        return Err("email must be at most 254 characters".into());
    }
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err("email must be in the format user@domain".into());
    }
    if !parts[1].contains('.') {
        return Err("email domain must contain a dot".into());
    }
    Ok(())
}

/// Validate a setting key against the known whitelist.
pub fn validate_setting_key(key: &str) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "events_retention_days",
        "ws_keepalive_interval_secs",
        "session_lifetime_hours",
        "max_login_attempts",
        "node_offline_threshold_secs",
        "stats_broadcast_interval_ms",
    ];
    if ALLOWED.contains(&key) {
        Ok(())
    } else {
        Err(format!("unknown setting key: {key}"))
    }
}

/// Validate a setting value for a given key.
pub fn validate_setting_value(key: &str, value: &serde_json::Value) -> Result<(), String> {
    let as_i64 = || value.as_i64().ok_or_else(|| format!("{key} must be an integer"));
    match key {
        "events_retention_days" => {
            let v = as_i64()?;
            if !(1..=365).contains(&v) { return Err(format!("{key} must be 1-365, got {v}")); }
        }
        "ws_keepalive_interval_secs" => {
            let v = as_i64()?;
            if !(1..=300).contains(&v) { return Err(format!("{key} must be 1-300, got {v}")); }
        }
        "session_lifetime_hours" => {
            let v = as_i64()?;
            if !(1..=720).contains(&v) { return Err(format!("{key} must be 1-720, got {v}")); }
        }
        "max_login_attempts" => {
            let v = as_i64()?;
            if !(1..=100).contains(&v) { return Err(format!("{key} must be 1-100, got {v}")); }
        }
        "node_offline_threshold_secs" => {
            let v = as_i64()?;
            if !(5..=3600).contains(&v) { return Err(format!("{key} must be 5-3600, got {v}")); }
        }
        "stats_broadcast_interval_ms" => {
            let v = as_i64()?;
            if !(100..=60000).contains(&v) { return Err(format!("{key} must be 100-60000, got {v}")); }
        }
        _ => {}
    }
    Ok(())
}

/// Validate a network address string: non-empty, max 256 chars, contains a colon (host:port).
pub fn validate_addr(s: &str, field: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if s.len() > 256 {
        return Err(format!("{field} must be at most 256 characters"));
    }
    if !s.contains(':') {
        return Err(format!("{field} must be in host:port format"));
    }
    Ok(())
}

/// Validate a generic string has bounded length.
pub fn validate_string_length(s: &str, field: &str, max: usize) -> Result<(), String> {
    if s.len() > max {
        return Err(format!("{field} must be at most {max} characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_usernames() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_1").is_ok());
        assert!(validate_username("a").is_ok());
        assert!(validate_username("test.user-name").is_ok());
    }

    #[test]
    fn test_invalid_usernames() {
        assert!(validate_username("").is_err());
        assert!(validate_username("_admin").is_err());
        assert!(validate_username("-user").is_err());
        assert!(validate_username("user name").is_err());
        assert!(validate_username(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_password_validation() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("short").is_err());
        assert!(validate_password(&"a".repeat(129)).is_err());
    }

    #[test]
    fn test_email_validation() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("a@b.c").is_ok());
        assert!(validate_email("missing-at").is_err());
        assert!(validate_email("no@dot").is_err());
        assert!(validate_email("@empty.com").is_err());
    }

    #[test]
    fn test_setting_key_validation() {
        assert!(validate_setting_key("events_retention_days").is_ok());
        assert!(validate_setting_key("unknown_key").is_err());
    }

    #[test]
    fn test_addr_validation() {
        assert!(validate_addr("192.168.1.1:8080", "addr").is_ok());
        assert!(validate_addr("no-port", "addr").is_err());
        assert!(validate_addr("", "addr").is_err());
    }
}

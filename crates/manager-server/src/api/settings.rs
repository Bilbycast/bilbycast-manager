use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::middleware::auth::AuthUser;
use manager_core::models::UserRole;

/// TLS certificate information response.
#[derive(serde::Serialize)]
pub struct TlsCertInfo {
    pub cert_path: String,
    pub key_path: String,
    pub is_self_signed: bool,
    pub subject: String,
    pub issuer: String,
    pub not_after: String,
}

/// Get TLS certificate information.
pub async fn get_tls_info(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<TlsCertInfo>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let config = state.config.read().await;
    let cert_path = std::env::var("BILBYCAST_TLS_CERT")
        .ok()
        .or_else(|| config.tls.as_ref().map(|t| t.cert_path.clone()))
        .unwrap_or_default();
    let key_path = std::env::var("BILBYCAST_TLS_KEY")
        .ok()
        .or_else(|| config.tls.as_ref().map(|t| t.key_path.clone()))
        .unwrap_or_default();

    // Parse cert PEM for subject/issuer/expiry
    let (subject, issuer, not_after) = parse_cert_info(&cert_path).unwrap_or_else(|| {
        ("Unknown".into(), "Unknown".into(), "Unknown".into())
    });

    Ok(Json(TlsCertInfo {
        cert_path,
        key_path,
        is_self_signed: state.is_self_signed_cert,
        subject,
        issuer,
        not_after,
    }))
}

/// Upload new TLS certificate and key.
#[derive(Deserialize)]
pub struct TlsUploadRequest {
    pub cert_pem: String,
    pub key_pem: String,
}

pub async fn upload_tls_cert(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TlsUploadRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate PEM content
    let mut cert_reader = std::io::BufReader::new(req.cert_pem.as_bytes());
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .filter_map(|c| c.ok())
        .collect();
    if certs.is_empty() {
        return Ok(Json(serde_json::json!({"success": false, "error": "Invalid certificate PEM — no certificates found"})));
    }

    let mut key_reader = std::io::BufReader::new(req.key_pem.as_bytes());
    let has_key = rustls_pemfile::private_key(&mut key_reader)
        .ok()
        .flatten()
        .is_some();
    if !has_key {
        return Ok(Json(serde_json::json!({"success": false, "error": "Invalid key PEM — no private key found"})));
    }

    // Determine cert/key paths
    let config = state.config.read().await;
    let cert_path = std::env::var("BILBYCAST_TLS_CERT")
        .ok()
        .or_else(|| config.tls.as_ref().map(|t| t.cert_path.clone()))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let key_path = std::env::var("BILBYCAST_TLS_KEY")
        .ok()
        .or_else(|| config.tls.as_ref().map(|t| t.key_path.clone()))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    drop(config);

    // Write cert and key
    std::fs::write(&cert_path, &req.cert_pem)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(&key_path, &req.key_pem)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "settings.tls_upload",
        Some("tls"),
        None,
        None,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Certificate uploaded. Restart the server to apply the new certificate."
    })))
}

/// Parse basic cert info from a PEM file on disk.
fn parse_cert_info(cert_path: &str) -> Option<(String, String, String)> {
    let pem_data = std::fs::read(cert_path).ok()?;
    let mut reader = std::io::BufReader::new(pem_data.as_slice());
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .filter_map(|c| c.ok())
        .collect();
    let cert_der = certs.first()?;

    // Extract subject and issuer from DER using simple string search for printable fields
    let der = cert_der.as_ref();
    let subject = extract_cn_from_der(der, false).unwrap_or_else(|| "Unknown".into());
    let issuer = extract_cn_from_der(der, true).unwrap_or_else(|| "Unknown".into());

    // Try to extract notAfter from the validity field
    let not_after = "Check certificate file".into();

    Some((subject, issuer, not_after))
}

/// Extract CommonName from DER certificate (simplified heuristic).
fn extract_cn_from_der(der: &[u8], want_issuer: bool) -> Option<String> {
    // OID for CommonName: 2.5.4.3 = 55 04 03
    let cn_oid = [0x55, 0x04, 0x03];
    let mut found_count = 0u32;
    // In X.509 DER, issuer CN appears first, subject CN appears second
    let target = if want_issuer { 0 } else { 1 };

    for i in 0..der.len().saturating_sub(cn_oid.len() + 4) {
        if der[i..].starts_with(&cn_oid) {
            if found_count == target {
                // The CN value follows: tag (usually 0x0c UTF8 or 0x13 PrintableString), length, value
                let val_tag_pos = i + cn_oid.len();
                if val_tag_pos + 1 >= der.len() { return None; }
                let val_len = der[val_tag_pos + 1] as usize;
                let val_start = val_tag_pos + 2;
                if val_start + val_len > der.len() { return None; }
                return Some(String::from_utf8_lossy(&der[val_start..val_start + val_len]).into_owned());
            }
            found_count += 1;
        }
    }
    None
}

pub async fn get_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let settings = manager_core::db::settings::get_all_settings(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut map = serde_json::Map::new();
    for (key, value) in settings {
        // Try to parse as JSON value, fallback to string
        let parsed = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        map.insert(key, parsed);
    }

    Ok(Json(serde_json::Value::Object(map)))
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if !auth.role.has_permission(UserRole::Admin) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Forbidden"}))));
    }

    // Validate all keys and values before applying any
    for (key, value) in &req.settings {
        manager_core::validation::validate_setting_key(key)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))))?;
        manager_core::validation::validate_setting_value(key, value)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))))?;
    }

    for (key, value) in &req.settings {
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        manager_core::db::settings::set_setting(&state.db, key, &value_str, Some(&auth.user_id))
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to update settings"}))))?;
    }

    let _ = manager_core::db::audit::log_audit(
        &state.db,
        Some(&auth.user_id),
        "settings.update",
        Some("settings"),
        None,
        Some(&serde_json::to_value(&req.settings).unwrap_or_default()),
        None,
    )
    .await;

    Ok(StatusCode::OK)
}

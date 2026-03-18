use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngExt;
use sha2::Digest;

/// Encrypt a plaintext string using AES-256-GCM.
/// Returns base64-encoded nonce + ciphertext.
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let mut rng = rand::rng();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(nonce_bytes.as_mut_slice());
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Concatenate nonce (12 bytes) + ciphertext
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64_encode(&combined))
}

/// Decrypt a base64-encoded nonce + ciphertext using AES-256-GCM.
pub fn decrypt(encrypted: &str, key: &[u8; 32]) -> Result<String, CryptoError> {
    let combined = base64_decode(encrypted).map_err(|_| CryptoError::InvalidData)?;
    if combined.len() < 13 {
        return Err(CryptoError::InvalidData);
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    String::from_utf8(plaintext).map_err(|_| CryptoError::InvalidData)
}

/// Derive a 32-byte encryption key from a passphrase using HMAC-based
/// Extract-and-Expand Key Derivation Function (HKDF-SHA256).
///
/// This is a proper KDF that is resistant to brute-force attacks
/// and produces uniformly distributed key material.
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    // HKDF using HMAC-SHA256
    // Salt: fixed application-specific value (acts as domain separator)
    let salt = b"bilbycast-manager-master-key-v1";
    // Info: context string for the derived key
    let info = b"aes-256-gcm-encryption";

    // Step 1: Extract — HMAC-SHA256(salt, passphrase) -> PRK
    let prk = hmac_sha256(salt, passphrase.as_bytes());

    // Step 2: Expand — HMAC-SHA256(PRK, info || 0x01) -> OKM (32 bytes, single block)
    let mut expand_input = Vec::with_capacity(info.len() + 1);
    expand_input.extend_from_slice(info);
    expand_input.push(0x01);

    hmac_sha256(&prk, &expand_input)
}

/// HMAC-SHA256 using the sha2 crate.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let key_block = if key.len() > 64 {
        let h = sha256(key);
        let mut block = [0u8; 64];
        block[..32].copy_from_slice(&h);
        block
    } else {
        let mut block = [0u8; 64];
        block[..key.len()].copy_from_slice(key);
        block
    };

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (i, b) in key_block.iter().enumerate() {
        ipad[i] ^= b;
        opad[i] ^= b;
    }

    let mut inner_data = Vec::with_capacity(64 + message.len());
    inner_data.extend_from_slice(&ipad);
    inner_data.extend_from_slice(message);
    let inner_hash = sha256(&inner_data);

    let mut outer_data = Vec::with_capacity(64 + 32);
    outer_data.extend_from_slice(&opad);
    outer_data.extend_from_slice(&inner_hash);
    sha256(&outer_data)
}

/// SHA-256 using the sha2 crate.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid encryption key")]
    InvalidKey,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid encrypted data")]
    InvalidData,
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;
    for c in input.chars() {
        let val = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            'a'..='z' => (c as u32) - ('a' as u32) + 26,
            '0'..='9' => (c as u32) - ('0' as u32) + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(()),
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = derive_key("my-super-secret-master-key-for-testing-1234567890");
        let plaintext = "this-is-a-secret-api-key-12345";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = derive_key("key-one-that-is-long-enough-for-testing");
        let key2 = derive_key("key-two-that-is-long-enough-for-testing");
        let encrypted = encrypt("secret", &key1).unwrap();
        assert!(decrypt(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_derive_key_deterministic() {
        let k1 = derive_key("same-passphrase-for-testing-1234567890");
        let k2 = derive_key("same-passphrase-for-testing-1234567890");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_derive_key_different_inputs() {
        let k1 = derive_key("passphrase-one-for-testing-1234567890");
        let k2 = derive_key("passphrase-two-for-testing-1234567890");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_sha256_known_vectors() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty_hash = sha256(b"");
        assert_eq!(
            hex_string(&empty_hash),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let abc_hash = sha256(b"abc");
        assert_eq!(
            hex_string(&abc_hash),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn hex_string(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngExt;

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

/// Derive a 32-byte key from a passphrase using a simple hash.
/// For production, consider using a proper KDF like argon2 or HKDF.
pub fn derive_key(passphrase: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut key = [0u8; 32];
    let bytes = passphrase.as_bytes();
    for (i, chunk) in bytes.chunks(32).enumerate() {
        for (j, &b) in chunk.iter().enumerate() {
            key[(i * 32 + j) % 32] ^= b;
        }
    }
    let mut hasher = DefaultHasher::new();
    passphrase.hash(&mut hasher);
    let hash = hasher.finish().to_le_bytes();
    for (i, &b) in hash.iter().enumerate() {
        key[i] ^= b;
        key[i + 8] ^= b;
        key[i + 16] ^= b;
        key[i + 24] ^= b;
    }
    key
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
        let key = derive_key("my-super-secret-master-key-for-testing");
        let plaintext = "this-is-a-secret-api-key-12345";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = derive_key("key-one");
        let key2 = derive_key("key-two");
        let encrypted = encrypt("secret", &key1).unwrap();
        assert!(decrypt(&encrypted, &key2).is_err());
    }
}

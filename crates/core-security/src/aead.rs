//! AEAD-at-Rest encryption: `XChaCha20-Poly1305` (Spec §4, §7).
//!
//! Principle: any export of data to the outside is wrapped in cryptographic
//! encryption directly during writing. The backup file is initially born
//! encrypted — this minimizes the TOCTOU window where staging contains plaintext.
//!
//! Format: nonce (24 bytes) || ciphertext || tag (16 bytes).
//! The nonce is generated cryptographically randomly for each operation.

use crate::error::SecurityError;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, Key, KeyInit, Payload},
};
use rand::Rng;

/// Nonce length for XChaCha20-Poly1305 (24 bytes).
pub const NONCE_LEN: usize = 24;

/// Container for the symmetric encryption key with zeroing on Drop.
///
/// The key resides in memory only until the moment of use, after which it is
/// zeroed via `zeroize` (Spec §10 "Total use of Zeroizing").
pub struct EncryptionKey {
    /// Raw key bytes (32 bytes for XChaCha20).
    bytes: Box<[u8; 32]>,
}

impl EncryptionKey {
    /// Generates a new random key (32 bytes).
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = Box::new([0u8; 32]);
        rand::rng().fill_bytes(&mut *bytes);
        Self { bytes }
    }

    /// Creates a key from 32 bytes (e.g., from a KDF).
    ///
    /// # Errors
    ///
    /// - [`SecurityError::InvalidKeyLength`] — length ≠ 32.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SecurityError> {
        if bytes.len() != 32 {
            return Err(SecurityError::InvalidKeyLength {
                actual: bytes.len(),
                expected: 32,
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self {
            bytes: Box::new(arr),
        })
    }

    /// Returns a reference to the raw key bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

/// Encrypts plaintext with AAD (associated data) via XChaCha20-Poly1305.
///
/// Returns `nonce ‖ ciphertext+tag`. The nonce is generated randomly and
/// included in the output — the recipient needs only the key to decrypt.
///
/// # Errors
///
/// - [`SecurityError::Encryption`] — encryption error (extremely rare).
pub fn encrypt(
    key: &EncryptionKey,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SecurityError> {
    let key_arr = Key::<XChaCha20Poly1305>::from(*key.as_bytes());
    let cipher = XChaCha20Poly1305::new(&key_arr);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce: XNonce = nonce_bytes.into();

    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|e| SecurityError::Encryption(e.to_string()))?;

    let mut output = Vec::with_capacity(NONCE_LEN.saturating_add(ciphertext.len()));
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypts data (format `nonce ‖ ciphertext+tag`) with the same AAD.
///
/// # Errors
///
/// - [`SecurityError::Decryption`] — wrong key, corrupted data,
///   or AAD mismatch (cryptographic authentication failed).
/// - [`SecurityError::CiphertextTooShort`] — data shorter than the nonce (24 bytes).
pub fn decrypt(key: &EncryptionKey, data: &[u8], aad: &[u8]) -> Result<Vec<u8>, SecurityError> {
    if data.len() < NONCE_LEN {
        return Err(SecurityError::CiphertextTooShort {
            actual: data.len(),
            min: NONCE_LEN,
        });
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let key_arr = Key::<XChaCha20Poly1305>::from(*key.as_bytes());
    let cipher = XChaCha20Poly1305::new(&key_arr);
    // Convert a [u8;24] slice into XNonce: copy into an array and then `into`.
    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(nonce_bytes);
    let nonce: XNonce = nonce_arr.into();

    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|e| SecurityError::Decryption(e.to_string()))
}

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        // Explicitly zero the key when it goes out of scope (Spec §10).
        zeroize::Zeroize::zeroize(&mut *self.bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::generate();
        let plaintext = b"sensitive backup data";
        let aad = b"mailgrit-backup-v1";

        let ciphertext = encrypt(&key, plaintext, aad).unwrap();
        // Ciphertext ≠ plaintext (actual encryption).
        assert_ne!(&ciphertext[NONCE_LEN..], plaintext);

        let decrypted = decrypt(&key, &ciphertext, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_fails_with_wrong_aad() {
        let key = EncryptionKey::generate();
        let ciphertext = encrypt(&key, b"data", b"correct-aad").unwrap();
        // AAD mismatch → authentication fails.
        assert!(decrypt(&key, &ciphertext, b"wrong-aad").is_err());
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let ciphertext = encrypt(&key1, b"secret", b"aad").unwrap();
        assert!(decrypt(&key2, &ciphertext, b"aad").is_err());
    }

    #[test]
    fn decrypt_rejects_short_input() {
        let key = EncryptionKey::generate();
        let short = vec![0u8; 10]; // less than NONCE_LEN (24)
        assert!(matches!(
            decrypt(&key, &short, b""),
            Err(SecurityError::CiphertextTooShort {
                actual: 10,
                min: 24
            })
        ));
    }

    #[test]
    fn key_from_bytes_validates_length() {
        assert!(EncryptionKey::from_bytes(&[0u8; 31]).is_err());
        assert!(EncryptionKey::from_bytes(&[0u8; 32]).is_ok());
        assert!(EncryptionKey::from_bytes(&[0u8; 33]).is_err());
    }

    #[test]
    fn different_nonces_each_encryption() {
        let key = EncryptionKey::generate();
        let c1 = encrypt(&key, b"data", b"aad").unwrap();
        let c2 = encrypt(&key, b"data", b"aad").unwrap();
        // Random nonce → ciphertexts differ, even though the plaintext is the same.
        assert_ne!(c1, c2);
    }
}

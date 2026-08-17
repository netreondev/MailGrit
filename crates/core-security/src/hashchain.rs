//! Hash-chained audit log: `HMAC-SHA256` (Spec §III.6).
//!
//! Each new entry in the audit log contains a cryptographic hash:
//! ```text
//! H_n = HMAC-SHA256(Message_n ‖ H_{n-1}, K)
//! ```
//! where `K` is the isolated master key of the system. Any point modification
//! or deletion of rows breaks the chain on the next verification.
//!
//! Initial value H_0 = zeros (or a fixed genesis hash).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::aead::EncryptionKey;
use crate::ct_eq::constant_time_eq;
use crate::error::SecurityError;
use hmac::{Hmac, Mac};
use sha2::{Sha256, digest::KeyInit};

/// Length of HMAC-SHA256 in bytes (32).
pub const HMAC_LEN: usize = 32;

/// Initial value of the hash chain (`H_0)`: zeros.
pub const GENESIS_HASH: [u8; HMAC_LEN] = [0u8; HMAC_LEN];

/// Type alias for HMAC-SHA256.
type HmacSha256 = Hmac<Sha256>;

/// Computes the next link of the hash chain: `H_n = HMAC(message ‖ H_{n-1}, key)`.
///
/// `prev_hash` is the previous link (H_{n-1}); for the first entry — [`GENESIS_HASH`].
/// `message` is the serialized content of the audit entry (JSON, text).
///
/// # Errors
///
/// - [`SecurityError::Hmac`] — HMAC initialization error (critical, unlikely).
pub fn chain_hash(
    key: &EncryptionKey,
    prev_hash: &[u8; HMAC_LEN],
    message: &[u8],
) -> Result<[u8; HMAC_LEN], SecurityError> {
    // KeyInit::new_from_slice — the actual hmac 0.13 API for creating a MAC from a key slice.
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key.as_bytes())
        .map_err(|e| SecurityError::Hmac(e.to_string()))?;
    // Concatenation of message ‖ prev_hash in a strict order.
    mac.update(message);
    mac.update(prev_hash);
    let result = mac.finalize().into_bytes();
    // result has a fixed length of 32 bytes for HMAC-SHA256; the conversion is safe.
    let mut out = [0u8; HMAC_LEN];
    out.copy_from_slice(&result);
    Ok(out)
}

/// Verifies the integrity of the entire audit hash chain.
///
/// Takes a sequence of `(message, expected_hash)` pairs and a key.
/// Returns the index of the first entry with a broken chain, or `Ok(())` if the
/// entire chain is valid. The genesis check starts from [`GENESIS_HASH`].
///
/// # Errors
///
/// - [`SecurityError::ChainBroken`] — an entry with an incorrect hash was found
///   (possible retroactive modification of the log).
pub fn verify_chain<I>(key: &EncryptionKey, entries: I) -> Result<(), SecurityError>
where
    I: IntoIterator<Item = (Vec<u8>, [u8; HMAC_LEN])>,
{
    let mut prev_hash = GENESIS_HASH;
    for (index, (message, expected_hash)) in entries.into_iter().enumerate() {
        let computed = chain_hash(key, &prev_hash, &message)?;
        // Constant-time comparison: an early-exit `!=` would leak the position
        // of the first mismatching byte through timing (the same standard the
        // audit key-file verify-token uses).
        if !constant_time_eq(&computed, &expected_hash) {
            // u64 index is safe: the number of audit entries will not exceed u64.
            let idx = u64::try_from(index).unwrap_or(u64::MAX);
            return Err(SecurityError::ChainBroken { entry_index: idx });
        }
        prev_hash = expected_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_deterministic_for_same_inputs() -> Result<(), Box<dyn std::error::Error>> {
        let key = EncryptionKey::generate();
        let h1 = chain_hash(&key, &GENESIS_HASH, b"entry1")?;
        let h2 = chain_hash(&key, &GENESIS_HASH, b"entry1")?;
        assert_eq!(h1, h2, "identical inputs → identical HMAC");
        Ok(())
    }

    #[test]
    fn chain_progresses_with_each_entry() -> Result<(), Box<dyn std::error::Error>> {
        let key = EncryptionKey::generate();
        let h1 = chain_hash(&key, &GENESIS_HASH, b"entry1")?;
        let h2 = chain_hash(&key, &h1, b"entry2")?;
        assert_ne!(h1, h2, "each link differs from the previous one");
        Ok(())
    }

    #[test]
    fn chain_detects_tampered_message() -> Result<(), Box<dyn std::error::Error>> {
        let key = EncryptionKey::generate();
        let h1 = chain_hash(&key, &GENESIS_HASH, b"entry1")?;
        // Tamper with the message: compute the hash for "tampered", but the chain contains "entry1".
        let entries = vec![(b"entry1".to_vec(), h1)];
        assert!(verify_chain(&key, entries).is_ok());

        // Same chain, but the message is changed → violation.
        let tampered = vec![(b"TAMPERED".to_vec(), h1)];
        assert!(matches!(
            verify_chain(&key, tampered),
            Err(SecurityError::ChainBroken { entry_index: 0 })
        ));
        Ok(())
    }

    #[test]
    fn chain_detects_deleted_entry() -> Result<(), Box<dyn std::error::Error>> {
        let key = EncryptionKey::generate();
        let h1 = chain_hash(&key, &GENESIS_HASH, b"entry1")?;
        let h2 = chain_hash(&key, &h1, b"entry2")?;
        let h3 = chain_hash(&key, &h2, b"entry3")?;

        // The full chain is valid.
        let full = vec![
            (b"entry1".to_vec(), h1),
            (b"entry2".to_vec(), h2),
            (b"entry3".to_vec(), h3),
        ];
        assert!(verify_chain(&key, full).is_ok());

        // entry2 is deleted → h3 is computed from h1, but h2 is missing from the chain,
        // and prev_hash after entry1 = h1, while expected for entry3 = h3 (from h2).
        let deleted = vec![
            (b"entry1".to_vec(), h1),
            (b"entry3".to_vec(), h3), // skips entry2 → prev_hash mismatch
        ];
        assert!(matches!(
            verify_chain(&key, deleted),
            Err(SecurityError::ChainBroken { entry_index: 1 })
        ));
        Ok(())
    }

    #[test]
    fn empty_chain_verifies_ok() {
        let key = EncryptionKey::generate();
        assert!(verify_chain(&key, std::iter::empty()).is_ok());
    }

    #[test]
    fn chain_detects_wrong_key() -> Result<(), Box<dyn std::error::Error>> {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let h1 = chain_hash(&key1, &GENESIS_HASH, b"entry1")?;
        // Verification with a different key → violation.
        let entries = vec![(b"entry1".to_vec(), h1)];
        assert!(matches!(
            verify_chain(&key2, entries),
            Err(SecurityError::ChainBroken { entry_index: 0 })
        ));
        Ok(())
    }
}

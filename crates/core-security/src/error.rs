//! Domain errors of the cryptographic layer (Spec §19).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

/// Error of a cryptographic operation.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    /// Invalid key length.
    #[error("invalid key length: {actual} bytes (expected {expected})")]
    InvalidKeyLength {
        /// Actual length.
        actual: usize,
        /// Expected length.
        expected: usize,
    },
    /// Encryption error (extremely rare, usually hardware-related).
    #[error("encryption error: {0}")]
    Encryption(String),
    /// Decryption error (wrong key/AAD, corrupted data).
    #[error("decryption error (wrong key or corrupted data): {0}")]
    Decryption(String),
    /// Ciphertext shorter than the nonce (cannot be decrypted).
    #[error("ciphertext too short: {actual} bytes (minimum {min})")]
    CiphertextTooShort {
        /// Actual length.
        actual: usize,
        /// Minimum length.
        min: usize,
    },
    /// HMAC computation error (audit hash-chain).
    #[error("HMAC error: {0}")]
    Hmac(String),
    /// Key derivation error from the master password (KDF).
    #[error("key derivation from master password error: {0}")]
    Kdf(String),
    /// Audit hash-chain integrity violation (log tampered with).
    #[error("audit hash-chain integrity violation at entry #{entry_index}")]
    ChainBroken {
        /// Index of the entry with a broken chain.
        entry_index: u64,
    },
}

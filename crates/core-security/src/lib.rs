//! `core-security` — at-rest cryptography for exports and the audit chain.
//!
//! From the Spec:
//! - **Streaming AEAD-at-Rest** (§4, §7): export/backup is encrypted via
//!   `XChaCha20-Poly1305` directly during writing. The backup file is initially
//!   born encrypted — this minimizes the TOCTOU window. Secret material is
//!   wiped when it goes out of scope: the encryption key via
//!   `EncryptionKey::Drop` (`zeroize`), and the KDF output / decrypted
//!   plaintext via the `Zeroizing` wrappers they are returned in. The master
//!   password itself lives only in the app layer (`Zeroizing<String>` there).
//! - **Hash-chained audit** (§III.6): an `HMAC-SHA256` chain for the operations log.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny) is set
// centrally in [workspace.lints.rust] of the root Cargo.toml. Test modules
// follow the same policy (no test-only suppressions).

/// AEAD-at-Rest encryption (XChaCha20-Poly1305).
pub mod aead;

/// Domain errors of the cryptographic layer.
pub mod error;

/// Key derivation from the master password (Argon2id KDF).
pub mod kdf;

/// Hash-chained audit log (HMAC-SHA256).
pub mod hashchain;

/// Constant-time comparison (mitigates a timing channel when comparing tokens).
pub mod ct_eq;

/// Kani formal verification proof-harnesses (only when cfg(kani) is active).
#[cfg(kani)]
mod kani_harnesses;

pub use aead::{EncryptionKey, NONCE_LEN, decrypt, encrypt};
pub use ct_eq::constant_time_eq;
pub use error::SecurityError;
pub use hashchain::{GENESIS_HASH, HMAC_LEN, chain_hash, verify_chain};
pub use kdf::{DERIVED_KEY_LEN, SALT_LEN, derive_key, generate_salt};

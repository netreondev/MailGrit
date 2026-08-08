//! `core-security` — cryptographic protection of data at-rest.
//!
//! From the Spec:
//! - **Streaming AEAD-at-Rest** (§4, §7): export/backup is encrypted via
//!   `XChaCha20-Poly1305` directly during writing. The backup file is initially
//!   born encrypted — this minimizes the TOCTOU window. The encryption key
//!   itself is zeroed in memory when it goes out of scope (`EncryptionKey::Drop`
//!   via `zeroize`).
//! - **Hash-chained audit** (§III.6): an `HMAC-SHA256` chain for the operations log.

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

/// Constant-time comparison (timing-channel protection when comparing tokens).
pub mod ct_eq;

/// Kani formal verification proof-harnesses (only when cfg(kani) is active).
#[cfg(kani)]
mod kani_harnesses;

pub use aead::{EncryptionKey, NONCE_LEN, decrypt, encrypt};
pub use ct_eq::constant_time_eq;
pub use error::SecurityError;
pub use hashchain::{GENESIS_HASH, HMAC_LEN, chain_hash, verify_chain};
pub use kdf::{DERIVED_KEY_LEN, SALT_LEN, derive_key, generate_salt};

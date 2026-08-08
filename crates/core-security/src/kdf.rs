//! Key derivation from the master password (KDF): Argon2id (RFC 9106, Spec §10).
//!
//! The master password is NOT persisted — a 32-byte key for protecting
//! at-rest secrets (audit key, export key) is derived from it via a memory-hard
//! KDF. The salt is stored alongside the protected secret in plaintext (it is
//! not secret), but the secret itself cannot be recovered without the master
//! password.
//!
//! Argon2id is chosen as a trade-off between resistance to GPU/ASIC attacks
//! (d>0) and side-channel resistance (t>0); recommended by RFC 9106.

use crate::error::SecurityError;
use argon2::{Algorithm, Argon2, Params, Version};
use rand::Rng;

/// KDF salt length in bytes (16 is the RFC 9106 recommendation).
pub const SALT_LEN: usize = 16;

/// Length of the derived key in bytes (32 — for XChaCha20 / HMAC-SHA256).
pub const DERIVED_KEY_LEN: usize = 32;

/// Argon2id parameters: 64 MiB memory, 3 iterations, parallelism degree 4.
/// Security/performance balance for a desktop application (RFC 9106 §4).
/// `Params::new` validates the values and returns a `Result`; we unwrap here
/// (the constants are clearly valid, but the API requires `unwrap` in `const fn`
/// — `expect` is not allowed in `const`, so we create `Params` on the first call
/// to `derive_key`).
const M_COST: u32 = 65_536; // 64 MiB (in KiB)
const T_COST: u32 = 3;
const P_COST: u32 = 4;

/// Generates a cryptographically random salt (16 bytes).
#[must_use]
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    salt
}

/// Derives a 32-byte key from the master password and salt via Argon2id.
///
/// Identical `(password, salt)` → identical key (KDF determinism). The salt
/// must be unique for each protected secret; the master password is not stored.
/// With an incorrect master password, the derived key will not match the one
/// used during protection → decryption/verification will fail with an error.
///
/// # Errors
///
/// - [`SecurityError::Kdf`] — invalid salt length or an internal Argon2 error.
pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; DERIVED_KEY_LEN], SecurityError> {
    if salt.len() != SALT_LEN {
        return Err(SecurityError::Kdf(format!(
            "invalid salt length: {} (expected {})",
            salt.len(),
            SALT_LEN
        )));
    }
    // Params::new validates m/t/p_cost; the values above are clearly correct.
    let params = Params::new(M_COST, T_COST, P_COST, Some(DERIVED_KEY_LEN))
        .map_err(|e| SecurityError::Kdf(format!("invalid Argon2 parameters: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; DERIVED_KEY_LEN];
    argon2
        .hash_password_into(password, salt, &mut out)
        .map_err(|e| SecurityError::Kdf(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The KDF tests below exercise the real Argon2id algorithm (64 MiB memory,
    // 3 passes, parallelism 4). Miri interprets this pure-Rust code correctly,
    // but the memory-hard computation is prohibitively slow under interpretation
    // (tens of minutes per `derive_key` call). They are therefore skipped under
    // `cfg(miri)`; the `generate_salt` test (no KDF) still runs.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn derive_key_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let salt = generate_salt();
        let k1 = derive_key(b"master-pass", &salt)?;
        let k2 = derive_key(b"master-pass", &salt)?;
        assert_eq!(k1, k2, "identical inputs → identical KDF key");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn derive_key_differs_for_different_passwords() -> Result<(), Box<dyn std::error::Error>> {
        let salt = generate_salt();
        let k1 = derive_key(b"password1", &salt)?;
        let k2 = derive_key(b"password2", &salt)?;
        assert_ne!(k1, k2, "different passwords → different keys");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn derive_key_differs_for_different_salts() -> Result<(), Box<dyn std::error::Error>> {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let k1 = derive_key(b"same-pass", &salt1)?;
        let k2 = derive_key(b"same-pass", &salt2)?;
        assert_ne!(k1, k2, "different salts → different keys");
        Ok(())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn derive_key_rejects_wrong_salt_length() {
        assert!(derive_key(b"pass", &[0u8; 15]).is_err());
        assert!(derive_key(b"pass", &[0u8; 17]).is_err());
        assert!(derive_key(b"pass", &[0u8; SALT_LEN]).is_ok());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn derived_key_length_is_32() -> Result<(), Box<dyn std::error::Error>> {
        let salt = generate_salt();
        let k = derive_key(b"x", &salt)?;
        assert_eq!(k.len(), DERIVED_KEY_LEN);
        Ok(())
    }

    #[test]
    fn generate_salt_is_16_bytes_and_unique() {
        let s1 = generate_salt();
        let s2 = generate_salt();
        assert_eq!(s1.len(), SALT_LEN);
        assert_ne!(s1, s2, "salts must be unique");
    }
}

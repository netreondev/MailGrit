//! Kani formal-verification proof-harnesses for the cryptographic layer.
//!
//! Activated via cfg(kani): `cargo kani -p mailgrit-core-security`.
//! The harnesses below exercise: `EncryptionKey::from_bytes` length validation,
//! HMAC hash-chain determinism, `verify_chain` correctness (incl. tamper
//! detection), and the AEAD boundary/roundtrip paths.
//!
//! IMPORTANT — scope and CI status, read before relying on these:
//! - Kani runs in CI as a BLOCKING job (no `continue-on-error`, see
//!   `.github/workflows/ci.yml`, the `kani` job). A failed/timeout proof
//!   fails the build and blocks the PR; treat these as a hard verification gate.
//! - The AEAD harnesses call into the `chacha20poly1305` crate, which Kani
//!   models as an opaque (uninterpreted) function — it cannot see inside the
//!   cipher. Therefore `verify_aead_roundtrip` / `verify_aead_rejects_wrong_aad`
//!   effectively check determinism of the opaque calls and our own
//!   nonce/ciphertext split + boundary logic; they do NOT prove the cipher's
//!   cryptographic security. The `verify_aead_rejects_short_ciphertext` and
//!   `EncryptionKey` length harnesses, by contrast, check our own pure logic.

#![cfg(kani)]
// Kani proof harnesses are verification entry points and follow workspace
// lints (no suppressions). `expect` is avoided: where an invariant must hold,
// an `assert!` is used instead (its failure = a proof failure), and `unwrap_or`
// supplies a fallback value that the surrounding assertion already rejects.

use crate::aead::EncryptionKey;
use crate::hashchain::{GENESIS_HASH, chain_hash, verify_chain};

// ============================================================================
// EncryptionKey::from_bytes — length validation
// ============================================================================

#[kani::proof]
fn verify_encryption_key_from_bytes_length_validation() {
    // Invariant: from_bytes accepts exactly 32 bytes, rejects any other number.
    let len: usize = kani::any();
    // Restrict the range for bounded model-checking (0..=40 covers the boundaries).
    kani::assume(len <= 40);
    let bytes: Vec<u8> = vec![0u8; len];
    let result = EncryptionKey::from_bytes(&bytes);
    if len == 32 {
        assert!(result.is_ok(), "32 bytes → Ok");
    } else {
        assert!(result.is_err(), "not 32 bytes → Err");
    }
}

// ============================================================================
// chain_hash — determinism
// ============================================================================

#[kani::proof]
fn verify_chain_hash_deterministic() {
    // Invariant: identical (key, prev_hash, message) → identical HMAC.
    let key = EncryptionKey::generate();
    let mut message = [0u8; 8];
    for byte in &mut message {
        *byte = kani::any();
    }
    let r1 = chain_hash(&key, &GENESIS_HASH, &message);
    let r2 = chain_hash(&key, &GENESIS_HASH, &message);
    assert!(r1.is_ok(), "chain_hash must not fail");
    assert!(r2.is_ok(), "chain_hash must not fail");
    assert_eq!(r1, r2, "chain_hash is deterministic");
}

// ============================================================================
// verify_chain — integrity of a correctly built chain
// ============================================================================

#[kani::proof]
fn verify_verify_chain_empty_is_ok() {
    // Invariant: an empty chain is always valid.
    let key = EncryptionKey::generate();
    assert!(verify_chain(&key, std::iter::empty()).is_ok());
}

#[kani::proof]
fn verify_verify_chain_single_well_formed_entry() {
    // Invariant: a chain of a single entry with a correctly computed hash is valid.
    let key = EncryptionKey::generate();
    let mut message = [0u8; 4];
    for byte in &mut message {
        *byte = kani::any();
    }
    let h = chain_hash(&key, &GENESIS_HASH, &message);
    assert!(h.is_ok(), "chain_hash must not fail");
    let h = h.unwrap_or(GENESIS_HASH);
    let entries = vec![(message.to_vec(), h)];
    assert!(
        verify_chain(&key, entries).is_ok(),
        "a correctly built single-element chain is valid"
    );
}

// ============================================================================
// verify_chain — a correctly built multi-element chain is valid.
//
// This harness exercises the `for (index, (message, expected_hash)) in entries`
// loop in verify_chain with a small fixed iteration count (3 links), so Kani
// unrolls the loop body and covers the prev_hash carry-over between iterations.
// The property checked: a self-consistent chain (every expected_hash ==
// chain_hash(key, prev_hash, message)) never trips ChainBroken. NOTE: Kani is a
// non-blocking CI job (see file-level doc) — this harness is exploratory and
// does not gate PRs.

#[kani::proof]
fn verify_verify_chain_multi_element_loop_contract() {
    // Build a deterministically-correct 3-link chain, then assert the verifier
    // accepts it. This forces Kani through the verify_chain loop body 3 times,
    // covering the prev_hash carry-over between iterations.
    let key = EncryptionKey::generate();

    // Entry 0
    let mut msg0 = [0u8; 4];
    for b in &mut msg0 {
        *b = kani::any();
    }
    let h0 = chain_hash(&key, &GENESIS_HASH, &msg0);
    assert!(h0.is_ok(), "chain_hash must not fail");
    let h0 = h0.unwrap_or(GENESIS_HASH);

    // Entry 1 (chained off h0)
    let mut msg1 = [0u8; 4];
    for b in &mut msg1 {
        *b = kani::any();
    }
    let h1 = chain_hash(&key, &h0, &msg1);
    assert!(h1.is_ok(), "chain_hash must not fail");
    let h1 = h1.unwrap_or(GENESIS_HASH);

    // Entry 2 (chained off h1)
    let mut msg2 = [0u8; 4];
    for b in &mut msg2 {
        *b = kani::any();
    }
    let h2 = chain_hash(&key, &h1, &msg2);
    assert!(h2.is_ok(), "chain_hash must not fail");
    let h2 = h2.unwrap_or(GENESIS_HASH);

    let entries = vec![
        (msg0.to_vec(), h0),
        (msg1.to_vec(), h1),
        (msg2.to_vec(), h2),
    ];
    assert!(
        verify_chain(&key, entries).is_ok(),
        "a correctly built 3-element chain must verify"
    );
}

// ============================================================================
// verify_chain — tamper detection through the loop.
// Invariant: if ANY link's hash does not match the recomputed value, the loop
// returns ChainBroken at that index.
// ============================================================================

#[kani::proof]
fn verify_verify_chain_detects_tamper_at_index_one() {
    let key = EncryptionKey::generate();

    let mut msg0 = [0u8; 4];
    for b in &mut msg0 {
        *b = kani::any();
    }
    let h0 = chain_hash(&key, &GENESIS_HASH, &msg0);
    assert!(h0.is_ok(), "chain_hash must not fail");
    let h0 = h0.unwrap_or(GENESIS_HASH);

    let mut msg1 = [0u8; 4];
    for b in &mut msg1 {
        *b = kani::any();
    }
    let h1_correct = chain_hash(&key, &h0, &msg1);
    assert!(h1_correct.is_ok(), "chain_hash must not fail");
    // unwrap_or is unreachable on the happy path; the assertion above guarantees Ok.
    let mut h1_tampered = h1_correct.unwrap_or(GENESIS_HASH);
    h1_tampered[0] = h1_tampered[0].wrapping_add(1);

    let entries = vec![(msg0.to_vec(), h0), (msg1.to_vec(), h1_tampered)];
    assert!(
        matches!(
            verify_chain(&key, entries),
            Err(crate::error::SecurityError::ChainBroken { entry_index: 1 })
        ),
        "a tampered hash at index 1 must be detected at index 1"
    );
}

// ============================================================================
// AEAD roundtrip — encrypt then decrypt returns the original plaintext.
//
// SCOPE LIMITATION: the `chacha20poly1305` crate is opaque to Kani (treated as
// an uninterpreted function — Kani cannot see inside the cipher). So this
// harness does NOT prove the cryptographic correctness of XChaCha20-Poly1305;
// what it actually checks is our own glue logic (the nonce/ciphertext split in
// `aead::encrypt`/`decrypt`) on top of a cipher whose encrypt/decrypt Kani
// assumes to be deterministic inverses on the happy path. Combined with the
// wrong-AAD and short-ciphertext harnesses, this still pins our own error
// handling and boundary checks; the cipher's security remains trusted to the
// audited `chacha20poly1305` crate, not proven here.
// ============================================================================

#[kani::proof]
// bound on loops inside THIS harness (none over the data); does NOT bound the
// opaque cipher internals. (Attribute kept on its own line so the explanation
// stays a regular comment, not a dangling trailing comment.)
#[kani::unwind(2)]
fn verify_aead_roundtrip() {
    use crate::aead::{decrypt, encrypt};
    let key = EncryptionKey::generate();
    let mut plaintext = [0u8; 1];
    plaintext[0] = kani::any();
    let mut aad = [0u8; 1];
    aad[0] = kani::any();

    let ciphertext = encrypt(&key, &plaintext, &aad);
    assert!(ciphertext.is_ok(), "encrypt must succeed");
    let ciphertext = ciphertext.unwrap_or_default();
    // The ciphertext is always longer than the plaintext (nonce + tag overhead).
    assert!(
        ciphertext.len() > plaintext.len(),
        "ciphertext includes overhead"
    );

    let decrypted = decrypt(&key, &ciphertext, &aad);
    assert!(decrypted.is_ok(), "decrypt with correct AAD must succeed");
    let decrypted = decrypted.unwrap_or_default();
    assert_eq!(
        decrypted, plaintext,
        "encrypt→decrypt roundtrip must return the original plaintext"
    );
}

// ============================================================================
// AEAD authentication — a wrong AAD is rejected by the cipher.
//
// Same scope limitation as `verify_aead_roundtrip`: because `chacha20poly1305`
// is opaque to Kani, this harness relies on Kani assuming the cipher rejects a
// mismatched AAD. It mainly pins our own error-path plumbing (the `Result`
// returned by `decrypt` flows through unchanged). It is NOT a proof of the
// Poly1305 authentication property; that is trusted to the audited crate.
// (Note: AEAD is used for exports/backups — the audit log itself is not
// encrypted at rest; see SECURITY.md.)
// ============================================================================

#[kani::proof]
// bound on loops inside THIS harness (none over the data); does NOT bound the
// opaque cipher internals.
#[kani::unwind(2)]
fn verify_aead_rejects_wrong_aad() {
    use crate::aead::{decrypt, encrypt};
    let key = EncryptionKey::generate();
    let plaintext = [0u8; 1];
    let correct_aad = [0xAAu8; 1];
    let wrong_aad = [0x55u8; 1]; // differs in every bit from correct_aad

    let ciphertext = encrypt(&key, &plaintext, &correct_aad);
    assert!(ciphertext.is_ok(), "encrypt must succeed");
    let ciphertext = ciphertext.unwrap_or_default();
    assert!(
        decrypt(&key, &ciphertext, &wrong_aad).is_err(),
        "decrypt with a wrong AAD must fail authentication"
    );
}

// ============================================================================
// AEAD — short-ciphertext rejection path.
// Invariant: a ciphertext shorter than NONCE_LEN (24 bytes) is always rejected
// with CiphertextTooShort, without touching the cipher. Covers the boundary
// check at the top of decrypt().
// ============================================================================

#[kani::proof]
fn verify_aead_rejects_short_ciphertext() {
    use crate::aead::{NONCE_LEN, decrypt};
    let key = EncryptionKey::generate();
    // Symbolic length in 0..=NONCE_LEN-1 (strictly below the nonce length).
    let len: usize = kani::any();
    kani::assume(len < NONCE_LEN);
    let data = vec![0u8; len];
    assert!(
        matches!(
            decrypt(&key, &data, b""),
            Err(crate::error::SecurityError::CiphertextTooShort { .. })
        ),
        "a ciphertext shorter than the nonce must be rejected"
    );
}

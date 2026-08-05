//! Kani formal verification proof-harnesses for the cryptographic layer.
//!
//! Activated via cfg(kani): `cargo kani -p mailgrit-core-security`.
//! They prove: HMAC hash-chain determinism, verify_chain correctness,
//! key length validation.

#![cfg(kani)]
#![allow(clippy::pedantic, clippy::nursery, clippy::needless_pass_by_value)]

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
    let h1 = chain_hash(&key, &GENESIS_HASH, &message).expect("HMAC does not fail");
    let h2 = chain_hash(&key, &GENESIS_HASH, &message).expect("HMAC does not fail");
    assert_eq!(h1, h2, "chain_hash is deterministic");
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
    let h = chain_hash(&key, &GENESIS_HASH, &message).expect("HMAC");
    let entries = vec![(message.to_vec(), h)];
    assert!(
        verify_chain(&key, entries).is_ok(),
        "a correctly built single-element chain is valid"
    );
}

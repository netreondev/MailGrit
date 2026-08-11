//! Criterion benchmarks for the at-rest cryptography hot paths.
//!
//! Run: `cargo bench -p mailgrit-core-security`
//! (CI only *compiles* benches to keep them buildable; it does NOT time them,
//! so perf regressions are only caught by a local `cargo bench` run.)
//!
//! Guards against regressions in:
//!   - XChaCha20-Poly1305 encrypt/decrypt (export/backup encryption).
//!   - HMAC-SHA256 hash-chain verification (audit-log integrity on load).
//!
//! Argon2id KDF is deliberately NOT benched here: it is intentionally slow
//! (memory-hard) and a "regression" in its runtime is a *good* thing (harder to
//! brute-force). Pinning its speed would create a perverse incentive.
//!
//! Benchmark harness code is non-production: a failing `assert_*` here is a
//! benchmark setup bug, not a runtime failure. Results are asserted (never
//! silently dropped) so a benchmark cannot be optimized away into a no-op.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mailgrit_core_security::{
    EncryptionKey, GENESIS_HASH, chain_hash, decrypt, encrypt, verify_chain,
};

fn bench_aead(c: &mut Criterion) {
    let key = EncryptionKey::generate();
    let aad = b"mailgrit-backup-v1";

    let mut group = c.benchmark_group("aead_xchacha20_poly1305");
    // Payload sizes that reflect real audit-log export chunks / record sizes.
    for &size in &[256_usize, 4 * 1024, 64 * 1024] {
        let plaintext = vec![0xAB_u8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("encrypt", size), &plaintext, |b, pt| {
            b.iter(|| {
                let ct = encrypt(&key, black_box(pt), aad);
                // Assert (not expect): a bench-input bug fails the assertion.
                assert!(ct.is_ok(), "encrypt must succeed on valid input");
                let ct = black_box(ct);
                let _ = ct;
            });
        });

        // Pre-encrypt once so decrypt benchmarks only measure decryption.
        let ciphertext = encrypt(&key, &plaintext, aad);
        assert!(ciphertext.is_ok(), "pre-encrypt must succeed");
        let ciphertext = ciphertext.unwrap_or_default();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("decrypt", size), &ciphertext, |b, ct| {
            b.iter(|| {
                let pt = decrypt(&key, black_box(ct), aad);
                assert!(pt.is_ok(), "decrypt must round-trip");
                let pt = black_box(pt);
                let _ = pt;
            });
        });
    }
    group.finish();
}

fn bench_hash_chain(c: &mut Criterion) {
    let key = EncryptionKey::generate();

    // Build correctly-chained audit log entries of varying length, then measure
    // the verify_chain pass over the whole log (this runs on every audit-log open).
    let mut group = c.benchmark_group("verify_chain");
    for &n in &[100_usize, 1_000, 10_000] {
        let mut entries = Vec::with_capacity(n);
        let mut prev = GENESIS_HASH;
        for i in 0..n {
            let message = format!("audit-entry-{i}").into_bytes();
            let h = chain_hash(&key, &prev, &message);
            assert!(h.is_ok(), "chain_hash must succeed");
            let h = h.unwrap_or(GENESIS_HASH);
            entries.push((message, h));
            prev = h;
        }
        let expected_len = entries.len();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &entries, |b, entries| {
            b.iter(|| {
                let ok = verify_chain(&key, black_box(entries.iter().cloned()));
                assert!(
                    ok.is_ok(),
                    "verify_chain must succeed on a well-formed chain"
                );
                let ok = black_box(ok);
                let _ = ok;
            });
        });
        // Touch expected_len so the compiler cannot drop the build loop.
        assert_eq!(black_box(expected_len), n);
    }
    group.finish();
}

criterion_group!(benches, bench_aead, bench_hash_chain);
criterion_main!(benches);

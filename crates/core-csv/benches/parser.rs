//! Criterion benchmarks for the streaming CSV parser.
//!
//! Run: `cargo bench -p mailgrit-core-csv`
//! (CI only *compiles* benches to keep them buildable; it does NOT time them,
//! so perf regressions are only caught by a local `cargo bench` run — see
//! .github/workflows/ci.yml.)
//!
//! These benchmarks guard against performance regressions on the primary hot
//! path: parsing a large bulk-import CSV (up to `MAX_CSV_ROWS`). A regression here
//! would make a real 50k-row import noticeably slower for the user.
//!
//! Benchmark harness code is non-production: an `expect` failure here is a
//! benchmark setup bug, not a runtime failure. Where the harness calls into the
//! parser under measurement, the result is asserted (never silently dropped) so
//! a benchmark cannot be optimized away into a no-op.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mailgrit_core_csv::parse_csv_bytes;

/// Builds a synthetic CSV with a header + `n` valid data rows.
fn make_csv(n: usize) -> Vec<u8> {
    let mut s = String::from("domain,username,password,display_name,quota_mb\n");
    for i in 0..n {
        // Realistic row shape; varies per row so the parser can't trivially memoize.
        // A push failure here is impossible (writing to a String), so it is ignored.
        let _ = writeln!(
            s,
            "domain{i}.example.com,user{i},S3cur3P@ss{i}!,User Number {i},1024"
        );
    }
    s.into_bytes()
}

fn bench_parse_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_csv_bytes");
    for &n in &[100_usize, 1_000, 10_000, 50_000] {
        let data = make_csv(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &data, |b, data| {
            b.iter(|| {
                let parsed = parse_csv_bytes(black_box(data));
                assert!(parsed.is_ok(), "bench input must parse cleanly");
                let parsed = black_box(parsed);
                let _ = parsed;
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_parse_csv);
criterion_main!(benches);

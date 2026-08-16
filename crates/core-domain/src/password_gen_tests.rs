//! Password-generator tests ([`password_gen`](../password_gen.rs)).
//!
//! Factored into a separate file (via `#[path]`) to keep the main module
//! within the ≤400-line file-size limit. Included as the body of `mod tests`
//! (which carries `#[cfg(not(miri))]` in the parent file: these tests run the
//! generator 100-500 times per case, prohibitively slow under Miri).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;

#[test]
fn default_is_length_16_all_classes() {
    let g = PasswordGenerator::default_generator();
    assert_eq!(g.length, 16);
    assert!(
        g.classes.uppercase() && g.classes.lowercase() && g.classes.digits() && g.classes.special()
    );
}

#[test]
fn generate_has_correct_length() {
    let mut g = PasswordGenerator::default_generator();
    for &len in &[4usize, 8, 16, 32] {
        g.length = len;
        let pw = g.generate();
        assert_eq!(pw.chars().count(), len, "length must be {len}");
    }
}

#[test]
fn generate_clamps_extreme_lengths() {
    let mut g = PasswordGenerator::default_generator();
    g.length = 1;
    assert_eq!(g.generate().chars().count(), MIN_LENGTH);
    g.length = 10_000;
    assert_eq!(g.generate().chars().count(), MAX_LENGTH);
}

#[test]
fn generate_satisfies_default_policy() {
    use crate::password_policy::PasswordPolicy;
    let g = PasswordGenerator::default_generator();
    let policy = PasswordPolicy::default_policy();
    for _ in 0..200 {
        let pw = g.generate();
        let warnings = policy.validate(&pw);
        assert!(
            warnings.is_empty(),
            "generated password \"{}\" failed policy: {warnings:?}",
            pw.as_str()
        );
    }
}

#[test]
fn generate_never_contains_comma() {
    let g = PasswordGenerator::default_generator();
    for _ in 0..500 {
        assert!(
            !g.generate().contains(','),
            "comma in password breaks the CSV contract"
        );
    }
}

#[test]
fn generate_respects_disabled_classes() {
    let mut g = PasswordGenerator::default_generator();
    g.classes.set_special(false);
    g.classes.set_digits(false);
    g.classes.set_uppercase(false);
    for _ in 0..100 {
        let pw = g.generate();
        assert!(
            pw.chars().all(|c| c.is_ascii_lowercase()),
            "with classes disabled there should be only lowercase: \"{}\"",
            pw.as_str()
        );
    }
}

#[test]
fn generate_includes_each_enabled_class() {
    let g = PasswordGenerator::default_generator();
    for _ in 0..100 {
        let pw = g.generate();
        assert!(pw.chars().any(|c| c.is_ascii_uppercase()), "no uppercase");
        assert!(pw.chars().any(|c| c.is_ascii_lowercase()), "no lowercase");
        assert!(pw.chars().any(|c| c.is_ascii_digit()), "no digit");
        assert!(
            pw.chars().any(|c| SPECIALS.contains(&c)),
            "no special character"
        );
    }
}

#[test]
fn generate_empty_when_no_class_enabled() {
    let mut g = PasswordGenerator::default_generator();
    g.classes.set_uppercase(false);
    g.classes.set_lowercase(false);
    g.classes.set_digits(false);
    g.classes.set_special(false);
    assert_eq!(
        *g.generate(),
        "",
        "with no classes enabled generation is impossible"
    );
}

#[test]
fn generates_are_not_constant() {
    let g = PasswordGenerator::default_generator();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..50 {
        seen.insert(g.generate().to_string());
    }
    assert!(seen.len() > 1, "generator produces identical passwords");
}

// ---- Boundary-value coverage (mutation-killing) -------------------------

#[test]
fn clamped_for_label_bounds() {
    let mut g = PasswordGenerator::default_generator();
    // Below 8 → clamped to 8.
    g.length = 0;
    assert_eq!(g.clamped_for_label(), 8);
    g.length = 1;
    assert_eq!(g.clamped_for_label(), 8);
    g.length = 7;
    assert_eq!(g.clamped_for_label(), 8);
    // Exactly 8 → 8 (lower boundary, must NOT clamp).
    g.length = 8;
    assert_eq!(g.clamped_for_label(), 8);
    // Within [8, 32] → unchanged.
    g.length = 16;
    assert_eq!(g.clamped_for_label(), 16);
    g.length = 20;
    assert_eq!(g.clamped_for_label(), 20);
    // Exactly 32 → 32 (upper boundary, must NOT clamp).
    g.length = 32;
    assert_eq!(g.clamped_for_label(), 32);
    // Above 32 → clamped to 32.
    g.length = 33;
    assert_eq!(g.clamped_for_label(), 32);
    g.length = 64;
    assert_eq!(g.clamped_for_label(), 32);
    g.length = usize::MAX;
    assert_eq!(g.clamped_for_label(), 32);
}

#[test]
fn clamped_length_bounds() {
    let mut g = PasswordGenerator::default_generator();
    // Below MIN → MIN.
    g.length = 0;
    assert_eq!(g.clamped_length(), MIN_LENGTH);
    // Exactly MIN → MIN (boundary, must NOT clamp).
    g.length = MIN_LENGTH;
    assert_eq!(g.clamped_length(), MIN_LENGTH);
    // Exactly MAX → MAX (boundary, must NOT clamp).
    g.length = MAX_LENGTH;
    assert_eq!(g.clamped_length(), MAX_LENGTH);
    // Above MAX → MAX.
    g.length = MAX_LENGTH + 1;
    assert_eq!(g.clamped_length(), MAX_LENGTH);
    // Within range → unchanged.
    g.length = 32;
    assert_eq!(g.clamped_length(), 32);
}

// generate()'s fill loop `while chars.len() < length` is followed by
// `chars.truncate(length)`, so an off-by-one (`<` → `<=`) is masked by the
// truncate. Asserting the EXACT generated length (not just ≤) at the minimum
// length exercises the loop where a single extra char would make the output
// MAX+1 before truncate — keeping the length check meaningful, and pinning
// the contract that the output length equals the clamped target exactly.
#[test]
fn generate_exact_length_at_minimum() {
    let mut g = PasswordGenerator::default_generator();
    g.length = MIN_LENGTH;
    for _ in 0..200 {
        assert_eq!(g.generate().chars().count(), MIN_LENGTH);
    }
}

// shuffle() guards `if n < 2 { return }`. At n == 2 the shuffle must still
// produce a 2-char permutation of its input (one Fisher–Yates swap). If the
// guard is mutated to `<=` (skips n == 2) or `==`/`>`, the 2-char case is
// mishandled. Generate length-2 isn't possible (MIN_LENGTH=4), so drive
// shuffle indirectly: a min-length (4) password's charset is a fixed set,
// and over many runs a correct shuffle yields more than one distinct output.
#[test]
fn shuffle_permutes_min_length_output() {
    let mut g = PasswordGenerator::default_generator();
    g.length = MIN_LENGTH;
    let mut seen = std::collections::HashSet::new();
    for _ in 0..500 {
        seen.insert(g.generate().to_string());
    }
    // A working Fisher–Yates over 4 chars produces multiple orderings.
    assert!(
        seen.len() > 1,
        "shuffle appears to be a no-op at the minimum length"
    );
}

// shuffle() must randomize the ORDER of the characters, not just their
// values. The mandatory one-char-per-class block appends [upper, lower,
// digit, special] in a FIXED sequence, and only the shuffle scatters them.
// `shuffle_permutes_min_length_output` above cannot detect a no-op shuffle:
// each character is independently random, so distinct passwords appear even
// with a fixed class order (that test observed character randomness, not
// order randomness). Observe the class SIGNATURE instead: at MIN_LENGTH with
// all classes on, every password is a permutation of one upper, one lower,
// one digit and one special character, so a working Fisher–Yates yields many
// distinct signatures while a no-op shuffle yields exactly one ("Ulds").
#[test]
fn shuffle_randomizes_class_order() {
    let mut g = PasswordGenerator::default_generator();
    g.length = MIN_LENGTH;
    let classify = |c: char| {
        if c.is_ascii_uppercase() {
            'U'
        } else if c.is_ascii_lowercase() {
            'l'
        } else if c.is_ascii_digit() {
            'd'
        } else {
            's'
        }
    };
    let mut signatures = std::collections::HashSet::new();
    for _ in 0..200 {
        let pw = g.generate();
        let signature: String = pw.chars().map(classify).collect();
        assert_eq!(
            signature.len(),
            MIN_LENGTH,
            "expected exactly one character per class, got \"{signature}\""
        );
        signatures.insert(signature);
    }
    // 24 orderings exist; the probability that a real shuffle produces only
    // one of them across 200 draws is 24·(1/24)^200 ≈ 0 — not flaky.
    assert!(
        signatures.len() > 1,
        "character-class order never varies: shuffle looks like a no-op"
    );
}

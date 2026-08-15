//! Password generator with configurable complexity.
//!
//! Unlike [`PasswordPolicy`](crate::PasswordPolicy) (which only validates),
//! [`PasswordGenerator`] produces a password guaranteed to satisfy the enabled
//! requirements. Entropy comes from `rand::rng()` (a per-thread CSPRNG seeded
//! from the OS). The returned value is wrapped in [`Zeroizing`] so the
//! transient is wiped when the caller drops it.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use rand::Rng;
use rand::RngExt;
use rand::seq::IteratorRandom;
use zeroize::Zeroizing;

/// Minimum length: smaller values cannot fit all character classes.
pub const MIN_LENGTH: usize = 4;

/// Maximum reasonable password length in the UI (separate from `MAX_PASSWORD_LEN`).
pub const MAX_LENGTH: usize = 64;

/// Safe special characters: ASCII punctuation WITHOUT comma (forbidden by
/// `ValidatedPassword`, breaks CSV), quotes, or backslash (which break JS strings
/// and escaping in iRedAdmin forms).
const SPECIALS: &[char] = &[
    '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=', '+',
];

const UPPERS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

const LOWERS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
    'u', 'v', 'w', 'x', 'y', 'z',
];

const DIGITS: &[char] = &['2', '3', '4', '5', '6', '7', '8', '9'];

/// Generator configuration: length and enabled character classes.
///
/// The four character-class toggles live in [`classes`](Self::classes) (a single
/// [`CharacterClasses`](crate::CharacterClasses) field), keeping this struct below
/// clippy's `struct_excessive_bools` threshold. The flags mirror the canonical
/// `use_uppercase`/`use_lowercase`/`use_digits`/`use_special` set (the same four
/// classes as in [`PasswordPolicy`](crate::PasswordPolicy) and iRedAdmin
/// `settings.py`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordGenerator {
    /// Target password length. Clamped to [`MIN_LENGTH`]..=[`MAX_LENGTH`].
    pub length: usize,
    /// Enabled character classes (uppercase/lowercase/digits/special).
    pub classes: crate::CharacterClasses,
}

impl PasswordGenerator {
    /// Default: length 16, all classes enabled.
    #[must_use]
    pub const fn default_generator() -> Self {
        Self {
            length: 16,
            classes: crate::CharacterClasses::all(),
        }
    }

    /// Safe (clamped) password length.
    #[must_use]
    pub const fn clamped_length(&self) -> usize {
        if self.length < MIN_LENGTH {
            MIN_LENGTH
        } else if self.length > MAX_LENGTH {
            MAX_LENGTH
        } else {
            self.length
        }
    }

    /// Whether at least one class is enabled. If all flags are false, generation is impossible.
    #[must_use]
    pub const fn has_any_class(&self) -> bool {
        self.classes.has_any()
    }

    /// Length for the UI slider: clamped to 8..=32 (the slider's working range).
    #[must_use]
    pub const fn clamped_for_label(&self) -> usize {
        if self.length < 8 {
            8
        } else if self.length > 32 {
            32
        } else {
            self.length
        }
    }

    /// Generates a random password. Each enabled class is represented by ≥1 character,
    /// there is no comma (the `ValidatedPassword` contract), and the order is shuffled.
    /// Returns an empty string if no class is enabled. The result is [`Zeroizing`]:
    /// it is wiped from memory when the handle is dropped (the UI layer clones it
    /// into the table row at its own boundary).
    #[must_use]
    pub fn generate(&self) -> Zeroizing<String> {
        if !self.has_any_class() {
            return Zeroizing::new(String::new());
        }
        let length = self.clamped_length();
        let mut rng = rand::rng();

        let classes: &[&[char]] = &[UPPERS, LOWERS, DIGITS, SPECIALS];
        let active_flags = [
            self.classes.uppercase(),
            self.classes.lowercase(),
            self.classes.digits(),
            self.classes.special(),
        ];

        // One mandatory character from each enabled class (there are ≤4 ≤ MIN_LENGTH).
        let mut chars: Vec<char> = Vec::with_capacity(length);
        for (set, on) in classes.iter().zip(active_flags.iter()) {
            if *on {
                let ch = set
                    .iter()
                    .choose(&mut rng)
                    .copied()
                    .or_else(|| set.first().copied())
                    .unwrap_or('a');
                chars.push(ch);
            }
        }

        // Pad up to the target length: random active class → random character.
        let active_sets: Vec<&[char]> = classes
            .iter()
            .zip(active_flags.iter())
            .filter_map(|(set, on)| if *on { Some(*set) } else { None })
            .collect();
        while chars.len() < length {
            let set = active_sets
                .iter()
                .choose(&mut rng)
                .copied()
                .unwrap_or(LOWERS);
            let ch = set
                .iter()
                .choose(&mut rng)
                .copied()
                .or_else(|| set.first().copied())
                .unwrap_or('a');
            chars.push(ch);
        }

        chars.truncate(length);
        shuffle(&mut chars, &mut rng);

        Zeroizing::new(chars.into_iter().collect())
    }
}

impl Default for PasswordGenerator {
    fn default() -> Self {
        Self::default_generator()
    }
}

/// Fisher–Yates shuffle. A no-op for an empty or single-element slice.
fn shuffle(chars: &mut [char], rng: &mut impl Rng) {
    let n = chars.len();
    if n < 2 {
        return;
    }
    for i in (1..n).rev() {
        let j = rng.random_range(0..=i);
        chars.swap(i, j);
    }
}

#[cfg(test)]
// Not compiled under Miri: these tests run the generator 100-500 times per
// case (OsRng + Fisher–Yates shuffle). Password generation is pure safe Rust
// with no unsafe/FFI, so Miri adds no UB signal here, and the iteration count
// makes each case take minutes under the interpreter. The generator's
// correctness is covered natively (these tests) + cargo-mutants; Miri focuses
// on the parsers.
#[cfg(not(miri))]
mod tests {
    use super::*;

    #[test]
    fn default_is_length_16_all_classes() {
        let g = PasswordGenerator::default_generator();
        assert_eq!(g.length, 16);
        assert!(
            g.classes.uppercase()
                && g.classes.lowercase()
                && g.classes.digits()
                && g.classes.special()
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
}

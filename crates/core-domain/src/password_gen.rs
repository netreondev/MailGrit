//! Password generator with configurable complexity.
//!
//! Unlike [`PasswordPolicy`](crate::PasswordPolicy) (which only validates),
//! [`PasswordGenerator`] produces a password that satisfies the enabled
//! requirements by construction. Entropy comes from `rand::rng()` (a per-thread CSPRNG seeded
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

/// Lower bound of the UI length slider's working range.
pub const UI_MIN_LENGTH: usize = 8;
/// Upper bound of the UI length slider's working range (the display ceiling,
/// distinct from the generator's hard [`MAX_LENGTH`]).
///
/// Single source for the slider bounds and the app-side clamp (previously a
/// bare `32` in three places).
pub const UI_MAX_LENGTH: usize = 32;

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

    /// Length for the UI slider: clamped to [`UI_MIN_LENGTH`]..=[`UI_MAX_LENGTH`].
    #[must_use]
    pub const fn clamped_for_label(&self) -> usize {
        if self.length < UI_MIN_LENGTH {
            UI_MIN_LENGTH
        } else if self.length > UI_MAX_LENGTH {
            UI_MAX_LENGTH
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
#[path = "password_gen_tests.rs"]
mod tests;

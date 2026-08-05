//! Password generator with configurable complexity.
//!
//! Unlike [`PasswordPolicy`](crate::PasswordPolicy) (which only validates),
//! [`PasswordGenerator`] produces a password guaranteed to satisfy the enabled
//! requirements. Entropy comes from `rand::rng()` (`OsRng`).

use rand::Rng;
use rand::RngExt;
use rand::seq::IteratorRandom;

/// Minimum length: smaller values cannot fit all character classes.
const MIN_LENGTH: usize = 4;

/// Maximum reasonable password length in the UI (separate from `MAX_PASSWORD_LEN`).
const MAX_LENGTH: usize = 64;

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
#[allow(
    clippy::struct_excessive_bools,
    reason = "4 independent character-class flags are the canonical form (the same as in PasswordPolicy and iRedAdmin settings.py)"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordGenerator {
    /// Target password length. Clamped to [`MIN_LENGTH`]..=[`MAX_LENGTH`].
    pub length: usize,
    /// Use uppercase letters.
    pub use_uppercase: bool,
    /// Use lowercase letters.
    pub use_lowercase: bool,
    /// Use digits.
    pub use_digits: bool,
    /// Use special characters.
    pub use_special: bool,
}

impl PasswordGenerator {
    /// Default: length 16, all classes enabled.
    #[must_use]
    pub const fn default_generator() -> Self {
        Self {
            length: 16,
            use_uppercase: true,
            use_lowercase: true,
            use_digits: true,
            use_special: true,
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
        self.use_uppercase || self.use_lowercase || self.use_digits || self.use_special
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
    /// Returns an empty string if no class is enabled.
    #[must_use]
    pub fn generate(&self) -> String {
        if !self.has_any_class() {
            return String::new();
        }
        let length = self.clamped_length();
        let mut rng = rand::rng();

        let classes: &[&[char]] = &[UPPERS, LOWERS, DIGITS, SPECIALS];
        let active_flags = [
            self.use_uppercase,
            self.use_lowercase,
            self.use_digits,
            self.use_special,
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

        chars.into_iter().collect()
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
mod tests {
    #![allow(clippy::indexing_slicing)]
    use super::*;

    #[test]
    fn default_is_length_16_all_classes() {
        let g = PasswordGenerator::default_generator();
        assert_eq!(g.length, 16);
        assert!(g.use_uppercase && g.use_lowercase && g.use_digits && g.use_special);
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
                "generated password \"{pw}\" failed policy: {warnings:?}"
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
        g.use_special = false;
        g.use_digits = false;
        g.use_uppercase = false;
        for _ in 0..100 {
            let pw = g.generate();
            assert!(
                pw.chars().all(|c| c.is_ascii_lowercase()),
                "with classes disabled there should be only lowercase: \"{pw}\""
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
        g.use_uppercase = false;
        g.use_lowercase = false;
        g.use_digits = false;
        g.use_special = false;
        assert_eq!(
            g.generate(),
            "",
            "with no classes enabled generation is impossible"
        );
    }

    #[test]
    fn generates_are_not_constant() {
        let g = PasswordGenerator::default_generator();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            seen.insert(g.generate());
        }
        assert!(seen.len() > 1, "generator produces identical passwords");
    }
}

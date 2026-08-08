//! Character-class flag set shared by [`PasswordPolicy`](crate::PasswordPolicy)
//! and [`PasswordGenerator`](crate::PasswordGenerator).
//!
//! Both types store their four character-class toggles (uppercase/lowercase/
//! digits/special) in a single [`CharacterClasses`] field. This keeps each parent
//! struct below clippy's `struct_excessive_bools` threshold (no struct carries
//! more than three independent `bool` fields). The four flags themselves live in a
//! fixed `[bool; 4]` array (the established pattern in the app-desktop settings
//! config types) and are read/written through named accessors so the call sites
//! stay self-documenting.

/// Indexes into the [`CharacterClasses`] `[bool; 4]` storage.
const CLS_UPPER: usize = 0;
/// Indexes into the [`CharacterClasses`] `[bool; 4]` storage.
const CLS_LOWER: usize = 1;
/// Indexes into the [`CharacterClasses`] `[bool; 4]` storage.
const CLS_DIGIT: usize = 2;
/// Indexes into the [`CharacterClasses`] `[bool; 4]` storage.
const CLS_SPECIAL: usize = 3;

/// The four character-class flags as a positional `(uppercase, lowercase, digits,
/// special)` tuple.
///
/// Used as the parameter type for the [`CharacterClasses::from_tuple`]
/// constructor so the function takes a single typed value rather than four loose
/// `bool`s (which would trip clippy's `fn_params_excessive_bools`).
pub type ClassFlags = (bool, bool, bool, bool);

/// The four character-class toggles shared by the password policy and the
/// generator.
///
/// For [`PasswordPolicy`](crate::PasswordPolicy) the flags describe the *required*
/// classes; for [`PasswordGenerator`](crate::PasswordGenerator) they describe the
/// *enabled* classes. The names stay neutral (`uppercase`/`lowercase`/`digits`/
/// `special`) so the same type serves both roles. The struct is `Copy` (4 bytes)
/// and trivially constructable in `const` context, which keeps the
/// [`default_policy`](crate::PasswordPolicy::default_policy) and
/// [`default_generator`](crate::PasswordGenerator::default_generator) constructors
/// `const fn`.
///
/// The four flags live in a private `[bool; 4]` array (the same pattern as the
/// app-desktop `PolicyClasses`/`GeneratorClasses` config types) so that this
/// struct — like its parents — stays below clippy's `struct_excessive_bools`
/// threshold. Reads and writes go through the `uppercase`/`lowercase`/`digits`/
/// `special` accessors and their `set_*` counterparts; bulk construction uses
/// [`from_tuple`](Self::from_tuple) which takes a single [`ClassFlags`] value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterClasses([bool; 4]);

impl CharacterClasses {
    /// All four classes disabled. Useful for relaxed policies and for the
    /// "no class enabled" branch of the generator.
    #[must_use]
    pub const fn none() -> Self {
        Self([false, false, false, false])
    }

    /// All four classes enabled. Mirrors the iRedAdmin default policy and the
    /// generator default (length 16, every class on).
    #[must_use]
    pub const fn all() -> Self {
        Self([true, true, true, true])
    }

    /// Constructs a `CharacterClasses` from a [`ClassFlags`] `(uppercase,
    /// lowercase, digits, special)` tuple. Used at the config-to-domain boundary
    /// where the four flags arrive as a tuple from the serde-flattened config
    /// section. Taking a single tuple value avoids the four-bool-parameter lint.
    #[must_use]
    pub const fn from_tuple(flags: ClassFlags) -> Self {
        Self([flags.0, flags.1, flags.2, flags.3])
    }

    /// Whether at least one class is enabled. If every flag is `false`, password
    /// generation is impossible (the generator returns an empty string).
    #[must_use]
    pub const fn has_any(self) -> bool {
        self.0[CLS_UPPER] || self.0[CLS_LOWER] || self.0[CLS_DIGIT] || self.0[CLS_SPECIAL]
    }

    /// Whether the uppercase-letter class is enabled/required.
    #[must_use]
    pub const fn uppercase(self) -> bool {
        self.0[CLS_UPPER]
    }

    /// Whether the lowercase-letter class is enabled/required.
    #[must_use]
    pub const fn lowercase(self) -> bool {
        self.0[CLS_LOWER]
    }

    /// Whether the digit class is enabled/required.
    #[must_use]
    pub const fn digits(self) -> bool {
        self.0[CLS_DIGIT]
    }

    /// Whether the special-character class is enabled/required.
    #[must_use]
    pub const fn special(self) -> bool {
        self.0[CLS_SPECIAL]
    }

    /// Enables/disables the uppercase-letter class.
    pub const fn set_uppercase(&mut self, value: bool) {
        self.0[CLS_UPPER] = value;
    }

    /// Enables/disables the lowercase-letter class.
    pub const fn set_lowercase(&mut self, value: bool) {
        self.0[CLS_LOWER] = value;
    }

    /// Enables/disables the digit class.
    pub const fn set_digits(&mut self, value: bool) {
        self.0[CLS_DIGIT] = value;
    }

    /// Enables/disables the special-character class.
    pub const fn set_special(&mut self, value: bool) {
        self.0[CLS_SPECIAL] = value;
    }
}

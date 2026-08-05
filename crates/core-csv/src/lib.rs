//! `core-csv` — streaming parsing and validation of bulk user-import CSV.
//! Schema ([`CSV_HEADER`]): `domain,username,password,display_name,quota_mb`.
//! Strict memory limits; the output is a [`SanitizedUserRow`] (typestate) — raw
//! `String` does not propagate further. Failed rows accumulate in [`ParsedCsv::failed`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// In tests, unwrap/panic are permitted (a test failure is an intentional panic).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic
    )
)]

pub mod mapping;
pub mod parser;
mod util;

pub use mapping::{
    ColumnMapping, detect_mapping, parse_csv_bytes_auto, parse_csv_bytes_with_mapping,
    parse_csv_with_mapping,
};
pub use parser::{
    CSV_HEADER, CsvParseError, FailedRow, ParsedCsv, parse_csv, parse_csv_bytes,
    parse_csv_with_limit,
};

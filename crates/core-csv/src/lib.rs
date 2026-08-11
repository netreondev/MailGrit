//! `core-csv` — streaming parsing and validation of bulk user-import CSV.
//! Schema ([`CSV_HEADER`]): `domain,username,password,display_name,quota_mb`.
//! Strict memory limits; the output is a
//! [`SanitizedUserRow`](mailgrit_core_domain::SanitizedUserRow) (typestate) — raw
//! `String` does not propagate further. Failed rows accumulate in [`ParsedCsv::failed`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny) is set
// centrally in [workspace.lints.rust] of the root Cargo.toml. Test modules
// follow the same policy (no test-only suppressions).

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

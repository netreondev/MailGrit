//! `core-csv` — streaming parsing and validation of bulk user-import CSV.
//! Schema ([`CSV_HEADER`]): `domain,username,password,display_name,quota_mb`.
//! The wire format (bounded RFC-4180 reading AND escaping, incl. formula
//! neutralization) lives only here: [`record`] reads, [`escape`] writes —
//! what one module writes the other reads back (round-trip is test-pinned).
//! Strict memory limits; the output is a
//! [`SanitizedUserRow`](mailgrit_core_domain::SanitizedUserRow) (typestate) —
//! raw `String` does not propagate further. Failed rows accumulate in
//! [`ParsedCsv::failed`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny) is set
// centrally in [workspace.lints.rust] of the root Cargo.toml. Test modules
// follow the same policy (no test-only suppressions).

pub mod escape;
pub mod mapping;
pub mod parser;
pub mod record;
mod util;

pub use escape::escape_field;
pub use mapping::{
    ColumnMapping, detect_mapping, parse_csv_bytes_auto, parse_csv_bytes_with_mapping,
    parse_csv_with_mapping,
};
pub use parser::{
    CSV_HEADER, CsvParseError, FailedRow, ParsedCsv, parse_csv, parse_csv_bytes,
    parse_csv_with_limit,
};
pub use record::{MAX_RECORD_BYTES, Record, RecordOutcome, RecordReader};
pub use util::strip_bom;

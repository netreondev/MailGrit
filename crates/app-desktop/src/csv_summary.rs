//! Operational types of the application: a summary of the loaded CSV.
//!
//! Operation dispatch itself runs inside the login-webview via JS
//! (`webview_ops`), not via the tokio runtime, because the server behind
//! FortiWeb/WAF keeps the backend session at the browser. These types only
//! describe the operation parameters.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use mailgrit_core_csv::ParsedCsv;

/// A summary of the loaded CSV for display in the UI.
#[derive(Debug, Clone)]
pub struct CsvSummary {
    /// The number of valid rows.
    pub valid: usize,
    /// The number of rejected rows.
    pub failed: usize,
}

impl CsvSummary {
    /// Builds a summary from a parsed CSV.
    #[must_use]
    pub const fn from_parsed(parsed: &ParsedCsv) -> Self {
        Self {
            valid: parsed.rows.len(),
            failed: parsed.failed.len(),
        }
    }
}

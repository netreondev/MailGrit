//! CSV field escaping for export (RFC 4180 + formula-injection neutralization).
//!
//! The single wire-format writer side, paired with [`crate::record`] (the
//! reader side): what `escape_field` writes, the record reader parses back
//! (round-trip is pinned by tests in both modules).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// Characters that make spreadsheet apps (Excel, `LibreOffice`, Google Sheets)
/// evaluate a cell as a FORMULA when they start the cell value.
const FORMULA_PREFIXES: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// Escapes one CSV field for export:
///
/// 1. **Formula neutralization (OWASP):** a field starting with `=`, `+`, `-`,
///    `@`, tab, or CR is prefixed with `'`. RFC-4180 quoting alone does NOT
///    prevent evaluation — Excel and `LibreOffice` still evaluate quoted cells
///    (`=HYPERLINK(...)`, `@SUM(...)` were passing through the old escaper).
/// 2. **RFC 4180 quoting:** a field containing a comma, quote, or newline is
///    wrapped in quotes with inner quotes doubled.
///
/// Applied to every user-controlled field written to an export — including
/// free-text ones (`display_name`, FAIL reasons), which are the live
/// injection channels (domain/username are charset-restricted).
#[must_use]
pub fn escape_field(field: &str) -> String {
    let needs_quoting = field.chars().any(|c| matches!(c, ',' | '"' | '\n' | '\r'));
    let formula = field.starts_with(FORMULA_PREFIXES);
    if !needs_quoting && !formula {
        return field.to_owned();
    }
    let mut out = String::with_capacity(field.len().saturating_add(3));
    if needs_quoting {
        out.push('"');
    }
    if formula {
        // Inside the quotes the apostrophe remains the first cell character —
        // this is what stops the spreadsheet from treating the cell as a formula.
        out.push('\'');
    }
    for c in field.chars() {
        if c == '"' && needs_quoting {
            out.push('"'); // double it
        }
        out.push(c);
    }
    if needs_quoting {
        out.push('"');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_field_unchanged() {
        assert_eq!(escape_field("ivan.petrov"), "ivan.petrov");
        assert_eq!(escape_field(""), "");
        assert_eq!(escape_field("example.com"), "example.com");
    }

    #[test]
    fn comma_field_quoted() {
        assert_eq!(escape_field("Petrov, Ivan"), "\"Petrov, Ivan\"");
    }

    #[test]
    fn inner_quotes_doubled() {
        assert_eq!(escape_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn newline_field_quoted() {
        assert_eq!(escape_field("line1\nline2"), "\"line1\nline2\"");
        assert_eq!(escape_field("a\rb"), "\"a\rb\"");
    }

    // CSV formula injection (OWASP): =/+/-/@/\t/\r prefixes are neutralized
    // with a leading apostrophe — quoting alone does not stop evaluation.
    #[test]
    fn formula_prefixes_neutralized() {
        assert_eq!(
            escape_field("=HYPERLINK(\"http://evil\")"),
            "\"'=HYPERLINK(\"\"http://evil\"\")\""
        );
        assert_eq!(escape_field("@SUM(a1:a2)"), "'@SUM(a1:a2)");
        assert_eq!(escape_field("-2+1"), "'-2+1");
        assert_eq!(escape_field("+1"), "'+1");
    }

    #[test]
    fn neutralized_formulas_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        // The reader must parse the neutralized field back as data (with the
        // defensive apostrophe — the neutralized VALUE is what spreadsheets see).
        let escaped = escape_field("=cmd|' /C calc'!A0");
        let mut reader =
            crate::record::RecordReader::new(std::io::BufReader::new(escaped.as_bytes()));
        let Some(record) = reader.next_record()? else {
            return Err("expected one record".into());
        };
        match record.outcome {
            crate::record::RecordOutcome::Fields(f) => {
                assert_eq!(f, ["'=cmd|' /C calc'!A0"]);
            }
            crate::record::RecordOutcome::Failed(e) => {
                return Err(format!("round-trip failed: {e}").into());
            }
        }
        Ok(())
    }

    // A plain hyphen-containing value that does not START with a formula char
    // is untouched (only leading chars are dangerous).
    #[test]
    fn non_leading_hyphen_untouched() {
        assert_eq!(escape_field("Petrov - Ivan"), "Petrov - Ivan");
        assert_eq!(escape_field("a=b"), "a=b");
    }
}

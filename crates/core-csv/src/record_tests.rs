//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;
use std::io::BufReader;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn records(data: &[u8]) -> Result<Vec<Record>, std::io::Error> {
    let mut reader = RecordReader::new(BufReader::new(data));
    let mut out = Vec::new();
    while let Some(r) = reader.next_record()? {
        out.push(r);
    }
    Ok(out)
}

/// Checked accessor (repo style: no direct indexing).
fn at(recs: &[Record], i: usize) -> Result<&Record, String> {
    recs.get(i).ok_or_else(|| format!("missing record {i}"))
}

fn fields_of(r: &Record) -> Result<Vec<String>, String> {
    match &r.outcome {
        RecordOutcome::Fields(f) => Ok(f.clone()),
        RecordOutcome::Failed(e) => Err(format!("expected fields, got failure: {e}")),
    }
}

#[test]
fn plain_lines_split_on_comma() -> TestResult {
    let recs = records(
        b"a,b,c
",
    )?;
    assert_eq!(recs.len(), 1);
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b", "c"]);
    assert_eq!(at(&recs, 0)?.line_no, 1);
    Ok(())
}

#[test]
fn crlf_endings_stripped() -> TestResult {
    let recs = records(
        b"a,b
c,d
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b"]);
    assert_eq!(fields_of(at(&recs, 1)?)?, ["c", "d"]);
    Ok(())
}

#[test]
fn last_line_without_newline_is_a_record() -> TestResult {
    let recs = records(b"a,b")?;
    assert_eq!(recs.len(), 1);
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b"]);
    Ok(())
}

#[test]
fn trailing_comma_yields_empty_last_field() -> TestResult {
    let recs = records(
        b"a,b,
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "b", ""]);
    Ok(())
}

#[test]
fn quoted_comma_is_data() -> TestResult {
    let recs = records(
        b"\"Petrov, Ivan\",x
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["Petrov, Ivan", "x"]);
    Ok(())
}

#[test]
fn doubled_quote_unescapes() -> TestResult {
    let recs = records(
        b"\"say \"\"hi\"\"\"
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["say \"hi\""]);
    Ok(())
}

#[test]
fn quoted_newline_spans_physical_lines() -> TestResult {
    let recs = records(
        b"\"line1
line2\",second
",
    )?;
    assert_eq!(recs.len(), 1, "one logical record across two lines");
    assert_eq!(
        fields_of(at(&recs, 0)?)?,
        [
            "line1
line2",
            "second"
        ]
    );
    assert_eq!(
        at(&recs, 0)?.line_no,
        1,
        "line_no of the FIRST physical line"
    );
    Ok(())
}

#[test]
fn quote_mid_field_is_data_lenient() -> TestResult {
    let recs = records(
        b"a\"b,c
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a\"b", "c"]);
    Ok(())
}

#[test]
fn empty_quoted_field_is_empty() -> TestResult {
    let recs = records(
        b"a,\"\",b
",
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, ["a", "", "b"]);
    Ok(())
}

// The DoS regression: an endless line (no newline) over the cap must be a
// per-record failure that does NOT consume the following records, and the
// reader must never buffer more than cap+1 bytes of it.
#[test]
fn endless_line_is_failed_record_and_parser_resyncs() -> TestResult {
    let mut data = vec![b'a'; MAX_RECORD_BYTES + 100];
    data.extend_from_slice(
        b"
good.com,u,p,n,100
",
    );
    let recs = records(&data)?;
    assert_eq!(recs.len(), 2, "failed record + the next valid record");
    assert!(matches!(
        at(&recs, 0)?.outcome,
        RecordOutcome::Failed(CsvParseError::LineTooLong { line_no: 1, .. })
    ));
    assert_eq!(
        fields_of(at(&recs, 1)?)?,
        ["good.com", "u", "p", "n", "100"]
    );
    Ok(())
}

#[test]
fn invalid_utf8_is_per_record_failure() -> TestResult {
    let mut data = b"good.com,u,p,n,100
"
    .to_vec();
    data.extend_from_slice(&[0xff, 0xfe, b'\n']);
    data.extend_from_slice(
        b"good.com,u2,p2,n2,200
",
    );
    let recs = records(&data)?;
    assert_eq!(recs.len(), 3);
    assert_eq!(fields_of(at(&recs, 0)?)?.len(), 5);
    assert!(matches!(
        at(&recs, 1)?.outcome,
        RecordOutcome::Failed(CsvParseError::InvalidUtf8 { line_no: 2 })
    ));
    assert_eq!(
        fields_of(at(&recs, 2)?)?,
        ["good.com", "u2", "p2", "n2", "200"]
    );
    Ok(())
}

#[test]
fn field_over_cap_is_failed_record() -> TestResult {
    let big = "x".repeat(MAX_CSV_FIELD_BYTES + 1);
    let data = format!(
        "{big},u
"
    );
    let recs = records(data.as_bytes())?;
    assert!(matches!(
        at(&recs, 0)?.outcome,
        RecordOutcome::Failed(CsvParseError::FieldTooLong { line_no: 1, .. })
    ));
    Ok(())
}

// A quoted record spanning lines accumulates field content across lines;
// the per-field cap rejects it (the record fails, the reader resyncs).
#[test]
fn multi_line_record_over_field_cap_fails() -> TestResult {
    let mut data = String::from("\"");
    data.push_str(&"a".repeat(MAX_CSV_FIELD_BYTES - 10));
    data.push('\n'); // newline INSIDE quotes: the record continues
    data.push_str(&"b".repeat(MAX_CSV_FIELD_BYTES));
    data.push_str(
        "\"
next
",
    );
    let recs = records(data.as_bytes())?;
    assert!(
        matches!(
            at(&recs, 0)?.outcome,
            RecordOutcome::Failed(CsvParseError::FieldTooLong { .. })
        ),
        "an over-cap multi-line quoted field must fail the record"
    );
    // The reader resynchronized on the next record.
    assert_eq!(fields_of(at(&recs, 1)?)?, ["next"]);
    Ok(())
}

// Round-trip with the escape side (see escape.rs): every field escaped by
// escape_field parses back to the original value (for values that do not
// need formula neutralization).
#[test]
fn round_trip_with_escape_field() -> TestResult {
    let values = [
        "plain",
        "with,comma",
        "with \"quote\"",
        "multi
line",
        "",
        "tab	here",
    ];
    let mut line = String::new();
    for v in values {
        line.push_str(&crate::escape::escape_field(v));
        line.push(',');
    }
    // strip the trailing comma + newline-terminate
    let joined = line.strip_trailing_comma();
    let recs = records(
        format!(
            "{joined}
"
        )
        .as_bytes(),
    )?;
    assert_eq!(fields_of(at(&recs, 0)?)?, values);
    Ok(())
}

/// Test-only helper: strips one trailing comma.
trait StripTrailingComma {
    fn strip_trailing_comma(&self) -> String;
}
impl StripTrailingComma for String {
    fn strip_trailing_comma(&self) -> String {
        self.strip_suffix(',')
            .map_or_else(|| self.clone(), str::to_string)
    }
}

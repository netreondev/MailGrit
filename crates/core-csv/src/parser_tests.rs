//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;

const VALID_CSV: &str = concat!(
    "domain,username,password,display_name,quota_mb\n",
    "example.com,ivan.petrov,S3cur3P@ss1,Ivan Petrov,1024\n",
    "example.com,anna.kovalenko,Str0ngPwd!2,Anna Kovalenko,2048\n",
);

#[test]
fn parses_valid_csv_with_header() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_csv_bytes(VALID_CSV.as_bytes())?;
    assert_eq!(parsed.rows.len(), 2);
    assert!(parsed.failed.is_empty());
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .username
            .as_str(),
        "ivan.petrov"
    );
    assert_eq!(parsed.rows.get(1).ok_or("missing row 1")?.quota.mb(), 2048);
    Ok(())
}

#[test]
fn parses_without_header() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"example.com,user,Pass123,Name,512";
    let parsed = parse_csv_bytes(data)?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows.first().ok_or("missing row 0")?.quota.mb(), 512);
    Ok(())
}

#[test]
fn collects_failed_rows_without_stopping() -> Result<(), Box<dyn std::error::Error>> {
    // Second line is invalid (email instead of domain), third is valid.
    let data = concat!(
        "domain,username,password,display_name,quota_mb\n",
        "user@bad.com,u,p,n,100\n",
        "good.com,u2,p2,n2,200\n",
    );
    let parsed = parse_csv_bytes(data.as_bytes())?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.failed.len(), 1);
    assert_eq!(parsed.failed.first().ok_or("missing failed 0")?.line_no, 2);
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .username
            .as_str(),
        "u2"
    );
    Ok(())
}

#[test]
fn skips_blank_lines() -> Result<(), Box<dyn std::error::Error>> {
    let data = concat!(
        "\n",
        "domain,username,password,display_name,quota_mb\n",
        "\n",
        "good.com,u,p,n,200\n",
        "\n",
    );
    let parsed = parse_csv_bytes(data.as_bytes())?;
    assert_eq!(parsed.rows.len(), 1);
    assert!(parsed.failed.is_empty());
    Ok(())
}

#[test]
fn rejects_too_many_rows() -> Result<(), Box<dyn std::error::Error>> {
    let data = "good.com,u,p,n,200\ngood.com,u2,p2,n2,200\ngood.com,u3,p3,n3,300\n";
    match parse_csv_with_limit(data.as_bytes(), 2) {
        Ok(_) => return Err("expected TooManyRows error, got Ok".into()),
        Err(CsvParseError::TooManyRows { actual, max }) => {
            assert_eq!(max, 2);
            assert_eq!(actual, 3);
        }
        Err(other) => return Err(format!("expected TooManyRows, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn limit_allows_exact_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // Checks the boundary condition `processed >= max_rows` (strict inequality).
    let data = "good.com,u,p,n,200\ngood.com,u2,p2,n2,200\n";
    let parsed = parse_csv_with_limit(data.as_bytes(), 2)?;
    assert_eq!(parsed.rows.len(), 2);
    Ok(())
}

#[test]
fn empty_input_yields_empty_result() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_csv_bytes(b"")?;
    assert_eq!(parsed.rows.len(), 0);
    assert_eq!(parsed.failed.len(), 0);
    Ok(())
}

#[test]
fn quota_defaults_when_empty_field() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"good.com,u,p,n,";
    let parsed = parse_csv_bytes(data)?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(
        parsed.rows.first().ok_or("missing row 0")?.quota.mb(),
        mailgrit_core_domain::DEFAULT_QUOTA_MB
    );
    Ok(())
}

#[test]
fn wrong_column_count_goes_to_failed() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"good.com,u,p"; // 3 columns instead of 5
    let parsed = parse_csv_bytes(data)?;
    assert_eq!(parsed.rows.len(), 0);
    assert_eq!(parsed.failed.len(), 1);
    Ok(())
}

#[test]
fn detects_header_case_insensitively_trimmed() {
    // Splits like a simple unquoted line for header-detection purposes.
    let cells = |line: &str| line.split(',').map(str::to_string).collect::<Vec<_>>();
    // Canonical lower-case header.
    assert!(is_header(&cells(
        "domain,username,password,display_name,quota_mb"
    )));
    // Whitespace-trimmed.
    assert!(is_header(&cells(
        " domain , username , password , display_name , quota_mb "
    )));
    // Different casing (as exported by Excel/LibreOffice) — must still match.
    assert!(is_header(&cells(
        "Domain,Username,Password,Display_Name,Quota_MB"
    )));
    assert!(is_header(&cells(
        "DOMAIN,USERNAME,PASSWORD,DISPLAY_NAME,QUOTA_MB"
    )));
    // A real data line must NOT be mistaken for a header.
    assert!(!is_header(&cells("domain,username")));
    assert!(!is_header(&cells("example.com,user,S3cur3P@ss1,User,1024")));
}

// A quoted header (as Excel writes it) is recognized: unquoting happens
// in the record reader before header matching.
#[test]
fn quoted_header_is_recognized_and_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let data = concat!(
        "\"domain\",\"username\",\"password\",\"display_name\",\"quota_mb\"\n",
        "example.com,ivan.petrov,S3cr3P@ss1,Ivan Petrov,1024\n",
    );
    let parsed = parse_csv_bytes(data.as_bytes())?;
    assert_eq!(
        parsed.rows.len(),
        1,
        "the quoted header is skipped as a header"
    );
    assert!(parsed.failed.is_empty());
    Ok(())
}

// RFC 4180 data: commas/quotes inside quoted fields survive parsing —
// MailGrit's own (and Excel's) exports can be imported back.
#[test]
fn quoted_fields_parse_as_data() -> Result<(), Box<dyn std::error::Error>> {
    // display_name with a comma and doubled inner quotes: `Petrov, Ivan "IV"`.
    let data = concat!(
        "domain,username,password,display_name,quota_mb\n",
        "example.com,ivan.petrov,S3cr3,\"Petrov, Ivan \"\"IV\"\"\",1024\n",
    );
    let parsed = parse_csv_bytes(data.as_bytes())?;
    assert_eq!(parsed.rows.len(), 1);
    assert!(parsed.failed.is_empty(), "failed: {:?}", parsed.failed);
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .display_name
            .as_str(),
        "Petrov, Ivan \"IV\""
    );
    Ok(())
}

// Round-trip with the export side: escape_field output parses back into
// the original values (formula-neutralized values gain the apostrophe).
#[test]
fn own_export_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let display_name = "Petrov, Ivan \"IV\"";
    let mut line = String::from("example.com,ivan.petrov,S3cr3,");
    line.push_str(&crate::escape::escape_field(display_name));
    line.push_str(",1024\n");
    let parsed = parse_csv_bytes(line.as_bytes())?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(
        parsed
            .rows
            .first()
            .ok_or("missing row 0")?
            .display_name
            .as_str(),
        display_name
    );
    Ok(())
}

// Invalid UTF-8 no longer aborts the whole file (the old `lines()` made it
// a fatal I/O error): the row fails, its neighbours still import.
#[test]
fn invalid_utf8_is_per_row_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = b"good.com,u,p,n,100\n".to_vec();
    data.extend_from_slice(&[0xff, 0xfe, 0x0a]);
    data.extend_from_slice(b"good.com,u2,p2,n2,200\n");
    let parsed = parse_csv_bytes(&data)?;
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(parsed.failed.len(), 1);
    assert!(matches!(
        parsed.failed.first().ok_or("missing failed 0")?.error,
        CsvParseError::InvalidUtf8 { line_no: 2 }
    ));
    Ok(())
}

// An endless line over the cap is a per-row failure and the parser
// resynchronizes on the next record.
#[test]
fn endless_line_is_per_row_failure_and_parsing_continues() -> Result<(), Box<dyn std::error::Error>>
{
    let mut data = vec![b'a'; crate::record::MAX_RECORD_BYTES + 50];
    data.extend_from_slice(b"\ngood.com,u,p,n,100\n");
    let parsed = parse_csv_bytes(&data)?;
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.failed.len(), 1);
    assert!(matches!(
        parsed.failed.first().ok_or("missing failed 0")?.error,
        CsvParseError::LineTooLong { line_no: 1, .. }
    ));
    Ok(())
}

// Boundary limit tests (kill the `>` ↔ `==`/`>=` mutants in util::split_cells):
// a line of exactly MAX_LINE_BYTES passes, at +1 byte it is rejected.
#[test]
fn line_at_max_bytes_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = "d.com,u,p,n,";
    let filler_len = crate::util::MAX_LINE_BYTES - prefix.len();
    let filler = "a".repeat(filler_len);
    let line = format!("{prefix}{filler}");
    assert_eq!(line.len(), crate::util::MAX_LINE_BYTES);
    let parsed = parse_csv_bytes(line.as_bytes())?;
    assert!(
        parsed
            .failed
            .iter()
            .all(|f| !matches!(f.error, crate::CsvParseError::LineTooLong { .. })),
        "a line at the limit boundary must not be LineTooLong"
    );
    Ok(())
}

#[test]
fn line_over_max_bytes_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // A line 1 byte longer than MAX_LINE_BYTES → LineTooLong.
    let prefix = "d.com,u,p,n,";
    let filler_len = crate::util::MAX_LINE_BYTES
        .saturating_sub(prefix.len())
        .saturating_add(1);
    let filler = "a".repeat(filler_len);
    let line = format!("{prefix}{filler}");
    assert!(line.len() > crate::util::MAX_LINE_BYTES);
    let parsed = parse_csv_bytes(line.as_bytes())?;
    assert!(
        parsed
            .failed
            .iter()
            .any(|f| matches!(f.error, crate::CsvParseError::LineTooLong { .. })),
        "a line over the limit must be LineTooLong"
    );
    Ok(())
}

// A field of exactly MAX_CSV_FIELD_BYTES passes, at +1 byte it is FieldTooLong.
#[test]
fn field_at_max_bytes_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let max = mailgrit_core_domain::MAX_CSV_FIELD_BYTES;
    let domain = "a".repeat(max);
    let line = format!("{domain},u,p,n,100");
    let parsed = parse_csv_bytes(line.as_bytes())?;
    assert!(
        parsed
            .failed
            .iter()
            .all(|f| !matches!(f.error, crate::CsvParseError::FieldTooLong { .. })),
        "a field at the limit boundary must not be FieldTooLong"
    );
    Ok(())
}

#[test]
fn field_over_max_bytes_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let max = mailgrit_core_domain::MAX_CSV_FIELD_BYTES;
    let domain = "a".repeat(max.saturating_add(1));
    let line = format!("{domain},u,p,n,100");
    let parsed = parse_csv_bytes(line.as_bytes())?;
    assert!(
        parsed
            .failed
            .iter()
            .any(|f| matches!(f.error, crate::CsvParseError::FieldTooLong { .. })),
        "a field over the limit must be FieldTooLong"
    );
    Ok(())
}

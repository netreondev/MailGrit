# Privacy & Data Handling

**Українська:** [PRIVACY.uk.md](PRIVACY.uk.md)

MailGrit is a **local desktop application**. It is designed to minimize data
exposure and to keep all user data on the user's own machine.

## No telemetry

MailGrit does **not** collect, transmit, or share telemetry, analytics, crash
reports, or any usage information. **There is no "call home."** The application
makes **no network requests except to the iRedAdmin server URL you configure** —
all such traffic is for performing the operations you explicitly request.

## Where data is stored

All application data is kept in a single folder named **`mailgrit-data/`**,
located **next to the MailGrit executable** (portable mode). This folder
contains:

| File | Contents | Sensitive? |
|------|----------|------------|
| `config.toml` | App settings (theme, selected language, server URL) | Low |
| `mailgrit.log` | Rolling log file (operation dumps) | Medium (see masking below) |
| `mailgrit-audit.sqlite` | Hash-chained audit log of every operation | Not encrypted; see below |
| cookies | The iRedAdmin session cookie store for the embedded browser | Session-only |

**Nothing leaves your machine.** There is no cloud sync, no remote backup, and no
account system.

## Sensitive data: passwords and credentials

The CSV file you load contains **passwords**. Here is how they are handled:

- **In memory**: passwords are held in application memory only for as long as
  needed to perform the requested operations.
- **Audit log**: the audit log records operation metadata (action, success and
  failure counts, error texts) and is **not encrypted at rest** — entries are
  stored as-is in a local SQLite file, chained with HMAC-SHA256. The chain key
  is derived from your **master password** via Argon2id (a memory-hard
  key-derivation function). The master password is **never stored** on disk.
- **Export**: the optional CSV export can be written **encrypted** (sealed with
  XChaCha20-Poly1305, keyed by the master password) or as plain text. Plain-text
  export contains passwords in cleartext — the UI warns about this and it is off
  by default.
- **If you lose the master password**, the audit log can no longer be
  **verified** and encrypted exports **cannot be unlocked**.

## PII masking in logs

Operation diagnostics are written to `mailgrit.log` and shown in the UI. The
`mfMask` function masks sensitive fields in these dumps:

- **Passwords** (`newpw`, `confirmpw`) → masked (only a prefix shown, rest as `***`).
- **CSRF tokens** (`csrf_token`) → masked.
- **Emails / usernames** (`mail`, `username`) → partially masked
  (first character + `***` + domain).

This reduces accidental exposure of secrets in logs you may share for support.

## Deleting your data

To remove **all** MailGrit data:

1. Close MailGrit.
2. Delete the **`mailgrit-data/`** folder next to the executable.

This removes config, logs, the audit database, and cookies. If you enabled
encrypted export files, delete those files separately wherever you saved them.
Note that once deleted, encrypted exports are **unrecoverable** without
the master password (and unrecoverable at all once the files are gone).

## Cryptography and export-control note

MailGrit uses standard cryptographic primitives (XChaCha20-Poly1305 AEAD,
Argon2id KDF, HMAC-SHA256). Some countries regulate the import, export,
possession, or use of
encryption software. **You are responsible** for complying with any applicable
export-control or encryption laws in your jurisdiction. This document is a
factual description of the cryptography used, not a legal classification or
compliance certification.

## Third-party services

MailGrit does not use any third-party analytics, advertising, or tracking
service. The only third party it communicates with is the **iRedAdmin server you
configure**; that server's privacy practices are outside MailGrit's control.

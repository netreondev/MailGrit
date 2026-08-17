# MailGrit

[![CI](https://github.com/netreondev/MailGrit/actions/workflows/ci.yml/badge.svg)](https://github.com/netreondev/MailGrit/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust 1.97.1](https://img.shields.io/badge/rust-1.97.1-orange.svg)](https://www.rust-lang.org)
[![Platforms](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS%20ARM-lightgrey.svg)](#platforms)

> A cross-platform desktop client (Windows / Linux / macOS ARM) for bulk
> automation of **iRedAdmin**. Built with **Dioxus desktop** (no Node/JS
> toolchain), with an embedded browser for authentication and a native panel for
> bulk operations.

**Українська:** [README.uk.md](README.uk.md) · **Website:** [netreondev.github.io/MailGrit](https://netreondev.github.io/MailGrit)

---

## Quick start

```bash
cargo build --release -p mailgrit-app-desktop
# → target/release/mailgrit-app-desktop[.exe]
```

On first launch a `mailgrit-data/` folder is created **next to the binary**,
holding all application files.

## Usage

> ⚠️ **Authorized use only.** MailGrit performs bulk create/edit/delete on an
> iRedAdmin server. Use it **only on systems and accounts you own or are
> authorized to administer.** Unauthorized use is illegal and is your sole
> responsibility. See [DISCLAIMER.md](DISCLAIMER.md).

1. Launch `mailgrit-app-desktop`. Enter the iRedAdmin URL
   (e.g. `https://mail.example.com/iredadmin`) and press **Open login form**.
   An application window with the real iRedAdmin form opens.
2. Sign in to iRedAdmin — the app detects the login automatically (a hybrid
   predicate — see *How authentication works*) and opens the operations panel.
3. Load a CSV (`domain,username,password,display_name,quota_mb`) — see
   [CSV format](#csv-format) or start from [`docs/assets/example.csv`](docs/assets/example.csv).
4. Run bulk **create / edit / delete**.

### Operation mode

Only one mode is supported — **OSE (forms)**: bulk operations run as a JS `fetch`
POST of the standard iRedAdmin create/edit HTML form, with a CSRF token. Form
fields:
`csrf_token, domainName, username, newpw, confirmpw, cn, preferredLanguage,
mailQuota, submit_add_user`.

> Requests go through the embedded browser webview that holds your session, so
> they generally pass FortiWeb/WAF as well; WAF configurations vary and this is
> not tested against every setup — reports of problems are welcome.

### Interface

Frameless UI: a custom titlebar with the wordmark and window buttons, dark/light
themes with a toggle (saved to `config.toml`), a symmetric card grid, modal
confirmation for destructive operations, and progress indicators. All components
are a home-grown design system on CSS tokens (no Node/JS toolchain).

### Diagnostics

The **Form diagnostics** button performs a GET of the user-creation page and
returns the form HTML (field names, action URL, CSRF), shown in the UI and log.

The **full operation dump** logs, for every operation: request URL/method/
Content-Type, all form fields (with password/CSRF/email/PII masked), CSRF value
and GET-form status, HTTP response status/headers, the full server response body
(up to 5000 chars), and the success/error markers plus post-verification result.

## How authentication works

The login is detected **data-driven**, by the login webview navigating to
`/dashboard`. iRedAdmin, Django, and FortiWeb all use different cookie names,
and behind a WAF the backend session is held by the proxy, so a cookie replayed
in a separate HTTP client usually does not authenticate against the backend.
That is why operations are executed as JS `fetch()` **inside the same webview**
that holds the session.

## Data storage & encryption

- **Audit log** — every operation is appended to a local log chained with
  HMAC-SHA256; the `verify` command checks the chain and reports mismatches.
  The chain provides a consistency check, **not** confidentiality: the action
  payload is stored in the local SQLite audit file (`mailgrit-audit.sqlite`)
  as-is, **not** encrypted at rest. The data folder lives **next to the
  executable** (portable mode), so its privacy depends on where you place it.
  (Exports and backups, in contrast, **are** encrypted with streaming AEAD
  (XChaCha20-Poly1305) — see "Encrypted exports" below.)
- **Encrypted exports/backups** — bulk exports and backups are encrypted at rest
  with streaming AEAD (XChaCha20-Poly1305); the key is derived from the master
  password.
- **Master password** — derives the audit-log key and the export-encryption key
  via Argon2id (memory-hard KDF). The password is never stored; if lost, the
  audit log cannot be **verified** and encrypted exports cannot be unlocked.

## CSV format

Bulk input is a plain CSV. Grab a ready-to-edit sample and adapt it:

- [`docs/assets/example.csv`](docs/assets/example.csv) — 6 valid rows you can load
  as-is.

```
domain,username,password,display_name,quota_mb
example.com,john,S3cret!23,John Doe,512
example.com,bob,Passw0rd!9,,1024
corp.example.net,alice_lee,Qx7$mK2v,Аліса Лі,
```

### Columns

| Column | Required | Format | Default |
|--------|----------|--------|---------|
| `domain` | yes | DNS domain, no `@` (e.g. `example.com`, not `john@example.com`) | — |
| `username` | yes | mailbox local-part: `a–z A–Z 0–9 . _ -`, 1–64 chars; must not start/end with `.` or `-` | — |
| `password` | yes | plaintext; **must not contain a comma** (the CSV has no quoting); leading/trailing spaces are kept | — |
| `display_name` | no | free text, up to 256 chars (control chars stripped) | empty |
| `quota_mb` | no | integer, MiB, range `1`–`1 048 576` (1 TiB) | `1024` |

The mailbox address created in iRedAdmin is `username@domain`.

### Rules

- **Delimiter:** comma only — there is no quoting/escaping. Any comma inside a
  field splits it, so keep passwords comma-free.
- **Header:** recommended but optional. If the first non-empty line matches the
  five names above (case-insensitive), it is treated as a header. **Column order
  is flexible when a header is present** — names are matched, not positions, so
  localized or renamed headers still map. Without a header, columns must be in
  the exact order `domain,username,password,display_name,quota_mb`.
- **Encoding:** UTF-8. A leading UTF-8 BOM is stripped automatically.
- **Blank lines** are skipped.
- **Hard limits** guard against accidental huge inputs: at most **50 000 rows**,
  **16 KiB** per line, **4096 bytes** per field. See the limit constants in
  `core-domain`.

### Common mistakes

- Putting a full email (`john@example.com`) in the `domain` column — use just
  the domain (`example.com`).
- A comma inside the `password` — there is no quoting to escape it; pick a
  comma-free password.
- Quota units — the value is a bare MiB integer (`512`), not `512MB`.
- Wrong column count when there is no header — every row must be exactly five
  fields in canonical order.
- Username with spaces or non-Latin characters — only `a–z A–Z 0–9 . _ -`.

Rows that fail validation are reported individually and do **not** stop the
import — valid rows still go through, and each failed row shows its line number
and the reason.

## Internationalization (i18n)

The UI ships in **9 languages**:

| Code | Language    | Endonym     |
|------|-------------|-------------|
| en   | English     | English     |
| de   | German      | Deutsch     |
| fr   | French      | Français    |
| es   | Spanish     | Español     |
| it   | Italian     | Italiano    |
| pt   | Portuguese  | Português   |
| nl   | Dutch       | Nederlands  |
| pl   | Polish      | Polski      |
| uk   | Ukrainian   | Українська  |

Translations are embedded into the binary at compile time (`rust-i18n`); the
selected language is persisted in `config.toml`.

## Architecture

A Cargo workspace of five crates, each compiling/testing in isolation
(`cargo test -p <crate>`):

| Crate | Responsibility |
|-------|----------------|
| `core-domain` | Domain types (Newtype/Typestate), domain errors, limit constants, password policy |
| `core-csv` | CSV parsing & validation with BOM stripping and hard memory limits |
| `core-storage` | Local SQLite storage: operation log, hash-chained audit log |
| `core-security` | At-rest crypto: streaming AEAD, HMAC hash-chain, Argon2id KDF |
| `app-desktop` | Dioxus 0.7 desktop app: login window, cookie handling, native RSX panel |

```
core-domain ─┬─ core-csv ────┐
             ├─ core-storage ─┤
             └─ core-security ┴─ app-desktop
```

## Development

| Task | Command |
|------|---------|
| Format | `cargo fmt --all` (check: `cargo fmt --all -- --check`) |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::multiple_crate_versions` |
| Tests | `cargo nextest run --workspace` |
| Doc tests | `cargo test --workspace --doc` |
| Release build | `cargo build --release -p mailgrit-app-desktop` |
| Run with debug log | `$env:RUST_LOG="debug"; .\target\release\mailgrit-app-desktop.exe` |
| Supply chain | `cargo deny check advisories bans licenses sources` then `cargo audit` |
| Unused deps | `cargo machete --skip-target-dir` |
| SemVer | `cargo semver-checks --workspace` |

### Lint discipline

The workspace enforces a strict, declarative lint policy in `Cargo.toml`:
`panic`, `unwrap_used`, `expect_used`, `indexing_slicing`,
`arithmetic_side_effects`, `todo`/`unimplemented`/`unreachable`, `dbg_macro`,
and `print_stdout`/`print_stderr` are all `deny`; `unsafe_code` is `forbid`.
All clippy groups (`correctness`, `suspicious`, `complexity`, `perf`,
`pedantic`, `nursery`, `cargo`) are `deny`; technical terms and acronyms in
doc comments are wrapped in backticks so `doc_markdown` passes with no
exceptions.

### Toolchain

Rust **1.97.1**, pinned via `rust-toolchain.toml` (edition 2024).

### Verification in CI

Beyond the unit and integration tests, CI runs lints, dependency checks
(cargo-deny / cargo-audit / cargo-vet), fuzzing, and mutation testing on every
push. The full tool list and what each check covers is in
[CONTRIBUTING.md](CONTRIBUTING.md#verification-tooling); instructions for
verifying a downloaded release binary are in [SECURITY.md](SECURITY.md).

## Platforms

| Platform | Target |
|----------|--------|
| Windows | `x86_64-pc-windows-msvc` |
| Linux | `x86_64-unknown-linux-gnu` |
| macOS (Apple Silicon) | `aarch64-apple-darwin` |

CI runs the full quality gate (fmt, clippy, nextest, doc tests, dependency
checks, semver checks) on all three platforms, plus a release build matrix.
Additional checks — UB detection, mutation testing, and continuous fuzzing —
run on every push/PR and block the merge; model checking of the core-domain
parsers runs on a weekly schedule. Details in
[CONTRIBUTING.md](CONTRIBUTING.md#verification-tooling).

## License

Dual-licensed under **MIT OR Apache-2.0**, at your option. See
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE). Contributions are
made under the same dual license (inbound = outbound); see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Legal & privacy

| Document | Purpose |
|----------|---------|
| [DISCLAIMER.md](DISCLAIMER.md) | Authorized-use policy, no-warranty, and limitation of liability |
| [PRIVACY.md](PRIVACY.md) | Data handling — local-only storage, PII masking, no telemetry |
| [SECURITY.md](SECURITY.md) | Vulnerability reporting and security posture |
| [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) | Community standards (Contributor Covenant 2.1) |
| [NOTICE.md](NOTICE.md) | Copyright, licensing, and trademark attribution |

**Trademarks:** "iRedAdmin" is a trademark of its respective owners. MailGrit is
**not affiliated with, endorsed by, or sponsored by** iRedAdmin or its
developers; the name is used solely to indicate compatibility. See
[NOTICE.md](NOTICE.md).

## Support the project

MailGrit is free and open source. Support the development and research behind
this project — your contribution helps cover continued development, computing
infrastructure, testing, and further research related to MailGrit.

- [Donate](https://donatello.to/VladymyrM)

> **Note:** Donations are **voluntary support** for the project. A donation does
> **not** grant the donor any goods, services, guarantees, priority support,
> warranties, or any obligation on the part of the author. MailGrit remains
> distributed under the [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE)
> licenses, with no warranty — see [DISCLAIMER.md](DISCLAIMER.md).


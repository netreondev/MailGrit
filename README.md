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
2. Sign in to iRedAdmin — **you do not need to press anything else**: the app
   detects the login automatically (a hybrid predicate — see *How authentication
   works*) and switches to the operations panel on its own.
3. Load a CSV (`domain,username,password,display_name,quota_mb`).
4. Run bulk **create / edit / delete**.

### Operation mode

Only one mode is supported — **OSE (forms)**: bulk operations run as a JS `fetch`
POST of the standard iRedAdmin create/edit HTML form, with a CSRF token. Form
fields:
`csrf_token, domainName, username, newpw, confirmpw, cn, preferredLanguage,
mailQuota, submit_add_user`.

> Requests go through the embedded browser webview with a legitimate session, so
> the mode works behind FortiWeb/WAF too.

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
`/dashboard` — not by cookie-name guessing. iRedAdmin, Django, and FortiWeb all
use different cookie names, so guessing is brittle; behind a WAF the backend
session is held by the proxy, and replaying a cookie in a separate HTTP client
does not authenticate against the backend. Therefore operations are executed as
JS `fetch()` **inside the same webview** that holds the legitimate session.

## Security model

- **Encrypted hash-chained audit log** — every operation is appended to a
  tamper-evident chain (HMAC-SHA256), encrypted at rest with streaming AEAD
  (XChaCha20-Poly1305).
- **Master password** — derives the audit-log key and the export-encryption key
  via Argon2id (memory-hard KDF). The password is never stored; if lost, the
  audit log and encrypted exports cannot be unlocked.
- **`unsafe_code = "forbid"`** at the workspace level — no `unsafe` anywhere in
  the application crates (wry 0.53 returns HttpOnly cookies natively).

## CSV format

```
domain,username,password,display_name,quota_mb
example.com,john,S3cret!,John Doe,512
```

- BOM is stripped automatically; encoding must be UTF-8.
- Flexible column mapping: header names are matched case-insensitively against
  canonical fields, so localized or renamed headers still map.
- Hard limits (rows, line length, field length) protect against accidental huge
  inputs; see `core-domain` limit constants.

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
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
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
`pedantic`, `nursery`, `cargo`) are `deny`. The only documented exception is
`doc_markdown = "allow"` (false-positives on technical acronyms).

### Toolchain

Rust **1.97.1**, pinned via `rust-toolchain.toml` (edition 2024).

## Platforms

| Platform | Target |
|----------|--------|
| Windows | `x86_64-pc-windows-msvc` |
| Linux | `x86_64-unknown-linux-gnu` |
| macOS (Apple Silicon) | `aarch64-apple-darwin` |

CI runs the full quality gate (fmt, clippy, nextest, doc tests, cargo-deny,
cargo-audit, cargo-machete, cargo-semver-checks) on all three, plus a release
build matrix. Formal verification (Kani, Miri), mutation testing (cargo-mutants),
and continuous fuzzing (cargo-fuzz) run nightly and are non-blocking.

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
| [NOTICE.md](NOTICE.md) | Copyright, licensing, and trademark attribution |

**Trademarks:** "iRedAdmin" is a trademark of its respective owners. MailGrit is
**not affiliated with, endorsed by, or sponsored by** iRedAdmin or its
developers; the name is used solely to indicate compatibility. See
[NOTICE.md](NOTICE.md).

## Support the project

MailGrit is free and open source. If it saves you time, consider supporting its
development:

- [Donate](https://donatello.to/VladymyrM)


# Security Policy

**Українська:** [SECURITY.uk.md](SECURITY.uk.md)

## Reporting a vulnerability

We take security bugs seriously. Thank you for improving MailGrit — please report
vulnerabilities **responsibly** and **privately**, so a fix can be prepared
before public disclosure.

**Preferred channel:** open a **private security advisory** on GitHub:
[Report a vulnerability](https://github.com/netreondev/MailGrit/security/advisories/new).

Please include:

- A description of the issue and its potential impact.
- Steps to reproduce (a minimal example, if possible).
- The MailGrit version and operating system you tested on.
- Any suggested fix (optional).

We will acknowledge receipt within **5 business days** and aim to send an initial
assessment within **14 days**. Please do **not** disclose the issue publicly
until a fix is released and you are notified.

## Supported versions

Only the **latest release** receives security fixes. Releases are published on
the [releases page](https://github.com/netreondev/MailGrit/releases).

## Scope

This policy covers the **MailGrit source code** in this repository. It does
**not** cover:

- **iRedAdmin** itself or any third-party mail server — MailGrit is an
  independent client and is not affiliated with iRedAdmin.
- Third-party dependencies, which are tracked via `cargo audit` / `cargo-deny`
  in CI; please report those upstream to the relevant crate maintainer and the
  [RustSec advisory database](https://rustsec.org/).

## Security posture

MailGrit is engineered with defense in depth:

- **`unsafe_code = "forbid"`** at the workspace level — no `unsafe` Rust anywhere
  in the application crates (verified in CI by clippy/rustc).
- **Strict lint discipline** — `panic`/`unwrap`/`expect`/`indexing_slicing`/
  `arithmetic_side_effects` are all `deny`, reducing runtime panics.
- **Encrypted audit log** — every operation is appended to a hash-chained
  (HMAC-SHA256), tamper-evident log encrypted at rest with streaming AEAD
  (XChaCha20-Poly1305).
- **Master-password key protection** — the audit-log key is derived from a
  user-chosen master password via Argon2id (memory-hard KDF); the password is
  never stored.
- **Constant-time comparison** for cryptographic equality checks.

## Supply-chain integrity

CI runs on every push and pull request:

- `cargo deny check advisories bans licenses sources` — known vulnerabilities,
  disallowed licenses, unauthorized registries/git sources.
- `cargo audit` — independent advisory check.
- `cargo machete` — unused-dependency hygiene.
- `cargo semver-checks` — public-API compatibility.

Pinned Rust toolchain (`1.97.1`) for reproducible builds.

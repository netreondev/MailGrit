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
- **Hash-chained audit log (tamper-evident, not encrypted)** — every operation
  is appended to a hash-chained (HMAC-SHA256), tamper-evident log so that any
  deletion, reordering, or modification of a past entry is detected on `verify`.
  The chain provides **integrity**, not confidentiality: the action payload is
  stored as-is in the local SQLite audit file (`mailgrit-audit.sqlite`), **not**
  encrypted at rest; confidentiality relies on the OS-protected per-user
  app-data directory. Exports and backups, by contrast, **are** encrypted with
  streaming AEAD (XChaCha20-Poly1305).
- **Master-password key protection** — the audit-log key is derived from a
  user-chosen master password via Argon2id (memory-hard KDF); the password is
  never stored.
- **Constant-time comparison** for cryptographic equality checks.

## Supply-chain integrity

CI runs on every push and pull request:

- `cargo deny check advisories bans licenses sources` — known vulnerabilities,
  disallowed licenses, unauthorized registries/git sources.
- `cargo audit` — independent advisory check.
- `cargo vet` — every dependency is either audited by us (crypto/storage, see
  `supply-chain/audits.toml`), covered by a trusted third-party audit import
  (Mozilla / Google / Bytecode Alliance / Embark, see `supply-chain/config.toml`
  and `supply-chain/imports.lock`), or recorded as an explicit tracked exemption.
  Caveat: an **exemption** is a "not audited, but permitted" record, not an
  attestation; the large majority of the transitive tree is currently covered by
  exemptions rather than first-party audits. The practical value of this gate is
  that a PR introducing a **brand-new** dependency that is neither audited nor
  exempted fails the build, forcing a conscious decision. Every exemption
  carries an explicit note; there is **no expiry mechanism** on exemptions —
  cargo-vet (verified 2026-08-16 against 0.10.2, the newest release, and
  upstream `main`) has no `expire-date` field for exemptions (expiration
  exists only for wildcard *audits*), and unknown config keys are silently
  dropped when the tool rewrites the file. The exemptions are a bootstrap
  debt to be worked down through manual review, not a time-limited permit.
- `cargo machete` — unused-dependency hygiene.
- `cargo semver-checks` — public-API compatibility.
- `gitleaks` — scans git history for accidentally-committed secrets.
- `Semgrep` (SAST) — static analysis focused on JS-injection into the privileged
  webview and secret leakage into `tracing` logs.

Pinned Rust toolchain (`1.97.1`) for reproducible builds.

## Release verification

Every published binary is built with supply-chain hardening so a downstream user
can independently verify authenticity and dependency provenance. The release
workflow (`.github/workflows/release.yml`) produces, per platform:

- **Embedded dependency list** — `cargo auditable build` compiles the full
  dependency tree (crate names + versions + known advisories at build time)
  into the binary. Scan a downloaded binary without source access:

  ```bash
  cargo install cargo-audit cargo-auditable
  cargo audit bin ./mailgrit-app-desktop.exe
  ```

- **CycloneDX SBOM** — `mailgrit-sbom-<platform>.cdx.json` is attached to each
  release, capturing the resolved dependency graph at build time.

- **Build-provenance attestation** — each archive carries a build-origin
  attestation (a signed in-toto statement with a SLSA provenance predicate),
  generated by
  [actions/attest-build-provenance](https://github.com/actions/attest-build-provenance)
  keylessly via the workflow's ephemeral GitHub OIDC identity (no signing keys
  in the repo) and stored in the repository's GitHub artifact-attestation
  store. This replaced the third-party slsa-github-generator workflow, which
  no longer works under current runner images. Verify the binary was built
  from this repository:

  ```bash
  # Install: https://cli.github.com/
  gh attestation verify mailgrit-windows-x86_64.zip \
    --repo netreondev/MailGrit
  ```

- **cosign keyless signatures** — each archive (`*.zip` / `*.tar.gz`) carries a
  detached signature `<archive>.sig` and signing certificate `<archive>.pem`,
  issued via the workflow's ephemeral Fulcio identity and logged to the public
  Rekor transparency log. Verify authenticity:

  ```bash
  # Install: https://github.com/sigstore/cosign/releases
  cosign verify-blob \
    --certificate mailgrit-windows-x86_64.zip.pem \
    --signature mailgrit-windows-x86_64.zip.sig \
    --certificate-identity-regexp 'https://github.com/netreondev/MailGrit/.*' \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    mailgrit-windows-x86_64.zip
  ```

Do not run a binary that fails either the provenance or the signature check.


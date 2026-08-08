# Contributing to MailGrit

**Українська:** [CONTRIBUTING.uk.md](CONTRIBUTING.uk.md)

Thank you for your interest in contributing to MailGrit! This document explains
how to contribute, the standards we keep, and the signing requirement.

## Quick start

```bash
git clone https://github.com/netreondev/MailGrit.git
cd MailGrit
cargo build -p mailgrit-app-desktop          # build
cargo nextest run --workspace                # tests
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Rust **1.97.1** is required (pinned via `rust-toolchain.toml`, edition 2024).

## Before you open a pull request

- **Search existing issues/PRs** first to avoid duplicates.
- **Open an issue** for large changes or new features, to discuss the design
  before investing time in code.
- **Keep PRs focused** — one logical change per PR is easier to review.

## Code standards

MailGrit enforces a strict, automated quality bar. Your PR must pass all of:

| Check | Command |
|-------|---------|
| Formatting | `cargo fmt --all -- --check` |
| Lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Tests | `cargo nextest run --workspace` |
| Doc tests | `cargo test --workspace --doc` |
| Supply chain | `cargo deny check advisories bans licenses sources` |

Key rules:

- **No `unsafe`** — `unsafe_code = "forbid"` at the workspace level. There are no
  exceptions in the application crates.
- **No panics in production code** — `panic`, `unwrap_used`, `expect_used`,
  `indexing_slicing`, `arithmetic_side_effects` are all `deny` (tests may relax
  these locally with a documented `reason`).
- **`#![forbid(unsafe_code)]`** is set on each crate.
- **No Russian** in comments, docs, or strings — the project is documented in
  **English and Ukrainian** only.

### Verification & supply-chain tooling

In addition to the table above, CI runs (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

- **`cargo audit`** + **`cargo vet`** — independent advisory check and audited /
  imported dependency attestations. Adding a new dependency means it must be
  either covered by a trusted import or recorded in `supply-chain/audits.toml`
  / `supply-chain/config.toml`. Run `cargo vet suggest` to see what to review.
  Honesty note: `cargo vet` distinguishes a real audit (in `audits.toml`, where
  we have actually reviewed the crate — currently a small set of crypto/storage
  crates) from a tracked **exemption** (in `config.toml`, which records that a
  crate is *not* audited but is permitted anyway). The vast majority of the
  transitive tree is currently covered by exemptions, not first-party audits;
  the CI gate's real value is that it **blocks the introduction of a brand-new,
  unaudited-and-unexempted dependency**, not that it attests every existing one.
  When you add a security-relevant dependency (crypto, storage, network),
  prefer adding a real entry to `audits.toml` over an exemption.
- **Gitleaks** + **Semgrep** — secret scanning and SAST. Keep
  `.gitleaks.toml` / `.semgrepignore` up to date if you add new generated /
  vendored paths.
- **Fuzz regression** — `fuzz/seeds/` holds committed regression inputs replayed
  on every PR. If you fix a fuzz-found bug, add a seed under
  `fuzz/seeds/<target>/`.
- **Deterministic secret-leak test** — `crates/app-desktop/src/webview_secret_leak_tests.rs`
  asserts passwords never reach `tracing` output. If you add new logging near
  sensitive data, extend it.
- **Blocking CI gates (every push/PR)** — Kani, Miri, cargo-mutants, cargo-fuzz
  (these use the nightly Rust *toolchain* and run on every push/PR, not on a
  schedule). They fail the PR on a finding: a Miri UB, a failed Kani proof, a
  surviving mutant, or a fuzzer crash blocks the merge. Fix the root cause
  (don't suppress) before merging.

## Commit messages

Use the **Conventional Commits** format:

```
<type>(<scope>): <short summary in imperative mood>

<optional body — what and why, not how>
```

Examples: `feat(audit): rotate keys on lock`, `fix(csv): strip UTF-8 BOM before
mapping`, `docs: clarify export encryption`. Keep the summary ≤ 72 characters.

## Developer Certificate of Origin (DCO)

To ensure every contributor has the right to submit their work under MailGrit's
license (MIT OR Apache-2.0), we require the **Developer Certificate of Origin**.
This is the same lightweight process used by the Linux kernel and many Rust
projects — **no CLA to sign**, just a commit trailer.

**Every commit must be signed off** by adding a `Signed-off-by:` line. The
easiest way is to pass `-s` to `git commit`:

```bash
git commit -s -m "feat(audit): rotate keys on lock"
```

This adds:

```
Signed-off-by: Your Name <you@example.com>
```

Use your **real name** and a **reachable email** (the same identity you use on
GitHub). By signing off, you certify the following:

> Developer Certificate of Origin
> Version 1.1
>
> Copyright (C) 2004, 2006 The Linux Foundation and its contributors.
>
> Everyone is permitted to copy and distribute verbatim copies of this
> license document, but changing it is not allowed.
>
> Developer's Certificate of Origin 1.1
>
> By making a contribution to this project, I certify that:
>
> (a) The contribution was created in whole or in part by me and I have the
>     right to submit it under the open source license indicated in the file; or
>
> (b) The contribution is based upon previous work that, to the best of my
>     knowledge, is covered under an appropriate open source license and I have
>     the right under that license to submit that work with modifications,
>     whether created in whole or in part by me, under the same open source
>     license (unless I am permitted to submit under a different license), as
>     indicated in the file; or
>
> (c) The contribution was provided directly to me by some other person who
>     certified (a), (b) or (c) and I have not modified it.
>
> (d) I understand and agree that this project and the contribution are public
>     and that a record of the contribution (including all personal information
>     I submit with it, including my sign-off) is maintained indefinitely and may
>     be redistributed consistent with this project or the open source license(s)
>     involved.

If a commit is missing the sign-off, add it with:

```bash
git commit --amend -s --no-edit
```

## Licensing

All contributions are made under the **MIT OR Apache-2.0** dual license
(**inbound = outbound**). There is no Contributor License Agreement.

## Reporting security issues

**Do not open a public issue** for security vulnerabilities. See
[SECURITY.md](SECURITY.md) for private disclosure.

## Code of conduct

Be respectful and constructive. Harassment or discrimination of any kind is not
tolerated.

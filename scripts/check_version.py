#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Netreon™ and contributors

"""Assert the MailGrit version is consistent everywhere it is spelled out.

The version lives in several places that cargo cannot keep in sync by itself:
  - Cargo.toml                  [workspace.package] version + the four path-dep
                                 pins in [workspace.dependencies]
  - fuzz/Cargo.toml             (excluded from the workspace — its own copy)
  - docs/index.html             JSON-LD "softwareVersion"
  - docs/uk/index.html          JSON-LD "softwareVersion"
  - e2e/package.json            "version" (the Playwright suite's npm package)
  - e2e/package-lock.json       root "version" (lockfile v3 mirrors
                                 package.json; matched as the first "version"
                                 key, which in v3 layout is the root's)

A forgotten bump used to be discoverable only by a reader; now it fails CI
(the quality job) and the release pipeline (which additionally asserts the
git tag matches).

Usage:
  check_version.py                 # verify internal consistency
  check_version.py --tag v0.1.4    # also assert the tag matches the version

Exits 0 when consistent, 1 with a per-file report, 2 on a usage error
(unknown flag, missing/malformed --tag value). Malformed invocations must
fail loudly: silently skipping the tag assertion would defeat the release
gate this script enforces.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

# (file, pattern with one capture group, human description)
# The workspace Cargo.toml dep pins are matched as a GROUP: all four must
# equal the workspace version, so they are handled separately below.
SINGLE_SITES: list[tuple[Path, str, str]] = [
    (REPO_ROOT / "Cargo.toml", r'(?m)^\[workspace\.package\][\s\S]*?^version = "([^"]+)"',
     "[workspace.package] version"),
    (REPO_ROOT / "fuzz" / "Cargo.toml", r'(?m)^version = "([^"]+)"', "fuzz package version"),
    (REPO_ROOT / "docs" / "index.html", r'"softwareVersion": "([^"]+)"', "docs JSON-LD softwareVersion"),
    (REPO_ROOT / "docs" / "uk" / "index.html", r'"softwareVersion": "([^"]+)"', "docs/uk JSON-LD softwareVersion"),
    # e2e/package.json: the first "version" key is the root package version
    # ("name" and "version" lead the file). e2e/package-lock.json (lockfile
    # v3): the first "version" key is the ROOT package's — the nested
    # packages[""] copy repeats the same value further down, and every other
    # "version" key belongs to node_modules entries that follow it.
    (REPO_ROOT / "e2e" / "package.json", r'"version": "([^"]+)"', "e2e package.json version"),
    (REPO_ROOT / "e2e" / "package-lock.json", r'"version": "([^"]+)"', "e2e package-lock.json version"),
]

# The four path-dep version pins in the root Cargo.toml.
DEP_PIN_RE = re.compile(
    r'(?m)^(mailgrit-core-(?:csv|domain|storage|security)\s*=\s*\{\s*version\s*=\s*)"([^"]+)"'
)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _tag_value(value: str) -> str:
    if not value.startswith("v") or not value[1:].replace(".", "").isdigit():
        raise argparse.ArgumentTypeError(f"malformed tag {value!r} (expected vX.Y.Z)")
    return value


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="check_version.py",
        description="Assert the MailGrit version is consistent everywhere it is spelled out.",
    )
    parser.add_argument(
        "--tag",
        type=_tag_value,
        metavar="VX.Y.Z",
        help="also assert this git tag (e.g. v0.1.4) matches the workspace version",
    )
    # parse_args (not parse_known_args): unknown arguments are usage errors,
    # so a typo'd invocation fails instead of silently skipping the tag check.
    return parser.parse_args(argv)


def main() -> int:
    errors: list[str] = []
    versions: dict[str, str] = {}

    for path, pattern, what in SINGLE_SITES:
        if not path.is_file():
            errors.append(f"{what}: file missing: {path.relative_to(REPO_ROOT)}")
            continue
        match = re.search(pattern, read(path))
        if not match:
            errors.append(f"{what}: pattern not found in {path.relative_to(REPO_ROOT)}")
            continue
        versions[what] = match.group(1)

    workspace_version = versions.get("[workspace.package] version")

    # The path-dep pins must all equal the workspace version.
    if workspace_version:
        root_manifest = REPO_ROOT / "Cargo.toml"
        pins = DEP_PIN_RE.findall(read(root_manifest))
        if len(pins) != 4:
            errors.append(
                f"workspace path-dep pins: expected 4, found {len(pins)} "
                "(did the workspace Cargo.toml layout change?)"
            )
        for dep, pinned in pins:
            if pinned != workspace_version:
                errors.append(
                    f"workspace path-dep {dep}: pinned {pinned}, expected {workspace_version}"
                )

    distinct = set(versions.values())
    if len(distinct) > 1:
        for what, version in sorted(versions.items()):
            errors.append(f"{what}: {version}")
        errors.insert(0, "version mismatch across files:")

    tag = parse_args(sys.argv[1:]).tag
    if tag and workspace_version and tag[1:] != workspace_version:
        errors.append(
            f"git tag {tag} does not match the workspace version {workspace_version}"
        )

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"version consistent everywhere: {workspace_version}")
    if tag:
        print(f"tag {tag} matches")
    return 0


if __name__ == "__main__":
    sys.exit(main())

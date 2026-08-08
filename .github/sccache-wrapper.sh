#!/usr/bin/env bash
# Resilient RUSTC_WRAPPER around sccache with graceful degradation.
#
# Problem this solves:
#   Setting RUSTC_WRAPPER=sccache makes sccache a hard dependency of the build:
#   when GitHub's Actions Cache backend (ghac) is unavailable, sccache fails to
#   start its server ("cache storage failed to read: Unexpected (permanent)")
#   and — because cargo invokes `sccache rustc ...` — every compilation aborts,
#   failing the whole CI gate even though the code compiles fine uncached.
#
# This wrapper restores resilience: it probes sccache ONCE per job (caching the
# verdict in a temp file so cargo's hundreds of invocations stay cheap) and
#   - if sccache is healthy → delegates to `sccache rustc "$@"` (cached build)
#   - if sccache is broken   → falls through to plain `rustc "$@"` (uncached)
# Either way the build proceeds; only caching is skipped on a cache outage.
#
# Usage in a workflow:
#   - run: echo "RUSTC_WRAPPER=$GITHUB_WORKSPACE/.github/sccache-wrapper.sh" >> "$GITHUB_ENV"
#   (set AFTER the sccache-action step, so the sccache binary is on PATH)
#
# NOTE: cargo passes the real compiler path as the FIRST argument
# (`wrapper <rustc> <rustc-args...>`), so we forward "$@" to both sccache and
# the fallback rustc unchanged.

set -u

# The real compiler cargo wants us to run is argv[1]; the rest are its args.
REAL_RUSTC="${1:-rustc}"
shift || true

VERDICT_FILE="${SCCACHE_VERDICT_FILE:-${RUNNER_TEMP:-/tmp}/sccache-healthy}"

# Resolve the verdict once per job (the file is created on the first call).
if [[ ! -f "$VERDICT_FILE" ]]; then
  if command -v sccache >/dev/null 2>&1 \
     && timeout 20 sccache --show-stats >/dev/null 2>&1; then
    printf '1' >"$VERDICT_FILE"
  else
    printf '0' >"$VERDICT_FILE"
    echo "::warning::sccache unavailable; compiling uncached for this job." >&2
  fi
fi

if [[ "$(cat "$VERDICT_FILE" 2>/dev/null)" == "1" ]] && command -v sccache >/dev/null 2>&1; then
  exec sccache "$REAL_RUSTC" "$@"
else
  exec "$REAL_RUSTC" "$@"
fi

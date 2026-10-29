#!/usr/bin/env bash
# First-party CI bootstrap. No third-party GitHub Actions are invoked.
set -euo pipefail
if [[ "${1:-}" != "checkout" && "${1:-}" != "rust" && "${1:-}" != "node" ]]; then
  echo "usage: bash scripts/ci_bootstrap.sh {checkout|rust TOOLCHAIN|node}" >&2
  exit 2
fi
case "$1" in
  checkout)
    : "${GITHUB_WORKSPACE:?Requires GitHub Actions workspace}"
    : "${GITHUB_SHA:?Requires GitHub Actions commit SHA}"
    : "${GITHUB_REPOSITORY:?Requires GitHub Actions repository}"
    [[ "$GITHUB_REPOSITORY" == "pskeffington/pip_boy" ]] || { echo "Unexpected repository" >&2; exit 1; }
    cd "$GITHUB_WORKSPACE"
    git init -q
    git remote add origin "https://github.com/$GITHUB_REPOSITORY.git"
    # Fetch exactly the triggering commit (PR synthetic merge on pull_request).
    git -c protocol.version=2 fetch --no-tags --depth=1 origin "$GITHUB_SHA"
    git checkout --detach --force FETCH_HEAD
    [[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]] || { echo "Checkout SHA mismatch" >&2; exit 1; }
    git status --short --branch
    ;;
  rust)
    toolchain="${2:-}"
    [[ "$toolchain" == "stable" || "$toolchain" == "1.90.0" ]] || { echo "Unapproved Rust toolchain" >&2; exit 2; }
    command -v rustup >/dev/null || { echo "rustup missing on runner" >&2; exit 1; }
    rustup toolchain install "$toolchain" --profile minimal --component rustfmt --component clippy
    RUSTUP_TOOLCHAIN="$toolchain" rustc --version
    RUSTUP_TOOLCHAIN="$toolchain" cargo --version
    ;;
  node)
    command -v node >/dev/null || { echo "Node runtime missing on runner" >&2; exit 1; }
    node -e 'const v = process.versions.node.split(".").map(Number); if (v[0] !== 22) { process.stderr.write("Node 22 required; got "+process.version+"\n"); process.exit(1); }'
    node --version
    ;;
esac

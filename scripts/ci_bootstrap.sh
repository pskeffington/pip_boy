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
    if [[ -n "${RUSTUP_TOOLCHAIN:-}" && "$RUSTUP_TOOLCHAIN" != "$toolchain" ]]; then
      echo "Requested Rust $toolchain conflicts with RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN" >&2
      exit 1
    fi
    rustc --version
    cargo --version
    ;;
  node)
    # Pin the official Node.js binary, rather than trusting whichever major
    # version a hosted runner image happens to put first on PATH.
    version="22.23.4"
    case "$(uname -s)/$(uname -m)" in
      Linux/x86_64)
        archive="node-v${version}-linux-x64.tar.xz"
        digest="1805bcd35e6815559bd2eb97db438d3fdb0eb3a5e6a977b7c1badca76337e9ce"
        compression="xz"
        ;;
      Darwin/arm64)
        archive="node-v${version}-darwin-arm64.tar.gz"
        digest="c9e1dbc748b33ed2d14f42445191ecaf14c56259c7a9a26d0042c49ec2b66428"
        compression="gz"
        ;;
      Darwin/x86_64)
        archive="node-v${version}-darwin-x64.tar.gz"
        digest="efcc2b0db6f94bf53eddb038fe2b370277a042bd29e4f19b8f8e23598795b819"
        compression="gz"
        ;;
      *) echo "No pinned Node.js ${version} binary for $(uname -s)/$(uname -m)" >&2; exit 1 ;;
    esac
    root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/rust-boy-node-v${version}"
    install_dir="$root/node-v${version}"
    if [[ ! -x "$install_dir/bin/node" ]]; then
      mkdir -p "$root"
      archive_path="$root/$archive"
      curl --fail --location --proto '=https' --tlsv1.2 \
        "https://nodejs.org/dist/v${version}/${archive}" --output "$archive_path"
      printf '%s  %s\n' "$digest" "$archive_path" | shasum -a 256 --check --status - || {
        echo "Node.js ${version} archive checksum mismatch" >&2
        exit 1
      }
      mkdir -p "$install_dir"
      if [[ "$compression" == "xz" ]]; then
        tar -xJf "$archive_path" --strip-components=1 -C "$install_dir"
      else
        tar -xzf "$archive_path" --strip-components=1 -C "$install_dir"
      fi
    fi
    export PATH="$install_dir/bin:$PATH"
    if [[ -n "${GITHUB_PATH:-}" ]]; then
      printf '%s\n' "$install_dir/bin" >> "$GITHUB_PATH"
    fi
    [[ "$(node --version)" == "v${version}" ]] || {
      echo "Node.js ${version} required; got $(node --version)" >&2
      exit 1
    }
    node --version
    ;;
esac

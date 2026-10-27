#!/bin/bash
# Run Verus over the crates carrying the snapshot save/restore specification.
#
# The entry-point bodies are currently `external_body`, so this checks that the
# specs, contracts, and proof lemmas parse, type-check, and verify; it does not
# prove the implementations.
#
# Usage: scripts/verify-snapshot-spec.sh [crate...] [-- verus-args...]
#   crate defaults to: openvmm_defs openvmm_core openvmm_entry
#
# Environment:
#   VERUS_BIN_DIR  directory containing cargo-verus and verus
#                  (default: $HOME/programs/verus/source/target-verus/release)
#   RUSTUP_TOOLCHAIN  toolchain to use (default: the channel in Verus's
#                  rust-toolchain.toml)
#
# Prerequisites:
#   - A Verus build matching the pinned vstd (=0.0.0-2026-10-04-0306); tested
#     with Verus 426d8b0 on toolchain 1.98.1, with the patches in
#     verus-patches/ applied (0001 to 0005; see verus-patches/README.md).
#   - Repo packages restored, so the build finds protoc (run from the repo root;
#     needs libarchive-tools for bsdtar on Ubuntu):
#       cargo xflowey restore-packages
#
# Expected result: "==> OK: openvmm_defs openvmm_core openvmm_entry", with
# 0 errors reported for each crate. Cargo may reuse earlier results and print
# no "verification results" line for unchanged crates.

set -euo pipefail

VERUS_BIN_DIR="${VERUS_BIN_DIR:-$HOME/programs/verus/source/target-verus/release}"
if [[ ! -x "$VERUS_BIN_DIR/cargo-verus" ]]; then
    echo "error: cargo-verus not found in $VERUS_BIN_DIR (set VERUS_BIN_DIR)" >&2
    exit 1
fi
export PATH="$VERUS_BIN_DIR:$PATH"

# Verus must run on the toolchain it was built with (its rust-toolchain.toml).
if [[ -z "${RUSTUP_TOOLCHAIN:-}" ]]; then
    verus_toolchain_file="$VERUS_BIN_DIR/../../../rust-toolchain.toml"
    if [[ -f "$verus_toolchain_file" ]]; then
        RUSTUP_TOOLCHAIN="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$verus_toolchain_file")"
        export RUSTUP_TOOLCHAIN
    fi
fi

cd "$(dirname "$0")/.."

crates=()
while [[ $# -gt 0 && "$1" != "--" ]]; do
    crates+=("$1")
    shift
done
[[ "${1:-}" == "--" ]] && shift
if [[ ${#crates[@]} -eq 0 ]]; then
    crates=(openvmm_defs openvmm_core openvmm_entry)
fi

failed=()
for crate in "${crates[@]}"; do
    echo "==> cargo verus verify -p $crate"
    if ! cargo verus verify -p "$crate" -- "$@"; then
        failed+=("$crate")
    fi
done

if [[ ${#failed[@]} -gt 0 ]]; then
    echo "==> FAILED: ${failed[*]}" >&2
    exit 1
fi
echo "==> OK: ${crates[*]}"

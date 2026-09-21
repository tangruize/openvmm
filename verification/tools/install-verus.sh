#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(tr -d '[:space:]' < "$repo_root/verification/verus-version")"
source_root="$repo_root/toolchain/verus-src"
install_root="$source_root/source/target-verus/release"

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
    echo "error: automatic installation supports Linux x86_64 only" >&2
    exit 1
fi

"$repo_root/verification/tools/install-verus-source.sh"

if ! solver="$("$repo_root/verification/tools/find-z3.sh")"; then
    if [[ -n "${VERUS_Z3_PATH:-}" ]]; then
        exit 1
    fi
    (
        cd "$source_root/source"
        ./tools/get-z3.sh
    )
    solver="$("$repo_root/verification/tools/find-z3.sh")"
fi
export VERUS_Z3_PATH="$solver"
echo "Using $("$VERUS_Z3_PATH" --version) at $VERUS_Z3_PATH"

if [[ -x "$install_root/verus" ]]; then
    VERUS="$install_root/verus" "$repo_root/verification/tools/find-verus.sh" >/dev/null
    echo "Verus $version is already installed"
    exit 0
fi

(
    cd "$source_root/source"
    cargo build --release
    cargo run --release -p cargo-verus -- build --release --manifest-path vstd/Cargo.toml
)
VERUS="$install_root/verus" "$repo_root/verification/tools/find-verus.sh" >/dev/null
echo "Built Verus $version at $install_root"

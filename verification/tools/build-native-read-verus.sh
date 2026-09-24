#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail

# Keep the accepted debug candidate and the unpatched pinned release intact.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_root="$repo_root/toolchain/verus-src/source"
target="$source_root/target-verus/native-read-optimized"

if [[ ! -f "$source_root/rust_verify/src/native_ptr_read.rs" ]]; then
    echo "error: the locally patched native-read candidate is required" >&2
    exit 1
fi

cd "$source_root"
export VARGO_BUILD_VERSION
VARGO_BUILD_VERSION="$(tr -d '[:space:]' < "$repo_root/verification/verus-version")"
export CARGO_TARGET_DIR="$target"
export CARGO_BUILD_BUILD_DIR="$target"
export VERUS_Z3_PATH
VERUS_Z3_PATH="$("$repo_root/verification/tools/find-z3.sh")"

printf 'Native-read candidate source (base revision and patched files):\n'
cat "$repo_root/verification/verus-revision"
sha256sum rust_verify/src/{native_ptr_read,fn_call_to_vir,lib,verifier,verus_items}.rs \
    rust_verify_test/tests/raw_ptrs.rs
rustc --version
"$VERUS_Z3_PATH" --version

cargo build --locked --release \
    -p rust_verify -p verus -p cargo-verus \
    -p verus_builtin -p verus_builtin_macros -p verus_state_machines_macros
cargo run --locked --release -p cargo-verus -- \
    build --locked --release --manifest-path vstd/Cargo.toml

VERUS="$target/release/verus" "$repo_root/verification/tools/find-verus.sh"
sha256sum "$target/release/"{verus,rust_verify,cargo-verus} "$VERUS_Z3_PATH"
printf '\nSelect this candidate explicitly from the repository root:\n'
printf 'VERUS="$PWD/toolchain/verus-src/source/target-verus/native-read-optimized/release/verus" make verify\n'

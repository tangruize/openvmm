#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root/toolchain/verus-src/source"
export CARGO_TARGET_DIR="$PWD/target-verus/native-parser-engineer"
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"
export VARGO_BUILD_VERSION
VARGO_BUILD_VERSION="$(tr -d '[:space:]' < "$root/verification/verus-version")"
export VERUS_Z3_PATH
VERUS_Z3_PATH="$("$root/verification/tools/find-z3.sh")"
cargo build --locked --release -p rust_verify -p verus -p cargo-verus \
    -p verus_builtin -p verus_builtin_macros -p verus_state_machines_macros
cargo clean --release --manifest-path vstd/Cargo.toml -p vstd
cargo run --locked --release -p cargo-verus -- \
    build --locked --release --manifest-path vstd/Cargo.toml

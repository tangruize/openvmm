#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$PWD/target"

RUSTC_BOOTSTRAP=1 CARGO_UNSTABLE_LOCKFILE_PATH=true \
    CARGO_RESOLVER_LOCKFILE_PATH="$PWD/research/vmtime-scalar-interface/workspace/Cargo.lock" \
    cargo verus focus --offline --locked -p vmcore -- \
    --verify-only-module vmtime --verify-function 'VmTime::wrapping_add' \
    --multiple-errors 4 --num-threads 1 --triggers-mode silent

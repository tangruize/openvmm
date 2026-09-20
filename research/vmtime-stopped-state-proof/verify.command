#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$PWD/target"
export RUSTC_BOOTSTRAP=1
export CARGO_UNSTABLE_LOCKFILE_PATH=true
export CARGO_RESOLVER_LOCKFILE_PATH="$PWD/research/vmtime-stopped-state-proof/workspace/Cargo.lock"

mkdir -p research/vmtime-stopped-state-proof/workspace
if [[ ! -f "$CARGO_RESOLVER_LOCKFILE_PATH" ]]; then
    cp research/vmtime-scalar-interface/workspace/Cargo.lock "$CARGO_RESOLVER_LOCKFILE_PATH"
fi

time cargo verus focus --offline --locked -p vmcore -- \
    --verify-only-module vmtime --multiple-errors 8 \
    --num-threads 1 --triggers-mode silent

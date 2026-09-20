#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0

exec cargo verus focus --offline --locked -p vmcore -- \
    --verify-only-module vmtime \
    --verify-function 'VmTimeKeeper::reset_to' \
    --rlimit 50 --num-threads 1 --multiple-errors 8

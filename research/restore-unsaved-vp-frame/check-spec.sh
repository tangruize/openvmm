#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
verifier=$(verification/tools/find-verus.sh)
VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3" \
    "$verifier" --crate-type lib --rlimit 50 --triggers-mode silent \
    research/restore-unsaved-vp-frame/frozen_precondition.rs

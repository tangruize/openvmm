#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-prefix-proof"

# Only tests are added, after the exact proposed candidate has been checked.
# This does not alter the proposed patches or the production function.
git apply --directory=research/restore-vp-prefix-proof/candidate \
    "$out/candidate-behavior.patch"
cd "$out/candidate"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$root/target"
start=$SECONDS
cargo nextest run --locked --profile agent -p vmm_core \
    -E 'test(partition_unit::vp_set::restore_vp_index_tests)' \
    > "$out/candidate-behavior.log" 2>&1
printf 'candidate-behavior exit=0 elapsed_seconds=%s\n' "$((SECONDS - start))"

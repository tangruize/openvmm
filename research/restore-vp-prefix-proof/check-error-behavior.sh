#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-prefix-proof"
cd "$out/candidate"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$root/target"
start=$SECONDS
cargo nextest run --locked --profile agent -p vmm_core \
    -E 'test(named_closures_validate_dormant_entries_and_preserve_error_order)' \
    > "$out/candidate-error-behavior.log" 2>&1
printf 'candidate-error-behavior exit=0 elapsed_seconds=%s\n' "$((SECONDS - start))"

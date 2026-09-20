#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -uo pipefail
cd "$(dirname "$0")/../.." || exit
root="$PWD"
out="$root/research/restore-vp-prefix-proof"
verus="$(verification/tools/find-verus.sh)" || exit
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$root/toolchain/verus-src/source/z3"
export CARGO_TARGET_DIR="$root/target"
export RUSTUP_TOOLCHAIN=1.95.0
cd "$out/candidate" || exit

"$verus" --version
"$VERUS_Z3_PATH" --version

# This is the actual generic production body, not a standalone syntax probe.
start=$SECONDS
cargo verus focus -p vmm_core -- \
    --verify-only-module partition_unit::vp_set \
    --verify-function select_instantiated_vp_states \
    --no-lifetime --multiple-errors 20 --num-threads 1 \
    --triggers-mode silent > "$out/candidate-verus.log" 2>&1
verus_status=$?
printf 'candidate-verus exit=%s elapsed_seconds=%s\n' \
    "$verus_status" "$((SECONDS - start))"

start=$SECONDS
cargo check --locked -p vmm_core --tests > "$out/candidate-rust.log" 2>&1
rust_status=$?
printf 'candidate-rust exit=%s elapsed_seconds=%s\n' \
    "$rust_status" "$((SECONDS - start))"

# The Verus failure is retained as diagnostic evidence, never a proof result.
if (( rust_status != 0 )); then
    exit "$rust_status"
fi
exit "$verus_status"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-index-coverage"
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$root/toolchain/verus-src/source/z3"
export CARGO_TARGET_DIR="$root/target"
export RUSTUP_TOOLCHAIN=1.95.0
cd "$out/bitmap-candidate"
timeout 110s cargo verus focus -p vmm_core -- \
    --verify-only-module partition_unit::vp_set \
    --verify-function validate_restore_vp_indices \
    --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent \
    > "$out/bitmap-validator.log" 2>&1

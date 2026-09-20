#!/usr/bin/env bash
set -euo pipefail
root="$PWD"
out="$root/research/vmtime-duration-interface-candidate"
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$root/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$out/workspace/target"
cd "$out/workspace"
start=$SECONDS
set +e
cargo verus focus --offline --locked -p vmcore -- \
    --verify-only-module vmtime --verify-function 'VmTime::wrapping_add' \
    --multiple-errors 4 --num-threads 1 --triggers-mode silent \
    > "$out/production-body.log" 2>&1
code=$?
set -e
printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))" >> "$out/production-body.log"
tail -80 "$out/production-body.log"
exit "$code"

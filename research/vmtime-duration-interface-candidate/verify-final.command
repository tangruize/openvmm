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
timeout 110s cargo verus focus --offline --locked -p vmcore -- \
    --multiple-errors 4 --num-threads 1 --triggers-mode silent \
    > "$out/production-final.log" 2>&1
code=$?
set -e
printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))" >> "$out/production-final.log"
tail -70 "$out/production-final.log"
exit "$code"

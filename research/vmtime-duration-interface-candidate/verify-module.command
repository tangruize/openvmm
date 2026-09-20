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
    --verify-only-module vmtime --multiple-errors 4 --num-threads 1 \
    --triggers-mode silent > "$out/production-module.log" 2>&1
code=$?
set -e
printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))" >> "$out/production-module.log"
tail -90 "$out/production-module.log"
exit "$code"

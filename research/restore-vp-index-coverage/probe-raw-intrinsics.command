set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
timeout 30s "$verus" --crate-type lib \
    research/restore-vp-index-coverage/replace-intrinsics-raw.rs \
    --multiple-errors 5 --num-threads 1 --triggers-mode silent

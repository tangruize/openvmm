set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
timeout 110s cargo verus focus -p vm_topology -- \
    --verify-only-module processor --verify-function 'VpIndex::index' \
    --multiple-errors 8 --num-threads 1 --triggers-mode silent

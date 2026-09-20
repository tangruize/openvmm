set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
cargo verus focus -p vmm_core -- \
    --verify-only-module partition_unit::vp_set \
    --verify-function validate_restore_vp_indices \
    --no-lifetime --multiple-errors 8 --num-threads 1 \
    --triggers-mode silent

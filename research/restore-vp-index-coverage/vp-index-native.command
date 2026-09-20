set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export PATH="$(dirname "$verus"):$PATH"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
export RUSTUP_TOOLCHAIN=1.95.0
git --no-pager diff -- vm/vmcore/vm_topology/Cargo.toml \
    vm/vmcore/vm_topology/src/processor.rs \
    > research/restore-vp-index-coverage/vp-index-native.patch
timeout 110s cargo verus focus -p vm_topology -- \
    --verify-only-module processor --multiple-errors 8 --num-threads 1 \
    --triggers-mode silent

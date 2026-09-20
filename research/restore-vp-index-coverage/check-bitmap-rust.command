#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-index-coverage"
export CARGO_TARGET_DIR="$root/target"
export RUSTUP_TOOLCHAIN=1.95.0
cd "$out/bitmap-candidate"
cmp vmm_core/src/partition_unit/vp_set.rs "$out/bitmap-candidate-vp_set.rs"
cargo check --locked -p vm_topology -p vmm_core --tests \
    > "$out/bitmap-rust.log" 2>&1
printf 'candidate cargo check exit=0\n'
patch --batch --forward -p1 < "$out/bitmap-behavior.patch"
trap 'patch --batch --reverse -p1 < "$out/bitmap-behavior.patch"' EXIT
cargo nextest run --profile agent --locked -p vm_topology -p vmm_core \
    -E 'package(vm_topology) | test(restore_vp_index_tests)' \
    > "$out/bitmap-tests.log" 2>&1
printf 'candidate nextest exit=0\n'

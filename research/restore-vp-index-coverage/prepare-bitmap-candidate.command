#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
out="$PWD/research/restore-vp-index-coverage"
candidate="$out/bitmap-candidate"
test ! -e "$candidate"
mkdir "$candidate"
git archive HEAD | tar -x -C "$candidate"
git diff --binary -- Cargo.lock vm/vmcore/vm_topology/Cargo.toml \
    vm/vmcore/vm_topology/src/processor.rs > "$out/bitmap-intake-accessor.patch"
git diff --binary > "$out/bitmap-intake-tracked.patch"
git status --short > "$out/bitmap-intake-status.log"
git rev-parse HEAD argus/restore-v1-frozen > "$out/bitmap-intake-tips.log"
cp vm/vmcore/vm_topology/src/processor.proof.rs "$out/bitmap-intake-processor.proof.rs"
cp Cargo.lock "$candidate/Cargo.lock"
cp vm/vmcore/vm_topology/Cargo.toml "$candidate/vm/vmcore/vm_topology/Cargo.toml"
cp vm/vmcore/vm_topology/src/processor.rs "$candidate/vm/vmcore/vm_topology/src/processor.rs"
cp vm/vmcore/vm_topology/src/processor.proof.rs "$candidate/vm/vmcore/vm_topology/src/processor.proof.rs"
cp vmm_core/src/partition_unit/vp_set.rs "$out/bitmap-intake-vp_set.rs"
cp vmm_core/src/lib.rs "$out/bitmap-intake-lib.rs"
git show argus/restore-v1-frozen:vmm_core/src/partition_unit/vp_set.rs \
    > "$out/bitmap-frozen-vp_set.rs"
ln -s "$PWD/.packages" "$candidate/.packages"
ln -s "$PWD/toolchain" "$candidate/toolchain"
printf 'Prepared isolated current-tip candidate with retained accessor source.\n'

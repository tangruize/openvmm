#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-index-coverage"
candidate="$out/bitmap-candidate"
request="$root/research/freeze_requests/restore-vp-bitmap-native-update"
test ! -e "$candidate"
mkdir "$candidate"
git archive HEAD | tar -x -C "$candidate"
ln -s "$root/.packages" "$candidate/.packages"
ln -s "$root/toolchain" "$candidate/toolchain"
patch --batch --forward -d "$candidate" -p1 < "$request/run.patch"
cmp "$candidate/vmm_core/src/partition_unit/vp_set.rs" \
    "$out/bitmap-candidate-vp_set.rs"
bash "$out/check-bitmap-helper.command"
patch --batch --forward -d "$candidate" -p1 \
    < "$out/bitmap-validator-admission.patch"
trap 'patch --batch --reverse -d "$candidate" -p1 < "$out/bitmap-validator-admission.patch"' EXIT
set +e
bash "$out/check-bitmap-validator.command"
validator_status=$?
set -e
printf 'candidate validator exit=%s (expected 101: six unresolved declarations)\n' \
    "$validator_status"
patch --batch --reverse -d "$candidate" -p1 \
    < "$out/bitmap-validator-admission.patch"
trap - EXIT
test "$validator_status" -eq 101
bash "$out/check-bitmap-rust.command"

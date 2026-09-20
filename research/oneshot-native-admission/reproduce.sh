#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/oneshot-native-admission
sources=(Cargo.lock support/mesh/mesh_channel_core/Cargo.toml
    support/mesh/mesh_channel_core/src/error.rs
    support/mesh/mesh_channel_core/src/oneshot.rs
    support/mesh/mesh_channel_core/src/sync_unsafe_cell.rs)
git diff --exit-code -- "${sources[@]}"
git diff --cached --exit-code -- "${sources[@]}"
git apply --check "$evidence/representation-source.patch"
git apply "$evidence/representation-source.patch"
restore_source() {
    git apply --reverse "$evidence/representation-source.patch"
}
trap restore_source EXIT
verus_root="$(dirname "$(verification/tools/find-verus.sh)")"
export PATH="$verus_root:$PATH"
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s cargo verus focus -p mesh_channel_core --features mesh_protobuf/std -vv -- --verify-only-module oneshot --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent --log vir' \
    > "$evidence/reproduction-verifier.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s \
    cargo verus focus -p mesh_channel_core --features mesh_protobuf/std -vv -- \
    --verify-only-module oneshot --no-lifetime --multiple-errors 20 \
    --num-threads 1 --triggers-mode silent --log vir \
    > "$evidence/reproduction-verifier.log" 2>&1
status=$?
set -e
printf 'owning_crate_verification_exit=%s\n' "$status"
restore_source
trap - EXIT
git diff --exit-code -- "${sources[@]}"
printf '%s\n' 'production_source_and_manifests_restored=true'
exit "$status"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/oneshot-native-admission
phase="${1:?phase required}"
case "$phase" in
    receiver|representation) ;;
    *) printf 'unknown phase: %s\n' "$phase" >&2; exit 2 ;;
esac
exec > "$evidence/$phase-driver.log" 2>&1
set -x
verus_root="$(dirname "$(verification/tools/find-verus.sh)")"
export PATH="$verus_root:$PATH"
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s cargo verus focus -p mesh_channel_core --features mesh_protobuf/std -vv -- --verify-only-module oneshot --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent --log vir' \
    > "$evidence/$phase-verifier.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s \
    cargo verus focus -p mesh_channel_core --features mesh_protobuf/std -vv -- \
    --verify-only-module oneshot --no-lifetime --multiple-errors 20 \
    --num-threads 1 --triggers-mode silent --log vir \
    > "$evidence/$phase-verifier.log" 2>&1
status=$?
set -e
git --no-pager diff -- Cargo.lock support/mesh/mesh_channel_core/Cargo.toml \
    support/mesh/mesh_channel_core/src/error.rs \
    support/mesh/mesh_channel_core/src/oneshot.rs \
    support/mesh/mesh_channel_core/src/sync_unsafe_cell.rs \
    > "$evidence/$phase-source.patch"
printf 'owning_crate_verification_exit=%s\n' "$status"
exit "$status"

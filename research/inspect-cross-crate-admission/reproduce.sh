#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/inspect-cross-crate-admission
sources=(Cargo.lock support/inspect/Cargo.toml support/inspect/src/lib.rs
    openvmm/openvmm_core/src/partition.rs)
git diff --exit-code -- "${sources[@]}"
git diff --cached --exit-code -- "${sources[@]}"
git apply --check "$evidence/integration-source.patch"
git apply "$evidence/integration-source.patch"
restore_source() {
    git apply --reverse "$evidence/integration-source.patch"
}
trap restore_source EXIT
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s make verify MODULE=restore' \
    > "$evidence/reproduction-verifier.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s \
    make verify MODULE=restore > "$evidence/reproduction-verifier.log" 2>&1
result=$?
set -e
printf 'integration_verification_exit=%s\n' "$result"
restore_source
trap - EXIT
git diff --exit-code -- "${sources[@]}"
printf '%s\n' 'production_source_and_manifests_restored=true'
exit "$result"

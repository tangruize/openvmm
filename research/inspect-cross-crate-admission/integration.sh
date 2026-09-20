#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/inspect-cross-crate-admission
exec > "$evidence/integration-driver.log" 2>&1
set -x
git --no-pager diff -- Cargo.lock support/inspect/Cargo.toml support/inspect/src/lib.rs \
    openvmm/openvmm_core/src/partition.rs > "$evidence/integration-source.patch"
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s make verify MODULE=restore' \
    > "$evidence/integration-verifier.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s \
    make verify MODULE=restore > "$evidence/integration-verifier.log" 2>&1
result=$?
set -e
printf 'integration_verification_exit=%s\n' "$result"
exit "$result"

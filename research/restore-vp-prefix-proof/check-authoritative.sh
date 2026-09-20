#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -uo pipefail
cd "$(dirname "$0")/../.." || exit
out="$PWD/research/restore-vp-prefix-proof"
python="${ARGUS_SKILL_PYTHON:-python3}"

git diff --exit-code -- vmm_core/src/partition_unit/vp_set.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-guard-native-presence || exit
git rev-parse HEAD refs/heads/argus/restore-v1-frozen

"$python" -m argus_verus.tools.checks.make_verify --crate-root . \
    > "$out/current-make_verify.log" 2>&1
make_status=$?
printf 'make_verify exit=%s\n' "$make_status"
"$python" -m argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check \
    > "$out/current-boundary.log" 2>&1
boundary_status=$?
printf 'boundary exit=%s\n' "$boundary_status"
"$python" -m argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/current-spec_drift.log" 2>&1
spec_status=$?
printf 'spec_drift exit=%s\n' "$spec_status"
"$python" -m argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/current-exec_drift.log" 2>&1
exec_status=$?
printf 'exec_drift exit=%s\n' "$exec_status"
if (( make_status != 0 || spec_status != 0 || exec_status != 0 )); then
    exit 1
fi
exit "$boundary_status"

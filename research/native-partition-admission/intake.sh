#!/usr/bin/env bash
set -euo pipefail

test -d .verus_agent
test -e .git
printf '%s\n' '=== worktree ==='
git --no-pager status --short
printf '%s\n' '=== existing production edits ==='
git --no-pager diff -- openvmm/openvmm_core/src/partition.rs \
    openvmm/openvmm_core/src/worker support/inspect/src/lib.rs \
    vmm_core/src/partition_unit/vp_set.rs .verus_agent/scope_manifest.json \
    .verus_agent/tcb_manifest.json
printf '%s\n' '=== pinned verifier ==='
verification/tools/find-verus.sh
printf '%s\n' '=== check artifact timestamps ==='
stat -c '%y %n' .verus_agent/cache/checks/{make_verify,boundary,spec_drift,exec_drift}/latest.log
printf '%s\n' '=== maintained call structure ==='
set +e
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph --project . \
    build --out .verus_agent/cache/callgraph.json
result=$?
set -e
printf 'callgraph_exit=%s\n' "$result"
if test -f .verus_agent/proof_state.json; then
    printf '%s\n' 'proof_state=present'
else
    printf '%s\n' 'proof_state=absent'
fi

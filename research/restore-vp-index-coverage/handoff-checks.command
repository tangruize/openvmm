set -uo pipefail
out=research/restore-vp-index-coverage
python="${ARGUS_SKILL_PYTHON:-python3}"
git --no-pager diff --exit-code -- vmm_core/src/lib.rs \
    vmm_core/src/partition_unit/vp_set.rs \
    vm/vmcore/vm_topology/src/processor.rs vm/vmcore/src/save_restore.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    openvmm/openvmm_core/src/worker \
    research/freeze_requests/restore-guard-native-presence \
    research/freeze_requests/restore-vp-selector-named-closures || exit
"$python" -m argus_verus.tools.checks.make_verify --crate-root . \
    > "$out/make_verify.log" 2>&1
make_status=$?
printf 'make_verify exit=%s\n' "$make_status"
"$python" -m argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check \
    > "$out/boundary.log" 2>&1
boundary_status=$?
printf 'boundary exit=%s\n' "$boundary_status"
"$python" -m argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/spec_drift.log" 2>&1
spec_status=$?
printf 'spec_drift exit=%s\n' "$spec_status"
"$python" -m argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/exec_drift.log" 2>&1
exec_status=$?
printf 'exec_drift exit=%s\n' "$exec_status"
if (( make_status != 0 || spec_status != 0 || exec_status != 0 )); then
    exit 1
fi
exit "$boundary_status"

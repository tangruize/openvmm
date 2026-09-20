set -uo pipefail
out=research/restore-vp-index-coverage
python="${ARGUS_SKILL_PYTHON:-python3}"
export RUSTUP_TOOLCHAIN=1.95.0
git --no-pager diff --exit-code -- vmm_core/src/lib.rs \
    vmm_core/src/partition_unit/vp_set.rs vm/vmcore/src/save_restore.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    openvmm/openvmm_core/src/worker \
    research/freeze_requests/restore-guard-native-presence \
    research/freeze_requests/restore-vp-selector-named-closures || exit

cargo check --locked -p vm_topology -p vmm_core --tests \
    > "$out/continuation-rust.log" 2>&1
rust_status=$?
printf 'cargo check exit=%s\n' "$rust_status"
if (( rust_status != 0 )); then exit "$rust_status"; fi

cargo nextest run --profile agent --locked -p vm_topology -p vmm_core \
    -E 'package(vm_topology) | test(restore_vp_index_tests)' \
    > "$out/continuation-tests.log" 2>&1
test_status=$?
printf 'cargo nextest exit=%s\n' "$test_status"
if (( test_status != 0 )); then exit "$test_status"; fi

"$python" -m argus_verus.tools.checks.make_verify --crate-root . \
    > "$out/continuation-make_verify.log" 2>&1
make_status=$?
printf 'make_verify exit=%s\n' "$make_status"
"$python" -m argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check \
    > "$out/continuation-boundary.log" 2>&1
boundary_status=$?
printf 'boundary exit=%s\n' "$boundary_status"
"$python" -m argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/continuation-spec_drift.log" 2>&1
spec_status=$?
printf 'spec_drift exit=%s\n' "$spec_status"
"$python" -m argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent \
    > "$out/continuation-exec_drift.log" 2>&1
exec_status=$?
printf 'exec_drift exit=%s\n' "$exec_status"
for check in make_verify boundary spec_drift exec_drift; do
    cp ".verus_agent/cache/checks/$check/latest.log" "$out/continuation-$check.complete.log"
done
if (( make_status != 0 || spec_status != 0 || exec_status != 0 )); then
    exit 1
fi
exit "$boundary_status"

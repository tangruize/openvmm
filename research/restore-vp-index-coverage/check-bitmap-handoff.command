#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")/../.."
out=research/restore-vp-index-coverage
python="${ARGUS_SKILL_PYTHON:-python3}"
export RUSTUP_TOOLCHAIN=1.95.0

git diff --exit-code -- vmm_core/src/lib.rs \
    vmm_core/src/partition_unit/vp_set.rs vm/vmcore/src/save_restore.rs \
    openvmm/openvmm_core/src/worker \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-ready-file-admission \
    research/freeze_requests/restore-time-partition-presence \
    research/freeze_requests/restore-guard-native-presence \
    research/freeze_requests/restore-vp-selector-named-closures || exit
git diff --binary -- Cargo.lock vm/vmcore/vm_topology/Cargo.toml \
    vm/vmcore/vm_topology/src/processor.rs |
    cmp - "$out/bitmap-intake-accessor.patch" || exit
cmp vm/vmcore/vm_topology/src/processor.proof.rs \
    "$out/bitmap-intake-processor.proof.rs" || exit
git rev-parse HEAD argus/restore-v1-frozen |
    cmp - "$out/bitmap-intake-tips.log" || exit

status=0
for check in make_verify boundary spec_drift exec_drift; do
    args=(--crate-root .)
    if [[ "$check" != make_verify ]]; then
        args+=(--baseline-dir .verus_agent)
    fi
    if [[ "$check" == boundary ]]; then
        args+=(check)
    fi
    "$python" -m "argus_verus.tools.checks.$check" "${args[@]}" \
        > "$out/bitmap-$check.log" 2>&1
    result=$?
    printf '%s exit=%s\n' "$check" "$result"
    cp ".verus_agent/cache/checks/$check/latest.log" \
        "$out/bitmap-$check.complete.log"
    if (( result != 0 )); then
        status=1
    fi
done
exit "$status"

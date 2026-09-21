#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
module="${1:-all}"
evidence_options=()
case "$module" in
    all)
        command=verify
        package=openvmm_core
        selection=()
        ;;
    restore)
        command=focus
        package=openvmm_core
        evidence_options=(--allow-zero)
        selection=(--verify-only-module worker::dispatch
            --verify-function 'LoadedVm::restore_snapshot_state')
        ;;
    vmtime)
        command=focus
        package=vmcore
        selection=(--verify-only-module vmtime)
        ;;
    *)
        echo "error: unknown verification module '$module' (available: all, restore, vmtime)" >&2
        exit 2
        ;;
esac

verus_root="$(dirname "$("$repo_root/verification/tools/find-verus.sh")")"
solver="$("$repo_root/verification/tools/find-z3.sh")"
mkdir -p "$repo_root/target/verus"
echo "Verification scope: command=$command package=$package module=$module (lifetime and trait-conflict checking enabled)"
(
    cd "$repo_root"
    export VERUS_Z3_PATH="$solver"
    echo "Verification solver: $VERUS_Z3_PATH ($("$VERUS_Z3_PATH" --version); version checking enabled)"
    PATH="$verus_root:$PATH" "${ARGUS_SKILL_PYTHON:-python3}" \
        "$repo_root/verification/tools/fresh_verification.py" \
        "${evidence_options[@]}" "$command" "$package" -- \
        "${selection[@]}" \
        --multiple-errors 20 \
        --num-threads 1 \
        --triggers-mode silent
) 2>&1 | tee "$repo_root/target/verus/$module.log"

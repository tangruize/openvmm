#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
out="$PWD/research/restore-proof-erasure-repair"
python="${ARGUS_SKILL_PYTHON:-python3}"
export CARGO_BUILD_JOBS=4

run_check() {
    local name="$1"
    shift
    local started=$SECONDS
    printf 'START %s: ' "$name"
    printf '%q ' "$@"
    printf '\n'
    local result=0
    "$@" > "$out/$name.log" 2>&1 || result=$?
    printf 'END %s: exit=%s elapsed=%ss log=%s/%s.log\n' \
        "$name" "$result" "$((SECONDS - started))" "$out" "$name"
    return "$result"
}

run_check clippy env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo clippy --locked --all-targets \
    -p openvmm_core -p openvmm_defs -p state_unit
run_check doc env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo doc --locked --no-deps \
    -p openvmm_core -p openvmm_defs -p state_unit
run_check tests env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo nextest run --locked --profile agent \
    -p openvmm_core -p openvmm_defs -p state_unit
run_check make_verify "$python" -m argus_verus.tools.checks.make_verify --crate-root .

boundary_result=0
run_check boundary "$python" -m argus_verus.tools.checks.boundary \
    --crate-root . --baseline-dir .verus_agent check || boundary_result=$?
case "$boundary_result" in
    0) ;;
    3) printf 'Boundary completion remains blocked by the reported existing proof debt.\n' ;;
    *) exit "$boundary_result" ;;
esac
run_check boundary_admission "$python" -m argus_verus.tools.checks.boundary \
    --crate-root . --baseline-dir .verus_agent admission
run_check spec_drift "$python" -m argus_verus.tools.checks.spec_drift \
    --crate-root . --baseline-dir .verus_agent
run_check exec_drift "$python" -m argus_verus.tools.checks.exec_drift \
    --crate-root . --baseline-dir .verus_agent

#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
overlay=research/restore-tsc-consistency/test-overlay.patch
git apply --check "$overlay"
git apply "$overlay"
cleanup() {
    result=$?
    trap - EXIT
    if ! git apply --reverse "$overlay"; then
        echo "error: could not remove the diagnostic test overlay" >&2
        exit 2
    fi
    exit "$result"
}
trap cleanup EXIT
RUSTUP_TOOLCHAIN=1.95.0 cargo nextest run --profile agent -p vmm_core \
    -E 'test(restore_tsc_consistency_probe)' --success-output immediate

#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
overlay=research/restore-unsaved-vp-frame/test-overlay.patch
git apply --check "$overlay"
git apply "$overlay"
cleanup() {
    result=$?
    trap - EXIT
    if ! git apply --reverse "$overlay"; then
        echo "error: could not remove the unsaved-VP diagnostic overlay" >&2
        exit 2
    fi
    exit "$result"
}
trap cleanup EXIT
RUSTUP_TOOLCHAIN=1.95.0 cargo nextest run --profile agent -p vmm_core \
    -E 'test(restore_unsaved_vp_frame_probe::unsaved_frame::)' --success-output immediate

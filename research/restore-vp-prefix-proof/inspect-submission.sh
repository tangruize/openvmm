#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
git status --short --untracked-files=all
git diff --stat
git diff -- research/GROUND_TRUTH.md
git ls-files research/restore-vp-prefix-proof research/freeze_requests/restore-vp-selector-named-closures
git diff --exit-code -- vmm_core/src/partition_unit/vp_set.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-guard-native-presence

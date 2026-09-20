#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
out="$PWD/research/restore-vp-prefix-proof"

# The archive is a separate production checkout, with no authoritative source
# mutations and no replacement selector. Reuse only installed tools/artifacts.
test ! -e "$out/candidate"
mkdir "$out/candidate"
git archive HEAD | tar -x -C "$out/candidate"
mkdir -p "$out/candidate/toolchain"
ln -s "$PWD/toolchain/verus-src" "$out/candidate/toolchain/verus-src"
printf 'candidate=%s\n' "$out/candidate"
git diff --exit-code -- vmm_core/src/partition_unit/vp_set.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-guard-native-presence
git rev-parse HEAD refs/heads/argus/restore-v1-frozen
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph \
    --project . build --out "$out/current-callgraph.json"

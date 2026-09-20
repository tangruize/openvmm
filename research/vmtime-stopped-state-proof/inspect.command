#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -uo pipefail
python="${ARGUS_SKILL_PYTHON:-python3}"
"$python" -m argus_verus.callgraph.graph --project . build \
    --out research/vmtime-stopped-state-proof/callgraph.json
graph_status=$?
printf 'callgraph exit=%s\n' "$graph_status"
"$python" -m argus_verus.tools.source.callers --help

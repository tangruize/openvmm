#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-selector-authoritative-checks \
    --mode direct --timeout 900 \
    --command 'bash research/restore-vp-prefix-proof/check-authoritative.sh' \
    --description 'Record current restore admission and frozen-boundary integrity without applying request'

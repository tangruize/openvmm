#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-selector-prepared-candidate \
    --mode direct --timeout 900 \
    --command 'bash research/restore-vp-prefix-proof/check-prepared-candidate.sh' \
    --description 'Check normalized production closures with existing protoc restored'

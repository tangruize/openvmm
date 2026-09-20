#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-selector-named-closures \
    --mode direct --timeout 900 \
    --command 'bash research/restore-vp-prefix-proof/check-candidate.sh' \
    --description 'Check both normalized production closures with Verus and ordinary Rust'

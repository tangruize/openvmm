#!/usr/bin/env bash
set -euo pipefail
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id restore-duration-interface-production \
    --mode direct --timeout 600 \
    --command 'bash research/vmtime-duration-interface-candidate/verify.command'

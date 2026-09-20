#!/usr/bin/env bash
set -euo pipefail
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent status \
    --task-id restore-duration-interface-production

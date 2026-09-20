#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent status \
    --task-id restore-time-partition-presence-candidate

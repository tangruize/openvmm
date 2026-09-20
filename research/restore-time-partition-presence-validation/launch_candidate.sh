#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id restore-time-partition-presence-candidate \
    --mode direct --timeout 1200 \
    --command 'bash research/restore-time-partition-presence-validation/run_candidate.sh'

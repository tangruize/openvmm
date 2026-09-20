set -eu
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-index-coverage-handoff-checks \
    --mode direct --timeout 300 \
    --command 'bash research/restore-vp-index-coverage/handoff-checks.command > research/restore-vp-index-coverage/handoff-checks.log 2>&1' \
    --description 'Check restored production integration and frozen boundaries once'

set -eu
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-index-coverage-intake \
    --mode direct --timeout 300 \
    --command 'bash research/restore-vp-index-coverage/intake-focused.command > research/restore-vp-index-coverage/intake-focused.log 2>&1' \
    --description 'Check current production VP-index validator without applying selector normalization'

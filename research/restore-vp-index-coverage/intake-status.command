set -eu
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent status \
    --task-id vp-index-coverage-intake

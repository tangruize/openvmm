set -eu
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit \
    --task-id vp-index-native-proof-checks \
    --mode direct --timeout 600 \
    --command 'bash research/restore-vp-index-coverage/continuation-checks.command > research/restore-vp-index-coverage/continuation-checks.log 2>&1' \
    --description 'Check retained native VP-index accessor proof, Rust regressions and frozen integration'

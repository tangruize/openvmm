set -eu
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph --project . build \
    --out .verus_agent/cache/callgraph.json

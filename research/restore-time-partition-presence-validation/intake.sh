#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."
if [[ ! -e .git ]]; then
    printf '%s\n' 'No .git; stopping Git-dependent request preparation.'
    exit 1
fi
git --no-pager status --short
git branch --show-current
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request --help
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph --project . build --out research/restore-time-partition-presence-validation/callgraph.json

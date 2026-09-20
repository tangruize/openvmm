#!/usr/bin/env bash
set -euo pipefail
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-duration-interface-candidate/setup.py
printf '\nVerifier selection\n'
verification/tools/find-verus.sh
printf '\nNative runner usage\n'
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit --help

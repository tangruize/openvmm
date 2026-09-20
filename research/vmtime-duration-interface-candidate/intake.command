#!/usr/bin/env bash
set -euo pipefail
test -d .git
git --no-pager status --short
printf '\nWorking branch\n'
git branch --show-current
printf '\nAuthoritative Python and freeze-request module\n'
"${ARGUS_SKILL_PYTHON:-python3}" -c 'import sys, argus_verus.tools.operator.freeze_request as f; print(sys.executable); print(f.__file__)'
printf '\nFreeze request help\n'
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request --project-root . --help

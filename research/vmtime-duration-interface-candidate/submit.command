#!/usr/bin/env bash
set -euo pipefail
start=$SECONDS
set +e
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request \
    --project-root . submit \
    --id restore-duration-nanoseconds-interface \
    --freeze-patch research/vmtime-duration-interface-candidate/freeze.patch \
    --run-patch research/vmtime-duration-interface-candidate/run.patch \
    --rationale research/vmtime-duration-interface-candidate/rationale.md
code=$?
set -e
printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))"
exit "$code"

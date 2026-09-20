#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request \
    --project-root . submit \
    --id restore-time-partition-presence \
    --freeze-patch research/restore-time-partition-presence-validation/freeze.patch \
    --run-patch research/restore-time-partition-presence-validation/run.patch \
    --rationale research/restore-time-partition-presence-validation/rationale.md

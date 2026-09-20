#!/usr/bin/env bash
# Recorded submission and readiness invocations; the request is already submitted.
set -euo pipefail
cd "$(dirname "$0")/../.."
python="${ARGUS_SKILL_PYTHON:-python3}"
"$python" -m argus_verus.tools.operator.freeze_request --project-root . submit \
    --id restore-vp-bitmap-native-update \
    --freeze-patch research/restore-vp-index-coverage/bitmap-freeze.patch \
    --run-patch research/restore-vp-index-coverage/bitmap-run.patch \
    --rationale research/restore-vp-index-coverage/bitmap-rationale.md \
    > research/restore-vp-index-coverage/bitmap-submit.log 2>&1
"$python" - <<'PY' > research/restore-vp-index-coverage/bitmap-readiness.log
from pathlib import Path
from argus_verus.tools.operator.freeze_request import load, operator_ready_issues

root = Path.cwd()
request = load(root, "restore-vp-bitmap-native-update")
issues = operator_ready_issues(root, root / ".verus_agent", request)
print("operator_ready:", "BLOCKED" if issues else "READY")
for issue in issues:
    print(issue)
PY

#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
out=.verus_agent/cache/bitmap-delivery
mkdir -p "$out"
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY' > "$out/readiness.log" 2>&1
from pathlib import Path
import subprocess
from argus_verus.tools.operator.freeze_request import (
    _recorded_application,
    load,
    operator_ready_issues,
)

root = Path.cwd()
baseline = root / ".verus_agent"
request = load(root, "restore-vp-bitmap-native-update")
assert _recorded_application(root, baseline, request) is None, "request already applied"
assert sorted(p.name for p in request.rationale.parent.iterdir()) == [
    "freeze.patch", "rationale.md", "run.patch"
]
print("Native operator_ready_issues includes validate_request for this unapplied request.")
issues = operator_ready_issues(root, baseline, request)
for issue in issues:
    print(issue)
if issues:
    raise SystemExit("operator_ready: BLOCKED")
print("freeze_request: VALID (native readiness validation of current patch results)")
print("operator_ready: READY")
print("normalization: UNAPPLIED")
print("current working and frozen tips:")
print(subprocess.check_output(
    ["git", "rev-parse", "HEAD", "argus/restore-v1-frozen"], text=True
).strip())
assert not subprocess.check_output(["git", "status", "--porcelain"], text=True)
print("working tree: CLEAN after validation")
PY
cat "$out/readiness.log"

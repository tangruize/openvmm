#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
python="${ARGUS_SKILL_PYTHON:-python3}"
git diff --exit-code HEAD^ HEAD -- . ':!research'
test -z "$(git status --porcelain)"
"$python" - <<'PY'
from pathlib import Path
import subprocess

from argus_verus.tools.operator import freeze_request

root = Path.cwd()
out = root / "research/restore-vp-prefix-proof"
request = freeze_request.load(root, "restore-vp-selector-named-closures")
for name in ("freeze.patch", "run.patch", "rationale.md"):
    proposed = request.directory / name
    assert proposed.read_bytes() == (out / "request-inputs" / name).read_bytes()
    committed = subprocess.check_output(
        ["git", "show", f"HEAD:{proposed.relative_to(root).as_posix()}"]
    )
    assert committed == proposed.read_bytes()
initial_tips = (out / "prepare-candidate.log").read_text().splitlines()[1:3]
current_frozen = subprocess.check_output(
    ["git", "rev-parse", "refs/heads/argus/restore-v1-frozen"], text=True
).strip()
assert current_frozen == initial_tips[1], "frozen branch changed"
path = "vmm_core/src/partition_unit/vp_set.rs"
initial_source = subprocess.check_output(["git", "show", f"{initial_tips[0]}:{path}"])
assert (root / path).read_bytes() == initial_source, "selector changed"
issues = freeze_request.operator_ready_issues(root, root / ".verus_agent", request)
print(f"operator_ready_issues={list(issues)!r}")
assert not issues
print("current-tip request validation: PASS (performed once by operator_ready_issues)")
assert not subprocess.check_output(["git", "status", "--porcelain"])
print("request bytes committed unchanged; working tree clean; neither patch applied")
print(subprocess.check_output(["git", "log", "-1", "--format=%h %s"], text=True).strip())
PY

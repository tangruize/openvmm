#!/usr/bin/env bash
set -euo pipefail
out=research/vmtime-duration-interface-candidate
cp "$out/freeze.patch" "$out/rejected-scope-freeze.patch"
cp "$out/run.patch" "$out/rejected-scope-run.patch"
bash "$out/package.command"
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
from pathlib import Path
import json
import subprocess

scope = json.loads(Path(".verus_agent/scope_manifest.json").read_text())
for path in ("vm/vmcore/src/interrupt.rs", "vm/vmcore/src/vm_task.rs"):
    frozen = subprocess.check_output(["git", "show", f"{scope['frozen_branch']}:{path}"])
    working = subprocess.check_output(["git", "show", f"HEAD:{path}"])
    print(f"{path}: frozen and working branch bytes identical = {frozen == working}")
    assert frozen == working
PY

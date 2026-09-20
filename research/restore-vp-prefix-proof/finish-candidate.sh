#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
git apply --reverse --directory=research/restore-vp-prefix-proof/candidate \
    research/restore-vp-prefix-proof/candidate-behavior.patch
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import difflib
from pathlib import Path
import shutil
import subprocess

root = Path.cwd()
out = root / "research/restore-vp-prefix-proof"
package = root / "research/freeze_requests/restore-vp-selector-named-closures"
path = "vmm_core/src/partition_unit/vp_set.rs"
original = subprocess.check_output(["git", "show", f"HEAD:{path}"], text=True)
candidate = out / "candidate"
actual = (candidate / path).read_text()
patch = "".join(difflib.unified_diff(
    original.splitlines(keepends=True), actual.splitlines(keepends=True),
    fromfile=f"a/{path}", tofile=f"b/{path}",
))
assert patch == (package / "run.patch").read_text(), "tested candidate diverged"
assert (root / path).read_text() == original, "authoritative selector changed"
assert sorted(p.name for p in package.iterdir()) == [
    "freeze.patch", "rationale.md", "run.patch"
]
for name in ("freeze.patch", "run.patch", "rationale.md"):
    assert (package / name).read_bytes() == (out / "request-inputs" / name).read_bytes()
tips = subprocess.check_output(
    ["git", "rev-parse", "HEAD", "refs/heads/argus/restore-v1-frozen"], text=True
).splitlines()
assert tips == (out / "prepare-candidate.log").read_text().splitlines()[1:3]
print("tested candidate matches submitted run.patch")
print("three submitted files match the validated inputs")
print("both authoritative branch tips and production selector are unchanged")
assert candidate.is_dir() and not candidate.is_symlink()
assert candidate.resolve().parent == out.resolve()
shutil.rmtree(candidate)
print("removed only the isolated candidate directory")
PY
git diff --exit-code -- .verus_agent/scope_manifest.json \
    .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-guard-native-presence
git status --short

#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
for name in freeze.patch run.patch rationale.md; do
    cmp "research/restore-time-partition-presence-validation/$name" \
        "research/freeze_requests/restore-time-partition-presence/$name"
done
git diff --exit-code HEAD -- \
    openvmm/openvmm_core/src/worker/dispatch.rs \
    openvmm/openvmm_core/src/worker/dispatch.spec.rs \
    openvmm/openvmm_core/src/worker/dispatch.proof.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import json
from pathlib import Path
import shutil

evidence = Path("research/restore-time-partition-presence-validation").resolve()
completed = json.loads((evidence / "runner-completed.json").read_text())
assert completed["state"] == "done" and completed["exit_code"] == 0
assert completed["live"] is False
assert not (evidence / "candidate-workspace").exists()
package = Path("research/freeze_requests/restore-time-partition-presence")
assert {p.name for p in package.iterdir()} == {"freeze.patch", "run.patch", "rationale.md"}
target = evidence / "target"
assert target.is_dir() and not target.is_symlink()
assert target.resolve().parent == evidence
shutil.rmtree(target)
print("Exact three-file package retained; candidate source and build cache removed.")
print("Authoritative production/spec/proof/manifests match HEAD.")
PY
git --no-pager status --short

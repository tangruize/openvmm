set -euo pipefail
out=research/restore-vp-index-coverage
test ! -e "$out/continuation-checks-status.json"
cp .argus_subagents/vp-index-native-proof-checks.json "$out/continuation-checks-status.json"
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import json
from pathlib import Path

state = json.loads(Path("research/PIPELINE_STATE.json").read_text())
progress = state["restore_vp_index_native_representation"]
receipt = json.loads(Path(progress["handoff_receipt"]).read_text())
status = json.loads(Path(progress["handoff_status"]).read_text())
assert receipt["task_id"] == status["task_id"] == progress["handoff_task_id"]
assert receipt["run_id"] == status["run_id"] == progress["handoff_run_id"]
assert status["state"] == progress["handoff_state"] == "error"
assert status["exit_code"] == progress["handoff_exit_code"] == 1
assert progress["accessor_body_proved"] and not progress["validator_body_proved"]
print("Current receipt and result recorded without claiming validator coverage.")
PY
git --no-pager diff --check
git --no-pager diff --stat
git --no-pager status --short

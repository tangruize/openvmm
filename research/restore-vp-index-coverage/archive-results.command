set -eu
out=research/restore-vp-index-coverage
for check in make_verify boundary spec_drift exec_drift; do
    test ! -e "$out/$check.complete.log"
    cp ".verus_agent/cache/checks/$check/latest.log" "$out/$check.complete.log"
done
test ! -e "$out/handoff-checks-status.json"
cp .argus_subagents/vp-index-coverage-handoff-checks.json "$out/handoff-checks-status.json"
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import json
from pathlib import Path

state = json.loads(Path("research/PIPELINE_STATE.json").read_text())
first = state["vp_selector_delivery_precommit"]
final = state["vp_selector_delivery_precommit_final"]
assert first["exit_code"] == 1
assert first["superseded_by"] == final["task_id"]
assert final["state"] == "done" and final["exit_code"] == 0
for attempt in (first, final):
    receipt = json.loads(Path(attempt["receipt"]).read_text())
    status = json.loads(Path(attempt["status"]).read_text())
    assert receipt["task_id"] == status["task_id"] == attempt["task_id"]
    assert receipt["run_id"] == status["run_id"] == attempt["run_id"]
    assert status["exit_code"] == attempt["exit_code"]
    for key in ("stdout_log", "stderr_log", "captured_stdout", "captured_stderr"):
        assert Path(attempt[key]).is_file(), attempt[key]
coverage = state["restore_vp_index_coverage"]
assert not coverage["validator_body_proved"]
assert coverage["production_source_restored"]
assert Path(coverage["handoff_status"]).is_file()
print("Pipeline receipt bindings and log paths confirmed; validator remains unproved.")
PY
git --no-pager diff --stat
git --no-pager status --short

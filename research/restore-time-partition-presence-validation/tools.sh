#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import importlib.util
for name in [
    "argus_verus.tools.operator.freeze_request",
    "argus_skill.tools.subagent",
]:
    print(name, importlib.util.find_spec(name).origin)
PY
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request --project-root . submit --help
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_skill.tools.subagent submit --help
git diff --exit-code HEAD -- openvmm/openvmm_core/src/worker/dispatch.rs openvmm/openvmm_core/src/worker/dispatch.proof.rs .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json
git diff --stat argus/restore-v1-frozen HEAD -- openvmm/openvmm_core/src/worker .verus_agent

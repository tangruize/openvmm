#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/inspect-cross-crate-admission
exec > "$evidence/intake.log" 2>&1
set -x

"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import json
from pathlib import Path
for name in ("scope_manifest.json", "tcb_manifest.json"):
    path = Path(".verus_agent") / name
    value = json.loads(path.read_text())
    assert isinstance(value, dict), name
    print(name, json.dumps(value, sort_keys=True))
PY

git --no-pager status --short
git diff --exit-code -- . ':!research/**' ':!.autors/**'
git diff --cached --exit-code -- . ':!research/**' ':!.autors/**'
git --no-pager log -1 --format='%cI %s' -- . ':!research/**' ':!.autors/**'
verification/tools/find-verus.sh
cat verification/verus-version verification/verus-revision toolchain/verus-src/.argus-verus-revision

set +e
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph --project . build \
    --out .verus_agent/cache/callgraph.json
graph_status=$?
set -e
printf 'callgraph_exit=%s\n' "$graph_status"

RUSTUP_TOOLCHAIN=1.95.0 cargo tree --locked --offline -p openvmm_core \
    -e features -i inspect > "$evidence/production-features.log" 2>&1
printf 'production_features_exit=0\n'
for check in make_verify boundary spec_drift exec_drift; do
    stat -c '%y %n' ".verus_agent/cache/checks/$check/latest.log"
    cmp ".verus_agent/cache/checks/$check/latest.log" \
        "research/native-partition-admission/reused-$check.log"
done

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/oneshot-native-admission
exec > "$evidence/intake.log" 2>&1
set -x
git diff --exit-code -- . ':!research/**' ':!.autors/**'
git diff --cached --exit-code -- . ':!research/**' ':!.autors/**'
git --no-pager log -1 --format='%H %cI %s' -- . ':!research/**' ':!.autors/**'
verification/tools/find-verus.sh
cat verification/verus-version verification/verus-revision toolchain/verus-src/.argus-verus-revision
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import json
from pathlib import Path
for name in ("scope_manifest.json", "tcb_manifest.json"):
    value = json.loads((Path(".verus_agent") / name).read_text())
    assert isinstance(value, dict), name
    print(name, json.dumps(value, sort_keys=True))
PY
set +e
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.callgraph.graph --project . build \
    --out .verus_agent/cache/callgraph.json
status=$?
set -e
printf 'callgraph_exit=%s\n' "$status"
RUSTUP_TOOLCHAIN=1.95.0 cargo tree --locked --offline -p openvmm_core \
    --prefix none --format '{p} features=[{f}]' > "$evidence/production-tree.log" 2>&1
grep -E '^(inspect |mesh_channel_core |mesh_protobuf |mesh_node |parking_lot )' \
    "$evidence/production-tree.log" | sort -u > "$evidence/production-features.log"
cat "$evidence/production-features.log"
for check in make_verify boundary spec_drift exec_drift; do
    cmp ".verus_agent/cache/checks/$check/latest.log" \
        "research/inspect-cross-crate-admission/reused-$check.log"
    stat -c '%y %n' ".verus_agent/cache/checks/$check/latest.log"
done

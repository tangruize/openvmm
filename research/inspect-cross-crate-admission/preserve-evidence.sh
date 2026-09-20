#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/inspect-cross-crate-admission
exec > "$evidence/preserve-evidence.log" 2>&1
set -x

git diff --exit-code -- . ':!research/**' ':!.autors/**'
git diff --cached --exit-code -- . ':!research/**' ':!.autors/**'
printf '%s\n' 'production_source_and_manifests_restored=true'
git --no-pager log -1 --format='%cI %s' -- . ':!research/**' ':!.autors/**'
git apply --check "$evidence/dependency-source.patch"
git apply --check "$evidence/representation-source.patch"
git apply --check "$evidence/integration-source.patch"
bash -n "$evidence/intake.sh" "$evidence/dependency.sh" \
    "$evidence/integration.sh" "$evidence/reproduce.sh" "$evidence/preserve-evidence.sh"

for check in make_verify boundary spec_drift exec_drift; do
    stat -c '%y %n' ".verus_agent/cache/checks/$check/latest.log"
    cp ".verus_agent/cache/checks/$check/latest.log" "$evidence/reused-$check.log"
done
stat -c '%y %s %n' target/verus-partial/debug/deps/libinspect-54f38ca0a5f7ca07.vir
wc -l -c "$evidence/dependency.vir" "$evidence/representation.vir"
git --no-pager status --short

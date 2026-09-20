#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

evidence=research/native-partition-admission
source=openvmm/openvmm_core/src/partition.rs
git diff --exit-code -- "$source"
git diff --cached --exit-code -- "$source"
git apply --check "$evidence/trait-annotation.patch"
git apply "$evidence/trait-annotation.patch"
restore_source() {
    git apply --reverse "$evidence/trait-annotation.patch"
}
trap restore_source EXIT

git --no-pager diff -- "$source" > "$evidence/attempted-source.patch"
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" make verify MODULE=restore' \
    > "$evidence/verify.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" make verify MODULE=restore \
    > "$evidence/verify.log" 2>&1
result=$?
set -e
printf 'verification_exit=%s\n' "$result"
restore_source
trap - EXIT
git diff --exit-code -- "$source"
printf '%s\n' 'production_source_restored=true'
exit "$result"

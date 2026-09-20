#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

printf '%s\n' '=== restored production, build configuration, and frozen inputs ==='
git diff --exit-code -- . ':(exclude)research' ':(exclude).autors'
git diff --cached --exit-code -- . ':(exclude)research' ':(exclude).autors'
printf '%s\n' '=== last tracked source/configuration change ==='
git log -1 --format='%cI %s' -- ':(glob)**/*.rs' ':(glob)**/Cargo.toml' \
    Cargo.lock Makefile verification .cargo .verus_agent
printf '%s\n' '=== reused unchanged-source check evidence ==='
for check in make_verify boundary spec_drift exec_drift; do
    source=".verus_agent/cache/checks/$check/latest.log"
    destination="research/native-partition-admission/reused-$check.log"
    test ! -e "$destination"
    stat -c '%y %n' "$source"
    cp "$source" "$destination"
done
printf '%s\n' '=== retained worktree changes ==='
git --no-pager status --short
git diff --check

#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root=$PWD
evidence=research/restore-time-partition-presence-validation
scratch="$root/$evidence/target/delivery-precommit"

test ! -e "$scratch"
mkdir -p "$evidence/target"
printf '%s\n' \
    'No Cargo package is modified: only research artifacts will be committed.' \
    'Package-scoped clippy and doc steps therefore have no modified-package targets.' \
    'Run the required full formatter on an isolated copy; preserve all original artifacts.'

git worktree add --detach "$scratch" HEAD
tar --exclude="$evidence/target" -cf "$evidence/target/delivery-input.tar" \
    research/freeze_requests/restore-time-partition-presence \
    "$evidence" \
    research/GROUND_TRUTH.md \
    research/restore-tsc-consistency \
    research/restore-unsaved-vp-frame
tar -xf "$evidence/target/delivery-input.tar" -C "$scratch"
git -C "$scratch" add -- \
    research/freeze_requests/restore-time-partition-presence \
    "$evidence" \
    research/GROUND_TRUTH.md \
    research/restore-tsc-consistency \
    research/restore-unsaved-vp-frame

printf '%s\n' '+ cargo xtask fmt --fix (isolated checkout)'
set +e
(
    cd "$scratch"
    cargo xtask fmt --fix
) > "$evidence/delivery-fmt.log" 2>&1
result=$?
set -e
printf 'cargo xtask fmt --fix exit=%s\n' "$result"
git -C "$scratch" --no-pager diff --stat
git -C "$scratch" diff --binary > "$evidence/target/delivery-formatting.patch"
printf 'formatter output: %s/delivery-fmt.log\n' "$evidence"
printf 'isolated changes: %s/target/delivery-formatting.patch\n' "$evidence"
exit "$result"

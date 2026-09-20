#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/restore-partition-presence-proof
name="${1:?evidence name required}"
shift
if [[ -e "$evidence/$name.command" || -e "$evidence/$name.log" ]]; then
    printf 'evidence already exists: %s\n' "$name" >&2
    exit 2
fi
printf '%q ' "$@" > "$evidence/$name.command"
printf '\n' >> "$evidence/$name.command"
set +e
/usr/bin/time -f 'elapsed_s=%e exit=%x' "$@" > "$evidence/$name.log" 2>&1
status=$?
set -e
printf '%s exit=%s log=%s/%s.log\n' "$name" "$status" "$evidence" "$name"
exit "$status"

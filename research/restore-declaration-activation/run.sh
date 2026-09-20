#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/restore-declaration-activation
name="${1:?evidence name required}"
shift
printf '%q ' "$@" > "$evidence/$name.command"
printf '\n' >> "$evidence/$name.command"
set +e
/usr/bin/time -f 'elapsed_s=%e exit=%x' "$@" > "$evidence/$name.log" 2>&1
status=$?
set -e
printf '%s exit=%s log=%s/%s.log\n' "$name" "$status" "$evidence" "$name"
exit "$status"

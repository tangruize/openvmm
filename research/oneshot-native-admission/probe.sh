#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/oneshot-native-admission
verus="$(verification/tools/find-verus.sh)"
printf '%s\n' '"$(verification/tools/find-verus.sh)" --no-lifetime --rlimit 50 research/oneshot-native-admission/function-pointer.rs' \
    > "$evidence/function-pointer.command"
set +e
"$verus" --no-lifetime --rlimit 50 "$evidence/function-pointer.rs" \
    > "$evidence/function-pointer.log" 2>&1
status=$?
set -e
printf 'function_pointer_probe_exit=%s\n' "$status"
exit "$status"

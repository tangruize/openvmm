#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/oneshot-native-admission
check="${1:?check required}"
command=("${ARGUS_SKILL_PYTHON:-python3}" -m "argus_verus.tools.checks.$check"
    --crate-root .)
case "$check" in
    make_verify) ;;
    boundary) command+=(--baseline-dir .verus_agent check) ;;
    spec_drift|exec_drift) command+=(--baseline-dir .verus_agent) ;;
    *) printf 'unknown check: %s\n' "$check" >&2; exit 2 ;;
esac
printf '%q ' "${command[@]}" > "$evidence/$check.command"
printf '\n' >> "$evidence/$check.command"
set +e
"${command[@]}" > "$evidence/$check-driver.log" 2>&1
status=$?
set -e
cp ".verus_agent/cache/checks/$check/latest.log" "$evidence/$check.log"
printf '%s_exit=%s\n' "$check" "$status"
exit "$status"

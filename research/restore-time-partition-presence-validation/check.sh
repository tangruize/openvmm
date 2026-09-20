#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/../.."
case "$1" in
    make_verify) args=(--crate-root .) ;;
    boundary) args=(--crate-root . --baseline-dir .verus_agent check) ;;
    spec_drift|exec_drift) args=(--crate-root . --baseline-dir .verus_agent) ;;
    *) printf '%s\n' 'unknown check' >&2; exit 2 ;;
esac
evidence=research/restore-time-partition-presence-validation
command=("${ARGUS_SKILL_PYTHON:-python3}" -m "argus_verus.tools.checks.$1" "${args[@]}")
printf '%q ' /usr/bin/time -f 'elapsed_s=%e exit=%x' "${command[@]}" > "$evidence/$1.command"
printf '\n' >> "$evidence/$1.command"
/usr/bin/time -f 'elapsed_s=%e exit=%x' "${command[@]}" > "$evidence/$1.log" 2>&1
result=$?
cp ".verus_agent/cache/checks/$1/latest.log" "$evidence/$1.full.log"
cat "$evidence/$1.log"
exit "$result"

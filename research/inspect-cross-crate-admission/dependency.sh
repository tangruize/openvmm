#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
evidence=research/inspect-cross-crate-admission
phase="${1:-dependency}"
exec > "$evidence/$phase-driver.log" 2>&1
set -x
verus_root="$(dirname "$(verification/tools/find-verus.sh)")"
export PATH="$verus_root:$PATH"
printf '%s\n' '/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s cargo verus focus -p inspect --no-default-features --features derive,defer,std -vv -- --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent --log vir' \
    > "$evidence/$phase-verifier.command"
set +e
/usr/bin/time -f "elapsed_s=%e exit=%x" timeout --kill-after=5s 110s \
    cargo verus focus -p inspect --no-default-features --features derive,defer,std -vv -- \
    --no-lifetime --multiple-errors 20 --num-threads 1 --triggers-mode silent --log vir \
    > "$evidence/$phase-verifier.log" 2>&1
result=$?
set -e
git --no-pager diff -- Cargo.lock support/inspect/Cargo.toml support/inspect/src/lib.rs \
    > "$evidence/$phase-source.patch"
printf 'dependency_verification_exit=%s\n' "$result"
find target/verus-partial -type f -name '*inspect*.vir' -print \
    > "$evidence/$phase-artifacts.log"
if test -f .verus-log/crate.vir; then
    cp .verus-log/crate.vir "$evidence/$phase.vir"
fi
exit "$result"

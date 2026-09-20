#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
verus="$(verification/tools/find-verus.sh)"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
out=research/restore-vp-prefix-proof
"$verus" --version
"$VERUS_Z3_PATH" --version

# These isolate syntax admission, not the production selection theorem.
for probe in closure-pattern closure-pattern-spec closure-binding-control; do
    status=0
    "$verus" --crate-type lib "$out/$probe.rs" --no-lifetime \
        --multiple-errors 5 --num-threads 1 --triggers-mode silent \
        > "$out/$probe.log" 2>&1 || status=$?
    printf '%s exit=%s\n' "$probe" "$status"
    tail -20 "$out/$probe.log"
    case "$probe:$status" in
        closure-pattern:1|closure-pattern-spec:1)
            grep -q 'only variables are supported here, not general patterns' "$out/$probe.log"
            ;;
        closure-binding-control:0)
            grep -q 'verification results:: .* verified, 0 errors' "$out/$probe.log"
            ;;
        *) exit 1 ;;
    esac
done

#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
out=research/vmtime-duration-binding

if test "$#" -eq 0; then
    set -- pattern-type transmute
fi
for probe in "$@"; do
    case "$probe" in
        pattern-type|transmute) ;;
        *) printf 'Unknown probe: %s\n' "$probe" >&2; exit 2 ;;
    esac
    start=$SECONDS
    set +e
    timeout 30s "$verus" --crate-type lib --verify-root \
        --multiple-errors 2 --num-threads 1 --triggers-mode silent \
        "$out/$probe.rs" > "$out/$probe.log" 2>&1
    code=$?
    set -e
    printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))" >> "$out/$probe.log"
    cat "$out/$probe.log"
done

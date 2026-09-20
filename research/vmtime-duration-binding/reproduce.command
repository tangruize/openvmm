#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
out=research/vmtime-duration-binding
git apply --check "$out/production-annotation.patch"
git apply "$out/production-annotation.patch"
trap 'git apply --reverse research/vmtime-duration-binding/production-annotation.patch' EXIT
start=$SECONDS
set +e
timeout 110s bash "$out/focused.command" > "$out/production-body.log" 2>&1
code=$?
set -e
printf '\nexit=%s elapsed_s=%s\n' "$code" "$((SECONDS-start))" >> "$out/production-body.log"
cat "$out/production-body.log"
exit "$code"

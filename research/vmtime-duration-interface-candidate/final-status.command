#!/usr/bin/env bash
set -euo pipefail
git --no-pager status --short
git diff --check -- research/GROUND_TRUTH.md
printf '\nNo submitted request directory: '
test ! -e research/freeze_requests/restore-duration-nanoseconds-interface
printf 'confirmed\n'

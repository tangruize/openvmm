#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" research/restore-time-partition-presence-validation/validate_candidate.py \
    > research/restore-time-partition-presence-validation/candidate.log 2>&1

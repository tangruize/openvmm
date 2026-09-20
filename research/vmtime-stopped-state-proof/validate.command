#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
bash research/vmtime-stopped-state-proof/regression.command \
    > research/vmtime-stopped-state-proof/regression.log 2>&1
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-stopped-state-proof/checks.py \
    > research/vmtime-stopped-state-proof/checks.log 2>&1

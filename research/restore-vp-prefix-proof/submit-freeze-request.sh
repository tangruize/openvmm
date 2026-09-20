#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request \
    --project-root . submit --id restore-vp-selector-named-closures \
    --freeze-patch research/restore-vp-prefix-proof/request-inputs/freeze.patch \
    --run-patch research/restore-vp-prefix-proof/request-inputs/run.patch \
    --rationale research/restore-vp-prefix-proof/request-inputs/rationale.md

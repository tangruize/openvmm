#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")"
"${ARGUS_SKILL_PYTHON:-python3}" prepare.py
export CARGO_TARGET_DIR="$PWD/target"
export RUSTUP_TOOLCHAIN=1.95.0
cd workspace
cargo nextest run --offline --profile agent -p vmm_core \
    --test vmtime_reset_installation --success-output immediate

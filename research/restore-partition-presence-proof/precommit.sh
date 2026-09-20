#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
packages=(
    -p chipset_resources -p mesh_channel -p mesh_channel_core -p mesh_protobuf
    -p openvmm_core -p openvmm_defs -p pal_async -p state_unit -p virt
    -p vmm_core -p vmm_core_defs
)
runner=research/restore-partition-presence-proof/run.sh
bash "$runner" submission-clippy env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo clippy --all-targets "${packages[@]}"
bash "$runner" submission-doc env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo doc --no-deps "${packages[@]}"
bash "$runner" submission-fmt env RUSTUP_TOOLCHAIN=1.95.0 \
    cargo xtask fmt --fix

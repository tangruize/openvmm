#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
export RUSTUP_TOOLCHAIN=1.95.0
export RUSTC_BOOTSTRAP=1
export CARGO_UNSTABLE_LOCKFILE_PATH=true
export CARGO_RESOLVER_LOCKFILE_PATH="$PWD/research/vmtime-stopped-state-proof/workspace/Cargo.lock"

rustfmt --edition 2024 --check vm/vmcore/src/vmtime.rs
cargo check --offline --locked -p vmcore --all-targets
cargo nextest run --offline --locked --profile agent -p vmcore -E 'test(vmtime::)'

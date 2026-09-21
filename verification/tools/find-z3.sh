#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_root="$repo_root/toolchain/verus-src/source"
version="$(sed -n 's/^pub const Z3_VERSION: \&str = "\([^"]*\)";$/\1/p' \
    "$source_root/cargo-verus-toolchains/src/external_deps.rs")"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: cannot determine the pinned Verus solver version; run 'make verify-setup'" >&2
    exit 1
fi

candidate="${VERUS_Z3_PATH:-$source_root/z3}"
if [[ ! -x "$candidate" ]]; then
    echo "error: Z3 $version is unavailable at $candidate" >&2
    echo "run 'make verify-setup' or set VERUS_Z3_PATH to a compatible executable" >&2
    exit 1
fi
candidate="$(cd "$(dirname "$candidate")" && pwd)/$(basename "$candidate")"
actual="$("$candidate" --version)"
if [[ ! "$actual" =~ ^Z3\ version\ ([0-9]+\.[0-9]+\.[0-9]+)(\ |$) ]] \
    || [[ "${BASH_REMATCH[1]}" != "$version" ]]; then
    echo "error: expected Z3 $version, found '$actual' at $candidate" >&2
    exit 1
fi

printf '%s\n' "$candidate"

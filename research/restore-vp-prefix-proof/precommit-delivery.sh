#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-prefix-proof"
scratch="$root/target/vp-selector-delivery-format-final"

test ! -e "$scratch"
mkdir -p "$scratch"
git archive HEAD | tar -x -C "$scratch"
cp -a research/restore-vp-prefix-proof "$scratch/research/"
cp -a research/freeze_requests/restore-vp-selector-named-closures \
    "$scratch/research/freeze_requests/"
cp research/GROUND_TRUTH.md "$scratch/research/GROUND_TRUTH.md"
mkdir -p "$scratch/toolchain"
ln -s "$root/toolchain/verus-src" "$scratch/toolchain/verus-src"
ln -s "$root/.packages" "$scratch/.packages"

printf '%s\n' \
    'Only research artifacts are being committed; no Cargo package is modified.' \
    'Package-scoped clippy and doc therefore have no modified-package targets.' \
    'Running the required full formatter in an isolated archive to preserve frozen source.'
set +e
(
    cd "$scratch"
    export RUSTUP_TOOLCHAIN=1.95.0
    export CARGO_TARGET_DIR="$root/target"
    cargo xtask fmt --fix
) > "$out/delivery-fmt-final.log" 2>&1
status=$?
set -e
printf 'cargo xtask fmt --fix exit=%s\n' "$status"
test "$status" -eq 0

"${ARGUS_SKILL_PYTHON:-python3}" - "$root" "$scratch" <<'PY'
from pathlib import Path
import re
import sys

root, scratch = map(Path, sys.argv[1:])
log = (root / "research/restore-vp-prefix-proof/delivery-fmt-final.log").read_text()
plain = re.sub(r"\x1b\[[0-9;]*m", "", log)
failures = [
    line for line in plain.splitlines()
    if "while running " in line or line.startswith("error:")
]
if failures:
    print("\n".join(failures))
    raise SystemExit("formatter reported a failed inner pass despite outer success")
for relative in (
    "research/restore-vp-prefix-proof",
    "research/freeze_requests/restore-vp-selector-named-closures",
):
    for copied in (scratch / relative).rglob("*"):
        if copied.is_file():
            original = root / copied.relative_to(scratch)
            if original.read_bytes() != copied.read_bytes():
                raise SystemExit(f"formatter changed submitted artifact: {relative}/{copied.name}")
assert (root / "research/GROUND_TRUTH.md").read_bytes() == (
    scratch / "research/GROUND_TRUTH.md"
).read_bytes()
print("all in-scope submitted artifacts are unchanged by the full formatter")
PY
git diff --check
git diff --exit-code -- vmm_core/src/partition_unit/vp_set.rs \
    .verus_agent/scope_manifest.json .verus_agent/tcb_manifest.json \
    research/freeze_requests/restore-guard-native-presence

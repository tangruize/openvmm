#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/vmtime-reset-normalization-request"
scratch="$out/precommit-workspace"
package="research/freeze_requests/restore-reset-constructor-normalization"

test ! -e "$scratch"
mkdir -p "$scratch"
git archive HEAD | tar -x -C "$scratch"
mkdir -p "$scratch/toolchain"
ln -s "$root/toolchain/verus-src" "$scratch/toolchain/verus-src"
ln -s "$root/.packages" "$scratch/.packages"
cp -a "$package" "$scratch/research/freeze_requests/"
git -C "$scratch" --git-dir="$root/.git" --work-tree="$scratch" \
    apply "$root/$package/run.patch"
cp "$scratch/vm/vmcore/src/vmtime.rs" "$out/preformat-vmtime.rs"

export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$out/precommit-target"
cd "$scratch"
printf '%s\n' 'Checking the isolated proposed working-base result, not dirty scalar work.'
cargo clippy --all-targets -p vmcore > "$out/clippy.log" 2>&1
printf '%s\n' 'cargo clippy --all-targets -p vmcore: PASS'
cargo doc --no-deps -p vmcore > "$out/doc.log" 2>&1
printf '%s\n' 'cargo doc --no-deps -p vmcore: PASS'
cargo xtask fmt --fix > "$out/fmt.log" 2>&1
printf '%s\n' 'cargo xtask fmt --fix: exited successfully'

"${ARGUS_SKILL_PYTHON:-python3}" - "$root" "$scratch" "$out" "$package" <<'PY'
from pathlib import Path
import re
import sys

root, scratch, out, package = map(Path, sys.argv[1:])
log = re.sub(r"\x1b\[[0-9;]*m", "", (out / "fmt.log").read_text())
failures = [
    line for line in log.splitlines()
    if "while running " in line or line.startswith("error:")
]
if failures:
    raise SystemExit("\n".join(failures))
assert (out / "preformat-vmtime.rs").read_bytes() == (
    scratch / "vm/vmcore/src/vmtime.rs"
).read_bytes(), "formatter changed the proposed method's source"
for name in ("freeze.patch", "run.patch", "rationale.md"):
    assert (root / package / name).read_bytes() == (
        scratch / package / name
    ).read_bytes(), f"formatter changed package member {name}"
print("All formatter passes succeeded; proposal and package bytes unchanged.")
PY

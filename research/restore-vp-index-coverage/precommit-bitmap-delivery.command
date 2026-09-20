#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/restore-vp-index-coverage"
scratch="$root/target/bitmap-delivery-format"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_TARGET_DIR="$root/target"

case "${1:-all}" in
    all)
        cargo clippy --all-targets -p vm_topology > "$out/delivery-clippy.log" 2>&1
        printf 'cargo clippy --all-targets -p vm_topology exit=0\n'
        cargo doc --no-deps -p vm_topology > "$out/delivery-doc.log" 2>&1
        printf 'cargo doc --no-deps -p vm_topology exit=0\n'
        ;;
    --format-only) ;;
    *) printf 'usage: %s [--format-only]\n' "$0" >&2; exit 2 ;;
esac

test ! -e "$scratch"
mkdir -p "$scratch"
git archive HEAD | tar -x -C "$scratch"
cp Cargo.lock "$scratch/Cargo.lock"
cp -a vm/vmcore/vm_topology "$scratch/vm/vmcore/"
cp -a research/restore-vp-index-coverage "$scratch/research/"
cp -a research/freeze_requests/restore-vp-bitmap-native-update \
    "$scratch/research/freeze_requests/"
cp research/GROUND_TRUTH.md research/PIPELINE_STATE.json "$scratch/research/"
ln -s "$root/toolchain" "$scratch/toolchain"
ln -s "$root/.packages" "$scratch/.packages"

(
    cd "$scratch"
    cargo xtask fmt --fix
) > "$out/delivery-fmt.log" 2>&1
printf 'cargo xtask fmt --fix exit=0 (isolated full workspace)\n'

"${ARGUS_SKILL_PYTHON:-python3}" - "$root" "$scratch" <<'PY'
from pathlib import Path
import re
import sys

root, scratch = map(Path, sys.argv[1:])
log = (root / "research/restore-vp-index-coverage/delivery-fmt.log").read_text()
plain = re.sub(r"\x1b\[[0-9;]*m", "", log)
failures = [
    line for line in plain.splitlines()
    if "while running " in line or line.startswith("error:")
]
if failures:
    print("\n".join(failures))
    raise SystemExit("formatter reported a failed inner pass")
changed = []
for relative in (
    "vm/vmcore/vm_topology",
    "research/restore-vp-index-coverage",
    "research/freeze_requests/restore-vp-bitmap-native-update",
):
    for copied in (scratch / relative).rglob("*"):
        if copied.is_file():
            if copied.relative_to(scratch).as_posix() == "research/restore-vp-index-coverage/delivery-fmt.log":
                continue
            original = root / copied.relative_to(scratch)
            if original.read_bytes() != copied.read_bytes():
                changed.append(str(copied.relative_to(scratch)))
for relative in ("Cargo.lock", "research/GROUND_TRUTH.md", "research/PIPELINE_STATE.json"):
    if (root / relative).read_bytes() != (scratch / relative).read_bytes():
        changed.append(relative)
if changed:
    print("\n".join(changed))
    raise SystemExit("formatter changed delivery files; inspect before committing")
print("all delivered source and artifacts unchanged by full formatter")
PY
git diff --check

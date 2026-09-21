#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
out="$root/research/mutex-observation-interface"
scratch="$out/target/delivery-format"
export RUSTUP_TOOLCHAIN=1.95.0
export CARGO_BUILD_JOBS=4

printf '%s\n' 'Preserving accepted changes in openvmm_core, openvmm_defs, and state_unit.'
for check in clippy doc; do
    printf 'starting %s\n' "$check"
    if [[ "$check" == clippy ]]; then
        command=(cargo clippy --locked --all-targets -p openvmm_core -p openvmm_defs -p state_unit)
    else
        command=(cargo doc --locked --no-deps -p openvmm_core -p openvmm_defs -p state_unit)
    fi
    printf '%q ' "${command[@]}"
    printf '\n'
    set +e
    "${command[@]}" > "$out/delivery-$check.log" 2>&1
    result=$?
    set -e
    printf '%s exit=%s\n' "$check" "$result"
    if [[ "$result" != 0 ]]; then
        printf 'Stopped: inspect %s/delivery-%s.log\n' "$out" "$check"
        exit "$result"
    fi
done

test ! -e "$scratch"
mkdir -p "$scratch"
git archive HEAD | tar -x -C "$scratch"
"${ARGUS_SKILL_PYTHON:-python3}" -B - "$root" "$scratch" <<'PY'
from pathlib import Path
import shutil
import subprocess
import sys

root, scratch = map(Path, sys.argv[1:])
modified = subprocess.check_output(
    ["git", "diff", "--name-only", "-z"], cwd=root
).split(b"\0")
untracked = subprocess.check_output(
    ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=root
).split(b"\0")
for raw in dict.fromkeys(modified + untracked):
    if not raw:
        continue
    path = Path(raw.decode())
    original, copied = root / path, scratch / path
    if not original.is_file():
        raise RuntimeError(f"unexpected non-file intake path: {path}")
    copied.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(original, copied)
(scratch / "toolchain").mkdir(exist_ok=True)
(scratch / "toolchain/verus-src").symlink_to(root / "toolchain/verus-src", target_is_directory=True)
(scratch / ".packages").symlink_to(root / ".packages", target_is_directory=True)
PY

printf '%s\n' 'Running the complete formatter last in an exact source snapshot.'
set +e
(
    cd "$scratch"
    export CARGO_TARGET_DIR="$root/target"
    cargo xtask fmt --fix
) > "$out/delivery-fmt.log" 2>&1
result=$?
set -e
printf 'fmt exit=%s\n' "$result"
test "$result" -eq 0
"${ARGUS_SKILL_PYTHON:-python3}" -B - "$root" "$scratch" <<'PY'
from pathlib import Path
import re
import subprocess
import sys

root, scratch = map(Path, sys.argv[1:])
log = (root / "research/mutex-observation-interface/delivery-fmt.log").read_text()
plain = re.sub(r"\x1b\[[0-9;]*m", "", log)
failures = [line for line in plain.splitlines()
            if "while running " in line or line.startswith("error:")]
if failures:
    print("\n".join(failures[:40]))
    raise SystemExit("formatter reported a failed inner pass")
files = subprocess.check_output(["git", "diff", "--name-only", "-z"], cwd=root).split(b"\0")
files += subprocess.check_output(
    ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=root
).split(b"\0")
for raw in dict.fromkeys(files):
    if not raw:
        continue
    path = Path(raw.decode())
    if path.name.startswith("delivery-") and path.suffix == ".log":
        continue
    if (root / path).read_bytes() != (scratch / path).read_bytes():
        raise SystemExit(f"formatter changed proposed commit content: {path}")
print("All pending accepted work and the exact request package survive the formatter unchanged.")
PY

#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Prepare an unapplied, behavior-preserving production-source proposal."""

import difflib
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/restore-partition-presence-proof/proposal"
DISPATCH = "openvmm/openvmm_core/src/worker/dispatch.rs"
ORIGINAL = '''        anyhow::ensure!(
            restore_time.is_none()
                || saved_state.units.iter().any(|unit| unit.name == "partition"),
            "time-adjusted snapshot restore requires partition state"
        );'''
NORMALIZED = '''        let partition_presence_valid = restore_time.is_none()
            || saved_state.units.iter().any(|unit| unit.name.as_str() == "partition");
        anyhow::ensure!(
            partition_presence_valid,
            "time-adjusted snapshot restore requires partition state"
        );'''
ANNOTATED = '''        let partition_presence_valid = restore_time.is_none()
            || saved_state.units.iter().any(
                #[verus_spec(result: bool =>
                    ensures result == (unit.name@ == "partition"@)
                )]
                |unit| unit.name.as_str() == "partition",
            );
        anyhow::ensure!(
            partition_presence_valid,
            "time-adjusted snapshot restore requires partition state"
        );'''


def baseline(ref, path):
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout


def diff(path, before, after):
    if before == after:
        return ""
    header = f"diff --git a/{path} b/{path}\n"
    if not before:
        header += "new file mode 100644\n"
    return header + "".join(
        difflib.unified_diff(
            before.splitlines(keepends=True), after.splitlines(keepends=True),
            fromfile=f"a/{path}" if before else "/dev/null",
            tofile=f"b/{path}",
        )
    )


def replace_once(text, before, after):
    assert text.count(before) == 1, "Expected the original production guard once"
    return text.replace(before, after, 1)


OUT.mkdir(exist_ok=True)
frozen = baseline("argus/restore-v1-frozen", DISPATCH)
freeze = diff(DISPATCH, frozen, replace_once(frozen, ORIGINAL, NORMALIZED))
assert (OUT / "freeze.patch").read_text() == freeze

paths = subprocess.check_output(
    ["git", "diff", "--name-only"], cwd=ROOT, text=True
).splitlines()
paths = [
    path for path in paths
    if path == "Cargo.lock" or path.endswith((".rs", "Cargo.toml"))
]
run = []
for path in paths:
    before = baseline("HEAD", path)
    after = (ROOT / path).read_text()
    if path == DISPATCH:
        after = replace_once(after, ORIGINAL, ANNOTATED)
    run.append(diff(path, before, after))
for path in sorted((ROOT / "verification/dependencies/anyhow").rglob("*")):
    if path.is_file() and not any(part.startswith(".") for part in path.relative_to(
        ROOT / "verification/dependencies/anyhow"
    ).parts):
        relative = path.relative_to(ROOT).as_posix()
        run.append(diff(relative, "", path.read_text()))
with (OUT / "run-native.patch").open("x") as output:
    output.write("".join(run))
print("Prepared independent patches; no frozen source change applied")

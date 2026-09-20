#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Generate independent proposals from current bases and the actual candidate."""

import difflib
from pathlib import Path
import subprocess


root = Path(__file__).resolve().parents[2]
out = root / "research/restore-vp-prefix-proof"
path = "vmm_core/src/partition_unit/vp_set.rs"
working = subprocess.check_output(
    ["git", "show", f"HEAD:{path}"], cwd=root, text=True
)
frozen = subprocess.check_output(
    ["git", "show", f"argus/restore-v1-frozen:{path}"], cwd=root, text=True
)
candidate = (out / "candidate" / path).read_text()
assert working == frozen, "selector bases have diverged; re-inspect them"
assert (root / path).read_text() == working, "authoritative selector changed"

frozen_candidate = candidate
for annotation in (
    "use vstd::prelude::*;\n",
    "#[vstd::prelude::verus_verify]\n",
):
    assert frozen_candidate.count(annotation) == 1
    frozen_candidate = frozen_candidate.replace(annotation, "")

inputs = out / "request-inputs"
inputs.mkdir(exist_ok=True)
for name, before, after in (
    ("freeze.patch", frozen, frozen_candidate),
    ("run.patch", working, candidate),
):
    patch = "".join(
        difflib.unified_diff(
            before.splitlines(keepends=True),
            after.splitlines(keepends=True),
            fromfile=f"a/{path}",
            tofile=f"b/{path}",
        )
    )
    assert patch, f"{name} would be empty"
    (inputs / name).write_text(patch)
    print(f"{name}: {len(patch.splitlines())} lines")

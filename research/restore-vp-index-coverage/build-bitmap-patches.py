#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Generate independent patches after retaining the accessor on the working tip."""

import difflib
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
out = root / "research/restore-vp-index-coverage"
vp_set = "vmm_core/src/partition_unit/vp_set.rs"


def baseline(ref, path):
    return subprocess.check_output(
        ["git", "show", f"{ref}:{path}"], cwd=root, text=True
    )


def diff(path, before, after):
    return "".join(
        difflib.unified_diff(
            before.splitlines(keepends=True),
            after.splitlines(keepends=True),
            fromfile=f"a/{path}" if before else "/dev/null",
            tofile=f"b/{path}",
        )
    )


(out / "bitmap-freeze.patch").write_text(
    diff(
        vp_set,
        baseline("argus/restore-v1-frozen", vp_set),
        (out / "bitmap-frozen-vp_set.rs").read_text(),
    )
)
run_patch = diff(
    vp_set,
    baseline("HEAD", vp_set),
    (out / "bitmap-candidate-vp_set.rs").read_text(),
)
(out / "bitmap-run.patch").write_text(run_patch)

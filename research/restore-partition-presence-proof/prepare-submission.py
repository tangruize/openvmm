#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Retarget run.patch without changing its previously verified source result."""

import difflib
import json
from pathlib import Path
import subprocess
import sys

from argus_verus.tools.operator.freeze_request import Request, _patch_results


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "research/freeze_requests/restore-guard-native-presence"
CACHE = ROOT / ".verus_agent/cache/restore-guard-submission"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


if sys.argv[1:] == ["capture"]:
    CACHE.mkdir(parents=True, exist_ok=False)
    request = Request(
        "restore-guard-native-presence", PACKAGE,
        PACKAGE / "freeze.patch", PACKAGE / "run.patch", PACKAGE / "rationale.md",
    )
    with _patch_results(ROOT, ROOT / ".verus_agent", request) as results:
        paths = git("diff", "--name-only", "HEAD", results.run_commit).splitlines()
        for path in paths:
            target = CACHE / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((results.run_project / path).read_bytes())
        (CACHE / "paths.json").write_text(json.dumps(paths, indent=2) + "\n")
        (CACHE / "original-run.patch").write_bytes((PACKAGE / "run.patch").read_bytes())
        (CACHE / "freeze.patch").write_bytes((PACKAGE / "freeze.patch").read_bytes())
    print(f"Captured the validated candidate result for {len(paths)} paths")
elif sys.argv[1:] == ["rebase"]:
    assert (PACKAGE / "freeze.patch").read_bytes() == (CACHE / "freeze.patch").read_bytes()
    paths = json.loads((CACHE / "paths.json").read_text())
    patch = []
    changed = 0
    for path in paths:
        current = ROOT / path
        before = current.read_text() if current.exists() else ""
        after = (CACHE / path).read_text()
        if before == after:
            continue
        changed += 1
        patch.append(f"diff --git a/{path} b/{path}\n")
        if not current.exists():
            patch.append("new file mode 100644\n")
        patch.extend(difflib.unified_diff(
            before.splitlines(keepends=True), after.splitlines(keepends=True),
            fromfile=f"a/{path}" if current.exists() else "/dev/null",
            tofile=f"b/{path}",
        ))
    (PACKAGE / "run.patch").write_text("".join(patch))
    print(f"Retargeted {changed} patch paths; candidate source and freeze.patch unchanged")
else:
    raise SystemExit("usage: prepare-submission.py capture|rebase")

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Ownership controls against the real mesh_protobuf production bodies.

Select the native-loan candidate with VERUS and put its directory first in PATH.
The opt-in functions are proof regressions, not alternate production bodies.
"""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]
CASES = {
    "stale": "assertion failed",
    "slot": "assertion failed",
    "duplicate": "use of moved value",
    "live": "cannot borrow",
}


@pytest.mark.parametrize("case,diagnostic", CASES.items())
def test_production_ownership_control(case, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "explicitly select the native-loan candidate"
    command = [
        sys.executable,
        str(ROOT / "verification/tools/fresh_verification.py"),
        "focus",
        "mesh_protobuf",
        "--",
        "--verify-only-module",
        "table::decode",
        "--verify-function",
        f"negative_{case}",
        "--cfg",
        f"verus_inplace_negative_{case}",
        "--rlimit",
        "50",
    ]
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        timeout=120,
    )
    log = tmp_path / f"{case}.log"
    log.write_text(result.stdout)
    assert result.returncode != 0, f"negative control unexpectedly verified: {log}"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

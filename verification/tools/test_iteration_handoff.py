# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Rejection controls for source-bound saved-time iteration, not table decoding."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,function,diagnostic", [
    ("first", "check_saved_time_iteration", "assertion failed"),
    ("default", "check_saved_time_iteration", "assertion failed"),
    ("number", "negative_iterator_number", "assertion failed"),
    ("stale", "negative_iterator_stale", "assertion failed"),
    ("domain", "negative_iterator_domain", "precondition not satisfied"),
])
def test_saved_time_iterator_rejects_invalid_claims(case, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-parser candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "protobuf",
        "--verify-function", function, "--cfg", f"verus_iteration_negative_{case}",
        "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid iterator claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

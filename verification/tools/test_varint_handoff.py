# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Negative controls for the production saved-time varint parser proof."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,diagnostic", [
    ("value", "assertion failed"),
    ("suffix", "assertion failed"),
    ("unterminated", "precondition not satisfied"),
    ("missing", "precondition not satisfied"),
    ("overlong", "precondition not satisfied"),
])
def test_saved_time_varint_rejects_invalid_claims(case, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-parser candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "protobuf",
        "--verify-function", f"negative_varint_{case}",
        "--cfg", f"verus_varint_negative_{case}", "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid varint parser claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

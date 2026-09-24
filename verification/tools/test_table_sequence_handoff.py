# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Rejection controls for nonempty production-table SavedState recovery."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("first", "vmcore", "vmtime", "recover_vmtime_sequence", "assertion failed"),
    ("default", "vmcore", "vmtime", "recover_vmtime_sequence", "assertion failed"),
    ("second_take", "vmcore", "vmtime", "recover_vmtime_sequence", "assertion failed"),
    ("stale", "vmcore", "vmtime", "check_sequence_retained", "assertion failed"),
    ("uninit", "vmcore", "vmtime", "check_sequence_uninitialized", "precondition not satisfied"),
    ("pointer", "vmcore", "vmtime", "read_vmtime_sequence", "precondition not satisfied"),
    ("number", "vmcore", "vmtime", "read_vmtime_sequence", "compiler constants are not identical"),
    ("entry", "mesh_protobuf", "table::decode", "*saved_read_fields_inner*", "compiler constants are not identical"),
    ("resource", "mesh_protobuf", "table::decode", "*saved_read_fields_inner*", "metadata pointee differs from decode"),
])
def test_production_table_rejects_invalid_claims(
    case, package, module, function, diagnostic, tmp_path,
):
    assert os.environ.get("VERUS"), "select the native-parser candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", f"verus_table_negative_{case}",
        "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid production-table claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

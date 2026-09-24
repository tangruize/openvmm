# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Negative controls for the production saved-time erased field read."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("stale", "vmcore", "vmtime", "check_present_retained", "assertion failed"),
    ("default", "vmcore", "vmtime", "check_present_retained", "assertion failed"),
    ("suffix", "vmcore", "vmtime", "check_parsed_storage_recovery", "assertion failed"),
    ("uninit", "vmcore", "vmtime", "negative_present_invalid_initialization", "precondition not satisfied"),
    ("missing", "vmcore", "vmtime", "negative_present_requires_varint", "precondition not satisfied"),
    ("entry", "mesh_protobuf", "table::decode", "read_u64_erased_field", "compiler constants are not identical"),
    ("resource", "mesh_protobuf", "table::decode", "read_u64_erased_field", "metadata pointee differs from decode"),
    ("pointer", "mesh_protobuf", "table::decode", "read_u64_erased_field", "precondition not satisfied"),
])
def test_saved_time_read_rejects_invalid_claims(
    case, package, module, function, diagnostic, tmp_path,
):
    assert os.environ.get("VERUS"), "select the native-parser candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", f"verus_read_negative_{case}",
        "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid erased field read claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

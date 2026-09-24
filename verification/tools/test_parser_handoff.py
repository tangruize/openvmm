# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Controls for the source-bound symbolic SavedStateBlob parser."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,function,diagnostic", [
    ("time", "check_saved_blob_wrong_time", "assertion failed"),
    ("payload", "check_saved_blob_requires_payload", "precondition not satisfied"),
    ("url", "check_saved_blob_requires_type", "precondition not satisfied"),
    ("source", "check_saved_blob_wrong_source", "same source-generated trait implementation"),
    ("type", "check_saved_blob_wrong_type", "same source-generated trait implementation"),
])
def test_saved_blob_parser_rejects_invalid_claims(case, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-parser candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmcore", "--", "--verify-only-module", "vmtime",
        "--verify-function", function, "--cfg", f"verus_parser_negative_{case}",
        "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid parser claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("first", "vmcore", "vmtime", "parse_saved_blob_sequence", "assertion failed"),
    ("default", "vmcore", "vmtime", "parse_saved_blob_sequence", "assertion failed"),
    ("domain", "vmcore", "vmtime", "check_saved_sequence_requires_domain", "precondition not satisfied"),
    ("binding", "vmcore", "vmtime", "check_saved_sequence_imported_binding", "same source-generated trait implementation"),
    ("resource", "vmcore", "vmtime", "check_saved_sequence_imported_binding", "same source-generated trait implementation"),
    ("bytes", "mesh_protobuf", "protobuf", "check_fresh_reader_body", "assertion failed"),
    ("range", "mesh_protobuf", "protobuf", "check_fresh_reader_body", "assertion failed"),
])
def test_symbolic_blob_and_constructor_claims(case, package, module, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the symbolic-parser candidate"
    prefix = "verus_constructor_negative" if package == "mesh_protobuf" else "verus_blob_sequence_negative"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", f"{prefix}_{case}", "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (tmp_path / f"{case}.log").write_text(result.stdout)
    assert result.returncode != 0, "invalid symbolic parser claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

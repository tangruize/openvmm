# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Rejection controls through the actual default_field_dyn adapter."""

import json
import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]
CASES = {
    "slot": ("negative_default_slot", "precondition not satisfied"),
    "storage": ("check_default_adapter", "cannot assign to `*storage` because it is borrowed"),
    "flag": ("check_default_adapter", "cannot assign to `*flag` because it is borrowed"),
    "duplicate": ("negative_default_duplicate", "borrow of moved value"),
    "stale": ("check_default_initialized_success", "assertion failed"),
    "unproved": ("negative_default_unproved", "precondition not satisfied"),
    "conversion": ("negative_generic_conversion", "assertion failed"),
    "varint_stale": ("check_varint_initialized_nonzero", "assertion failed"),
}

@pytest.mark.parametrize("case,expected", CASES.items())
def test_default_adapter_rejects_invalid_ownership(case, expected, tmp_path):
    assert os.environ.get("VERUS"), "select the reviewed native-default candidate"
    function, diagnostic = expected
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "table::decode",
        "--verify-function", function, "--cfg",
        "verus_varint_negative_stale" if case == "varint_stale"
        else f"verus_default_negative_{case}",
        "--rlimit", "50",
    ]
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    log = tmp_path / f"{case}.log"
    log.write_text(result.stdout)
    assert result.returncode != 0, f"invalid ownership verified: {log}"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,function,diagnostic", [
    ("field", "negative_vmtime_field", "field name or type does not match"),
    ("offset", "negative_vmtime_offset", "requires the actual zero offset"),
    ("entry", "negative_vmtime_entry", "compiler constants are not identical"),
    ("live", "negative_vmtime_live", "cannot assign to `*storage` because it is borrowed"),
    ("duplicate", "negative_vmtime_duplicate", "use of moved value"),
    ("stale", "check_vmtime_initialized_nonzero", "assertion failed"),
])
def test_vmtime_transport_rejects_invalid_refinement(case, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the checked VmTime candidate"
    result = subprocess.run([
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmcore", "--", "--verify-only-module", "vmtime",
        "--verify-function", function, "--cfg", f"verus_vmtime_negative_{case}",
        "--rlimit", "50",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    log = tmp_path / f"vmtime-{case}.log"
    log.write_text(result.stdout)
    assert result.returncode != 0, f"invalid refinement verified: {log}"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,diagnostic", [
    ("target", "compiler constants are not identical"),
    ("provenance", "dangling pointer (it has no provenance)"),
    ("tag", "accessing memory based on pointer with alignment 1"),
])
def test_default_metadata_rejects_invalid_target_or_pointer(case, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the reviewed native-vmtime candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "table::decode",
        "--verify-function", f"negative_default_metadata_{case}",
        "--cfg", f"verus_dispatch_negative_{case}", "--rlimit", "50",
    ]
    (tmp_path / f"metadata-{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    log = tmp_path / f"metadata-{case}.log"
    log.write_text(result.stdout)
    (tmp_path / f"metadata-{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, f"invalid metadata verified: {log}"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,diagnostic", [
    ("entry", "compiler constants are not identical"),
    ("type", "constant dispatch adapter argument or result types differ"),
    ("flag", "assertion failed"),
    ("live", "cannot assign to `*storage` because it is borrowed"),
    ("duplicate", "borrow of moved value"),
    ("stale", "assertion failed"),
])
def test_erased_dispatch_rejects_invalid_connection(case, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-dispatch candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "table::decode",
        "--verify-function", "negative_erased_wrong_type" if case == "type" else "default_u64_erased_storage",
        "--cfg", f"verus_erased_negative_{case}", "--rlimit", "50",
    ]
    (tmp_path / f"erased-{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    log = tmp_path / f"erased-{case}.log"
    log.write_text(result.stdout)
    (tmp_path / f"erased-{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, f"invalid erased dispatch verified: {log}"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

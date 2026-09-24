# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Negative controls through the production MessageReader and FieldIterator."""

import json
import os
from pathlib import Path
import subprocess
import sys

import pytest


ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("case,function,diagnostic", [
    ("nonempty", "negative_nonempty_reader", "assertion failed"),
    ("missing_premise", "negative_unjustified_empty_reader", "precondition not satisfied"),
])
def test_empty_iteration_requires_actual_empty_bytes(case, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-reader candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "mesh_protobuf", "--", "--verify-only-module", "protobuf",
        "--verify-function", function, "--cfg", f"verus_reader_negative_{case}",
        "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "unjustified empty-reader premise verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case", ["number", "offset", "decoder"])
def test_generated_saved_state_metadata_is_not_substitutable(case, tmp_path):
    assert os.environ.get("VERUS"), "select the native-reader candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmcore", "--", "--verify-only-module", "vmtime",
        "--verify-function", "check_saved_state_reader_metadata",
        "--cfg", f"verus_reader_negative_{case}", "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "incorrect generated metadata verified"
    assert "compiler constants are not identical" in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("preservation_zero", "vmcore", "vmtime", "check_reader_preserves_nonzero", "assertion failed"),
    ("field_loan", "mesh_protobuf", "table::decode", "negative_reader_field_loan", "precondition not satisfied"),
    ("wrong_decoder", "mesh_protobuf", "table::decode", "negative_reader_wrong_decoder", "precondition not satisfied"),
    ("stale", "vmcore", "vmtime", "check_reader_defaults_retained_nonzero", "assertion failed"),
    ("uninit", "mesh_protobuf", "table::decode", "negative_reader_uninitialized_true_flag", "precondition not satisfied"),
    ("whole_uninit", "vmcore", "vmtime", "negative_reader_whole_uninitialized", "precondition not satisfied"),
    ("whole_owner", "vmcore", "vmtime", "negative_reader_whole_owner", "same storage as the pointer operation"),
    ("whole_live", "vmcore", "vmtime", "negative_reader_whole_live", "cannot assign"),
    ("metadata_pointer", "mesh_protobuf", "table::decode", "negative_reader_metadata_pointer", "precondition not satisfied"),
    ("metadata_extent", "mesh_protobuf", "table::decode", "negative_reader_metadata_extent", "precondition not satisfied"),
])
def test_production_reader_ownership_and_results(case, package, module, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-whole-saved candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", f"verus_reader_negative_{case}",
        "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "invalid production-reader claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,function,diagnostic", [
    ("stale", "check_message_retained_nonzero", "assertion failed"),
    ("default", "check_message_initialized_nonzero", "assertion failed"),
    ("second_take", "check_message_uninitialized", "assertion failed"),
    ("owner", "check_message_invalid_owner", "precondition not satisfied"),
    ("reader", "check_message_requires_empty", "precondition not satisfied"),
])
def test_typed_production_message(case, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-typed-message candidate"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", "vmcore", "--", "--verify-only-module", "vmtime",
        "--verify-function", function, "--cfg", f"verus_message_negative_{case}",
        "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "invalid typed production-decoder claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("time", "vmcore", "vmtime", "check_public_message_wrong_time", "assertion failed"),
    ("reader", "vmcore", "vmtime", "check_public_message_requires_empty", "precondition not satisfied"),
    ("source", "vmcore", "vmtime", "check_public_message_wrong_source", "same source-generated trait implementation"),
    ("entry", "mesh_protobuf", "table::decode", "_VERUS_VERIFIED_read_fields_inner", "compiler constants are not identical"),
])
def test_noresources_production_message(case, package, module, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-public-decode candidate"
    cfg = f"verus_pair_negative_{case}" if case == "entry" else f"verus_public_message_negative_{case}"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", cfg, "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "invalid NoResources decoder claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout


@pytest.mark.parametrize("case,package,module,function,diagnostic", [
    ("time", "vmcore", "vmtime", "check_public_decode_wrong_time", "assertion failed"),
    ("input", "vmcore", "vmtime", "check_public_decode_requires_empty", "precondition not satisfied"),
    ("nonempty", "vmcore", "vmtime", "check_public_decode_nonempty", "precondition not satisfied"),
    ("source", "vmcore", "vmtime", "check_public_decode_wrong_source", "same source-generated trait implementation"),
    ("state", "mesh_protobuf", "protobuf", "check_reader_requires_fresh_state", "immediately preceding fresh state"),
    ("intervening", "mesh_protobuf", "protobuf", "check_reader_rejects_intervening_use", "immediately preceding fresh state"),
    ("resources", "mesh_protobuf", "protobuf", "check_state_requires_empty_resources", "assertion failed"),
])
def test_public_bytes_decoder(case, package, module, function, diagnostic, tmp_path):
    assert os.environ.get("VERUS"), "select the native-public-constructors candidate"
    cfg = f"verus_constructor_negative_{case}" if package == "mesh_protobuf" else f"verus_public_decode_negative_{case}"
    command = [
        sys.executable, str(ROOT / "verification/tools/fresh_verification.py"),
        "focus", package, "--", "--verify-only-module", module,
        "--verify-function", function, "--cfg", cfg, "--rlimit", "50",
    ]
    (tmp_path / f"{case}.command.json").write_text(json.dumps(command))
    result = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, timeout=120,
    )
    (tmp_path / f"{case}.log").write_text(result.stdout)
    (tmp_path / f"{case}.status").write_text(str(result.returncode))
    assert result.returncode != 0, "invalid public-byte decoder claim verified"
    assert diagnostic in result.stdout, result.stdout
    assert "panicked" not in result.stdout, result.stdout
    assert "not supported" not in result.stdout, result.stdout

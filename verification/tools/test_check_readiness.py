# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Regression checks for the active Argus plugin, without mutating frozen files."""

import json
import os
import shutil
import subprocess
import sys
from argparse import Namespace
from pathlib import Path
from types import SimpleNamespace

import fresh_verification
import pytest
from argus_verus.tools.checks import boundary, exec_drift, make_verify, spec_drift
from argus_verus.tools.source import symbols

ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("arch", ["X86Topology", "Aarch64Topology"])
def test_each_production_topology_body_is_checked(tmp_path, arch):
    path = ROOT / "openvmm/openvmm_core/src/worker/dispatch.rs"
    original = path.read_text()
    start = original.index(
        f"impl ExtractTopologyConfig for ProcessorTopology<{arch}>"
    )
    offset = original.index("proc_count: self.vp_count()", start)
    changed = (
        original[:offset]
        + original[offset:].replace("proc_count: self.vp_count()", "proc_count: 0", 1)
    )
    candidate = tmp_path / "dispatch.rs"
    candidate.write_text(changed)
    report = exec_drift.compare_modules(str(path), str(candidate))
    assert report["summary"]["consistent"] is False
    assert report["summary"]["functions"]["mismatched"] == 1
    mismatches = [row for row in report["functions"] if row["status"] == "MISMATCH"]
    assert len(mismatches) == 1
    assert f"ProcessorTopology<{arch}>::to_config" in str(mismatches[0])


def _scope():
    return {
        "frozen_branch": "accepted",
        "top_spec": ["left/api.spec.rs"],
        "goal": [{"file": "left/lib.rs", "symbol": "top"}],
        "src_roots": ["left", "right"],
    }


@pytest.mark.parametrize(
    "change", ["none", "top", "top_file", "tcb", "dependency", "declaration", "scope"]
)
def test_scoped_spec_discovery_preserves_frozen_contracts(tmp_path, monkeypatch, change):
    monkeypatch.chdir(tmp_path)
    baseline = {
        "left/lib.rs": "verus! { fn top() ensures valid(), {} }",
        "left/api.spec.rs": "verus! { pub open spec fn valid() -> bool { true } }",
        "right/lib.rs": """verus! {
            #[verifier::external_body]
            fn leaf() ensures meaning(), {}
        }""",
        "right/helper.proof.rs": (
            "verus! { pub closed spec fn meaning() -> bool { true } }"
        ),
        "archive/candidate.proof.rs": (
            "verus! { pub closed spec fn meaning() -> bool { false } }"
        ),
        ".verus_agent/scope_manifest.json": json.dumps(_scope()),
        ".verus_agent/tcb_manifest.json": json.dumps({
            "sanctioned": [{"kind": "symbol", "value": "leaf", "marker": "external_body"}],
        }),
    }
    for name, contents in baseline.items():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents)
    # An unrelated archive with the same symbol must not enter either snapshot.
    (tmp_path / "archive/candidate.proof.rs").write_text(
        baseline["archive/candidate.proof.rs"].replace("false", "true")
    )
    mutations = {
        "top": ("left/lib.rs", "valid()", "false"),
        "top_file": ("left/api.spec.rs", "true", "false"),
        "tcb": ("right/lib.rs", "meaning()", "false"),
        "dependency": ("right/helper.proof.rs", "true", "false"),
        "declaration": (".verus_agent/tcb_manifest.json", '"leaf"', '"top"'),
        "scope": (".verus_agent/scope_manifest.json", '"left", "right"', '"left"'),
    }
    if change != "none":
        name, old, new = mutations[change]
        (tmp_path / name).write_text(baseline[name].replace(old, new))
    monkeypatch.setattr(
        spec_drift, "_setup_drift",
        lambda _args: (
            tmp_path, spec_drift.FrozenBranch("accepted", "refs/heads/accepted", "baseline"),
            ["left/lib.rs", "right/lib.rs"], tmp_path / ".verus_agent",
        ),
    )
    monkeypatch.setattr(spec_drift, "_git_show_file", lambda _ref, path: baseline.get(path))
    monkeypatch.setattr(
        spec_drift, "_discover_spec_proof_files_at_ref",
        lambda _ref: [name for name in baseline if name.endswith((".spec.rs", ".proof.rs"))],
    )
    monkeypatch.setattr(
        spec_drift, "_frozen_file",
        lambda _ref, _root, path: baseline[path].encode() if path in baseline else None,
    )
    result = spec_drift.cmd_spec_drift(
        Namespace(crate_root=str(tmp_path), baseline_dir=".verus_agent")
    )
    assert result == (0 if change == "none" else 1)


@pytest.mark.parametrize("filename", ["scope_manifest.json", "tcb_manifest.json"])
@pytest.mark.parametrize("state", ["accepted", "dirty", "changed", "missing_baseline"])
def test_manifest_acceptance_requires_frozen_identity(tmp_path, monkeypatch, filename, state):
    directory = tmp_path / ".verus_agent"
    directory.mkdir()
    scope = json.dumps(_scope())
    (directory / "scope_manifest.json").write_text(scope)
    (directory / "tcb_manifest.json").write_text('{"sanctioned": []}')

    def git_query(_root, *args):
        if args == ("rev-parse", "--is-inside-work-tree"):
            return "true\n"
        if args[0] == "status":
            return " M manifest\n" if state == "dirty" else ""
        if args[0] == "log":
            return "[verus][argus] proposed working result\n"
        if args[0] == "diff":
            assert args == (
                "diff", "--quiet", "refs/heads/accepted", "--", str(directory / filename)
            )
            return "" if state == "accepted" else None
        raise AssertionError(args)

    monkeypatch.setattr(boundary, "_git_query", git_query)
    violation = boundary._human_manifest_violation(
        tmp_path, directory, filename, "frozen decision"
    )
    assert (violation is None) == (state == "accepted")


def test_debt_inventory_matches_current_markers():
    inventory = json.loads((ROOT / "verification/restore/UNINTERP.json").read_text())
    entries = [
        ("uninterp", entry) for entry in inventory["entries"]
    ] + [
        ("external_body", entry) for entry in inventory["temporary_external_bodies"]
    ]
    rules = boundary.load_tcb_manifest(ROOT / ".verus_agent")
    expected = set()
    paths = set()
    for marker, entry in entries:
        assert entry["deferred_fact"] and entry["consumer"] and entry["removal"]
        path = entry.get("marker_file", entry["source_evidence"][0])
        paths.add(path)
        name = entry["symbol"].removeprefix("restore_proof::")
        expected.add((marker, path, name))
    observed = set()
    for path in paths:
        for marker, _line, symbol, _content in symbols.scan_trust_leaf_details(
            (ROOT / path).read_text()
        ):
            if not boundary.leaf_sanctioned(marker, path, symbol, rules):
                observed.add((marker, path, symbol))
    assert observed == expected
    assert len(expected) == 14


@pytest.mark.parametrize(
    "marker",
    ["external_body", "uninterp", "external", "external_fn_specification",
     "axiom", "assume_termination", "externals_available_without_declaration"],
)
@pytest.mark.parametrize("command", ["check", "admission"])
def test_admission_retains_completion_and_permanent_trust_rejections(
    tmp_path, monkeypatch, marker, command
):
    monkeypatch.setattr(boundary, "scope_provenance_violation", lambda *_args: None)
    monkeypatch.setattr(boundary, "manifest_provenance_violation", lambda *_args: None)
    monkeypatch.setattr(boundary, "resolved_src_roots", lambda *_args: [tmp_path])
    monkeypatch.setattr(boundary, "scan_assumption_details", lambda *_args: [])
    monkeypatch.setattr(
        boundary, "unsanctioned_trust_leaves",
        lambda *_args: [(marker, "source.rs", "debt", 1)],
    )
    expected = 1
    if marker in {"external_body", "uninterp"}:
        expected = 0 if command == "admission" else 3
    assert boundary.cmd_check(
        Namespace(crate_root=str(tmp_path), baseline_dir=".verus_agent", command=command)
    ) == expected


@pytest.mark.parametrize(
    "output,returncode,expected",
    [
        ("verification results:: 0 verified, 0 errors\n", 0, 1),
        ("no results\n", 0, 1),
        ("verification results:: 2 verified, 1 errors\n", 0, 1),
        ("verification results:: 2 verified, 0 errors\n", 2, 2),
        (
            (
                "verification results:: 11 verified, 0 errors\n"
                "verification results:: 0 verified, 0 errors\n"
            ), 0, 0,
        ),
    ],
)
def test_make_verify_requires_nonzero_error_free_evidence(
    tmp_path, monkeypatch, output, returncode, expected
):
    def run(command, **kwargs):
        assert command == ("make", "verify", "MODULE=")
        kwargs["stdout"].write(output)
        return SimpleNamespace(returncode=returncode)

    monkeypatch.setattr(make_verify.subprocess, "run", run)
    assert make_verify.main(["--crate-root", str(tmp_path)]) == expected


def _verification_metadata(tmp_path):
    def package(name, verify):
        return {
            "id": f"path+file:///test/{name}#0.0.0", "name": name,
            "metadata": {"verus": {"verify": verify}},
        }

    packages = [
        package("root", True), package("proof_dependency", True),
        package("ordinary_dependency", False), package("unrelated", True),
    ]
    packages[2]["metadata"] = None
    ids = [package["id"] for package in packages]
    return {
        "packages": packages,
        "workspace_members": ids,
        "target_directory": str(tmp_path / "target"),
        "resolve": {"nodes": [
            {"id": ids[0], "dependencies": [ids[2]]},
            {"id": ids[1], "dependencies": []},
            {"id": ids[2], "dependencies": [ids[1]]},
            {"id": ids[3], "dependencies": []},
        ]},
    }


@pytest.mark.parametrize("command", ["verify", "focus"])
def test_fresh_verification_selects_only_scoped_verification_packages(tmp_path, command):
    metadata = _verification_metadata(tmp_path)
    selected = fresh_verification.verification_packages(metadata, "root", command)
    expected = [metadata["packages"][0]["id"]]
    if command == "verify":
        expected.append(metadata["packages"][1]["id"])
    assert selected == sorted(expected)


@pytest.mark.parametrize("package", ["missing", "ordinary_dependency"])
def test_fresh_verification_rejects_invalid_roots(tmp_path, package):
    with pytest.raises(ValueError):
        fresh_verification.verification_packages(
            _verification_metadata(tmp_path), package, "verify"
        )


@pytest.mark.parametrize("command", ["verify", "focus"])
def test_unchanged_verification_always_invalidates_selected_artifacts(
    tmp_path, monkeypatch, command
):
    from io import StringIO

    metadata = _verification_metadata(tmp_path)
    calls = []

    def run(argv, **kwargs):
        calls.append(argv)
        return SimpleNamespace(stdout=json.dumps(metadata))

    class Process:
        stdout = None

        def __enter__(self):
            self.stdout = StringIO("verification results:: 11 verified, 0 errors\n")
            return self

        def __exit__(self, *_args):
            self.stdout.close()

        def wait(self):
            return 0

    def popen(argv, **kwargs):
        calls.append(argv)
        return Process()

    monkeypatch.setattr(fresh_verification.subprocess, "run", run)
    monkeypatch.setattr(fresh_verification.subprocess, "Popen", popen)
    arguments = [
        command, "root", "--", "--num-threads", "1",
    ]
    assert fresh_verification.main(arguments) == 0
    first_calls = list(calls)
    assert fresh_verification.main(arguments) == 0
    assert calls == first_calls * 2
    assert first_calls[0] == [
        "cargo", "metadata", "--format-version=1", "--locked", "--offline",
    ]
    target = tmp_path / "target"
    if command == "focus":
        target /= "verus-partial"
    clean = ["cargo", "clean", "--profile", "dev", "--target-dir", str(target)]
    for package in fresh_verification.verification_packages(metadata, "root", command):
        clean.extend(["--package", package])
    assert first_calls[1] == clean
    assert first_calls[2] == [
        "cargo", "verus", command, "-p", "root", "--", "--num-threads", "1",
    ]


@pytest.mark.parametrize(
    "output,exit_code,allow_zero,expected",
    [
        ("Finished (cached)\n", 0, False, 1),
        ("Finished (cached)\n", 0, True, 1),
        ("verification results:: 0 verified, 0 errors\n", 0, False, 1),
        ("verification results:: 0 verified, 0 errors\n", 0, True, 0),
        ("verification results:: 11 verified, 1 errors\n", 0, False, 1),
        ("verification results:: 11 verified, 0 errors\n", 2, False, 2),
        ("verification results:: 11 verified, 0 errors\n", 0, False, 0),
    ],
)
def test_fresh_verification_requires_current_results(
    tmp_path, monkeypatch, output, exit_code, allow_zero, expected
):
    from io import StringIO

    monkeypatch.setattr(
        fresh_verification.subprocess, "run",
        lambda *_args, **_kwargs: SimpleNamespace(
            stdout=json.dumps(_verification_metadata(tmp_path))
        ),
    )

    class Process:
        def __enter__(self):
            self.stdout = StringIO(output)
            return self

        def __exit__(self, *_args):
            self.stdout.close()

        def wait(self):
            return exit_code

    monkeypatch.setattr(
        fresh_verification.subprocess, "Popen", lambda *_args, **_kwargs: Process()
    )
    arguments = ["--allow-zero"] if allow_zero else []
    assert fresh_verification.main([*arguments, "focus", "root"]) == expected


def test_failed_verification_cache_invalidation_stops_before_verus(tmp_path, monkeypatch):
    def run(argv, **kwargs):
        if argv[1] == "metadata":
            return SimpleNamespace(stdout=json.dumps(_verification_metadata(tmp_path)))
        raise fresh_verification.subprocess.CalledProcessError(2, argv)

    def popen(*_args, **_kwargs):
        pytest.fail("verification must not run after failed cache invalidation")

    monkeypatch.setattr(fresh_verification.subprocess, "run", run)
    monkeypatch.setattr(fresh_verification.subprocess, "Popen", popen)
    assert fresh_verification.main(["verify", "root"]) == 1


def _executable(path, text):
    path.write_text(text)
    path.chmod(0o755)


@pytest.fixture
def solver_project(tmp_path):
    tools = tmp_path / "verification/tools"
    tools.mkdir(parents=True)
    for name in ("find-z3.sh", "verify.sh", "install-verus.sh"):
        shutil.copy(ROOT / "verification/tools" / name, tools / name)
    source = tmp_path / "toolchain/verus-src/source"
    pins = source / "cargo-verus-toolchains/src"
    pins.mkdir(parents=True)
    (pins / "external_deps.rs").write_text('pub const Z3_VERSION: &str = "4.16.0";\n')
    _executable(source / "z3", "#!/bin/sh\necho 'Z3 version 4.16.0 - 64 bit'\n")
    unrelated = tmp_path / "bin"
    unrelated.mkdir()
    _executable(unrelated / "z3", "#!/bin/sh\necho 'Z3 version 4.12.5 - 64 bit'\n")
    (tmp_path / "verification/verus-version").write_text("test-verus\n")
    install = source / "target-verus/release"
    install.mkdir(parents=True)
    _executable(install / "verus", "#!/bin/sh\nexit 0\n")
    _executable(
        tools / "find-verus.sh",
        f"#!/bin/sh\nprintf '%s\\n' '{install / 'verus'}'\n",
    )
    _executable(tools / "install-verus-source.sh", "#!/bin/sh\nexit 0\n")
    (tools / "fresh_verification.py").write_text(
        "import os, sys\n"
        f"assert os.environ['VERUS_Z3_PATH'] == {str(source / 'z3')!r}\n"
        "print('test verifier invoked', sys.argv[1:])\n"
    )
    env = os.environ.copy()
    env.pop("VERUS_Z3_PATH", None)
    env["PATH"] = str(unrelated) + os.pathsep + env["PATH"]
    env["ARGUS_SKILL_PYTHON"] = sys.executable
    return tools, source, env


def test_default_solver_ignores_incompatible_path_binary(solver_project):
    tools, source, env = solver_project
    result = subprocess.run([str(tools / "find-z3.sh")], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 0
    assert result.stdout.strip() == str(source / "z3")


@pytest.mark.parametrize("compatible", [True, False])
def test_explicit_solver_override_must_match_pin(solver_project, compatible):
    tools, source, env = solver_project
    candidate = source / "z3" if compatible else tools.parents[1] / "bin/z3"
    env["VERUS_Z3_PATH"] = str(candidate)
    result = subprocess.run([str(tools / "find-z3.sh")], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == (0 if compatible else 1)
    if compatible:
        assert result.stdout.strip() == str(candidate)
    else:
        assert "expected Z3 4.16.0" in result.stderr and "4.12.5" in result.stderr
        assert not result.stdout


@pytest.mark.parametrize("problem", ["missing", "wrong", "malformed_pin"])
def test_default_solver_setup_errors_fail_closed(solver_project, problem):
    tools, source, env = solver_project
    if problem == "missing":
        (source / "z3").unlink()
    elif problem == "wrong":
        _executable(source / "z3", "#!/bin/sh\necho 'Z3 version 4.12.5 - 64 bit'\n")
    else:
        (source / "cargo-verus-toolchains/src/external_deps.rs").write_text("// missing pin\n")
    result = subprocess.run([str(tools / "find-z3.sh")], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 1
    assert not result.stdout and "error:" in result.stderr


@pytest.mark.parametrize("module", ["", "vmtime", "restore"])
def test_verification_entry_points_export_validated_solver(solver_project, module):
    tools, source, env = solver_project
    result = subprocess.run([str(tools / "verify.sh"), module], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 0, result.stderr
    assert f"Verification solver: {source / 'z3'}" in result.stdout
    assert "Z3 version 4.16.0" in result.stdout
    assert "test verifier invoked" in result.stdout
    assert "lifetime and trait-conflict checking enabled" in result.stdout
    assert "no-solver-version-check" not in result.stdout


def test_wrong_override_stops_verification_before_cleaning(solver_project):
    tools, _source, env = solver_project
    env["VERUS_Z3_PATH"] = str(tools.parents[1] / "bin/z3")
    result = subprocess.run([str(tools / "verify.sh"), "vmtime"], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 1
    assert "expected Z3 4.16.0" in result.stderr
    assert "test verifier invoked" not in result.stdout


@pytest.mark.parametrize("problem", ["missing", "wrong"])
def test_setup_repairs_default_solver_even_with_installed_verus(solver_project, problem):
    tools, source, env = solver_project
    if problem == "missing":
        (source / "z3").unlink()
    else:
        _executable(source / "z3", "#!/bin/sh\necho 'Z3 version 4.12.5 - 64 bit'\n")
    (source / "tools").mkdir()
    _executable(
        source / "tools/get-z3.sh",
        "#!/bin/sh\n"
        "printf '%s\\n' '#!/bin/sh' \"echo 'Z3 version 4.16.0 - 64 bit'\" > z3\n"
        "chmod +x z3\n",
    )
    result = subprocess.run([str(tools / "install-verus.sh")], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 0, result.stderr
    assert "Using Z3 version 4.16.0" in result.stdout
    assert "Verus test-verus is already installed" in result.stdout


def test_setup_does_not_replace_explicit_incompatible_solver(solver_project):
    tools, _source, env = solver_project
    override = tools.parents[1] / "bin/z3"
    before = override.read_bytes()
    env["VERUS_Z3_PATH"] = str(override)
    result = subprocess.run([str(tools / "install-verus.sh")], env=env, capture_output=True, text=True, check=False)
    assert result.returncode == 1
    assert "expected Z3 4.16.0" in result.stderr
    assert override.read_bytes() == before

#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Compile and exercise the proposed production guard only in a disposable copy."""

import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = Path(__file__).resolve().parent
WORKSPACE = EVIDENCE / "candidate-workspace"
DISPATCH = Path("openvmm/openvmm_core/src/worker/dispatch.rs")
PROTECTED = [
    DISPATCH,
    DISPATCH.with_name("dispatch.spec.rs"),
    DISPATCH.with_name("dispatch.proof.rs"),
    Path(".verus_agent/scope_manifest.json"),
    Path(".verus_agent/tcb_manifest.json"),
]


def run(command, cwd=ROOT, **kwargs):
    print("+", shlex.join(map(str, command)), "cwd=" + str(cwd), flush=True)
    started = time.monotonic()
    if cwd == WORKSPACE:
        environment = kwargs.pop("env", os.environ.copy())
        environment["GIT_CEILING_DIRECTORIES"] = str(EVIDENCE)
        kwargs["env"] = environment
    result = subprocess.run(command, cwd=cwd, check=True, **kwargs)
    print(f"exit=0 elapsed_s={time.monotonic() - started:.3f}", flush=True)
    return result


before = {path: (ROOT / path).read_bytes() for path in PROTECTED}
refs = run(
    ["git", "show-ref", "--heads"], capture_output=True
).stdout
assert not WORKSPACE.exists(), "refusing to replace an existing candidate workspace"
WORKSPACE.mkdir()
try:
    archive = run(["git", "archive", "--format=tar", "HEAD"], stdout=subprocess.PIPE).stdout
    import io

    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        files.extractall(WORKSPACE, filter="data")
    del archive
    for relative in ["toolchain/verus-src", ".packages"]:
        destination = WORKSPACE / relative
        if destination.is_dir():
            assert not any(destination.iterdir()), relative
            destination.rmdir()
        assert (ROOT / relative).is_dir(), relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.symlink_to(ROOT / relative, target_is_directory=True)
    frozen_dispatch = run(
        ["git", "show", "argus/restore-v1-frozen:" + str(DISPATCH)],
        capture_output=True,
    ).stdout
    assert frozen_dispatch == before[DISPATCH] == (WORKSPACE / DISPATCH).read_bytes()
    assert (EVIDENCE / "freeze.patch").read_bytes() == (EVIDENCE / "run.patch").read_bytes()
    print("Both branch inputs have identical dispatch.rs; the two candidate patches are identical.", flush=True)
    run(["git", "apply", "--check", EVIDENCE / "run.patch"], cwd=WORKSPACE)
    run(["git", "apply", EVIDENCE / "run.patch"], cwd=WORKSPACE)
    patched = (WORKSPACE / DISPATCH).read_text()
    helper_call = "Self::validate_snapshot_restore_partition_presence(&saved_state, restore_time)?;"
    body = patched.split("    async fn restore_snapshot_state(", 1)[1]
    assert body.index(helper_call) < body.index("if let Some") < body.index("self.restore(saved_state)")
    assert body.index("self.restore(saved_state)") < body.index(".advance_tsc(")
    print("Candidate TOP calls the guard before frequency setup, restore, and VP adjustment.", flush=True)
    run(["git", "apply", "--check", EVIDENCE / "instrumentation.patch"], cwd=WORKSPACE)
    run(["git", "apply", EVIDENCE / "instrumentation.patch"], cwd=WORKSPACE)
    fixture_dir = WORKSPACE / EVIDENCE.relative_to(ROOT)
    fixture_dir.mkdir(parents=True, exist_ok=True)
    for filename in ["candidate_tests.rs", "partition_instrumentation.rs"]:
        shutil.copyfile(EVIDENCE / filename, fixture_dir / filename)
    accepted = ROOT / "research/restore-tsc-consistency/production_path.rs"
    backend, separator, _ = accepted.read_text().partition("fn exercise_restore(")
    assert separator and backend.rstrip().endswith("}")
    (fixture_dir / "accepted_backend.rs").write_text(backend)
    prior_helper = Path("research/restore-unsaved-vp-frame/partition_payload.rs")
    (WORKSPACE / prior_helper).parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / prior_helper, WORKSPACE / prior_helper)
    environment = os.environ.copy()
    environment["RUSTUP_TOOLCHAIN"] = "1.95.0"
    environment["CARGO_TARGET_DIR"] = str(EVIDENCE / "target")
    run(
        [
            "cargo", "nextest", "run", "--profile", "agent", "-p", "openvmm_core",
            "-E", "test(restore_partition_presence_probe::)",
            "--success-output", "immediate",
        ],
        cwd=WORKSPACE,
        env=environment,
    )
finally:
    shutil.rmtree(WORKSPACE)
    assert all((ROOT / path).read_bytes() == data for path, data in before.items())
    assert run(["git", "show-ref", "--heads"], capture_output=True).stdout == refs
    print("Authoritative TOP/spec/proof/manifests and branch refs unchanged; candidate copy removed.", flush=True)

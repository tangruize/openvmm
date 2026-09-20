#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check the proposed annotations on the real production path, unapplied here."""

from pathlib import Path
import os
import shlex
import shutil
import subprocess
import sys

from argus_verus.tools.operator.freeze_request import Request, _patch_results


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "research/restore-partition-presence-proof"
PROPOSAL = ROOT / "research/freeze_requests/restore-guard-native-presence"
request = Request(
    "restore-guard-native-presence", PROPOSAL,
    PROPOSAL / "freeze.patch", PROPOSAL / "run.patch", PROPOSAL / "rationale.md",
)
scope = sys.argv[1] if len(sys.argv) == 2 else "guard"
assert scope in ("guard", "integration")
with _patch_results(ROOT, ROOT / ".verus_agent", request) as results:
    project = results.run_project
    for name in ("toolchain", ".packages"):
        (project / name).symlink_to(ROOT / name, target_is_directory=True)
    env = os.environ.copy()
    env["PATH"] = str(ROOT / "toolchain/verus-src/source/target-verus/release") + ":" + env["PATH"]
    env["VERUS_Z3_PATH"] = str(ROOT / "toolchain/verus-src/source/z3")
    env["CARGO_TARGET_DIR"] = str(ROOT / "target/verus-partial")
    checks = [
        ("proposal-guard-replay", [
            "cargo", "verus", "focus", "-p", "openvmm_core", "--",
            "--verify-only-module", "worker::dispatch",
            "--verify-function", "LoadedVm::validate_snapshot_restore_partition_presence",
            "--no-lifetime", "--multiple-errors", "20",
            "--num-threads", "1", "--triggers-mode", "silent",
        ], env),
    ] if scope == "guard" else [
        ("proposal-integration", [
            sys.executable, "-m", "argus_verus.tools.checks.make_verify", "--crate-root", ".",
        ], env),
        ("proposal-rust", [
            "cargo", "check", "-p", "openvmm_core", "--locked",
        ], {**env, "RUSTUP_TOOLCHAIN": "1.95.0", "CARGO_TARGET_DIR": str(ROOT / "target")}),
    ]
    for name, command, check_env in checks:
        with (EVIDENCE / (name + ".command")).open("x") as output:
            output.write("cd " + shlex.quote(str(project)) + " && ")
            output.write(shlex.join([
                "env", f"PATH={check_env['PATH']}", f"VERUS_Z3_PATH={check_env['VERUS_Z3_PATH']}",
                f"CARGO_TARGET_DIR={check_env['CARGO_TARGET_DIR']}",
                *([f"RUSTUP_TOOLCHAIN={check_env['RUSTUP_TOOLCHAIN']}"]
                  if "RUSTUP_TOOLCHAIN" in check_env else []),
                *command,
            ]) + "\n")
        print(f"Checking unapplied run.patch: {name}", flush=True)
        result = subprocess.run(command, cwd=project, env=check_env, check=False)
        if name == "proposal-integration":
            shutil.copyfile(
                project / ".verus_agent/cache/checks/make_verify/latest.log",
                EVIDENCE / "proposal-integration-full.log",
            )
        if result.returncode:
            raise SystemExit(result.returncode)

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Reproduce the native comparison or the isolated production diagnostic."""

import argparse
import difflib
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
ACCEPTED = ROOT / "research/vmtime-reset-installation"
WORKSPACE = OUT / "workspace"


def verifier():
    return Path(subprocess.check_output(
        [str(ROOT / "verification/tools/find-verus.sh")], cwd=ROOT, text=True
    ).strip())


def environment():
    env = os.environ.copy()
    env["PATH"] = str(verifier().parent) + os.pathsep + env["PATH"]
    solver = ROOT / "toolchain/verus-src/source/z3"
    version = subprocess.check_output([str(solver), "-version"], text=True).strip()
    assert version.startswith("Z3 version 4.16.0"), version
    env["VERUS_Z3_PATH"] = str(solver)
    return env


def run(command, name, cwd, env=None):
    with (OUT / f"{name}.log").open("w") as log:
        print(" ".join(map(str, command)), file=log, flush=True)
        print("Lifetime checking enabled; rlimit=50; no new trust.", file=log, flush=True)
        if env is not None:
            print(f"VERUS_Z3_PATH={env['VERUS_Z3_PATH']}", file=log, flush=True)
        start = time.monotonic()
        result = subprocess.run(
            command, cwd=cwd, env=env, stdout=log, stderr=subprocess.STDOUT,
            timeout=110,
        )
        print(
            f"exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
            file=log,
        )
    print(f"{name}: exit={result.returncode}; full output: {OUT / (name + '.log')}")
    return result.returncode


def patch(before, after, relative, name):
    (OUT / name).write_text("".join(difflib.unified_diff(
        before.splitlines(keepends=True), after.splitlines(keepends=True),
        fromfile=f"a/{relative}", tofile=f"b/{relative}",
    )))


def native(invocation_only=False):
    verus = verifier()
    env = environment()
    codes = {}
    names = ("constructor_invocation",) if invocation_only else (
        "constructor_value", "constructor_invocation",
    )
    for name in names:
        codes[name] = run(
            [str(verus), str(OUT / f"{name}.rs"), "--crate-type", "lib",
             "--edition", "2024", "--rlimit", "50", "--num-threads", "1"],
            name, OUT, env,
        )
    negative = (OUT / "constructor_value.log").read_text()
    positive = (OUT / "constructor_invocation.log").read_text()
    if not invocation_only:
        assert codes["constructor_value"] != 0
    assert "using a datatype constructor as a function value" in negative
    assert codes["constructor_invocation"] == 0
    assert "0 errors" in positive


def production():
    if WORKSPACE.exists():
        raise RuntimeError("workspace exists; inspect retained evidence instead of rerunning")
    relative = Path("vm/vmcore/src/vmtime.rs")
    original = (ROOT / relative).read_text()
    assert original == (ACCEPTED / "workspace" / relative).read_text()
    assert (ROOT / "support/mesh/mesh_channel/src/rpc.rs").read_bytes() == (
        ACCEPTED / "workspace/support/mesh/mesh_channel/src/rpc.rs"
    ).read_bytes()
    shutil.copytree(ACCEPTED / "workspace", WORKSPACE, symlinks=True)
    for name in ("Cargo.lock", "vmm_core/Cargo.toml"):
        shutil.copy2(ROOT / name, WORKSPACE / name)
    subprocess.run(
        ["cp", "-a", "--reflink=auto", str(ACCEPTED / "target"), str(OUT / "target")],
        check=True,
    )
    anchor = "    async fn reset_to(&mut self, vmtime: VmTime) {"
    old_call = ".call(KeeperRequest::Reset, vmtime)"
    new_call = ".call(|rpc| KeeperRequest::Reset(rpc), vmtime)"
    assert original.count(anchor) == 1 and original.count(old_call) == 1
    normalized = original.replace(old_call, new_call)
    candidate = normalized.replace(anchor, "    #[verus_verify]\n" + anchor)
    patch(original, normalized, relative, "normalization.patch")
    patch(original, candidate, relative, "production.patch")
    env = environment()
    env["CARGO_TARGET_DIR"] = str(OUT / "target")
    try:
        (WORKSPACE / relative).write_text(candidate)
        code = run(
            ["cargo", "verus", "focus", "-p", "vmcore", "--",
             "--verify-only-module", "vmtime",
             "--verify-function", "VmTimeKeeper::reset_to",
             "--rlimit", "50", "--num-threads", "1"],
            "production", WORKSPACE, env,
        )
    finally:
        (WORKSPACE / relative).write_text(original)
        assert (ROOT / relative).read_text() == original
        assert (WORKSPACE / relative).read_text() == original
        print("Isolated experimental source restored; production source untouched.")
    assert code == 101, code
    validate_results()


def validate_results():
    negative = (OUT / "constructor_value.log").read_text()
    positive = (OUT / "constructor_invocation.log").read_text()
    assert "using a datatype constructor as a function value" in negative
    assert "exit=1," in negative
    assert "verification results:: 2 verified, 0 errors" in positive
    assert "exit=0," in positive
    diagnostic = (OUT / "production.log").read_text()
    assert "exit=101," in diagnostic
    assert "using a datatype constructor as a function value" not in diagnostic
    for missing in (
        "vmcore::vmtime::VmTimeKeeper",
        "vmcore::vmtime::TimeState::is_started",
        "vmcore::vmtime::TimeState",
        "mesh_channel::rpc::RpcError",
        "mesh_channel::rpc::PendingRpc",
        "vmcore::vmtime::KeeperRequest",
        "default%call` is not supported",
    ):
        assert missing in diagnostic, missing
    assert "due to 7 previous errors" in diagnostic
    assert "verification results::" not in diagnostic
    relative = Path("vm/vmcore/src/vmtime.rs")
    assert (ROOT / relative).read_bytes() == (WORKSPACE / relative).read_bytes()
    assert (ROOT / relative).read_bytes() == (ACCEPTED / "workspace" / relative).read_bytes()
    print("Recorded frontend/declaration/VC classifications match the complete logs.")
    print("Production and restored isolated reset_to match the accepted input.")
    print("No unchanged verifier run repeated.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "experiment", choices=("native", "invocation", "production", "validate")
    )
    args = parser.parse_args()
    {
        "native": native,
        "invocation": lambda: native(invocation_only=True),
        "production": production,
        "validate": validate_results,
    }[args.experiment]()

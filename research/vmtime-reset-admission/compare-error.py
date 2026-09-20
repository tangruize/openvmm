# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Compare Error admission on an isolated checkout of the current run tip."""

import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent


def run(command, cwd, env, log):
    start = time.monotonic()
    with log.open("x") as output:
        print(" ".join(map(str, command)), file=output, flush=True)
        result = subprocess.run(
            command, cwd=cwd, env=env, stdout=output,
            stderr=subprocess.STDOUT, timeout=110,
        )
        print(f"exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
              file=output)
    print(f"{log.name}: exit={result.returncode}", flush=True)
    return result.returncode


def main():
    workspace = OUT / "error-workspace"
    workspace.mkdir()
    entries = subprocess.check_output(
        ["git", "ls-tree", "--name-only", "HEAD"], cwd=ROOT, text=True,
    ).splitlines()
    entries.remove("research")
    archive = subprocess.Popen(
        ["git", "archive", "HEAD", *entries], cwd=ROOT, stdout=subprocess.PIPE,
    )
    unpack = subprocess.run(["tar", "-x", "-C", workspace], stdin=archive.stdout)
    archive.stdout.close()
    if archive.wait() != 0 or unpack.returncode != 0:
        raise RuntimeError("Failed to extract the current run tip")
    (workspace / "toolchain").symlink_to(ROOT / "toolchain", target_is_directory=True)
    source = workspace / "support/mesh/mesh_channel/src/rpc.rs"
    original = source.read_bytes()
    subprocess.run(
        ["patch", "--batch", "-p1", "-i", OUT / "rpc-transparent-error.patch"],
        cwd=workspace, check=True, stdout=subprocess.DEVNULL,
    )
    verus = subprocess.check_output(
        [ROOT / "verification/tools/find-verus.sh"], cwd=ROOT, text=True,
    ).strip()
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.95.0")
    env["PATH"] = str(Path(verus).parent) + os.pathsep + env["PATH"]
    env["VERUS_Z3_PATH"] = str(ROOT / "toolchain/verus-src/source/z3")
    env["CARGO_TARGET_DIR"] = str(OUT / "target")
    command = [
        "cargo", "verus", "focus", "--offline", "--locked", "-p", "mesh_channel",
        "--", "--verify-only-module", "rpc", "--verify-function", "*source*",
        "--rlimit", "50", "--num-threads", "1", "--multiple-errors", "8",
        "--log", "vir",
    ]
    baseline = run(
        command + ["--log-dir", str(OUT / "error-baseline-vir")],
        workspace, env, OUT / "error-production-baseline.log",
    )
    with source.open("a") as output:
        output.write('\n#[cfg(verus_keep_ghost)]\ninclude!("rpc.spec.rs");\n')
    (source.parent / "rpc.spec.rs").write_bytes((OUT / "error-trait.rs").read_bytes())
    candidate = run(
        command + ["--log-dir", str(OUT / "error-candidate-vir")],
        workspace, env, OUT / "error-production-candidate.log",
    )
    assert baseline != 0 and candidate != 0
    assert (ROOT / "support/mesh/mesh_channel/src/rpc.rs").read_bytes() == original
    print("Active RpcError source unchanged; isolated real derived implementation retained.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

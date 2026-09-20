# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Record the four authoritative production checks, once, without hiding failures."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

root = Path.cwd()
out = root / "research/vmtime-duration-interface-candidate"
env = dict(os.environ)
env.update(
    RUSTUP_TOOLCHAIN="1.95.0",
    RUSTC_BOOTSTRAP="1",
    CARGO_UNSTABLE_LOCKFILE_PATH="true",
    CARGO_RESOLVER_LOCKFILE_PATH=str(
        root / "research/vmtime-scalar-interface/workspace/Cargo.lock"
    ),
)
codes = []
for check in ("make_verify", "boundary", "spec_drift", "exec_drift"):
    command = [
        sys.executable, "-m", f"argus_verus.tools.checks.{check}", "--crate-root", "."
    ]
    if check != "make_verify":
        command += ["--baseline-dir", ".verus_agent"]
    if check == "boundary":
        command.append("check")
    start = time.monotonic()
    with (out / f"{check}.log").open("w") as output:
        result = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
    elapsed = time.monotonic() - start
    shutil.copyfile(
        root / f".verus_agent/cache/checks/{check}/latest.log",
        out / f"{check}.complete.log",
    )
    summary = f"{check}: exit={result.returncode}, elapsed_s={elapsed:.3f}"
    (out / f"{check}.result").write_text(summary + "\n")
    print(summary, flush=True)
    print((out / f"{check}.log").read_text(), flush=True)
    codes.append(result.returncode)
sys.exit(max(codes))

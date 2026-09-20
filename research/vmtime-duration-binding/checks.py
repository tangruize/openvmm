# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run one required check, or extend executable comparison to vmcore."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from argus_verus.tools.checks.exec_drift import compare_modules


root = Path.cwd()
out = root / "research/vmtime-duration-binding"
check = sys.argv[1]
if check == "vmcore":
    scope = json.loads((root / ".verus_agent/scope_manifest.json").read_text())
    source = "vm/vmcore/src/vmtime.rs"
    baseline = out / "frozen-vmtime.rs"
    baseline.write_bytes(
        subprocess.check_output(["git", "show", f"{scope['frozen_branch']}:{source}"])
    )
    try:
        report = compare_modules(str(baseline), str(root / source))
    finally:
        baseline.unlink()
    result = {
        "source": source,
        "baseline": scope["frozen_branch"],
        "summary": report["summary"],
        "nonmatching": [
            {"name": entry["name"], "status": entry["status"]}
            for entry in report["functions"] + report["structs"]
            if entry["status"] != "MATCH"
        ],
    }
    (out / "vmtime-exec-drift.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    sys.exit(1 if result["nonmatching"] or not report["summary"]["consistent"] else 0)

if check not in ("make_verify", "boundary", "spec_drift", "exec_drift"):
    sys.exit(f"Unknown check: {check}")
env = dict(os.environ)
env.update(
    RUSTUP_TOOLCHAIN="1.95.0",
    RUSTC_BOOTSTRAP="1",
    CARGO_UNSTABLE_LOCKFILE_PATH="true",
    CARGO_RESOLVER_LOCKFILE_PATH=str(
        root / "research/vmtime-scalar-interface/workspace/Cargo.lock"
    ),
)
command = [
    sys.executable, "-m", f"argus_verus.tools.checks.{check}", "--crate-root", "."
]
if check != "make_verify":
    command.extend(["--baseline-dir", ".verus_agent"])
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
print(summary)
print((out / f"{check}.log").read_text())
sys.exit(result.returncode)

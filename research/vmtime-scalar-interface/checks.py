# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run the frozen-root checks and compare the production VmTime executable."""

import json
import os
from pathlib import Path
import subprocess
import sys
import time

from argus_verus.tools.checks.exec_drift import compare_modules


root = Path.cwd()
evidence = root / "research/vmtime-scalar-interface"
env = dict(os.environ)
env.update(
    RUSTUP_TOOLCHAIN="1.95.0",
    RUSTC_BOOTSTRAP="1",
    CARGO_UNSTABLE_LOCKFILE_PATH="true",
    CARGO_RESOLVER_LOCKFILE_PATH=str(evidence / "workspace/Cargo.lock"),
)

for check in ("make_verify", "boundary", "spec_drift", "exec_drift"):
    command = [
        sys.executable,
        "-m",
        f"argus_verus.tools.checks.{check}",
        "--crate-root",
        ".",
    ]
    if check != "make_verify":
        command.extend(["--baseline-dir", ".verus_agent"])
    if check == "boundary":
        command.append("check")
    start = time.monotonic()
    with (evidence / f"{check}.log").open("w") as output:
        result = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
    print(
        f"{check}: exit={result.returncode}, elapsed={time.monotonic() - start:.3f}s",
        flush=True,
    )
    if check != "boundary" and result.returncode:
        sys.exit(result.returncode)

scope = json.loads((root / ".verus_agent/scope_manifest.json").read_text())
source = "vm/vmcore/src/vmtime.rs"
baseline = evidence / "frozen-vmtime.rs"
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
(evidence / "vmtime-exec-drift.json").write_text(json.dumps(result, indent=2) + "\n")
print("VmTime executable:", json.dumps(result["summary"]), flush=True)
if not report["summary"]["consistent"] or report["summary"]["structs"]["mismatched"]:
    sys.exit(1)

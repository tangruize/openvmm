# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run required checks and the affected module's executable comparison once."""

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import time

from argus_verus.tools.checks.exec_drift import compare_modules


root = Path.cwd()
evidence = root / "research/vmtime-stopped-state-proof"
env = dict(os.environ)
env.update(
    RUSTUP_TOOLCHAIN="1.95.0",
    RUSTC_BOOTSTRAP="1",
    CARGO_UNSTABLE_LOCKFILE_PATH="true",
    CARGO_RESOLVER_LOCKFILE_PATH=str(evidence / "workspace/Cargo.lock"),
)
results = {}
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
    (evidence / f"{check}.command").write_text(shlex.join(command) + "\n")
    start = time.monotonic()
    with (evidence / f"{check}.log").open("w") as output:
        result = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
    results[check] = {
        "exit": result.returncode,
        "seconds": round(time.monotonic() - start, 3),
    }
    print(check, results[check], flush=True)
    complete_log = root / ".verus_agent/cache/checks" / check / "latest.log"
    if complete_log.is_file():
        shutil.copyfile(complete_log, evidence / f"{check}.complete.log")

scope = json.loads((root / ".verus_agent/scope_manifest.json").read_text())
source = "vm/vmcore/src/vmtime.rs"
command = ["git", "show", f"{scope['frozen_branch']}:{source}"]
(evidence / "frozen-source.command").write_text(shlex.join(command) + "\n")
baseline = evidence / "frozen-vmtime.rs"
baseline.write_bytes(subprocess.check_output(command))
try:
    report = compare_modules(str(baseline), str(root / source))
finally:
    baseline.unlink()

comparison = {
    "source": source,
    "baseline": scope["frozen_branch"],
    "summary": report["summary"],
    "nonmatching": [
        {"name": entry["name"], "status": entry["status"]}
        for entry in report["functions"] + report["structs"]
        if entry["status"] != "MATCH"
    ],
}
(evidence / "vmtime-exec-drift.json").write_text(
    json.dumps(comparison, indent=2) + "\n"
)
(evidence / "checks.json").write_text(json.dumps(results, indent=2) + "\n")
print("vmtime executable:", json.dumps(comparison["summary"]), flush=True)
sys.exit(
    int(
        any(results[check]["exit"] for check in results if check != "boundary")
        or not report["summary"]["consistent"]
        or bool(report["summary"]["structs"]["mismatched"])
    )
)

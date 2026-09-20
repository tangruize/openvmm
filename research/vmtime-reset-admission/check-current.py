# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run the four handoff checks once, including executable comparisons outside scope."""

import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time

from argus_verus.tools.checks.exec_drift import compare_modules

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent


def main():
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.95.0", CARGO_NET_OFFLINE="true")
    results = {}
    for check in ("make_verify", "boundary", "spec_drift", "exec_drift"):
        command = [
            sys.executable, "-m", f"argus_verus.tools.checks.{check}",
            "--crate-root", ".",
        ]
        if check != "make_verify":
            command.extend(["--baseline-dir", ".verus_agent"])
        if check == "boundary":
            command.append("check")
        start = time.monotonic()
        with (OUT / f"current-{check}.log").open("x") as log:
            print(shlex.join(command), file=log, flush=True)
            result = subprocess.run(
                command, cwd=ROOT, env=env, stdout=log,
                stderr=subprocess.STDOUT, timeout=110,
            )
        results[check] = {
            "exit": result.returncode,
            "seconds": round(time.monotonic() - start, 3),
        }
        complete = ROOT / ".verus_agent/cache/checks" / check / "latest.log"
        with (OUT / f"current-{check}.complete.log").open("xb") as log:
            log.write(complete.read_bytes())
        print(check, results[check], flush=True)

    scope = json.loads((ROOT / ".verus_agent/scope_manifest.json").read_text())
    comparisons = {}
    for source in ("vm/vmcore/src/vmtime.rs", "support/mesh/mesh_channel/src/rpc.rs"):
        baseline = OUT / f"current-frozen-{Path(source).name}"
        with baseline.open("xb") as saved:
            saved.write(subprocess.check_output(
                ["git", "show", f"{scope['frozen_branch']}:{source}"], cwd=ROOT
            ))
        try:
            report = compare_modules(str(baseline), str(ROOT / source))
        finally:
            baseline.unlink()
        comparisons[source] = {
            "summary": report["summary"],
            "nonmatching": [
                {"name": entry["name"], "status": entry["status"]}
                for entry in report["functions"] + report["structs"]
                if entry["status"] != "MATCH"
            ],
        }
        print(source, report["summary"], flush=True)
    with (OUT / "current-checks.json").open("x") as output:
        json.dump({"checks": results, "executables": comparisons}, output, indent=2)
        output.write("\n")
    return int(
        any(result["exit"] for name, result in results.items() if name != "boundary")
        or any(comparison["nonmatching"] for comparison in comparisons.values())
    )


if __name__ == "__main__":
    raise SystemExit(main())

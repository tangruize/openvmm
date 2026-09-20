# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run the four required checks once against the live repository."""

import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
PYTHON = os.environ.get("ARGUS_SKILL_PYTHON", "python3")

checks = (
    ("make_verify", ["--crate-root", "."]),
    ("boundary", ["--crate-root", ".", "--baseline-dir", ".verus_agent", "check"]),
    ("spec_drift", ["--crate-root", ".", "--baseline-dir", ".verus_agent"]),
    ("exec_drift", ["--crate-root", ".", "--baseline-dir", ".verus_agent"]),
)
for name, arguments in checks:
    start = time.monotonic()
    with (OUT / f"{name}.log").open("w") as log:
        result = subprocess.run(
            [PYTHON, "-m", f"argus_verus.tools.checks.{name}", *arguments],
            cwd=ROOT,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
    print(
        f"{name}: exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
        flush=True,
    )

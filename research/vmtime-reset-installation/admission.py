# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Annotation-only native admission probe; always restore the isolated source."""

import difflib
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
WORKSPACE = OUT / "workspace"
relative = Path("vm/vmcore/src/vmtime.rs")
source = WORKSPACE / relative
original = source.read_text()
assert source.read_bytes() == (ROOT / relative).read_bytes()
anchor = "    async fn reset_to(&mut self, vmtime: VmTime) {"
assert original.count(anchor) == 1
annotated = original.replace(anchor, "    #[verus_verify]\n" + anchor)
patch = "".join(
    difflib.unified_diff(
        original.splitlines(keepends=True),
        annotated.splitlines(keepends=True),
        fromfile=f"a/{relative}",
        tofile=f"b/{relative}",
    )
)
(OUT / "admission.patch").write_text(patch)
verus = subprocess.check_output(
    [str(ROOT / "verification/tools/find-verus.sh")], cwd=ROOT, text=True
).strip()
env = os.environ.copy()
env["PATH"] = str(Path(verus).parent) + os.pathsep + env["PATH"]
env["CARGO_TARGET_DIR"] = str(OUT / "target")
command = [
    "cargo", "verus", "focus", "-p", "vmcore", "--",
    "--verify-only-module", "vmtime",
    "--verify-function", "VmTimeKeeper::reset_to",
    "--rlimit", "50", "--num-threads", "1",
]
try:
    source.write_text(annotated)
    start = time.monotonic()
    with (OUT / "admission.log").open("w") as log:
        print(" ".join(command), file=log, flush=True)
        print("Lifetime checking enabled; no new trust or scaffold.", file=log, flush=True)
        result = subprocess.run(
            command, cwd=WORKSPACE, env=env, stdout=log, stderr=subprocess.STDOUT
        )
        print(
            f"exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
            file=log,
        )
finally:
    source.write_text(original)
    assert source.read_bytes() == (ROOT / relative).read_bytes()
    print("Experimental annotation removed; isolated and live production source match.")
print(f"Native verifier exit={result.returncode}")
raise SystemExit(result.returncode)

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Apply one recorded annotation overlay, measure native admission, and undo it."""

import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
SOURCES = (
    ROOT / "vm/vmcore/src/vmtime.rs",
    ROOT / "support/mesh/mesh_channel/src/rpc.rs",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", choices=("keeper", "rpc", "transparent-error"))
    parser.add_argument("--log", required=True, type=Path)
    args = parser.parse_args()
    log = args.log.resolve()
    if not log.is_relative_to(OUT) or log.exists():
        parser.error("--log must be a new file inside research/vmtime-reset-admission")
    if not (ROOT / ".git").exists():
        parser.error("this reproducer requires the existing Git worktree")
    patches = [OUT / "keeper-declarations.patch"]
    if args.candidate != "keeper":
        patches.append(
            OUT / (
                "rpc-interface.patch" if args.candidate == "rpc"
                else "rpc-transparent-error.patch"
            )
        )
    original = {source: source.read_bytes() for source in SOURCES}
    subprocess.run(["git", "apply", "--check", *patches], cwd=ROOT, check=True)
    subprocess.run(["git", "apply", *patches], cwd=ROOT, check=True)
    candidate = {source: source.read_bytes() for source in SOURCES}
    try:
        result = subprocess.run(
            [
                sys.executable, OUT / "measure.py", log,
                "bash", OUT / "current-native.command",
            ],
            cwd=ROOT,
        )
    finally:
        for source in SOURCES:
            if source.read_bytes() != candidate[source]:
                raise RuntimeError(
                    f"Concurrent edit to {source}; refusing to overwrite it. "
                    "The experiment overlay has not been reversed."
                )
        subprocess.run(["git", "apply", "--reverse", *patches], cwd=ROOT, check=True)
        for source in SOURCES:
            if source.read_bytes() != original[source]:
                raise RuntimeError(f"Source restoration mismatch: {source}")
        print("Restored both production inputs byte-for-byte.", flush=True)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())

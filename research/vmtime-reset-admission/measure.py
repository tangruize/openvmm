# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run a bounded native experiment and retain its complete diagnostic."""

import argparse
from pathlib import Path
import shlex
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("a command is required")
    start = time.monotonic()
    with args.log.open("x") as log:
        print(shlex.join(args.command), file=log, flush=True)
        try:
            result = subprocess.run(
                args.command, stdout=log, stderr=subprocess.STDOUT, timeout=110
            )
        except subprocess.TimeoutExpired:
            print("INCOMPLETE: exceeded 110 seconds", file=log, flush=True)
            raise
        print(
            f"exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
            file=log,
        )
    print(f"{args.log}: exit={result.returncode}")
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run Verus with current evidence even when Cargo considers artifacts fresh."""

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

RESULT = re.compile(r"verification results::?\s*(\d+)\s+verified,\s*(\d+)\s+errors?")


def verification_packages(metadata: dict, package: str, command: str) -> list[str]:
    packages = {item["id"]: item for item in metadata["packages"]}
    roots = [
        key for key in metadata["workspace_members"]
        if packages[key]["name"] == package
    ]
    if len(roots) != 1:
        raise ValueError(f"expected one workspace package named {package!r}")
    if not (packages[roots[0]]["metadata"] or {}).get("verus", {}).get("verify", False):
        raise ValueError(f"{package} is not enabled for Verus")
    selected = set(roots)
    if command == "verify":
        nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
        pending = list(roots)
        while pending:
            for dependency in nodes[pending.pop()]["dependencies"]:
                if dependency not in selected:
                    selected.add(dependency)
                    pending.append(dependency)
    return sorted(
        key for key in selected
        if (packages[key]["metadata"] or {}).get("verus", {}).get("verify", False)
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-zero", action="store_true")
    parser.add_argument("command", choices=("verify", "focus"))
    parser.add_argument("package")
    parser.add_argument("verus_args", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)

    started = time.monotonic()
    try:
        metadata = json.loads(subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--locked", "--offline"],
            check=True, stdout=subprocess.PIPE, text=True,
        ).stdout)
        selected = verification_packages(metadata, args.package, args.command)
        target = Path(metadata["target_directory"])
        if args.command == "focus":
            target /= "verus-partial"
        clean = ["cargo", "clean", "--profile", "dev", "--target-dir", str(target)]
        for package in selected:
            clean.extend(["--package", package])
        print(f"Fresh verification: clearing {len(selected)} selected package(s) in {target}", flush=True)
        subprocess.run(clean, check=True)
        prepared = time.monotonic()

        flags = args.verus_args
        if flags[:1] == ["--"]:
            flags = flags[1:]
        command = ["cargo", "verus", args.command, "-p", args.package, "--", *flags]
        verified = errors = reports = 0
        with subprocess.Popen(
            command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        ) as process:
            assert process.stdout is not None
            for line in process.stdout:
                print(line, end="", flush=True)
                result = RESULT.search(line)
                if result:
                    reports += 1
                    verified += int(result[1])
                    errors += int(result[2])
            exit_code = process.wait()
        elapsed = time.monotonic() - prepared
        if exit_code == 0 and (
            not reports or errors or (not verified and not args.allow_zero)
        ):
            print("error: expected fresh nonzero verification with zero errors", file=sys.stderr)
            exit_code = 1
        print(
            f"Fresh verification summary: {verified} verified, {errors} errors; "
            f"exit {exit_code}; preparation {prepared - started:.3f}s; "
            f"verification {elapsed:.3f}s; total {time.monotonic() - started:.3f}s"
        )
        return exit_code
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"error: fresh verification failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

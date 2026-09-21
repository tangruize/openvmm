# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check an isolated source-matched inventory overlay using existing Cargo artifacts."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def dependencies():
    base = ROOT / "target/debug/.fingerprint"
    roots = list(base.glob("state_unit-*/lib-state_unit.json"))
    if len(roots) != 1:
        raise RuntimeError(f"expected one current state_unit fingerprint, found {roots}")
    index = {}
    for path in base.glob("*/lib-*"):
        if path.suffix:
            continue
        value = int.from_bytes(bytes.fromhex(path.read_text().strip()), "little")
        key = (path.name.removeprefix("lib-"), value)
        index.setdefault(key, []).append(path.with_suffix(".json"))
    found = {}
    pending = [roots[0]]
    while pending:
        path = pending.pop()
        if path in found:
            continue
        document = json.loads(path.read_text())
        found[path] = document
        for _, name, _, value in document["deps"]:
            if name == "build_script_build":
                continue
            matches = index.get((name, value), [])
            if len(matches) != 1:
                raise RuntimeError(f"dependency {name} has {len(matches)} matching artifacts")
            pending.extend(matches)

    def artifact(path):
        name = path.stem.removeprefix("lib-")
        suffix = path.parent.name.rsplit("-", 1)[1]
        return ROOT / f"target/debug/deps/lib{name}-{suffix}"

    externs = []
    for _, name, _, value in found[roots[0]]["deps"]:
        stem = artifact(index[(name, value)][0])
        rmeta = stem.with_suffix(".rmeta")
        if not rmeta.is_file():
            raise RuntimeError(f"missing compiled dependency: {rmeta}")
        externs += ["--extern", f"{name}={rmeta}"]
    imports = []
    for path in found:
        if path == roots[0]:
            continue
        vir = artifact(path).with_suffix(".vir")
        if vir.is_file():
            name = path.stem.removeprefix("lib-")
            imports += ["--import", f"{name}={vir}"]
    return externs, imports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=("baseline", "candidate"))
    args = parser.parse_args()
    workspace = HERE / f"production-{args.case}"
    source = Path("vmm_core/state_unit/src/lib.rs")
    original = (ROOT / source).read_bytes()
    destination = workspace / source
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(original)
    annotation = ROOT / "research/destination-inventory-observation/native-admission.patch"
    subprocess.run(["patch", "--batch", "--forward", "-p1", "-i", str(annotation)],
                   cwd=workspace, check=True, stdout=subprocess.PIPE)
    if args.case == "candidate":
        subprocess.run(["patch", "--batch", "--forward", "-p1", "-i",
                        str(HERE / "candidate-integration.patch")],
                       cwd=workspace, check=True, stdout=subprocess.PIPE)
        shutil.copyfile(HERE / "candidate.spec.rs",
                        destination.parent / "mutex_observation.spec.rs")
    externs, imports = dependencies()
    verus = subprocess.check_output([ROOT / "verification/tools/find-verus.sh"],
                                    text=True).strip()
    solver = subprocess.check_output([ROOT / "verification/tools/find-z3.sh"],
                                     text=True).strip()
    command = [verus, "--internal-test-mode", "--edition=2024",
               "--crate-type=lib", "--crate-name", "state_unit",
               "--num-threads", "1", "--triggers-mode", "silent", "--multiple-errors", "20",
               "--verify-root", "--verify-function", "StateUnits::inventory",
               "-L", f"dependency={ROOT / 'target/debug/deps'}",
               *externs, *imports, str(destination)]
    env = dict(os.environ, VERUS_Z3_PATH=solver)
    started = time.monotonic()
    with (HERE / f"production-{args.case}.log").open("w") as log:
        log.write("command: " + repr(command) + "\n")
        log.flush()
        result = subprocess.run(command, cwd=workspace, env=env,
                                stdout=log, stderr=subprocess.STDOUT)
        log.write(f"\nexit={result.returncode}; elapsed={time.monotonic() - started:.3f}s\n")
    if (ROOT / source).read_bytes() != original:
        raise RuntimeError("live production source changed during isolated experiment")
    print(f"{args.case}: exit={result.returncode}; log=production-{args.case}.log")
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Generate independent boundary patches and a minimal real-RpcError comparison."""

import argparse
import difflib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
RPC = "support/mesh/mesh_channel/src/rpc.rs"
SPEC = "support/mesh/mesh_channel/src/rpc.spec.rs"
MANIFEST = "support/mesh/mesh_channel/Cargo.toml"
SCOPE = ".verus_agent/scope_manifest.json"
TCB = ".verus_agent/tcb_manifest.json"
INCLUDE = '\n#[cfg(verus_keep_ghost)]\ninclude!("rpc.spec.rs");\n'


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


def annotated(source):
    anchor = "#[derive(Debug, Error)]\npub enum RpcError"
    assert source.count(anchor) == 1
    return source.replace(
        anchor, "#[derive(Debug, Error)]\n#[vstd::prelude::verus_verify]\npub enum RpcError",
    )


def patch(before, after):
    return "".join(
        "".join(difflib.unified_diff(
            before.get(path, "").splitlines(keepends=True),
            content.splitlines(keepends=True),
            fromfile=f"a/{path}" if path in before else "/dev/null",
            tofile=f"b/{path}",
        ))
        for path, content in after.items()
        if content != before.get(path, "")
    )


def measure(command, workspace, env, output):
    start = time.monotonic()
    with output.open("x") as log:
        print(" ".join(map(str, command)), file=log, flush=True)
        result = subprocess.run(
            command, cwd=workspace, env=env, stdout=log,
            stderr=subprocess.STDOUT, timeout=110,
        )
        print(f"exit={result.returncode}, seconds={time.monotonic() - start:.3f}",
              file=log)
    print(f"{output.name}: exit={result.returncode}", flush=True)
    return result.returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(OUT):
        parser.error("--output must be inside research/vmtime-reset-admission")
    output.mkdir()
    interface = (OUT / "error-interface.rs").read_text()
    frozen = json.loads((ROOT / SCOPE).read_text())["frozen_branch"]
    inputs = {}
    for label, ref in (("freeze", frozen), ("run", "HEAD")):
        before = {
            path: git("show", f"{ref}:{path}")
            for path in (RPC, MANIFEST, "Cargo.lock", SCOPE, TCB)
        }
        inputs[ref] = before
        after = dict(before)
        after[RPC] = before[RPC] + INCLUDE
        after[SPEC] = interface
        if label == "run":
            after[RPC] = annotated(after[RPC])
        else:
            after[MANIFEST] = before[MANIFEST].replace(
                "[dependencies]\n",
                "[package.metadata.verus]\nverify = true\n\n"
                "[dependencies]\nvstd.workspace = true\n",
            )
            start = before["Cargo.lock"].index('name = "mesh_channel"\n')
            end = before["Cargo.lock"].index("[[package]]", start)
            entry = before["Cargo.lock"][start:end]
            assert '\n "vstd",\n' not in entry
            updated = entry.replace('\n "tracing",\n', '\n "tracing",\n "vstd",\n')
            assert updated != entry
            after["Cargo.lock"] = before["Cargo.lock"][:start] + updated + before["Cargo.lock"][end:]
        scope = json.loads(before[SCOPE])
        scope["src_roots"].append("support/mesh/mesh_channel/src")
        after[SCOPE] = json.dumps(scope, indent=2) + "\n"
        tcb = json.loads(before[TCB])
        tcb["sanctioned"].append({
            "kind": "symbol", "value": "ExCoreError",
            "marker": "external_trait_specification",
        })
        after[TCB] = json.dumps(tcb, indent=2) + "\n"
        (output / f"{label}.patch").write_text(patch(before, after))

    workspace = output / "workspace"
    workspace.mkdir()
    entries = git("ls-tree", "--name-only", "HEAD").splitlines()
    entries.remove("research")
    archive = subprocess.Popen(
        ["git", "archive", "HEAD", *entries], cwd=ROOT, stdout=subprocess.PIPE,
    )
    unpack = subprocess.run(["tar", "-x", "-C", workspace], stdin=archive.stdout)
    archive.stdout.close()
    if archive.wait() != 0 or unpack.returncode != 0:
        raise RuntimeError("Failed to extract current run tip")
    (workspace / "toolchain").symlink_to(ROOT / "toolchain", target_is_directory=True)
    active = {path: (ROOT / path).read_bytes() for path in inputs["HEAD"]}
    source = workspace / RPC
    baseline = annotated(inputs["HEAD"][RPC])
    source.write_text(baseline)
    verus = git("rev-parse", "--show-toplevel").strip() + "/toolchain/verus-src/source/target-verus/release"
    env = dict(os.environ, RUSTUP_TOOLCHAIN="1.95.0")
    env["PATH"] = verus + os.pathsep + env["PATH"]
    env["VERUS_Z3_PATH"] = str(ROOT / "toolchain/verus-src/source/z3")
    env["CARGO_TARGET_DIR"] = str(OUT / "target")
    command = [
        "cargo", "verus", "focus", "--offline", "--locked", "-p", "mesh_channel",
        "--", "--verify-only-module", "rpc", "--verify-function", "*source*",
        "--rlimit", "50", "--num-threads", "1", "--multiple-errors", "8",
    ]
    baseline_code = measure(command, workspace, env, output / "baseline.log")
    source.write_text(baseline + INCLUDE)
    (workspace / SPEC).write_text(interface)
    candidate_code = measure(command, workspace, env, output / "candidate.log")
    before_log = (output / "baseline.log").read_text()
    after_log = (output / "candidate.log").read_text()
    assert baseline_code == 101 and candidate_code == 101
    assert "trait core::error::Error not declared to Verus" in before_log
    assert "trait core::error::Error not declared to Verus" not in after_log
    assert "`core::error::Error::source` is not supported" not in after_log
    assert "as_dyn_error` is not supported" in after_log
    assert "mesh_channel_core::error::RecvError" in after_log
    assert "external_auto_derives" not in source.read_text()
    assert source.read_text().replace(
        "#[vstd::prelude::verus_verify]\n", "",
    ).removesuffix(INCLUDE) == inputs["HEAD"][RPC]
    for ref, files in inputs.items():
        for path, content in files.items():
            assert git("show", f"{ref}:{path}") == content, (ref, path)
    for path, content in active.items():
        assert (ROOT / path).read_bytes() == content, path
    print("Compared current frozen/run inputs; active inputs unchanged.")
    print("Real RpcError and both derives retained; only Error/source obstruction removed.")
    print("No new temporary marker, operation postcondition, or generated-body exclusion.")


if __name__ == "__main__":
    main()

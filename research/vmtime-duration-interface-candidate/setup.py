# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Materialize an isolated copy of the actual production workspace."""

import json
from pathlib import Path
import shutil
import subprocess

root = Path.cwd()
out = root / "research/vmtime-duration-interface-candidate"
workspace = out / "workspace"
workspace.mkdir(exist_ok=True)
tracked = subprocess.check_output(["git", "ls-files", "-z"]).decode().split("\0")
for name in filter(None, tracked):
    if name.startswith(("research/", ".autors/")):
        continue
    source = root / name
    destination = workspace / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source.is_symlink():
        if destination.is_symlink():
            assert destination.readlink() == source.readlink()
        else:
            destination.symlink_to(source.readlink())
    else:
        shutil.copy2(source, destination)
shutil.copy2(
    root / "vm/vmcore/src/vmtime.proof.rs",
    workspace / "vm/vmcore/src/vmtime.proof.rs",
)
shutil.copy2(
    root / "research/vmtime-scalar-interface/workspace/Cargo.lock",
    workspace / "Cargo.lock",
)
(workspace / "toolchain").mkdir(exist_ok=True)
if not (workspace / "toolchain/verus-src").exists():
    (workspace / "toolchain/verus-src").symlink_to(root / "toolchain/verus-src")
scope = json.loads((root / ".verus_agent/scope_manifest.json").read_text())
paths = [
    ".verus_agent/scope_manifest.json",
    ".verus_agent/tcb_manifest.json",
    "vm/vmcore/Cargo.toml",
    "vm/vmcore/src/vmtime.rs",
    "vm/vmcore/src/vmtime.proof.rs",
    "vm/vmcore/src/vmtime_duration/mod.rs",
    "vm/vmcore/src/vmtime_duration/observation.spec.rs",
]
for label, ref in (("frozen", scope["frozen_branch"]), ("working", "HEAD")):
    for name in paths:
        result = subprocess.run(
            ["git", "show", f"{ref}:{name}"], capture_output=True, check=False
        )
        destination = out / "bases" / label / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if result.returncode == 0:
            destination.write_bytes(result.stdout)
        elif not any(
            text in result.stderr.decode()
            for text in ("does not exist", "exists on disk, but not in")
        ):
            raise RuntimeError(result.stderr.decode())
shutil.copy2(root / "vm/vmcore/src/vmtime.rs", out / "intake-vmtime.rs")
print("Isolated workspace copied from tracked production files and current scalar proof.")
print("Selected scoped Cargo.lock copied; production files and lockfile unchanged.")
print("Frozen and working patch bases captured without branch or worktree changes.")

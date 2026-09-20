# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Add only a test target to an isolated copy of the current production workspace."""

from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
WORKSPACE = OUT / "workspace"


def prepare():
    if WORKSPACE.exists():
        raise RuntimeError("workspace already exists; preserve experiment inputs")
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z"], cwd=ROOT, text=True
    ).split("\0")
    inputs = [
        name for name in tracked
        if name and not name.startswith(("research/", ".autors/"))
    ]
    inputs.extend(["Cargo.lock", "vm/vmcore/src/vmtime.proof.rs"])
    for name in dict.fromkeys(inputs):
        source = ROOT / name
        destination = WORKSPACE / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            destination.symlink_to(source.readlink())
        else:
            shutil.copy2(source, destination)
    (WORKSPACE / "toolchain").mkdir(exist_ok=True)
    (WORKSPACE / "toolchain/verus-src").symlink_to(ROOT / "toolchain/verus-src")
    (WORKSPACE / ".packages").symlink_to(ROOT / ".packages")
    for name in inputs:
        assert (WORKSPACE / name).read_bytes() == (ROOT / name).read_bytes(), name
    manifest = WORKSPACE / "vmm_core/Cargo.toml"
    manifest.write_text(
        manifest.read_text()
        + '\n[[test]]\nname = "vmtime_saved_state_binding"\n'
        + f'path = "{OUT / "probe.rs"}"\n'
    )
    print("Current production inputs copied byte-for-byte, including dirty scalar work.")
    print("Only the isolated vmm_core manifest adds the diagnostic test target.")


if __name__ == "__main__":
    prepare()

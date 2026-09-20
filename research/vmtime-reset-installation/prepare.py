# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Create a research-only workspace, retaining the actual production bodies."""

from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
WORKSPACE = OUT / "workspace"


def prepare():
    if WORKSPACE.exists():
        raise RuntimeError("workspace already exists; do not overwrite experiment inputs")
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z"], cwd=ROOT
    ).decode().split("\0")
    for name in filter(None, tracked):
        if name.startswith(("research/", ".autors/")):
            continue
        source = ROOT / name
        destination = WORKSPACE / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            destination.symlink_to(source.readlink())
        else:
            shutil.copy2(source, destination)
    for name in ("Cargo.lock", "vm/vmcore/src/vmtime.proof.rs"):
        shutil.copy2(ROOT / name, WORKSPACE / name)
    (WORKSPACE / "toolchain").mkdir(exist_ok=True)
    (WORKSPACE / "toolchain/verus-src").symlink_to(ROOT / "toolchain/verus-src")
    (WORKSPACE / ".packages").symlink_to(ROOT / ".packages")
    manifest = WORKSPACE / "vmm_core/Cargo.toml"
    manifest.write_text(
        manifest.read_text()
        + '\n[[test]]\nname = "vmtime_reset_installation"\n'
        + f'path = "{OUT / "probe.rs"}"\n'
    )
    for name in (
        "vm/vmcore/src/vmtime.rs",
        "vm/vmcore/src/vmtime.proof.rs",
        "vm/vmcore/Cargo.toml",
        "vmm_core/src/vmtime_unit.rs",
        "vmm_core/state_unit/src/lib.rs",
    ):
        assert (WORKSPACE / name).read_bytes() == (ROOT / name).read_bytes(), name
    print("Production bodies and existing dirty scalar inputs copied byte-for-byte.")
    print("Only the isolated vmm_core manifest adds a test target; no new dependency.")


if __name__ == "__main__":
    prepare()

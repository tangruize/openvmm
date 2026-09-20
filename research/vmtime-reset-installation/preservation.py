# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check that experimental source changes have not escaped the diagnostic."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
WORKSPACE = OUT / "workspace"
paths = (
    ".verus_agent/scope_manifest.json",
    ".verus_agent/tcb_manifest.json",
    "openvmm/openvmm_core/src/worker/dispatch.rs",
    "openvmm/openvmm_core/src/worker/dispatch.spec.rs",
    "openvmm/openvmm_core/src/worker/dispatch.proof.rs",
    "vm/vmcore/Cargo.toml",
    "vm/vmcore/src/vmtime.rs",
    "vm/vmcore/src/vmtime.proof.rs",
    "vm/vmcore/src/vm_task.rs",
    "vmm_core/src/vmtime_unit.rs",
    "vmm_core/state_unit/src/lib.rs",
    "support/mesh/mesh_channel/src/rpc.rs",
    "support/mesh/mesh_channel_core/src/mpsc.rs",
    "support/mesh/mesh_channel_core/src/oneshot.rs",
)
for name in paths:
    assert (ROOT / name).read_bytes() == (WORKSPACE / name).read_bytes(), name
print(f"{len(paths)} live inputs equal the intake workspace, including dirty scalar work.")
subprocess.run(
    [
        "git", "diff", "--exit-code", "--", "Cargo.lock",
        "research/freeze_requests/restore-duration-nanoseconds-interface",
    ],
    cwd=ROOT,
    check=True,
)
print("Root lockfile and committed Duration request have no working-tree changes.")

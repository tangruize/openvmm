# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check production preservation and applicability of accepted lifecycle evidence."""

import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
WORKSPACE = OUT / "workspace"
ACCEPTED = ROOT / "research/vmtime-reset-installation/workspace"

scope = json.loads((ROOT / ".verus_agent/scope_manifest.json").read_text())
tcb = json.loads((ROOT / ".verus_agent/tcb_manifest.json").read_text())
assert scope["goal"] == [{
    "file": "openvmm/openvmm_core/src/worker/dispatch.rs",
    "symbol": "LoadedVm::restore_snapshot_state",
}]
assert tcb["sanctioned"] == [
    {"kind": "symbol", "value": "InitializedVm::load", "marker": "external_body"},
    {"kind": "symbol", "value": "ExRestoreReadyFile", "marker": "external_type_specification"},
    {"kind": "symbol", "value": "ExRestoreReadyFile", "marker": "external_body"},
]

tracked = subprocess.check_output(
    ["git", "ls-files", "-z"], cwd=ROOT, text=True
).split("\0")
inputs = [
    name for name in tracked
    if name and not name.startswith(("research/", ".autors/"))
]
inputs.extend(["Cargo.lock", "vm/vmcore/src/vmtime.proof.rs"])
for name in dict.fromkeys(inputs):
    current = (ROOT / name).read_bytes()
    isolated = (WORKSPACE / name).read_bytes()
    if name == "vmm_core/Cargo.toml":
        suffix = (
            '\n[[test]]\nname = "vmtime_saved_state_binding"\n'
            + f'path = "{OUT / "probe.rs"}"\n'
        ).encode()
        assert isolated.endswith(suffix)
        isolated = isolated[:-len(suffix)]
    if name == "Cargo.lock":
        live_lock = tomllib.loads(current.decode())
        built_lock = tomllib.loads(isolated.decode())
        live_vmcore = next(p for p in live_lock["package"] if p["name"] == "vmcore")
        built_vmcore = next(p for p in built_lock["package"] if p["name"] == "vmcore")
        if "vstd" not in live_vmcore["dependencies"]:
            assert "vstd" in built_vmcore["dependencies"]
            built_vmcore["dependencies"].remove("vstd")
        assert live_lock == built_lock, name
    else:
        assert current == isolated, name
print(
    f"Production preservation: {len(set(inputs))} inputs match; only the isolated "
    "test target and vmcore vstd lock bookkeeping are normalized."
)

accepted_inputs = [
    "Cargo.toml",
    ".verus_agent/scope_manifest.json",
    ".verus_agent/tcb_manifest.json",
    "openvmm/openvmm_core/src/worker/dispatch.rs",
    "openvmm/openvmm_core/src/worker/dispatch.spec.rs",
    "openvmm/openvmm_core/src/worker/dispatch.proof.rs",
    "openvmm/openvmm_core/src/hypervisor_backend.rs",
    "vm/vmcore/Cargo.toml",
    "vm/vmcore/src/vmtime.rs",
    "vm/vmcore/src/vmtime.proof.rs",
    "vm/vmcore/src/vm_task.rs",
    "vm/vmcore/src/save_restore.rs",
    "vmm_core/src/vmtime_unit.rs",
    "vmm_core/state_unit/src/lib.rs",
    "support/mesh/mesh_channel/src/rpc.rs",
    "support/mesh/mesh_channel_core/src/mpsc.rs",
    "support/mesh/mesh_channel_core/src/oneshot.rs",
    "support/pal/pal_async/src/task.rs",
    "support/pal/pal_async/src/io_pool.rs",
    "support/pal/pal_async/src/unix/mod.rs",
    "support/pal/pal_async/src/unix/epoll.rs",
    "support/pal/pal_async/src/unix/kqueue.rs",
    "support/pal/pal_async/src/windows/iocp.rs",
]
for name in accepted_inputs:
    assert (ROOT / name).read_bytes() == (ACCEPTED / name).read_bytes(), name
print(f"Accepted installation/lifecycle evidence: {len(accepted_inputs)} inputs unchanged.")
assert (WORKSPACE / "Cargo.lock").read_bytes() == (ACCEPTED / "Cargo.lock").read_bytes()
print("Built dependency lock matches accepted evidence; only intake vmcore vstd bookkeeping differs.")
print("Frozen manifests parse and contain exactly the expected goal and TCB entries.")

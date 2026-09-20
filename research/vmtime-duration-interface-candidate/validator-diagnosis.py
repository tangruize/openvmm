# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Explain the native rejection without rerunning or changing the validator."""

from dataclasses import asdict
import json
from pathlib import Path
import subprocess

from argus_verus.tools.checks import spec_drift

root = Path.cwd()
out = root / "research/vmtime-duration-interface-candidate"
workspace = out / "workspace"
scope = json.loads((root / ".verus_agent/scope_manifest.json").read_text())
rules = json.loads((workspace / ".verus_agent/tcb_manifest.json").read_text())["sanctioned"]
files = [
    root / "openvmm/openvmm_core/src/worker/dispatch.rs",
    root / "openvmm/openvmm_core/src/worker/dispatch.spec.rs",
    root / "openvmm/openvmm_core/src/worker/dispatch.proof.rs",
    root / "research/restore-vp-index-coverage/bitmap-intake-processor.proof.rs",
    root / "vm/vmcore/vm_topology/src/processor.proof.rs",
    workspace / "vm/vmcore/src/vmtime.proof.rs",
    workspace / "vm/vmcore/src/vmtime_duration/observation.spec.rs",
]
snapshot = spec_drift.SpecSnapshot()
for path in files:
    relative = path.relative_to(root).as_posix()
    snapshot.functions.update(
        {
            key: asdict(value)
            for key, value in spec_drift.extract_specs_from_file(
                relative, sanction_rules=rules
            ).items()
        }
    )
selected = spec_drift._filter_to_target_fns(
    snapshot, {"LoadedVm::restore_snapshot_state"}
)
print("Native conservative View reachability (not another package validation):")
for key, data in selected.functions.items():
    if data["qualified_name"] in ("VmTime::view", "VpIndex::view"):
        print(json.dumps({
            "function": key,
            "trust_protectors": data.get("trust_protectors", []),
        }))
print("Existing View source presence at the branch tips:")
for path in (
    "research/restore-vp-index-coverage/bitmap-intake-processor.proof.rs",
    "vm/vmcore/vm_topology/src/processor.proof.rs",
    "vm/vmcore/src/vmtime.proof.rs",
):
    present = {}
    for label, ref in (("frozen", scope["frozen_branch"]), ("working", "HEAD")):
        result = subprocess.check_output(["git", "ls-tree", "--name-only", ref, "--", path])
        present[label] = bool(result.strip())
    print(json.dumps({"path": path, **present}))

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Extract native imports recorded by the successful production consumer."""

from pathlib import Path
import tomllib

root = Path(__file__).resolve().parents[2]
consumer = root / "target/verus-partial/debug/deps/openvmm_core-fa59a9b373c670f4.d"
text = consumer.read_text()
imports = sorted({word.rstrip(":") for word in text.split() if ".vir" in word})
owners = [
    "openvmm/openvmm_defs",
    "support/mesh/mesh_channel",
    "support/mesh/mesh_channel_core",
    "support/mesh/mesh_protobuf",
    "support/pal/pal_async",
    "vm/devices/chipset_resources",
    "vmm_core",
    "vmm_core/state_unit",
    "vmm_core/virt",
    "vmm_core/vmm_core_defs",
]

print(f"Consumer dep-info: {consumer.relative_to(root)}")
for owner in owners:
    manifest = root / owner / "Cargo.toml"
    package = tomllib.loads(manifest.read_text())["package"]
    assert package["metadata"]["verus"]["verify"] is True, manifest
    matching = [
        path for path in imports if Path(path).name.startswith(f"lib{package['name']}-")
    ]
    assert len(matching) == 1, (manifest, matching)
    assert Path(matching[0]).is_file(), matching[0]
    print(f"{package['name']}: {Path(matching[0]).relative_to(root)}")
print("All ten opted-in declaration owners are recorded as native consumer imports.")
print("Validity evidence is the successful owner/consumer checks, not file existence.")

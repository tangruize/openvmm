# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check live inputs against the accepted diagnostic, without changing them."""

import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
ACCEPTED = ROOT / "research/vmtime-reset-installation/workspace"
tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
selected = [
    name for name in tracked if name
    and not name.startswith(("research/", ".autors/"))
    and (name.endswith((".rs", ".toml"))
         or name.startswith((".verus_agent/", "verification/"))
         or name == "Makefile")
]
selected.append("vm/vmcore/src/vmtime.proof.rs")
for name in selected:
    live = (ROOT / name).read_bytes()
    previous = (ACCEPTED / name).read_bytes()
    if name == "vmm_core/Cargo.toml":
        suffix = (
            '\n[[test]]\nname = "vmtime_reset_installation"\n'
            f'path = "{ACCEPTED.parent / "probe.rs"}"\n'
        ).encode()
        assert previous == live + suffix, name
    else:
        assert live == previous, name
print(f"{len(selected)} source/manifest/verification inputs match accepted evidence.")
print("Only manifest normalization: remove accepted vmm_core test-target suffix.")
for name in ("scope_manifest.json", "tcb_manifest.json"):
    data = json.loads((ROOT / ".verus_agent" / name).read_text())
    assert isinstance(data, dict) and data, name
print("Both frozen manifests parse as nonempty JSON objects and match intake.")

live_lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
accepted_lock = tomllib.loads((ACCEPTED / "Cargo.lock").read_text())
for package in accepted_lock["package"]:
    if package["name"] == "vmcore":
        package["dependencies"].remove("vstd")
        break
else:
    raise AssertionError("accepted vmcore lock entry missing")
assert live_lock == accepted_lock
print("Only lockfile difference: accepted vmcore entry adds already-resolved vstd.")
subprocess.run(
    ["git", "diff", "--exit-code", "--", "Cargo.lock",
     "research/freeze_requests/restore-duration-nanoseconds-interface"],
    cwd=ROOT, check=True,
)
print("Live lockfile and committed Duration request unchanged.")

source = ROOT / "toolchain/verus-src"
revision = subprocess.check_output(
    ["git", "-C", str(source), "rev-parse", "HEAD"], text=True
).strip()
assert revision == (ROOT / "verification/verus-revision").read_text().strip()
subprocess.run(
    ["git", "-C", str(source), "diff", "HEAD", "--exit-code"], check=True
)
print(f"Pinned frontend HEAD matches verification/verus-revision: {revision}.")
print("No tracked verifier changes; no maintained proof_state.json snapshot:",
      not (ROOT / ".verus_agent/proof_state.json").exists())

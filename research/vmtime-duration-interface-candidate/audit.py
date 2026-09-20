# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check production preservation and candidate executable equality."""

import json
from pathlib import Path
import subprocess

from argus_verus.tools.checks.exec_drift import compare_modules

root = Path.cwd()
out = root / "research/vmtime-duration-interface-candidate"
workspace = out / "workspace"
tracked = subprocess.check_output(["git", "ls-files", "-z"]).decode().split("\0")
candidate_changes = {
    ".verus_agent/scope_manifest.json",
    ".verus_agent/tcb_manifest.json",
    "Cargo.lock",
    "vm/vmcore/src/vmtime.rs",
}
unexpected = []
for name in filter(None, tracked):
    if name.startswith(("research/", ".autors/")) or name in candidate_changes:
        continue
    source, copied = root / name, workspace / name
    if source.is_symlink():
        if not copied.is_symlink() or source.readlink() != copied.readlink():
            unexpected.append(name)
    elif source.read_bytes() != copied.read_bytes():
        unexpected.append(name)
assert not unexpected, f"Unexpected production/candidate difference: {unexpected}"
source = "vm/vmcore/src/vmtime.rs"
assert (root / source).read_bytes() == (out / "intake-vmtime.rs").read_bytes()
for name in (".verus_agent/scope_manifest.json", ".verus_agent/tcb_manifest.json"):
    assert (root / name).read_bytes() == (out / "bases/working" / name).read_bytes()
report = compare_modules(str(out / "bases/frozen" / source), str(workspace / source))
summary = {
    "summary": report["summary"],
    "nonmatching": [
        {"name": entry["name"], "status": entry["status"]}
        for entry in report["functions"] + report["structs"]
        if entry["status"] != "MATCH"
    ],
    "live_production_matches_intake": True,
    "live_manifests_unchanged": True,
    "other_tracked_workspace_files_match": True,
}
(out / "candidate-exec-drift.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
assert report["summary"]["consistent"] and not summary["nonmatching"]

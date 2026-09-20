# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Generate independent proposals from captured branch bases and candidate source."""

import difflib
from pathlib import Path

out = Path("research/vmtime-duration-interface-candidate")
workspace = out / "workspace"
paths = [
    ".verus_agent/scope_manifest.json",
    ".verus_agent/tcb_manifest.json",
    "vm/vmcore/Cargo.toml",
    "vm/vmcore/src/vmtime.rs",
    "vm/vmcore/src/vmtime.proof.rs",
    "vm/vmcore/src/vmtime_duration/mod.rs",
    "vm/vmcore/src/vmtime_duration/observation.spec.rs",
]
for label, patch_name in (("frozen", "freeze.patch"), ("working", "run.patch")):
    chunks = []
    for name in paths:
        base_path = out / "bases" / label / name
        before = base_path.read_text() if base_path.exists() else ""
        after = (workspace / name).read_text()
        if label == "frozen":
            if name.endswith("vmtime.proof.rs"):
                continue
            if name.endswith("vmtime.rs"):
                anchor = "/// Roughly analogous to [`std::time::Instant`], but for VM time."
                assert before.count(anchor) == 1
                after = before.replace(
                    anchor,
                    '#[cfg(verus_keep_ghost)]\n'
                    '#[path = "vmtime_duration/mod.rs"]\n'
                    "pub mod duration_observation;\n\n" + anchor,
                )
        if before == after:
            continue
        chunks.append(f"diff --git a/{name} b/{name}\n")
        if not base_path.exists():
            chunks.append("new file mode 100644\n")
        chunks.extend(
            difflib.unified_diff(
                before.splitlines(keepends=True),
                after.splitlines(keepends=True),
                fromfile=f"a/{name}" if base_path.exists() else "/dev/null",
                tofile=f"b/{name}",
            )
        )
    (out / patch_name).write_text("".join(chunks))
    print(f"{patch_name}: {len(''.join(chunks).splitlines())} lines")

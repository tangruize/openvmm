# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Generate independent, unapplied patches; never modify the branch inputs."""

import difflib
import json
import subprocess

from production import HERE, ROOT


def base_file(ref, path):
    return subprocess.check_output(["git", "show", f"{ref}:{path}"], cwd=ROOT, text=True)


def delta(path, before, after):
    header = f"diff --git a/{path} b/{path}\n"
    if before is None:
        header += "new file mode 100644\n"
    return header + "".join(difflib.unified_diff(
        (before or "").splitlines(keepends=True), after.splitlines(keepends=True),
        fromfile=f"a/{path}" if before is not None else "/dev/null",
        tofile=f"b/{path}",
    ))


def main():
    source = "vmm_core/state_unit/src"
    sanctioned = [
        ("ExRawMutexTrait", "external_trait_specification"),
        ("ExRawMutex", "external_type_specification"),
        ("ExRawMutex", "external_body"),
        ("ExMutex", "external_type_specification"),
        ("ExMutex", "external_body"),
        ("ExMutexGuard", "external_type_specification"),
        ("ExMutexGuard", "external_body"),
        ("guard_origin", "uninterp"),
        ("guard_observation", "uninterp"),
        ("mutex_lock", "external_fn_specification"),
        ("guard_deref", "external_fn_specification"),
    ]
    frozen = json.loads((ROOT / ".verus_agent/scope_manifest.json").read_text())["frozen_branch"]
    for ref, name in [(frozen, "freeze.patch"), ("HEAD", "run.patch")]:
        patch = ""
        for path, key, extra in [
            (".verus_agent/scope_manifest.json", "src_roots", [f"{source}/mutex_observation"]),
            (".verus_agent/tcb_manifest.json", "sanctioned",
             [dict(kind="symbol", value=symbol, marker=marker) for symbol, marker in sanctioned]),
        ]:
            before = base_file(ref, path)
            parsed = json.loads(before)
            if any(entry in parsed[key] for entry in extra):
                raise RuntimeError(f"proposal already present in {ref}:{path}")
            parsed[key].extend(extra)
            patch += delta(path, before, json.dumps(parsed, indent=2) + "\n")

        manifest = "vmm_core/state_unit/Cargo.toml"
        before = base_file(ref, manifest)
        after = before
        if "[package.metadata.verus]" not in before:
            after = after.replace("[dependencies]", "[package.metadata.verus]\nverify = true\n\n[dependencies]", 1)
        if "vstd.workspace = true" not in before:
            after = after.replace("[dependencies]", "[dependencies]\nvstd.workspace = true", 1)
        if after != before:
            patch += delta(manifest, before, after)

        path = f"{source}/lib.rs"
        before = base_file(ref, path)
        anchor = "#![forbid(unsafe_code)]\n"
        if before.count(anchor) != 1:
            raise RuntimeError("unexpected state_unit module anchor")
        after = before.replace(anchor, anchor + "\n#[cfg(verus_keep_ghost)]\npub mod mutex_observation;\n", 1)
        patch += delta(path, before, after)
        patch += delta(f"{source}/mutex_observation/mod.rs", None,
                       '// Copyright (c) Microsoft Corporation.\n'
                       '// Licensed under the MIT License.\n\n'
                       'include!("observation.spec.rs");\n')
        patch += delta(f"{source}/mutex_observation/observation.spec.rs", None,
                       (HERE / "candidate.spec.rs").read_text())
        (HERE / name).write_text(patch)
        print(f"prepared {name} against {ref}; not applied")


if __name__ == "__main__":
    main()

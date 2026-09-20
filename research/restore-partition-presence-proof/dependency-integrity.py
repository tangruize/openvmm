#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Check the native helper against the pinned, licensed dependency source."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
original = Path("/home/ruize/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/anyhow-1.0.99")
native = ROOT / "verification/dependencies/anyhow"
checked = 0
for path in original.rglob("*"):
    if path.is_file():
        relative = path.relative_to(original)
        if relative.as_posix() not in ("Cargo.toml", "src/lib.rs"):
            assert path.read_bytes() == (native / relative).read_bytes(), relative
            checked += 1

source = (native / "src/lib.rs").read_text()
removals = [
    "#[cfg_attr(verus_keep_ghost, allow(unused_mut))]\n",
    "    use vstd::prelude::{verus_spec, verus_verify};\n",
    "    #[verus_verify]\n",
    "    #[verus_spec(result => ensures result == !cond.value())]\n",
    "        use vstd::prelude::*;\n\n        verus! {\n",
    "            spec fn value(&self) -> bool;\n\n",
    "            open spec fn value(&self) -> bool {\n                *self\n            }\n\n",
    "            open spec fn value(&self) -> bool {\n                **self\n            }\n\n",
]
for annotation in removals:
    assert source.count(annotation) == 1, annotation
    source = source.replace(annotation, "", 1)
signature = "            fn not(self) -> (result: bool)\n                ensures result == !self.value();"
assert source.count(signature) == 1
source = source.replace(signature, "            fn not(self) -> bool;", 1)
assert source.endswith("        }\n        }\n    }\n}\n")
source = source[:-len("        }\n    }\n}\n")] + "    }\n}\n"
assert source == (original / "src/lib.rs").read_text()
print(f"{checked} other dependency files byte-identical, including both licenses")
print("lib.rs is byte-identical after removing only the enumerated ghost annotations")
print("Both Bool bodies and the generic not body remain the original runtime implementations")

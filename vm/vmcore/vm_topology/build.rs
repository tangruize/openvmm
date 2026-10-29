// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![expect(missing_docs)]

fn main() {
    println!("cargo:rustc-check-cfg=cfg(verus_keep_ghost)");
    build_rs_guest_arch::emit_guest_arch()
}

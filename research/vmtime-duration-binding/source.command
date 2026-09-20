#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
"$verus" --version
printf '\nVerifier-selected sysroot with the production Cargo override\n'
RUSTUP_TOOLCHAIN=1.95.0 "$verus" --print sysroot
sysroot="$(RUSTUP_TOOLCHAIN=1.95.0 "$verus" --print sysroot | sort -u)"
library="$sysroot/lib/rustlib/src/rust/library"
printf '\nCompiler matching that sysroot\n'
"$sysroot/bin/rustc" -vV
printf '\nCargo wrapper selects the sibling Verus launcher\n'
sed -n '585,594p' toolchain/verus-src/source/cargo-verus/src/subcommands.rs
printf '\nLauncher explicitly selects its compiled-in toolchain\n'
sed -n '195,205p' toolchain/verus-src/source/verus/src/main.rs
printf '\nActual std reexport\n'
sed -n '30,37p' "$library/std/src/time.rs"
printf '\nActual core Duration fields, scale and method\n'
sed -n '22,29p;78,85p;626,633p' "$library/core/src/time.rs"
printf '\nActual Nanoseconds representation and as_inner body\n'
sed -n '12,23p;45,51p;94,98p' "$library/core/src/num/niche_types.rs"
printf '\nActual transmute reexport and bodyless declaration\n'
sed -n '64,69p' "$library/core/src/mem/mod.rs"
sed -n '837,843p' "$library/core/src/intrinsics/mod.rs"
printf '\nFrozen opaque Duration declaration\n'
sed -n '151,157p' toolchain/verus-src/source/vstd/std_specs/core.rs
printf '\nNative-core import support (not an automatic source import)\n'
sed -n '63,76p' toolchain/verus-src/source/rust_verify/src/config.rs
printf '\nDatatype opacity and duplicate-declaration checks\n'
sed -n '325,338p;1731,1748p' toolchain/verus-src/source/vir/src/well_formed.rs
printf '\nPattern-type translation boundary\n'
sed -n '1463,1467p' toolchain/verus-src/source/rust_verify/src/rust_to_vir_base.rs
printf '\nExisting native semantic-interface search\n'
if rg -n 'as_nanos|Nanoseconds|transmute|Duration' \
    toolchain/verus-src/source/vstd/std_specs; then
    :
else
    code=$?
    test "$code" -eq 1
fi

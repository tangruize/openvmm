set -euo pipefail
root="$(rustc +1.98.1 --print sysroot)"
printf 'sysroot=%s\n' "$root"
test -f "$root/lib/rustlib/src/rust/library/core/src/mem/mod.rs"
test -f "$root/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs"
rg -n -A 25 -B 3 'pub const fn replace' \
    "$root/lib/rustlib/src/rust/library/core/src/mem/mod.rs"
rg -n -A 36 -B 3 'fn position' \
    "$root/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs"

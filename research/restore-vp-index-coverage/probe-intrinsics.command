set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
out=research/restore-vp-index-coverage
for probe in replace-intrinsic-read replace-intrinsic-write; do
    status=0
    timeout 30s "$verus" --crate-type lib "$out/$probe.rs" \
        --multiple-errors 5 --num-threads 1 --triggers-mode silent \
        > "$out/$probe.log" 2>&1 || status=$?
    printf '%s exit=%s\n' "$probe" "$status"
done
root="$(rustc +1.98.1 --print sysroot)"
rg -n -A 9 -B 10 'pub const unsafe fn (read_via_copy|write_via_move)' \
    "$root/lib/rustlib/src/rust/library/core/src/intrinsics/mod.rs"

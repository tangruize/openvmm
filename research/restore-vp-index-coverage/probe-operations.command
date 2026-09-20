set -euo pipefail
verus="$(verification/tools/find-verus.sh)"
export VERUS_Z3_PATH="$PWD/toolchain/verus-src/source/z3"
out=research/restore-vp-index-coverage
"$verus" --version
"$VERUS_Z3_PATH" --version
failed=0
for probe in replace-admission position-admission swap-control; do
    status=0
    timeout 30s "$verus" --crate-type lib "$out/$probe.rs" \
        --multiple-errors 5 --num-threads 1 --triggers-mode silent \
        > "$out/$probe.log" 2>&1 || status=$?
    printf '%s exit=%s\n' "$probe" "$status"
    case "$probe:$status" in
        replace-admission:1)
            grep -q 'core::mem::replace.*is not supported' "$out/$probe.log" || failed=1
            ;;
        position-admission:1)
            grep -q 'position.*is not supported' "$out/$probe.log" || failed=1
            ;;
        swap-control:0)
            grep -q 'verification results:: 1 verified, 0 errors' "$out/$probe.log" || failed=1
            ;;
        *) failed=1 ;;
    esac
done
exit "$failed"

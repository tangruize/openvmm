set -eu
printf 'Authoritative Python: %s\n' "${ARGUS_SKILL_PYTHON:-python3}"
if test -e .git; then
    git --no-pager status --short
else
    printf 'No .git; no Git operations will be attempted.\n'
fi

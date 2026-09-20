#!/usr/bin/env bash
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

set -euo pipefail
cd "$(dirname "$0")/../.."
git add -- research/GROUND_TRUTH.md research/restore-vp-prefix-proof \
    research/freeze_requests/restore-vp-selector-named-closures
# Preserve literal unified-diff context and original tool output.
git diff --cached --check -- . ':!*.patch' ':!*.log'
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
import subprocess

files = subprocess.check_output(
    ["git", "diff", "--cached", "--name-only"], text=True
).splitlines()
assert files
for path in files:
    assert (
        path == "research/GROUND_TRUTH.md"
        or path.startswith("research/restore-vp-prefix-proof/")
        or path.startswith("research/freeze_requests/restore-vp-selector-named-closures/")
    ), f"out-of-scope staged file: {path}"
print(f"committing {len(files)} in-scope research files; no runtime package changes")
PY
git commit -m "Preserve validated VP selector syntax request and production evidence" \
    -m "Keep both proposed patches unapplied; retain focused production diagnostics and payload-preservation evidence for review." \
    -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
Copilot-Session: 44d109cf-ab3b-4284-b684-c97cc316d868"

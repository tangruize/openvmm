#!/usr/bin/env bash
set -euo pipefail
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-duration-interface-candidate/package.py
git apply --stat research/vmtime-duration-interface-candidate/freeze.patch
git apply --stat research/vmtime-duration-interface-candidate/run.patch

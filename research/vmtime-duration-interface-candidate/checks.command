#!/usr/bin/env bash
set -euo pipefail
"${ARGUS_SKILL_PYTHON:-python3}" research/vmtime-duration-interface-candidate/checks.py

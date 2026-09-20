#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Inspect the existing Human-presentation gate without resubmitting."""

from pathlib import Path

from argus_verus.tools.operator import freeze_request


root = Path(__file__).resolve().parents[2]
request = freeze_request.load(root, "restore-vp-selector-named-closures")
issues = freeze_request.operator_ready_issues(root, root / ".verus_agent", request)
for issue in issues:
    print(issue)
print(f"human_presentation_issue_count={len(issues)}")

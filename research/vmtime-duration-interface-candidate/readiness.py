# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Report native delivery prerequisites without applying or committing a request."""

import json
from pathlib import Path

from argus_verus.tools.operator.freeze_request import (
    RequestError,
    load,
    operator_ready_issues,
)

root = Path.cwd()
try:
    request = load(root, "restore-duration-nanoseconds-interface")
except RequestError as error:
    print(json.dumps({"native_package": "absent", "error": str(error)}, indent=2))
    raise SystemExit(1)
issues = operator_ready_issues(root, root / ".verus_agent", request)
print(json.dumps({"operator_ready_issues": issues}, indent=2))

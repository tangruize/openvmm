# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Run the bounded read-interface controls; negative proofs must fail as specified."""

import os
import re
import subprocess
import sys
import time

from production import HERE, ROOT, dependencies


def main():
    externs, _ = dependencies()
    library = [value for value in externs if value.startswith("parking_lot=")]
    if len(library) != 1:
        raise RuntimeError("expected one Cargo-selected parking_lot")
    verus = subprocess.check_output([ROOT / "verification/tools/find-verus.sh"],
                                    text=True).strip()
    solver = subprocess.check_output([ROOT / "verification/tools/find-z3.sh"],
                                     text=True).strip()
    flags = [verus, "--edition=2024", "--crate-type=lib", "--num-threads", "1",
             "--triggers-mode", "silent", "--multiple-errors", "15",
             "--extern", library[0], "-L", f"dependency={ROOT / 'target/debug/deps'}"]
    cases = [
        ("observe", "observe.rs", (), 0, "2 verified, 0 errors"),
        ("controls", "controls.rs", (), 1, "3 verified, 6 errors"),
        ("lifetime-escape", "lifetime-escape.rs", (), 1, "error[E0515]"),
        ("lifetime-release", "lifetime-release.rs", (), 1, "error[E0597]"),
        ("lifetime-alias", "lifetime-alias.rs", (), 1, "error[E0502]"),
        ("unconstrained-mutation", "unconstrained-mutation.rs", (), 0, "1 verified, 0 errors"),
    ]
    summary = []
    selected = set(sys.argv[1:])
    unknown = selected.difference(case[0] for case in cases)
    if unknown:
        raise RuntimeError(f"unknown probe selections: {unknown}")
    for name, source, selectors, expected_exit, expected in cases:
        if selected and name not in selected:
            continue
        command = [*flags, *selectors, str(HERE / source)]
        started = time.monotonic()
        result = subprocess.run(command, cwd=HERE,
                                env=dict(os.environ, VERUS_Z3_PATH=solver),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        elapsed = time.monotonic() - started
        (HERE / f"{name}-checked.log").write_text(
            "command: " + repr(command) + "\n" + result.stdout
            + f"\nexit={result.returncode}; elapsed={elapsed:.3f}s\n"
        )
        if result.returncode != expected_exit or expected not in result.stdout:
            raise RuntimeError(f"unexpected {name} outcome; see {name}-checked.log")
        if name == "controls":
            expected_lines = [
                i for i, line in enumerate((HERE / source).read_text().splitlines(), 1)
                if any(assertion in line for assertion in [
                    "assert(*guard == 7)", "assert(guard_origin(&guard) == mutex)",
                    "assert(*ga == *gb)", "assert(guard_origin(&ga) == b)",
                    "assert(after == before)",
                    "assert(*guard_observation(&guard) == before)",
                ])
            ]
            # The same-guard positive assertion is not an expected failure.
            expected_lines.remove(expected_lines[0])
            actual_lines = [
                int(line) for line in re.findall(r"controls.rs:(\d+):\d+", result.stdout)
            ]
            if actual_lines != expected_lines:
                raise RuntimeError(f"unexpected control failure sites: {actual_lines}")
        summary.append(f"{name}: expected exit {result.returncode}; {expected}; {elapsed:.3f}s")
    print("\n".join(summary))


if __name__ == "__main__":
    main()

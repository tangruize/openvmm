#!/usr/bin/env python3

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""Use native Verus closure syntax so the frozen-source checker can parse it."""

import difflib
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/restore-partition-presence-proof/proposal"
PATH = "openvmm/openvmm_core/src/worker/dispatch.rs"
original = '''        anyhow::ensure!(
            restore_time.is_none()
                || saved_state.units.iter().any(|unit| unit.name == "partition"),
            "time-adjusted snapshot restore requires partition state"
        );'''
normalized = '''        let partition_presence_valid = restore_time.is_none()
            || saved_state.units.iter().any(|unit: &state_unit::SavedStateUnit| -> bool {
                unit.name.as_str() == "partition"
            });
        anyhow::ensure!(
            partition_presence_valid,
            "time-adjusted snapshot restore requires partition state"
        );'''
guard = '''verus! {
impl LoadedVm {
    #[cfg(guest_arch = "x86_64")]
    fn validate_snapshot_restore_partition_presence(
        saved_state: &SavedState,
        restore_time: Option<(Duration, u64, Option<u64>)>,
    ) -> (result: anyhow::Result<()>)
        ensures
            result.is_ok() == (
                restore_time.is_none()
                || exists|i: int| 0 <= i < saved_state.units@.len()
                    && #[trigger] saved_state.units@[i].name@ == "partition"@
            ),
    {
        let partition_presence_valid = restore_time.is_none()
            || saved_state.units.iter().any(
                |unit: &state_unit::SavedStateUnit| -> (matched: bool)
                    ensures matched == (unit.name@ == "partition"@)
                {
                    unit.name.as_str() == "partition"
                },
            );
        proof {
            if !partition_presence_valid {
                assert forall|i: int| 0 <= i < saved_state.units@.len()
                    implies #[trigger] saved_state.units@[i].name@ != "partition"@
                by {
                    let unit = saved_state.units@.as_ref()[i];
                    assert(*unit == saved_state.units@[i]);
                    assert(unit.name@ != "partition"@);
                }
            }
        }
        anyhow::ensure!(
            partition_presence_valid,
            "time-adjusted snapshot restore requires partition state"
        );
        Ok(())
    }
}
} // verus!

impl LoadedVm {
'''


def baseline(ref):
    return subprocess.check_output(
        ["git", "show", f"{ref}:{PATH}"], cwd=ROOT, text=True,
    )


def diff(before, after):
    return f"diff --git a/{PATH} b/{PATH}\n" + "".join(
        difflib.unified_diff(
            before.splitlines(keepends=True), after.splitlines(keepends=True),
            fromfile=f"a/{PATH}", tofile=f"b/{PATH}",
        )
    )


frozen = baseline("argus/restore-v1-frozen")
assert frozen.count(original) == 1
with (OUT / "freeze-native-syntax.patch").open("x") as output:
    output.write(diff(frozen, frozen.replace(original, normalized, 1)))

current = (ROOT / PATH).read_text()
start = current.index('impl LoadedVm {\n    #[cfg(guest_arch = "x86_64")]\n    fn validate_snapshot_restore_partition_presence')
end = current.index("    /// Restores snapshot-owned state", start)
candidate = current[:start] + guard + current[end:]
run = (OUT / "run-native.patch").read_text()
start = run.index(f"diff --git a/{PATH} b/{PATH}\n")
end = run.index("diff --git ", start + 1)
run = run[:start] + diff(baseline("HEAD"), candidate) + run[end:]
with (OUT / "run-native-syntax.patch").open("x") as output:
    output.write(run)
print("Prepared native-syntax proposal; no production source was changed")

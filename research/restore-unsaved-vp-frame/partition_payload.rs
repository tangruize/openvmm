// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub(super) fn probe_partition_vps(
    buffer: &vmcore::save_restore::SavedStateBlob,
) -> Vec<(VpIndex, vmcore::save_restore::SavedStateBlob)> {
    let state: state::Partition = buffer.parse().unwrap();
    state
        .vps
        .into_iter()
        .map(|vp| (VpIndex::new(vp.vp_index), vp.data))
        .collect()
}

pub(super) fn probe_without_vps(
    buffer: vmcore::save_restore::SavedStateBlob,
) -> vmcore::save_restore::SavedStateBlob {
    let mut state: state::Partition = buffer.parse().unwrap();
    state.vps.clear();
    vmcore::save_restore::SavedStateBlob::new(state)
}

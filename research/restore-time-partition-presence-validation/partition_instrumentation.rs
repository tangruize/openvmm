// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

include!("../restore-unsaved-vp-frame/partition_payload.rs");

pub fn candidate_partition_vps(
    buffer: &vmcore::save_restore::SavedStateBlob,
) -> Vec<(VpIndex, vmcore::save_restore::SavedStateBlob)> {
    probe_partition_vps(buffer)
}

pub fn candidate_without_vps(
    buffer: vmcore::save_restore::SavedStateBlob,
) -> vmcore::save_restore::SavedStateBlob {
    probe_without_vps(buffer)
}

pub fn candidate_with_vp_state(
    buffer: vmcore::save_restore::SavedStateBlob,
    vp_state: vmcore::save_restore::SavedStateBlob,
) -> vmcore::save_restore::SavedStateBlob {
    let mut state: state::Partition = buffer.parse().unwrap();
    assert_eq!(state.vps.len(), 1);
    assert_eq!(state.vps[0].vp_index, 0);
    state.vps[0].data = vp_state;
    vmcore::save_restore::SavedStateBlob::new(state)
}

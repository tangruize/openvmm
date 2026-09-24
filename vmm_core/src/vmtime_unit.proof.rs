// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl KeeperUnit<'_> {
    #[verifier::prophetic]
    pub closed spec fn restore_completed(&self, before: &Self, state: &SavedStateBlob) -> bool {
        self.0.local_stopped_time()
            == Some(mesh::payload::protobuf::saved_time_last(state@.bytes))
            && *final(self.0) == *final(before.0)
    }
}

// Completion uses the unproved keeper restore contract; the subsequent save
// observation is body-proved and reads the original borrowed keeper.
async fn check_keeper_completion(
    keeper: &mut VmTimeKeeper, state: SavedStateBlob,
) -> (saved: vmcore::vmtime::SavedState)
    requires
        old(keeper).local_stopped_time() is Some,
        mesh::payload::protobuf::saved_time_values(state@.bytes) is Some,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
    ensures
        final(keeper).local_stopped_time()
            == Some(mesh::payload::protobuf::saved_time_last(state@.bytes)),
        saved.saved_time() == mesh::payload::protobuf::saved_time_last(state@.bytes),
        cfg!(verus_keeper_negative_saved_value) ==> saved.saved_time() == 73,
{
    {
        let mut unit = KeeperUnit(keeper);
        let result = unit.restore(state).await;
        assert(result is Ok);
    }
    keeper.save()
}

async fn check_empty_keeper_completion(
    keeper: &mut VmTimeKeeper, state: SavedStateBlob,
) -> (saved: vmcore::vmtime::SavedState)
    requires
        old(keeper).local_stopped_time() is Some,
        state@.bytes.len() == 0,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
    ensures
        final(keeper).local_stopped_time() == Some(0),
        saved.saved_time() == 0,
{
    check_keeper_completion(keeper, state).await
}

#[cfg(verus_keeper_negative_cancel)]
fn check_keeper_cancellation(keeper: &mut VmTimeKeeper, state: SavedStateBlob)
    requires
        old(keeper).local_stopped_time() is Some,
        mesh::payload::protobuf::saved_time_values(state@.bytes) is Some,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
        old(keeper).local_stopped_time()
            != Some(mesh::payload::protobuf::saved_time_last(state@.bytes)),
{
    let ghost expected = mesh::payload::protobuf::saved_time_last(state@.bytes);
    {
        let mut unit = KeeperUnit(keeper);
        let _future = unit.restore(state);
    }
    assert(keeper.local_stopped_time() == Some(expected));
}

#[cfg(verus_keeper_negative_unawaited)]
fn check_keeper_unawaited(keeper: &mut VmTimeKeeper, state: SavedStateBlob)
    requires
        old(keeper).local_stopped_time() is Some,
        mesh::payload::protobuf::saved_time_values(state@.bytes) is Some,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
{
    let mut unit = KeeperUnit(keeper);
    let future = unit.restore(state);
    assert(vstd::future::FutureAdditionalSpecFns::awaited(&future));
}

#[cfg(verus_keeper_negative_domain)]
async fn check_keeper_requires_domain(keeper: &mut VmTimeKeeper, state: SavedStateBlob)
    requires
        old(keeper).local_stopped_time() is Some,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
{
    let mut unit = KeeperUnit(keeper);
    let _ = unit.restore(state).await;
}

#[cfg(verus_keeper_negative_stopped)]
async fn check_keeper_requires_stopped(keeper: &mut VmTimeKeeper, state: SavedStateBlob)
    requires
        mesh::payload::protobuf::saved_time_values(state@.bytes) is Some,
        vmcore::vmtime::saved_type_url().accepts(state@.type_url),
{
    let mut unit = KeeperUnit(keeper);
    let _ = unit.restore(state).await;
}

}

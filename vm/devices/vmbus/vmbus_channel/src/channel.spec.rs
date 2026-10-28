// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The channel task's share of its unit's FracGhost, and the protocol of
// the channel's internal state requests, for human review.

use super::ChannelHandle;
use super::Device;
use super::GenericChannelHandle;
use super::StateRequest;
use super::VmbusDevice;
use mesh::error::RemoteError;
use mesh_rpc_verus::MeshProtocol;
use mesh_rpc_verus::RpcAdditionalSpecFns;
use vmcore::device_state::spec::DeviceGhostState;
use vmcore::save_restore::SavedStateBlob;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// A vmbus device's spec: what its save returns (its snapshotable state),
/// as `SaveRestore`'s `saved_state()` is for a chipset device. Defined by
/// each device when it is specified.
#[verifier::external_trait_specification]
#[verifier::external_trait_extension(VmbusDeviceSpec via VmbusDeviceSpecImpl)]
pub trait ExVmbusDevice: Send + 'static + crate::channel::IntoAny + inspect::InspectMut {
    type ExternalTraitSpecificationFor: VmbusDevice;

    /// The blob the device's save returns, or `None` if it does not support
    /// save/restore (`supports_save_restore` returns `None`).
    spec fn saved_blob(&self) -> Option<SavedStateBlobView>;

    /// Well formed: what a valid device of this type satisfies.
    spec fn device_wf(&self) -> bool;
}

/// The protocol of a channel's internal state requests (the channel's
/// handle to the channel's task); `k` is the id of the channel unit's
/// FracGhost. The same as the outer request protocol
/// (`state_unit::StateRequest`'s): while the channel runs, its task holds
/// the whole FracGhost; while it is stopped, the caller and the task each
/// hold one half.
///
/// - `Start`: the caller gives its half (no reply).
/// - `Stop`: no ghost is sent; the task returns a half recording its
///   device's state (not `Unknown`).
/// - `Save`: the caller lends its half, recording a known state; the task
///   returns it, with a blob that matches it.
/// - `Restore(blob)`, `Reset`: the caller sends its half; the task returns a
///   half; on a successful restore it records `Saved(blob@)`.
/// - `Inspect`: no ghost.
impl MeshProtocol for StateRequest {
    type Const = Loc;
    type GhostIn = Option<FracGhost<UnitSnapshot>>;
    type GhostOut = Option<FracGhost<UnitSnapshot>>;
    /// Nothing is attached to the internal channel's ends: the halves are
    /// created by the channel's offer (`offer_generic`).
    type SenderGhost = ();
    type ReceiverGhost = ();

    /// (Closed: `StateRequest` is private to this module.)
    closed spec fn request_ok(k: Loc, msg: StateRequest, gin: Self::GhostIn) -> bool {
        match msg {
            StateRequest::Start => gin matches Some(h) && h.valid(k, 0.5real),
            StateRequest::Stop(rpc) => {
                &&& gin is None
                &&& forall|r: (), gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) && !(h@ is Unknown)
                        ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Reset(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: (), gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Save(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& !(gin->Some_0@ is Unknown)
                &&& forall|r: Result<Option<SavedStateBlob>, RemoteError>, gout: Self::GhostOut|
                    gout == gin && match r {
                        Ok(Some(blob)) => gin->Some_0@ == UnitSnapshot::Saved(blob@),
                        Ok(None) => gin->Some_0@ == UnitSnapshot::NoSavedState,
                        Err(_) => true,
                    } ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Restore(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) && (r is Ok ==> h@
                        == UnitSnapshot::Saved(rpc.spec_input()@)) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateRequest::Inspect(_) => gin is None,
        }
    }
}

/// The view of a channel's task (`Device`).
pub struct DeviceView {
    /// Whether the task handles the guest's requests (otherwise it queues
    /// them, until `Start`).
    pub running: bool,
}

impl View for Device {
    type V = DeviceView;

    /// Trusted: `running` is `self.state` being `DeviceState::Running`.
    /// (Uninterpreted: `Device` has private fields of types not declared to
    /// Verus.)
    uninterp spec fn view(&self) -> DeviceView;
}

impl Device {
    /// The channel task's invariant for its share of the channel unit's
    /// FracGhost, `share`, and its device, `channel`: the share follows the
    /// rule for the device's snapshot (what the device's save returns); the
    /// device is well formed; and while the task handles the guest's
    /// requests (which change the device), it holds the whole share.
    pub(in crate::channel) open spec fn keeps_share(
        &self,
        share: DeviceGhostState,
        channel: &dyn VmbusDevice,
    ) -> bool {
        &&& share.follows_rule(share.frac.id(), UnitSnapshot::from_saved(channel.saved_blob()))
        &&& channel.device_wf()
        &&& self@.running ==> share.running
    }
}

impl GenericChannelHandle {
    /// The id of the channel unit's FracGhost. Trusted: the constant of its
    /// internal channel, `self.state_req.constant()`. (Uninterpreted:
    /// `GenericChannelHandle` holds a `pal_async::task::Task`, an alias of a
    /// type of the `async_task` crate, so the type is opaque to Verus.)
    pub uninterp spec fn id(&self) -> Loc;
}

impl<T: ?Sized> ChannelHandle<T> {
    /// The id of the channel unit's FracGhost. Trusted: `self.0.id()`.
    /// (Uninterpreted: `ChannelHandle`'s `PhantomData<fn() -> Box<T>>`
    /// field is a function-pointer type, which Verus does not support, so
    /// the type is opaque to Verus.)
    pub uninterp spec fn id(&self) -> Loc;
}

} // verus!

// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// The VP protocol (`VpSet` to the VP threads, one `VpEvent` channel per VP)
// and `VpSet`'s view, for human review. See the design doc, "The VP
// protocol".

use super::StateEvent;
use super::Vp;
use super::VpEvent;
use super::VpSet;
use super::RegisterSetError;
use mesh::error::RemoteError;
use mesh_rpc_verus::*;
use vm_topology::processor::VpIndex;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
use vmcore::save_restore::spec::SavedStateBlobView;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// The VP protocol. `k` is the id of the VP's FracGhost (a `UnitSnapshot`:
/// `Unknown`, or `Saved(blob)`, the VP's saved state, without guest time).
/// While the VP is stopped, `VpSet` and the VP thread each hold one half;
/// while it runs (or has halted by itself), the thread holds both. While
/// `VpSet` holds its half, the half records what the VP's save returns
/// (the rule, kept by the VP thread).
///
/// TRUSTED (`SNAPSHOT-SPEC-TRUSTED.md`): VP runners are never dropped while
/// their `VpSet` is in use, so no call on this channel fails with a channel
/// error, which would lose `VpSet`'s half (a failed call returns no ghost).
/// `VpSet`'s contracts rely on it (design doc, "The VP protocol", C1).
///
/// - `Start`: `VpSet` gives its half; the thread, holding both, runs the VP.
/// - `Stop`: no ghost is sent; the thread stops the VP and returns a half
///   recording its saved state.
/// - `State(..)`: see `StateEvent::request_ok`.
impl MeshProtocol for VpEvent {
    type Const = Loc;
    /// `VpSet`'s half, given or lent.
    type GhostIn = Option<FracGhost<UnitSnapshot>>;
    /// A half returned to `VpSet`.
    type GhostOut = Option<FracGhost<UnitSnapshot>>;
    /// `VpSet`'s half, attached at creation (`vp_channel`).
    type SenderGhost = FracGhost<UnitSnapshot>;
    /// The VP thread's half, attached at creation.
    type ReceiverGhost = FracGhost<UnitSnapshot>;

    open(super) spec fn request_ok(k: Loc, msg: VpEvent, gin: Self::GhostIn) -> bool {
        match msg {
            // One-way: `VpSet`'s half goes to the thread.
            VpEvent::Start => gin matches Some(h) && h.valid(k, 0.5real),
            VpEvent::Stop(rpc) => {
                &&& gin is None
                &&& forall|r: (), gout: Self::GhostOut|
                    gout matches Some(h) && h.valid(k, 0.5real) && h@ is Saved
                        ==> #[trigger] rpc.reply_ok(r, gout)
            },
            VpEvent::State(event) => event.request_ok(k, gin),
        }
    }
}

impl StateEvent {
    /// The protocol of the state events, which `VpSet` sends while the VPs
    /// are stopped (`Inspect`, and the time-ABI events, also while they
    /// run).
    ///
    /// - `Save`: `VpSet` lends its half, which records a saved state; the
    ///   same half comes back, and on `Ok` the blob is the recorded one.
    /// - `Restore(blob)`: `VpSet` sends its half; a half comes back, also on
    ///   `Err`, recording `Saved(blob@)` on `Ok`.
    /// - `Reset`, `Scrub`, `SetInitialRegs`: `VpSet` sends its half; a half
    ///   comes back, also on `Err`, with any value.
    /// - `Inspect`, `CheckTimers`, `AdvanceLapic`, `GetDumpVpState`: no
    ///   ghost (`AdvanceLapic` changes only the LAPIC timers, which a saved
    ///   state's view leaves out: the time rule).
    /// - `Debug` (`gdb`): out of scope; verified code cannot send it.
    pub(super) open spec fn request_ok(self, k: Loc, gin: Option<FracGhost<UnitSnapshot>>) -> bool {
        match self {
            StateEvent::Save(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real) && h@ is Saved
                &&& forall|r: Result<SavedStateBlob, SaveError>, gout: Option<FracGhost<UnitSnapshot>>|
                    gout == gin && (r matches Ok(blob) ==> gin->Some_0@ == UnitSnapshot::Saved(blob@))
                        ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateEvent::Restore(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RestoreError>, gout: Option<FracGhost<UnitSnapshot>>|
                    gout matches Some(h) && h.valid(k, 0.5real) && (r is Ok ==> h@
                        == UnitSnapshot::Saved(rpc.spec_input()@)) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateEvent::Reset(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Option<FracGhost<UnitSnapshot>>|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateEvent::Scrub(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RemoteError>, gout: Option<FracGhost<UnitSnapshot>>|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateEvent::SetInitialRegs(rpc) => {
                &&& gin matches Some(h) && h.valid(k, 0.5real)
                &&& forall|r: Result<(), RegisterSetError>, gout: Option<FracGhost<UnitSnapshot>>|
                    gout matches Some(h) && h.valid(k, 0.5real) ==> #[trigger] rpc.reply_ok(r, gout)
            },
            StateEvent::Inspect(_) => gin is None,
            #[cfg(guest_arch = "x86_64")]
            StateEvent::CheckTimers(rpc) => gin is None && forall|r: Result<(), RemoteError>|
                #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>),
            #[cfg(guest_arch = "x86_64")]
            StateEvent::AdvanceLapic(rpc) => gin is None && forall|r: Result<(), RemoteError>|
                #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>),
            #[cfg(feature = "dump")]
            StateEvent::GetDumpVpState(rpc) => gin is None && forall|
                r: Result<hyperv_dump::VpState, anyhow::Error>,
            | #[trigger] rpc.reply_ok(r, None::<FracGhost<UnitSnapshot>>),
            #[cfg(feature = "gdb")]
            StateEvent::Debug(_) => false,
        }
    }
}

/// The view of a `VpSet`: whether the VPs are started, and what `VpSet`'s
/// half of each VP's FracGhost records (`Unknown` while the VPs are
/// started: `VpSet` then holds no half).
pub struct VpSetView {
    /// Started (by `start`, with no `stop` since).
    pub started: bool,
    /// Per VP, in index order, what `VpSet`'s half records (the
    /// instantiated VPs: a prefix of the topology's).
    pub vps: Seq<UnitSnapshot>,
    /// The topology's VP count (`vp_capacity`); a snapshot has one entry per
    /// VP up to it.
    pub capacity: nat,
}

impl VpSetView {
    pub open spec fn inv(&self) -> bool {
        self.started ==> forall|i: int| 0 <= i < self.vps.len() ==> #[trigger] self.vps[i]
            is Unknown
    }

    /// Every VP's half records a saved state.
    pub open spec fn all_saved(&self) -> bool {
        forall|i: int| 0 <= i < self.vps.len() ==> #[trigger] self.vps[i] is Saved
    }

    /// Each VP's index and recorded saved state, in index order
    /// (meaningful when `all_saved()`): what `VpSet::save` returns.
    pub open spec fn saved_vps(&self) -> Seq<(u32, SavedStateBlobView)> {
        Seq::new(self.vps.len(), |i: int| (i as u32, self.vps[i]->Saved_0))
    }
}

impl View for VpSet {
    type V = VpSetView;

    closed spec fn view(&self) -> VpSetView {
        VpSetView {
            started: self.started,
            vps: self.vps@.map_values(|vp: Vp| vp.recorded()),
            capacity: self.vp_capacity as nat,
        }
    }
}

impl Vp {
    /// What `VpSet`'s half of this VP's FracGhost records (`Unknown` without
    /// a half).
    closed spec fn recorded(&self) -> UnitSnapshot {
        match self.share@ {
            Some(h) => h@,
            None => UnitSnapshot::Unknown,
        }
    }

    /// `VpSet` holds its half (of this VP's FracGhost, whose id is the
    /// channel's constant) exactly while the VPs are stopped.
    closed spec fn wf(&self, started: bool) -> bool {
        &&& self.send.attached() is None
        &&& if started {
            self.share@ is None
        } else {
            self.share@ matches Some(h) && h.valid(self.send.constant(), 0.5real)
        }
    }
}

impl VpSet {
    pub closed spec fn wf(&self) -> bool {
        &&& self@.inv()
        &&& forall|i: int| 0 <= i < self.vps@.len() ==> (#[trigger] self.vps@[i]).wf(self.started)
    }
}

} // verus!

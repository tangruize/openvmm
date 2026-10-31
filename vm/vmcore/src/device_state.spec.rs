// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of a device's ghost state, for human review.

use super::DeviceSlot;
use crate::save_restore::ProtobufSaveRestore;
use crate::save_restore::SavedStateBlob;
use crate::save_restore::spec::ProtobufSaveRestoreSpec;
use crate::save_restore::spec::SavedStateBlobView;
use crate::save_restore::spec::UnitSnapshot;
use closeable_mutex::LockInvariant;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// A state unit's share of its FracGhost, and whether the unit runs. Kept
/// with the unit's state: in a ghost field of the unit, in a chipset
/// device's lock (`DeviceSlot`), or in an atomic's invariant (the time-ABI
/// MSRs). See "Design change: the FracGhost lives with the device" in
/// `snapshot-spec-docs/DESIGN.md`.
pub tracked struct DeviceGhostState {
    /// The device's share: the whole while running, one half while stopped.
    pub tracked frac: FracGhost<UnitSnapshot>,
    /// Whether the device is running.
    pub ghost running: bool,
}

impl DeviceGhostState {
    /// The rule, the same for every device, for a device whose FracGhost has
    /// id `id` and whose state is `snapshot`: the device holds the whole or
    /// one half of that FracGhost, the whole while running; and the
    /// FracGhost, unless still `Unknown`, records `snapshot`.
    pub open spec fn follows_rule(self, id: Loc, snapshot: UnitSnapshot) -> bool {
        &&& self.frac.id() == id
        &&& (self.frac.frac() == 1real || self.frac.frac() == 0.5real)
        &&& self.running ==> self.whole()
        &&& !(self.frac@ is Unknown) ==> self.frac@ == snapshot
    }

    /// The device holds the whole FracGhost.
    pub open spec fn whole(self) -> bool {
        self.frac.frac() == 1real
    }
}

/// What a chipset device's lock needs of the device to state its share's
/// invariant: the blob the device's save returns (its snapshotable state,
/// encoded). Every `ProtobufSaveRestore` type has it; a trait for devices
/// seen through a trait object (`dyn`) to have it too.
pub trait DeviceSnapshot {
    /// The blob the device's save returns.
    spec fn saved_blob(&self) -> SavedStateBlobView;

    /// The device is well formed (its `device_wf()`).
    spec fn device_wf(&self) -> bool;
}

impl<T: ProtobufSaveRestore> DeviceSnapshot for T {
    open spec fn saved_blob(&self) -> SavedStateBlobView {
        ProtobufSaveRestoreSpec::saved_blob(self)
    }

    open spec fn device_wf(&self) -> bool {
        ProtobufSaveRestoreSpec::device_wf(self)
    }
}

impl<T: ?Sized + DeviceSnapshot> DeviceSlot<T> {
    /// The id of the unit's FracGhost, whose share the slot keeps.
    pub closed spec fn share_id(&self) -> Loc {
        self.share@.frac.id()
    }

    /// Whether the unit omits the device's saved state.
    pub closed spec fn omit(&self) -> bool {
        self.omit@
    }

    /// The device's snapshot: what its unit's save returns (nothing if the
    /// unit omits the device's saved state).
    pub closed spec fn snapshot(&self) -> UnitSnapshot {
        if self.omit@ {
            UnitSnapshot::NoSavedState
        } else {
            UnitSnapshot::Saved(self.device.saved_blob())
        }
    }
}

/// The lock invariant of a chipset device's lock, for the lock's own
/// parameter `(id, omit)` (given when the lock is created): the slot keeps
/// the share of **this** unit's FracGhost (id `id`), and records whether
/// the unit omits the saved state (`omit`); the device is well formed
/// (`device_wf()`); and the share follows the rule
/// (`DeviceGhostState::follows_rule`) for the device's snapshot.
impl<T: ?Sized + DeviceSnapshot> LockInvariant for DeviceSlot<T> {
    type Param = (Loc, bool);

    closed spec fn inv(&self, param: (Loc, bool)) -> bool {
        &&& self.share_id() == param.0
        &&& self.omit() == param.1
        &&& self.device.device_wf()
        &&& self.share@.follows_rule(param.0, self.snapshot())
    }
}

} // verus!

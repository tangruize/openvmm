// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the dual PIC, for human review.

use super::super::DualPic;
use super::super::IRQ_MASK;
use super::super::InitStage;
use super::super::Pic;
use super::state::SavedInitStage;
use super::state::SavedPic;
use super::state::SavedState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vstd::prelude::*;

verus! {

/// The view of one PIC: its snapshotable registers and its role.
/// (Not in the view: `lines`, the current levels of its input lines, which
/// are not saved; they belong to the lines driving them.)
pub struct PicView {
    /// The initialization stage.
    pub(super) init: InitStage,
    /// Whether this is the primary PIC (static configuration).
    pub(super) primary: bool,
    /// The interrupt vector base (ICW2).
    pub(super) icw2: u8,
    /// The interrupt mask register (OCW1).
    pub(super) imr: u8,
    /// The last OCW3.
    pub(super) ocw3: u8,
    /// The in-service register.
    pub(super) isr: u8,
    /// The edge/level control register.
    pub(super) elcr: u8,
    /// Which lines have been low since their last interrupt.
    pub(super) line_low_latch: u8,
}

impl PicView {
    /// The vector base's low three bits are zero (ICW2 writes clear them;
    /// `restore` rejects a saved state that sets them).
    pub(super) open spec fn inv(self) -> bool {
        self.icw2 & IRQ_MASK == 0
    }

    /// The saved form of this PIC, field by field.
    pub(super) open spec fn saved(self) -> SavedPic {
        SavedPic {
            init: match self.init {
                InitStage::Uninitialized => SavedInitStage::Uninitialized,
                InitStage::ExpectingIcw2 => SavedInitStage::ExpectingIcw2,
                InitStage::ExpectingIcw3 => SavedInitStage::ExpectingIcw3,
                InitStage::ExpectingIcw4 => SavedInitStage::ExpectingIcw4,
                InitStage::Initialized => SavedInitStage::Initialized,
            },
            icw2: self.icw2,
            imr: self.imr,
            ocw3: self.ocw3,
            isr: self.isr,
            elcr: self.elcr,
            line_low_latch: self.line_low_latch,
        }
    }
}

/// The view of the dual PIC: its two PICs. (Not in the view: the runtime
/// glue, i.e. the CPU's interrupt line and the port I/O regions, and the
/// statistics.)
pub struct DualPicView {
    /// The primary PIC.
    pub(super) primary: PicView,
    /// The secondary PIC, cascaded on the primary's IRQ 2.
    pub(super) secondary: PicView,
}

impl DualPicView {
    /// Both PICs' invariants, and the first is the primary.
    pub(super) open spec fn inv(self) -> bool {
        &&& self.primary.inv()
        &&& self.secondary.inv()
        &&& self.primary.primary
        &&& !self.secondary.primary
    }

    /// The saved state of a dual PIC in this state: what its `save`
    /// returns.
    pub(super) open spec fn saved_state(self) -> SavedState {
        SavedState { primary: self.primary.saved(), secondary: self.secondary.saved() }
    }
}

impl View for Pic {
    type V = PicView;

    closed spec fn view(&self) -> PicView {
        PicView {
            init: self.init,
            primary: self.primary,
            icw2: self.icw2,
            imr: self.imr,
            ocw3: self.ocw3.0,
            isr: self.isr,
            elcr: self.elcr,
            line_low_latch: self.line_low_latch,
        }
    }
}

impl View for DualPic {
    type V = DualPicView;

    closed spec fn view(&self) -> DualPicView {
        DualPicView { primary: self.pics[0]@, secondary: self.pics[1]@ }
    }
}

impl DualPic {
    /// Well formed: the view's invariant holds.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

/// The PIC's saved state has no time fields: its view is the value
/// itself.
impl View for SavedState {
    type V = SavedState;

    open spec fn view(&self) -> SavedState {
        *self
    }
}

/// The dual PIC saves its view's saved state (without `primary`, which is
/// static configuration).
impl SaveRestoreSpecImpl for DualPic {
    closed spec fn saved_view(view: DualPicView) -> SavedState {
        view.saved_state()
    }

    /// The dual PIC is well formed (`wf()`).
    closed spec fn device_wf(&self) -> bool {
        self.wf()
    }
}

} // verus!

// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// High-level spec of the IOAPIC, for human review.

use super::super::IoApicDevice;
use super::super::IrqEntry;
use super::state::SavedState;
use vmcore::save_restore::spec::SaveRestoreSpecImpl;
use vstd::prelude::*;

verus! {

/// The view of the IOAPIC: its ID and index registers, and per input line
/// its redirection entry (64 bits) and level. It is also the view of its
/// saved state. (Not in the view: the static line range, the interrupt
/// routing and each entry's registered route, and the statistics.)
pub struct IoApicView {
    /// The IOAPIC ID.
    pub(super) id: u8,
    /// The selected register (the index register).
    pub(super) index: u8,
    /// Each line's redirection entry.
    pub(super) redirection: Seq<u64>,
    /// Each line's level (high or low).
    pub(super) line_levels: Seq<bool>,
}

impl IoApicView {
    /// One level per redirection entry.
    pub(super) open spec fn inv(self) -> bool {
        self.redirection.len() == self.line_levels.len()
    }

    /// Whether an entry with these bits, and its line at `level`, has an
    /// interrupt to deliver: unmasked (bit 16), level-triggered (bit 15)
    /// with a fixed or lowest-priority delivery mode (bits 8 to 10: 0 or
    /// 1), its line high, and remote IRR (bit 14) clear. At runtime such an
    /// interrupt is delivered at once (`IrqEntry::evaluate`), so the IOAPIC
    /// never keeps one.
    pub(super) open spec fn pending(bits: u64, level: bool) -> bool {
        &&& (bits >> 16u64) & 1 == 0
        &&& (bits >> 15u64) & 1 == 1
        &&& ((bits >> 8u64) & 7 == 0 || (bits >> 8u64) & 7 == 1)
        &&& level
        &&& (bits >> 14u64) & 1 == 0
    }

    /// An entry as after delivering its pending interrupt, if any: remote
    /// IRR set (what `restore` does, `IrqEntry::evaluate`).
    pub(super) open spec fn delivered(bits: u64, level: bool) -> u64 {
        if Self::pending(bits, level) {
            bits | (1u64 << 14u64)
        } else {
            bits
        }
    }

    /// This state with every pending interrupt delivered: the meaning of a
    /// saved state with these entries and levels.
    pub(super) open spec fn with_delivered(self) -> IoApicView {
        IoApicView {
            redirection: Seq::new(
                self.redirection.len(),
                |i: int| Self::delivered(self.redirection[i], self.line_levels[i]),
            ),
            ..self
        }
    }
}

impl View for IoApicDevice {
    type V = IoApicView;

    closed spec fn view(&self) -> IoApicView {
        IoApicView {
            id: self.id,
            index: self.index.0,
            redirection: self.irqs@.map_values(|e: IrqEntry| e.redirection.bits()),
            line_levels: self.irqs@.map_values(|e: IrqEntry| e.line_level),
        }
    }
}

impl IoApicDevice {
    /// Well formed: the view's invariant holds.
    pub closed spec fn wf(&self) -> bool {
        self@.inv()
    }
}

/// The view of the IOAPIC's saved state: its meaning, i.e. what `restore`
/// makes of it. Two encodings `restore` reads the same way have the same
/// view: an empty `line_levels` (saved before line levels were saved) is
/// every line low, and a pending interrupt is delivered.
impl View for SavedState {
    type V = IoApicView;

    closed spec fn view(&self) -> IoApicView {
        let n = self.redirection_entries@.len();
        IoApicView {
            id: self.id,
            index: self.index,
            redirection: self.redirection_entries@,
            line_levels: if self.line_levels@.len() == 0 {
                Seq::new(n, |i: int| false)
            } else {
                self.line_levels@
            },
        }.with_delivered()
    }
}

/// The IOAPIC saves its view, read with pending interrupts delivered (it
/// has none at runtime).
impl SaveRestoreSpecImpl for IoApicDevice {
    closed spec fn saved_view(view: IoApicView) -> IoApicView {
        view.with_delivered()
    }

    /// The IOAPIC is well formed (`wf()`).
    closed spec fn device_wf(&self) -> bool {
        self.wf()
    }
}

} // verus!

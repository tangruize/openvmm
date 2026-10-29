// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use chipset_device::ChipsetDevice;
use chipset_device_resources::LineSetId;
use inspect::InspectMut;
#[cfg(verus_keep_ghost)]
use mesh_rpc_verus::ReceiverAdditionalSpecFns;
use state_unit::SpawnedUnit;
use state_unit::StateUnit;
#[cfg(verus_keep_ghost)]
use state_unit::StateUnitSpec;
use state_unit::StateUnits;
use state_unit::UnitHandle;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(verus_keep_ghost)]
use vmcore::device_state::spec::DeviceGhostState;
use vmcore::line_interrupt::LineSet;
use vmcore::line_interrupt::LineSetTarget;
#[cfg(verus_keep_ghost)]
use vmcore::save_restore::spec::UnitSnapshot;
use vmcore::vm_task::VmTaskDriverSource;
#[cfg(verus_keep_ghost)]
use vstd::prelude::verus_spec;
#[cfg(verus_keep_ghost)]
use vstd::prelude::verus_verify;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::resource::frac::FracGhost;

/// The `StateUnit` spec of a line set's state unit.
#[cfg(verus_keep_ghost)]
mod spec {
    include!("line_sets.spec.rs");
}

#[cfg(verus_keep_ghost)]
mod proof {
    include!("line_sets.proof.rs");
}

pub struct LineSets {
    map: HashMap<LineSetId, (Arc<LineSet>, usize)>,
    pub units: Vec<SpawnedUnit<()>>,
}

impl LineSets {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            units: Vec::new(),
        }
    }

    pub fn line_set(
        &mut self,
        driver: &VmTaskDriverSource,
        units: &StateUnits,
        id: LineSetId,
    ) -> (&LineSet, &UnitHandle) {
        let (line_set, index) = self.map.entry(id.clone()).or_insert_with(|| {
            let set = Arc::new(LineSet::new());
            let unit = units
                .add(format!("lines/{}", id.name()))
                .spawn(driver.simple(), {
                    let set = set.clone();
                    async move |mut recv| {
                        let unit = LineSetUnit::new(&set, &mut recv);
                        state_unit::run_unit(unit, recv).await;
                    }
                })
                .unwrap();
            let index = self.units.len();
            self.units.push(unit);
            (set, index)
        });
        (line_set, self.units[*index].handle())
    }
}

#[derive(InspectMut)]
#[inspect(transparent)]
struct LineSetUnit<'a>(
    &'a LineSet,
    /// The unit's share of its FracGhost (Verus only).
    #[cfg(verus_keep_ghost)]
    #[inspect(skip)]
    Tracked<DeviceGhostState>,
);

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<'a> LineSetUnit<'a> {
    /// The unit for `set`, taking its half from `recv`, its channel's
    /// receiver.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(unit =>
        requires
            old(recv).attached() matches Some(h) && h.valid(old(recv).constant(), 0.5real),
        ensures
            unit.wf(),
            unit.id() == old(recv).constant(),
            !unit.running(),
            final(recv).constant() == old(recv).constant(),
    ))]
    fn new(set: &'a LineSet, recv: &mut mesh::Receiver<state_unit::StateRequest>) -> Self {
        let _ = recv;
        // PROOF TODO: replace this placeholder (`assume_new()`) with the real
        // ghost: the half taken from `recv` (`take_receiver_ghost`).
        #[cfg(verus_keep_ghost)]
        return Self(set, Tracked::assume_new());
        #[cfg(not(verus_keep_ghost))]
        return Self(set);
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl StateUnit for LineSetUnit<'_> {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn stop(&mut self) {}

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn save(
        &mut self,
    ) -> Result<Option<vmcore::save_restore::SavedStateBlob>, vmcore::save_restore::SaveError> {
        Ok(None)
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut Option<FracGhost<UnitSnapshot>>>))]
    async fn restore(
        &mut self,
        _: vmcore::save_restore::SavedStateBlob,
    ) -> Result<(), vmcore::save_restore::RestoreError> {
        Err(vmcore::save_restore::RestoreError::SavedStateNotSupported)
    }
}

pub struct LineSetTargetDevice<T> {
    device: Arc<vmcore::device_state::DeviceMutex<T>>,
}

impl<T: ChipsetDevice> LineSetTargetDevice<T> {
    pub fn new(device: Arc<vmcore::device_state::DeviceMutex<T>>) -> Self {
        Self { device }
    }
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl<T: ChipsetDevice> LineSetTarget for LineSetTargetDevice<T> {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    fn set_irq(&self, vector: u32, high: bool) {
        self.device
            .lock()
            .supports_line_interrupt_target()
            .unwrap()
            .set_irq(vector, high);
    }
}

// Verus: declarations of this file's types used in Verus-visible
// signatures, as opaque types.
#[cfg(verus_keep_ghost)]
vstd::prelude::verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExLineSetTargetDevice<T>(LineSetTargetDevice<T>);

} // verus!

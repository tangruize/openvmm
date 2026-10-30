// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Declarations and lemmas supporting `dispatch.spec.rs`; no human review
// needed.

use super::LoadedVmView;
use crate::worker::dispatch::InitializedVm;
use crate::worker::dispatch::LoadedVmInner;
use crate::worker::dispatch::microvm::SnapshotBoundary;
use crate::worker::dispatch::restore::SnapshotRestore;
use openvmm_defs::worker::spec::SavedStateView;
use vmcore::save_restore::spec::SavedStateBlobView;
use vstd::prelude::*;

verus! {

// `LoadedVmInner` is `verus_verify` itself (`dispatch.rs`); the types of its
// fields, declared to Verus by name only (fields hidden), and the traits
// of its trait objects.

#[verifier::external_trait_specification]
pub trait ExMicrovmPartition: Send + Sync {
    type ExternalTraitSpecificationFor: crate::partition::microvm::MicrovmPartition;
}

#[verifier::external_trait_specification]
pub trait ExRequestYield: Send + Sync {
    type ExternalTraitSpecificationFor: vmm_core::partition_unit::RequestYield;
}

#[verifier::external_trait_specification]
pub trait ExHvlitePartition:
    inspect::Inspect + Send + Sync + vmm_core::partition_unit::RequestYield
        + crate::partition::microvm::MicrovmPartition
{
    type ExternalTraitSpecificationFor: crate::partition::HvlitePartition;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExResourceResolver(vm_resource::ResourceResolver);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExGuestMemoryManager(membacking::GuestMemoryManager);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHvsockRelay(vmbus_server::hvsock::HvsockRelay);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmbusServerHandle(vmm_core::vmbus_unit::VmbusServerHandle);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExNumaTopology(openvmm_defs::config::NumaTopology);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMemoryLayout(vm_topology::memory::MemoryLayout);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExX86Topology(vm_topology::processor::x86::X86Topology);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExHypervisorConfig(openvmm_defs::config::HypervisorConfig);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExBaseChipsetManifest(vmotherboard::options::BaseChipsetManifest);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmChipsetCapabilities(vmotherboard::options::VmChipsetCapabilities);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMemoryRange(memory_range::MemoryRange);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExChipsetMmioRanges(crate::worker::memory_layout::ChipsetMmioRanges);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFirmwareEvent(get_resources::ged::FirmwareEvent);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMachineProfile(openvmm_defs::microvm::MachineProfile);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLoadMode(openvmm_defs::config::LoadMode);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExIgvmFile(igvm::IgvmFile);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVmgsClient(vmgs_broker::VmgsClient);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExIommuDevices(crate::worker::dispatch::IommuDevices);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPcieHostBridge(vm_topology::pcie::PcieHostBridge);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExGenericInitiatorSource(crate::worker::dispatch::GenericInitiatorSource);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDynamicDeviceUnit(vmotherboard::DynamicDeviceUnit);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExChipsetDevices(vmotherboard::ChipsetDevices);

#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExDynamicVpciDeviceEntry(crate::worker::dispatch::DynamicVpciDeviceEntry);

#[cfg(guest_arch = "x86_64")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub(in crate::worker::dispatch) struct ExTimeAbiState(crate::worker::dispatch::time_abi::TimeAbiState);

#[cfg(target_os = "linux")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVfioManagerClient(vfio_assigned_device::manager::VfioManagerClient);

#[cfg(target_os = "linux")]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVfioCdevManagerClient(vfio_assigned_device::manager::VfioCdevManagerClient);

/// Declares std's `AsRef` to Verus (no claims; a bound of `ArchTopology`'s
/// associated type).
#[verifier::external_trait_specification]
pub trait ExAsRef<T: core::marker::PointeeSized>: core::marker::PointeeSized {
    type ExternalTraitSpecificationFor: core::convert::AsRef<T>;
}

#[verifier::external_trait_specification]
pub trait ExArchTopology: Sized {
    type ExternalTraitSpecificationFor: vm_topology::processor::ArchTopology;

    type ArchVpInfo: Copy + AsRef<vm_topology::processor::VpInfo>;

    type BuilderState;
}

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVpInfo(vm_topology::processor::VpInfo);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExProcessorTopology<T: vm_topology::processor::ArchTopology>(
    vm_topology::processor::ProcessorTopology<T>,
);

// (`SnapshotRestore` is `verus_verify` itself, `dispatch/restore.rs`.)

// `SnapshotRestore`'s other field types, declared to Verus by name only (fields hidden).
// (`std::fs::File` is declared in `openvmm_defs`, `rpc.spec.rs`.)

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExInstant(pal_async::timer::Instant);

// (`SnapshotBoundary` is `verus_verify` itself, `dispatch/microvm.rs`.)

#[verifier::external_type_specification]
#[verifier::external_body]
pub(crate) struct ExInitializedVm(InitializedVm);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotScratchPolicy(
    chipset_resources::microvm::MicrovmSnapshotScratchPolicy,
);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExMicrovmSnapshotBoundaryRequest(
    chipset_resources::microvm::MicrovmSnapshotBoundaryRequest,
);

// (`anyhow::Error` is declared in vmcore, `save_restore.proof.rs`;
// `mesh::RecvError` in vmm_core, `partition_unit/vp_set.proof.rs`.)

impl LoadedVmView {
    /// `InitializedVm::load`'s contract for a restore follows from the VM it
    /// builds and `restore_from_snapshot`'s: `self` is the freshly built VM
    /// (no unit's state recorded), `after` the VM after
    /// `restore_from_snapshot` succeeded.
    pub proof fn lemma_load_restores(self, state: SavedStateView, after: LoadedVmView)
        requires
            // The fresh VM.
            self.snapshot.units == Map::<Seq<char>, SavedStateBlobView>::empty(),
            // `restore_from_snapshot`'s `Ok` result.
            after.snapshot.units == self.snapshot.units.union_prefer_right(state.units),
        ensures
            after.snapshot.units == state.units,
    {
        assert(self.snapshot.units.union_prefer_right(state.units) =~= state.units);
    }

    /// Restoring a saved state into a VM with the same stateful units and the
    /// same inventory gives back the snapshot state the saved state holds.
    /// With `saved` from `LoadedVm::save`, this is the round trip: the VM's
    /// snapshot state is the one at save time. `self` is the VM before the
    /// restore, `after` the VM after it.
    pub proof fn lemma_restore_gives_saved_state(self, saved: SavedStateView, after: LoadedVmView)
        requires
            // The VM restored into has the units the saved state has.
            self.snapshot.units.dom() == saved.units.dom(),
            self.snapshot.inventory == saved.inventory,
            // `restore`'s postcondition on success.
            after.snapshot.inventory == self.snapshot.inventory,
            after.snapshot.units == self.snapshot.units.union_prefer_right(saved.units),
        ensures
            after.snapshot == saved,
    {
        assert(after.snapshot.units =~= saved.units);
    }
}

} // verus!

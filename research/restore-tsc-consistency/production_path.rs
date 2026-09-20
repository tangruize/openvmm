// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use super::*;
use std::convert::Infallible;
use std::time::Duration;
use test_with_tracing::test;
use virt::state::StateElement;
use virt::vp::*;
use virt::x86::X86PartitionCapabilities;
use vm_topology::processor::TopologyBuilder;
use vm_topology::processor::x86::X2ApicState;
use vm_topology::processor::x86::X86VpInfo;

// Read field 11 from the production save_all/codec path without discarding
// any other register from the state passed to production restore_all.
#[derive(mesh::payload::Protobuf)]
struct TscWire {
    #[mesh(11)]
    tsc: Tsc,
}

struct RegisterBackend {
    caps: X86PartitionCapabilities,
    info: X86VpInfo,
    tsc: u64,
    apic: Apic,
    commits: Vec<u64>,
}

impl inspect::InspectMut for RegisterBackend {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        req.respond().field("tsc", self.tsc);
    }
}

macro_rules! reset_only_registers {
    ($(($get:ident, $set:ident, $ty:ty)),* $(,)?) => {
        $(
            fn $get(&mut self) -> Result<$ty, Self::Error> {
                Ok(<$ty>::at_reset(&self.caps, &self.info))
            }

            fn $set(&mut self, value: &$ty) -> Result<(), Self::Error> {
                assert_eq!(value, &<$ty>::at_reset(&self.caps, &self.info));
                Ok(())
            }
        )*
    };
}

impl AccessVpState for &mut RegisterBackend {
    type Error = Infallible;

    fn caps(&self) -> &X86PartitionCapabilities {
        &self.caps
    }

    fn commit(&mut self) -> Result<(), Self::Error> {
        self.commits.push(self.tsc);
        Ok(())
    }

    fn tsc(&mut self) -> Result<Tsc, Self::Error> {
        Ok(Tsc { value: self.tsc })
    }

    fn set_tsc(&mut self, value: &Tsc) -> Result<(), Self::Error> {
        self.tsc = value.value;
        Ok(())
    }

    fn apic(&mut self) -> Result<Apic, Self::Error> {
        Ok(self.apic.clone())
    }

    fn set_apic(&mut self, value: &Apic) -> Result<(), Self::Error> {
        self.apic = value.clone();
        Ok(())
    }

    reset_only_registers! {
        (registers, set_registers, Registers),
        (activity, set_activity, Activity),
        (xsave, set_xsave, Xsave),
        (xcr, set_xcr, Xcr0),
        (xss, set_xss, Xss),
        (mtrrs, set_mtrrs, Mtrrs),
        (pat, set_pat, Pat),
        (virtual_msrs, set_virtual_msrs, VirtualMsrs),
        (debug_regs, set_debug_regs, DebugRegisters),
        (cet, set_cet, Cet),
        (cet_ss, set_cet_ss, CetSs),
        (tsc_aux, set_tsc_aux, TscAux),
        (tsc_deadline, set_tsc_deadline, TscDeadline),
        (synic_msrs, set_synic_msrs, SyntheticMsrs),
        (synic_message_page, set_synic_message_page, SynicMessagePage),
        (synic_event_flags_page, set_synic_event_flags_page, SynicEventFlagsPage),
        (synic_message_queues, set_synic_message_queues, SynicMessageQueues),
        (synic_timers, set_synic_timers, SynicTimers),
        (nested_state, set_nested_state, NestedState),
    }
}

impl ProtobufSaveRestore for RegisterBackend {
    fn save(&mut self) -> Result<SavedStateBlob, SaveError> {
        self.access_state(Vtl::Vtl0)
            .save_all()
            .map(SavedStateBlob::new)
            .map_err(|e| SaveError::Other(e.into()))
    }

    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        self.access_state(Vtl::Vtl0)
            .restore_all(&state.parse::<VpSavedState>()?)
            .map_err(|e| RestoreError::Other(e.into()))
    }
}

impl Processor for RegisterBackend {
    type StateAccess<'a> = &'a mut Self;

    fn set_debug_state(
        &mut self,
        _vtl: Vtl,
        _state: Option<&virt::x86::DebugState>,
    ) -> Result<(), Infallible> {
        panic!("the probe does not set debug state");
    }

    async fn run_vp(
        &mut self,
        _stop: StopVp<'_>,
        _dev: &impl CpuIo,
    ) -> Result<Infallible, VpHaltReason> {
        panic!("the probe must never run the guest");
    }

    fn flush_async_requests(&mut self) {}

    fn access_state(&mut self, vtl: Vtl) -> Self::StateAccess<'_> {
        assert_eq!(vtl, Vtl::Vtl0);
        self
    }
}

struct NoIo;

impl CpuIo for NoIo {
    fn is_mmio(&self, _address: u64) -> bool {
        panic!("unexpected guest I/O");
    }
    fn acknowledge_pic_interrupt(&self) -> Option<u8> {
        panic!("unexpected guest I/O");
    }
    fn handle_eoi(&self, _irq: u32) {
        panic!("unexpected guest I/O");
    }
    async fn read_mmio(&self, _vp: VpIndex, _address: u64, _data: &mut [u8]) {
        panic!("unexpected guest I/O");
    }
    async fn write_mmio(&self, _vp: VpIndex, _address: u64, _data: &[u8]) {
        panic!("unexpected guest I/O");
    }
    async fn read_io(&self, _vp: VpIndex, _port: u16, _data: &mut [u8]) {
        panic!("unexpected guest I/O");
    }
    async fn write_io(&self, _vp: VpIndex, _port: u16, _data: &[u8]) {
        panic!("unexpected guest I/O");
    }
    fn fatal_error(&self, _error: Box<dyn std::error::Error + Send + Sync>) -> VpHaltReason {
        panic!("unexpected guest I/O");
    }
}

fn saved_tsc(state: &SavedStateBlob) -> u64 {
    let state: VpSavedState = state.parse().unwrap();
    let wire: TscWire = mesh::payload::decode(&mesh::payload::encode(state)).unwrap();
    wire.tsc.value
}

fn serialized_tsc(states: &[(VpIndex, SavedStateBlob)]) -> u64 {
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].0, VpIndex::new(0));
    saved_tsc(&states[0].1)
}

// Executable TSC coordinate of the candidate request interpretation, not a
// restore implementation. Splitting seconds/nanoseconds avoids the production
// u128 intermediate overflow; even Duration::MAX and u64::MAX fit after division
// and addition of a u64 saved counter. Values above u64 remain distinct images,
// not successful production outcomes or additional input exclusions.
fn requested_tsc(state: &SavedStateBlob, adjustment: Option<(Duration, u64)>) -> u128 {
    let cycles = match adjustment {
        None => 0,
        Some((duration, hz)) => {
            u128::from(duration.as_secs()) * u128::from(hz)
                + u128::from(duration.subsec_nanos()) * u128::from(hz) / 1_000_000_000
        }
    };
    u128::from(saved_tsc(state)) + cycles
}

fn register_backend(tsc: u64) -> RegisterBackend {
    let topology = TopologyBuilder::new_x86()
        .x2apic(X2ApicState::Unsupported)
        .build(1)
        .unwrap();
    let mut caps = X86PartitionCapabilities::from_cpuid(&topology, &mut |_, _| [0; 4]).unwrap();
    caps.can_freeze_time = true;
    let info = topology.vp_arch(VpIndex::new(0));
    RegisterBackend {
        apic: Apic::at_reset(&caps, &info),
        caps,
        info,
        tsc,
        commits: Vec::new(),
    }
}

fn exercise_restore(source_tsc: u64, adjustment: Option<(Duration, u64)>, expected_tsc: u64) {
    let mut source = register_backend(source_tsc);
    let saved = source.save().unwrap();
    assert_eq!(saved_tsc(&saved), source_tsc);
    let request_tsc = requested_tsc(&saved, adjustment);
    assert_eq!(request_tsc, u128::from(expected_tsc));
    assert_eq!(requested_tsc(&saved, None), u128::from(source_tsc));
    let raw_image: VpSavedState = saved.parse().unwrap();
    let expected_image: VpSavedState = register_backend(expected_tsc)
        .save()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(raw_image == expected_image, source_tsc == expected_tsc);

    let mut backend = register_backend(77);
    let (halt, _halt_recv) = Halt::new();
    let mut set = VpSet::new([None, None, None], Arc::new(halt), 1);
    let mut runner = set.add(backend.info);

    pal_async::local::block_on(async {
        let control = async move {
            set.restore([(VpIndex::new(0), saved)]).await.unwrap();
            let restored = set.save().await.unwrap();
            let before = serialized_tsc(&restored);
            assert_eq!(before, source_tsc);
            assert_eq!(restored[0].1.parse::<VpSavedState>().unwrap(), raw_image);
            if let Some((duration, hz)) = adjustment {
                // Same APIC branch used by restore_snapshot_state on x86.
                set.advance_tsc(duration, hz, Some(1_000_000))
                    .await
                    .unwrap();
            }
            let final_state = set.save().await.unwrap();
            let after = serialized_tsc(&final_state);
            assert_eq!(after, expected_tsc);
            assert_eq!(u128::from(after), request_tsc);
            assert_eq!(
                final_state[0].1.parse::<VpSavedState>().unwrap(),
                expected_image
            );
            if adjustment.is_some() {
                assert_ne!(after, source_tsc);
            }
            println!(
                "selected_vp=0 saved_tsc={source_tsc} restored_tsc={before} \
                 final_serialized_tsc={after} policy_request_tsc={request_tsc} \
                 full_register_image_equal=true adjustment={adjustment:?}"
            );
            drop(set);
        };
        let ((), result) = futures::join!(control, runner.run(&mut backend, &NoIo));
        result.unwrap();
    });
    assert_eq!(backend.tsc, expected_tsc);
    let expected_commits = if adjustment.is_some() {
        vec![source_tsc, expected_tsc]
    } else {
        vec![source_tsc]
    };
    assert_eq!(backend.commits, expected_commits);
    println!("committed_tsc={:?}", backend.commits);
}

#[test]
fn production_restore_without_tsc_adjustment() {
    for tsc in [1_000, 1_001] {
        exercise_restore(tsc, None, tsc);
    }
}

#[test]
fn production_restore_with_nonzero_tsc_adjustment() {
    for tsc in [1_000, 1_001] {
        exercise_restore(tsc, Some((Duration::from_millis(250), 4_003)), tsc + 1_000);
    }
}

// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// This generated include is the unchanged register-backend portion of the
// accepted selected-and-saved probe, not a copied production restore method.
include!("accepted_backend.rs");

use pal_async::DefaultPool;
use state_unit::StateUnits;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use virt::Processor;
use virt::StopVp;
use virt::VpHaltReason;
use virt::io::CpuIo;
use vmcore::save_restore::ProtobufSaveRestore;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
use vmm_core::partition_unit::Halt;
use vmm_core::partition_unit::PartitionUnitParams;
use vmm_core::partition_unit::VmPartition;
use vmm_core::partition_unit::save_restore::candidate_partition_vps;
use vmm_core::partition_unit::save_restore::candidate_with_vp_state;
use vmm_core::partition_unit::save_restore::candidate_without_vps;

#[derive(mesh::payload::Protobuf, vmcore::save_restore::SavedStateRoot)]
#[mesh(package = "restore_partition_presence_probe")]
struct PartitionRegisters {
    #[mesh(1)]
    value: u64,
}

struct PartitionBackend {
    restores: Arc<AtomicUsize>,
}

impl inspect::InspectMut for PartitionBackend {
    fn inspect_mut(&mut self, req: inspect::Request<'_>) {
        req.respond();
    }
}

impl ProtobufSaveRestore for PartitionBackend {
    fn save(&mut self) -> Result<SavedStateBlob, SaveError> {
        Ok(SavedStateBlob::new(PartitionRegisters { value: 0 }))
    }

    fn restore(&mut self, state: SavedStateBlob) -> Result<(), RestoreError> {
        assert_eq!(state.parse::<PartitionRegisters>()?.value, 0);
        self.restores.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait::async_trait]
impl VmPartition for PartitionBackend {
    fn reset(&mut self) -> anyhow::Result<()> {
        panic!("the stopped restore probe must not reset");
    }

    fn scrub_vtl(&mut self, _vtl: Vtl) -> anyhow::Result<()> {
        panic!("the stopped restore probe must not scrub");
    }

    fn accept_initial_pages(&mut self, _pages: Vec<virt::InitialPageImport>) -> anyhow::Result<()> {
        panic!("the stopped restore probe must not load firmware");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Payload {
    Omitted,
    Complete,
    MissingVp,
}

fn exercise(initial_tsc: u64, payload: Payload, downtime: Option<Duration>) {
    let mut backend = register_backend(initial_tsc);
    let mut saved_backend = register_backend(1_000);
    let saved_vp = saved_backend.save().unwrap();
    let expected_tsc = if payload == Payload::Complete {
        u64::try_from(requested_tsc(&saved_vp, downtime.map(|d| (d, 4_003)))).unwrap()
    } else {
        initial_tsc
    };
    let restores = Arc::new(AtomicUsize::new(0));
    let observed_restores = restores.clone();
    DefaultPool::run_with(async |driver| {
        let mut units = StateUnits::new();
        let topology = TopologyBuilder::new_x86()
            .x2apic(X2ApicState::Unsupported)
            .build(1)
            .unwrap();
        let (halt, halt_recv) = Halt::new();
        let (notify_send, _notify_recv) = mesh::channel();
        let (mut partition, mut runners) = PartitionUnit::new(
            &driver,
            units.add("partition"),
            PartitionBackend { restores },
            PartitionUnitParams {
                vtl_guest_memory: [None, None, None],
                processor_topology: &topology,
                active_vp_count: Some(1),
                halt_vps: Arc::new(halt),
                halt_request_recv: halt_recv,
                client_notify_send: notify_send,
                debugger_rpc: None,
            },
        )
        .unwrap();
        assert_eq!(runners.len(), 1);
        let mut runner = runners.pop().unwrap();
        let control = async move {
            let inventory = units.inventory();
            assert_eq!(inventory, ["partition"]);
            let mut payloads = units.save().await.unwrap();
            assert_eq!(payloads.len(), 1);
            assert_eq!(payloads[0].name, "partition");
            match payload {
                Payload::Omitted => payloads.clear(),
                Payload::Complete | Payload::MissingVp => {
                    let saved = payloads.pop().unwrap();
                    payloads.push(state_unit::SavedStateUnit {
                        name: saved.name,
                        state: if payload == Payload::Complete {
                            candidate_with_vp_state(saved.state, saved_vp)
                        } else {
                            candidate_without_vps(saved.state)
                        },
                    });
                }
            }
            let state = SavedState { units: payloads, inventory };
            let restore_time = downtime.map(|d| (d, 4_003, Some(1_000_000)));
            let gate = LoadedVm::validate_snapshot_restore_partition_presence(&state, restore_time);
            let rejected_at_guard = payload == Payload::Omitted && downtime.is_some();
            if rejected_at_guard {
                assert_eq!(
                    gate.unwrap_err().to_string(),
                    "time-adjusted snapshot restore requires partition state"
                );
                assert_eq!(observed_restores.load(Ordering::SeqCst), 0);
            } else {
                gate.unwrap();
                units.validate_inventory(&state.inventory).unwrap();
                let restored = units.restore(state.units).await;
                if payload == Payload::MissingVp {
                    let error = format!("{:#}", anyhow::Error::new(restored.unwrap_err()));
                    assert!(error.contains("snapshot is missing state for vp0"), "{error}");
                    assert_eq!(observed_restores.load(Ordering::SeqCst), 1);
                } else {
                    restored.unwrap();
                    assert_eq!(
                        observed_restores.load(Ordering::SeqCst),
                        usize::from(payload == Payload::Complete)
                    );
                    if let Some(d) = downtime {
                        units.advance_time(d).await.unwrap();
                        partition.advance_tsc(d, 4_003, Some(1_000_000)).await.unwrap();
                    }
                    let guard = partition.temporarily_stop_vps().await;
                    assert!(!units.is_running());
                    drop(guard);
                }
            }
            let after = units.save().await.unwrap();
            let vp_states = candidate_partition_vps(&after[0].state);
            assert_eq!(serialized_tsc(&vp_states), expected_tsc);
            if payload == Payload::Complete {
                let mut expected = register_backend(expected_tsc);
                assert_eq!(
                    vp_states[0].1.parse::<VpSavedState>().unwrap(),
                    expected.save().unwrap().parse::<VpSavedState>().unwrap()
                );
            }
            println!(
                "payload={payload:?} downtime={downtime:?} initial_tsc={initial_tsc} \
                 final_tsc={expected_tsc} guard_rejected={rejected_at_guard} \
                 partition_restores={}",
                observed_restores.load(Ordering::SeqCst)
            );
            partition.teardown().await;
        };
        let run = async {
            let result = runner.run(&mut backend, &NoIo).await;
            drop(runner);
            result
        };
        let ((), result) = futures::join!(control, run);
        result.unwrap();
    });
    let mut expected_commits = Vec::new();
    if payload == Payload::Complete {
        expected_commits.push(1_000);
        if downtime.is_some() {
            expected_commits.push(expected_tsc);
        }
    }
    assert_eq!(backend.tsc, expected_tsc);
    assert_eq!(backend.commits, expected_commits);
    println!("committed_tsc={:?}", backend.commits);
}

#[self::test]
fn candidate_partition_presence_matrix() {
    for initial_tsc in [77, 78] {
        for downtime in [
            None,
            Some(Duration::ZERO),
            Some(Duration::from_nanos(1)),
            Some(Duration::from_millis(250)),
        ] {
            exercise(initial_tsc, Payload::Omitted, downtime);
            exercise(initial_tsc, Payload::Complete, downtime);
        }
    }
    exercise(77, Payload::MissingVp, Some(Duration::from_millis(250)));
}

#[self::test]
fn payload_presence_is_not_inventory_presence() {
    let time = Some((Duration::ZERO, 4_003, Some(1_000_000)));
    for inventory in [vec![], vec!["partition".to_owned()]] {
        let mut state = SavedState { units: vec![], inventory };
        assert!(LoadedVm::validate_snapshot_restore_partition_presence(&state, time).is_err());
        state.units.push(state_unit::SavedStateUnit {
            name: "not-partition".to_owned(),
            state: register_backend(1_000).save().unwrap(),
        });
        assert!(LoadedVm::validate_snapshot_restore_partition_presence(&state, time).is_err());
        assert!(LoadedVm::validate_snapshot_restore_partition_presence(&state, None).is_ok());
    }
}

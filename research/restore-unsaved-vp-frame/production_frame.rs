// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// Reuse the accepted register backend without changing or rerunning its tests.
include!("../restore-tsc-consistency/production_path.rs");

mod unsaved_frame {
    use super::*;
    use crate::partition_unit::PartitionUnit;
    use crate::partition_unit::PartitionUnitParams;
    use crate::partition_unit::VmPartition;
    use crate::partition_unit::save_restore::probe_partition_vps;
    use crate::partition_unit::save_restore::probe_without_vps;
    use pal_async::DefaultPool;
    use state_unit::StateUnits;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use test_with_tracing::test;

    #[derive(mesh::payload::Protobuf, vmcore::save_restore::SavedStateRoot)]
    #[mesh(package = "restore_unsaved_vp_probe")]
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

        fn accept_initial_pages(
            &mut self,
            _pages: Vec<virt::InitialPageImport>,
        ) -> anyhow::Result<()> {
            panic!("the stopped restore probe must not load firmware");
        }
    }

    fn exercise(initial_tsc: u64, omit_payload: bool, adjustment: bool) {
        let mut backend = register_backend(initial_tsc);
        let restores = Arc::new(AtomicUsize::new(0));
        let observed_restores = restores.clone();
        let expected_tsc = initial_tsc + if omit_payload && adjustment { 1_000 } else { 0 };

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
                assert_eq!(
                    serialized_tsc(&probe_partition_vps(&payloads[0].state)),
                    initial_tsc
                );
                if omit_payload {
                    payloads.clear();
                } else {
                    let partition = payloads.pop().unwrap();
                    payloads.push(state_unit::SavedStateUnit {
                        name: partition.name,
                        state: probe_without_vps(partition.state),
                    });
                }

                // These are the real two operations called by LoadedVm::restore.
                units.validate_inventory(&inventory).unwrap();
                let result = units.restore(payloads).await;
                if omit_payload {
                    result.unwrap();
                    assert_eq!(observed_restores.load(Ordering::SeqCst), 0);
                    let before = units.save().await.unwrap();
                    assert_eq!(
                        serialized_tsc(&probe_partition_vps(&before[0].state)),
                        initial_tsc
                    );
                    if adjustment {
                        let downtime = Duration::from_millis(250);
                        units.advance_time(downtime).await.unwrap();
                        partition
                            .advance_tsc(downtime, 4_003, Some(1_000_000))
                            .await
                            .unwrap();
                    }
                    let guard = partition.temporarily_stop_vps().await;
                    assert!(!units.is_running());
                    let after = units.save().await.unwrap();
                    assert_eq!(
                        serialized_tsc(&probe_partition_vps(&after[0].state)),
                        expected_tsc
                    );
                    println!(
                        "omitted_partition=true inventory_valid=true saved_vp_entries=0 \
                         partition_restore_calls=0 initial_tsc={initial_tsc} \
                         final_serialized_tsc={expected_tsc} adjustment={adjustment} \
                         stop_guard_acquired=true"
                    );
                    drop(guard);
                } else {
                    let error = result.unwrap_err();
                    let message = format!("{:#}", anyhow::Error::new(error));
                    assert!(
                        message.contains("snapshot is missing state for vp0"),
                        "{message}"
                    );
                    assert_eq!(observed_restores.load(Ordering::SeqCst), 1);
                    println!(
                        "omitted_partition=false inventory_valid=true saved_vp_entries=0 \
                         restore_rejected=true adjustment_not_reached=true error={message}"
                    );
                }
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
        assert_eq!(backend.tsc, expected_tsc);
        let expected_commits = if omit_payload && adjustment {
            vec![expected_tsc]
        } else {
            Vec::new()
        };
        assert_eq!(backend.commits, expected_commits);
        println!("committed_tsc={:?}", backend.commits);
    }

    #[test]
    fn omitted_partition_without_adjustment() {
        for initial_tsc in [77, 78] {
            exercise(initial_tsc, true, false);
        }
    }

    #[test]
    fn omitted_partition_with_nonzero_adjustment() {
        for initial_tsc in [77, 78] {
            exercise(initial_tsc, true, true);
        }
    }

    #[test]
    fn present_partition_with_missing_vp_is_rejected() {
        exercise(77, false, true);
    }
}

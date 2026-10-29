// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! The snapshot I/O boundary: queuing a stop for every VP before releasing a
//! deferred I/O completion, so the VP that issued the I/O cannot re-enter the
//! guest.

use super::VpEvent;
use super::VpSet;
use anyhow::Context as _;
use futures::future::JoinAll;
use vstd::prelude::Ghost;
use vstd::prelude::Tracked;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl VpSet {
    /// Stops all VPs at a deferred I/O completion boundary.
    ///
    /// Verus: as `stop`: if they were started, each VP's thread hands back
    /// a half recording the VP's saved state; otherwise nothing changes.
    /// Also on `Err` (the I/O peer failed; the stops are still drained).
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(
        requires
            old(self).wf(),
        ensures
            final(self).wf(),
            !final(self)@.started,
            final(self)@.vps.len() == old(self)@.vps.len(),
            final(self)@.capacity == old(self)@.capacity,
            old(self)@.started ==> final(self)@.all_saved(),
            !old(self)@.started ==> final(self)@ == old(self)@,
    ))]
    pub async fn stop_at_io_boundary(
        &mut self,
        release_io: mesh::OneshotSender<()>,
        io_completed: mesh::OneshotReceiver<()>,
    ) -> anyhow::Result<()> {
        let stops = self.started.then(|| {
            let stops = self
                .vps
                .iter()
                .map(|vp| {
                    // Through the mesh RPC wrapper, with placeholder ghosts.
                    let stopped = mesh_rpc_verus::call(
                        &vp.send,
                        VpEvent::Stop,
                        (),
                        Tracked::assume_new(),
                        Ghost::assume_new(),
                    );
                    async { stopped.recv().await.map(|((), _ghost)| ()).ok() }
                })
                .collect::<JoinAll<_>>();
            self.started = false;
            stops
        });

        // Every VP has observed a queued stop event before the deferred I/O is
        // completed, so the originating VP cannot re-enter guest execution.
        release_io.send(());
        let io_result = io_completed
            .await
            .context("deferred I/O completion channel closed");
        // `started` is already false, so a later stop cannot join these requests.
        // Drain them even if the I/O peer failed before acknowledging completion.
        if let Some(stops) = stops {
            stops.await;
        }
        io_result
    }
}

#[cfg(test)]
mod tests {
    use crate::partition_unit::vp_set::*;
    use test_with_tracing::test;
    use vm_topology::processor::VpInfo;

    fn vp_set(count: u32) -> (VpSet, Vec<VpRunner>) {
        let (halt, _halt_recv) = Halt::new();
        let mut vps = VpSet::new([None, None, None], Arc::new(halt), count as usize);
        let runners = (0..count)
            .map(|index| {
                vps.add(TargetVpInfo {
                    base: VpInfo {
                        vp_index: VpIndex::new(index),
                        vnode: 0,
                    },
                    #[cfg(guest_arch = "x86_64")]
                    apic_id: index,
                    #[cfg(guest_arch = "aarch64")]
                    mpidr: u64::from(index).into(),
                    #[cfg(guest_arch = "aarch64")]
                    gicr: None,
                    #[cfg(guest_arch = "aarch64")]
                    pmu_gsiv: None,
                })
            })
            .collect();
        (vps, runners)
    }

    fn boundary_waits_for_all_stops(io_succeeds: bool) {
        let (mut vps, mut runners) = vp_set(2);
        vps.start();
        for runner in &mut runners {
            assert!(matches!(runner.recv.try_recv(), Ok(VpEvent::Start)));
        }

        let (release, released) = mesh::oneshot();
        let (complete, completed) = mesh::oneshot();
        let mut boundary = Box::pin(vps.stop_at_io_boundary(release, completed));
        assert!(boundary.as_mut().now_or_never().is_none());
        released.now_or_never().unwrap().unwrap();
        let mut stops = runners.iter_mut().map(|runner| {
            let Ok(VpEvent::Stop(stop)) = runner.recv.try_recv() else {
                panic!("expected a stop request before releasing I/O");
            };
            stop
        });
        let first_stop = stops.next().unwrap();
        let second_stop = stops.next().unwrap();
        if io_succeeds {
            complete.send(());
        } else {
            drop(complete);
        }
        assert!(boundary.as_mut().now_or_never().is_none());
        first_stop.complete(());
        assert!(boundary.as_mut().now_or_never().is_none());
        second_stop.complete(());
        let result = boundary.as_mut().now_or_never().unwrap();
        if io_succeeds {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                "deferred I/O completion channel closed"
            );
        }
        drop(boundary);
        assert!(!vps.started);
        assert!(vps.stop().now_or_never().is_some());
        for runner in &mut runners {
            assert!(runner.recv.try_recv().is_err());
        }
    }

    #[test]
    fn successful_boundary_waits_for_all_stops() {
        boundary_waits_for_all_stops(true);
    }

    #[test]
    fn failed_boundary_waits_for_all_stops() {
        boundary_waits_for_all_stops(false);
    }

    #[test]
    fn boundary_waits_for_io_completion_after_all_stops() {
        for io_succeeds in [true, false] {
            let (mut vps, mut runners) = vp_set(1);
            vps.start();
            assert!(matches!(runners[0].recv.try_recv(), Ok(VpEvent::Start)));

            let (release, released) = mesh::oneshot();
            let (complete, completed) = mesh::oneshot();
            let mut boundary = Box::pin(vps.stop_at_io_boundary(release, completed));
            assert!(boundary.as_mut().now_or_never().is_none());
            released.now_or_never().unwrap().unwrap();
            let Ok(VpEvent::Stop(stop)) = runners[0].recv.try_recv() else {
                panic!("expected a stop request before releasing I/O");
            };
            stop.complete(());
            assert!(boundary.as_mut().now_or_never().is_none());

            if io_succeeds {
                complete.send(());
            } else {
                drop(complete);
            }
            let result = boundary.as_mut().now_or_never().unwrap();
            assert_eq!(result.is_ok(), io_succeeds);
            drop(boundary);
            assert!(!vps.started);
        }
    }

    #[test]
    fn failed_boundary_tolerates_dropped_runner() {
        let (mut vps, mut runners) = vp_set(2);
        vps.start();
        for runner in &mut runners {
            assert!(matches!(runner.recv.try_recv(), Ok(VpEvent::Start)));
        }
        let (release, released) = mesh::oneshot();
        let (complete, completed) = mesh::oneshot();
        let mut boundary = Box::pin(vps.stop_at_io_boundary(release, completed));
        assert!(boundary.as_mut().now_or_never().is_none());
        released.now_or_never().unwrap().unwrap();
        drop(complete);
        assert!(boundary.as_mut().now_or_never().is_none());
        drop(runners.pop().unwrap());
        assert!(boundary.as_mut().now_or_never().is_none());
        let Ok(VpEvent::Stop(stop)) = runners[0].recv.try_recv() else {
            panic!("expected a stop request before releasing I/O");
        };
        stop.complete(());
        assert!(boundary.as_mut().now_or_never().unwrap().is_err());
        drop(boundary);
        drop(runners);
        assert!(vps.teardown().now_or_never().is_some());
    }

    #[test]
    fn stopped_boundary_still_checks_io_completion() {
        let (mut vps, mut runners) = vp_set(1);
        let (release, released) = mesh::oneshot();
        let (complete, completed) = mesh::oneshot();
        let mut boundary = Box::pin(vps.stop_at_io_boundary(release, completed));
        assert!(boundary.as_mut().now_or_never().is_none());
        released.now_or_never().unwrap().unwrap();
        assert!(runners[0].recv.try_recv().is_err());
        drop(complete);
        assert!(boundary.as_mut().now_or_never().unwrap().is_err());
    }
}

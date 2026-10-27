// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! A verified state unit for a device that implements `StateUnitDevice`
//! (for now, the PIT). Verus only, for now.
//!
//! The unit holds the device in a `CloseableMutex` and answers each request
//! by the request protocol (`MeshProtocol for StateRequest`), doing the
//! FracGhost work in its own proof blocks through `DeviceGhost::ghost_mut`
//! (see "Design change: the FracGhost lives with the device" in
//! `SNAPSHOT-SPEC-DESIGN.md`).

use crate::StateRequest;
use crate::mesh_rpc_t::MeshProtocol;
use crate::mesh_rpc_t::complete;
use crate::mesh_rpc_t::split;
use closeable_mutex::CloseableMutex;
use closeable_mutex::LockInvariant;
use mesh::error::RemoteError;
use std::sync::Arc;
use vmcore::device_state::spec::DeviceGhost;
use vmcore::device_state::spec::StateUnitDevice;
use vmcore::save_restore::RestoreError;
use vmcore::save_restore::SaveError;
use vmcore::save_restore::SavedStateBlob;
use vmcore::save_restore::spec::UnitSnapshot;
use vstd::prelude::*;
use vstd::resource::Loc;
use vstd::resource::frac::FracGhost;

verus! {

/// The state unit of a device of type `T`.
#[verifier::reject_recursive_types(T)]
pub struct DeviceUnit<T> {
    device: Arc<CloseableMutex<T>>,
    /// The id of the device's FracGhost (its unit channel's constant).
    id: Ghost<Loc>,
}

impl<T: StateUnitDevice> DeviceUnit<T> {
    /// The id of the device's FracGhost.
    pub closed spec fn id(&self) -> Loc {
        self.id@
    }

    /// Well formed: the device's mutex has the unit's id as its key.
    pub closed spec fn wf(&self) -> bool {
        (*self.device).key() == self.id@
    }

    /// A unit for the device in `device`, whose FracGhost has id `id`.
    pub fn new(device: Arc<CloseableMutex<T>>, id: Ghost<Loc>) -> (unit: Self)
        requires
            (*device).key() == id@,
        ensures
            unit.wf(),
            unit.id() == id@,
    {
        DeviceUnit { device, id }
    }

    /// Answers `msg`, which came with `gin`, by the request protocol.
    /// (`Inspect` requests are answered by the caller.)
    pub fn handle(&mut self, msg: StateRequest, gin: Tracked<Option<FracGhost<UnitSnapshot>>>)
        requires
            old(self).wf(),
            StateRequest::request_ok(old(self).id(), msg, gin@),
            !(msg is Inspect),
        ensures
            final(self).wf(),
            final(self).id() == old(self).id(),
    {
        let Tracked(gin) = gin;
        match msg {
            StateRequest::Start(rpc) => {
                let tracked half = gin.tracked_unwrap();
                let mut g = CloseableMutex::close_verified(&self.device);
                let d = g.borrow_mut();
                proof {
                    d.lemma_lock_inv();
                }
                let ghost_field = d.ghost_mut();
                proof {
                    // With `StateUnits`' half the device holds the whole: it
                    // was stopped.
                    ghost_field.borrow_mut().frac.combine(half);
                    ghost_field.borrow_mut().frac.bounded();
                    ghost_field.borrow_mut().running = true;
                }
                d.start_device();
                proof {
                    d.lemma_lock_inv();
                }
                let _ = g.release();
                complete(rpc, Ok(()), Tracked(None::<FracGhost<UnitSnapshot>>));
            },
            StateRequest::Stop(rpc) => {
                let mut g = CloseableMutex::close_verified(&self.device);
                let d = g.borrow_mut();
                proof {
                    d.lemma_lock_inv();
                    // TRUSTED: a unit receives `Stop` only while it runs
                    // ("`StateUnits`' unit states are the units' real
                    // states").
                    assume(d.ghost().running);
                }
                d.stop_device();
                proof {
                    d.lemma_snapshot_known();
                }
                let ghost now = d.snapshot();
                let ghost_field = d.ghost_mut();
                let tracked half;
                proof {
                    // Record the state, and give `StateUnits` a half.
                    ghost_field.borrow_mut().frac.update(now);
                    half = ghost_field.borrow_mut().frac.split();
                    ghost_field.borrow_mut().running = false;
                }
                proof {
                    d.lemma_lock_inv();
                }
                let _ = g.release();
                complete(rpc, (), Tracked(Some(half)));
            },
            StateRequest::Save(rpc) => {
                let tracked lent = gin.tracked_unwrap();
                let mut g = CloseableMutex::close_verified(&self.device);
                let d = g.borrow_mut();
                proof {
                    d.lemma_lock_inv();
                }
                let ghost_field = d.ghost_mut();
                proof {
                    // The lent half agrees with the device's, which records
                    // its state.
                    ghost_field.borrow().frac.agree(&lent);
                }
                let result = d.save_blob();
                proof {
                    d.lemma_lock_inv();
                }
                let _ = g.release();
                let reply = match result {
                    Ok(blob) => Ok(Some(blob)),
                    Err(err) => Err(save_error(err)),
                };
                complete(rpc, reply, Tracked(Some(lent)));
            },
            StateRequest::Restore(rpc) => {
                let tracked lent = gin.tracked_unwrap();
                let (blob, rpc) = split::<
                    SavedStateBlob,
                    Result<(), RemoteError>,
                    Option<FracGhost<UnitSnapshot>>,
                >(rpc);
                let mut g = CloseableMutex::close_verified(&self.device);
                let d = g.borrow_mut();
                proof {
                    d.lemma_lock_inv();
                }
                let ghost_field = d.ghost_mut();
                proof {
                    // With the lent half the device holds the whole: it is
                    // stopped.
                    ghost_field.borrow_mut().frac.combine(lent);
                    ghost_field.borrow_mut().frac.bounded();
                }
                let result = d.restore_blob(blob);
                let ghost now = d.snapshot();
                let ghost_field = d.ghost_mut();
                let tracked half;
                proof {
                    // Record the restored state, and give a half back.
                    ghost_field.borrow_mut().frac.update(now);
                    half = ghost_field.borrow_mut().frac.split();
                }
                proof {
                    d.lemma_lock_inv();
                }
                let _ = g.release();
                let reply = match result {
                    Ok(()) => Ok(()),
                    Err(err) => Err(restore_error(err)),
                };
                complete(rpc, reply, Tracked(Some(half)));
            },
            StateRequest::Reset(rpc) => {
                let tracked lent = gin.tracked_unwrap();
                let mut g = CloseableMutex::close_verified(&self.device);
                let d = g.borrow_mut();
                proof {
                    d.lemma_lock_inv();
                }
                let ghost_field = d.ghost_mut();
                proof {
                    ghost_field.borrow_mut().frac.combine(lent);
                    ghost_field.borrow_mut().frac.bounded();
                }
                d.reset_device();
                let ghost now = d.snapshot();
                let ghost_field = d.ghost_mut();
                let tracked half;
                proof {
                    ghost_field.borrow_mut().frac.update(now);
                    half = ghost_field.borrow_mut().frac.split();
                }
                proof {
                    d.lemma_lock_inv();
                }
                let _ = g.release();
                complete(rpc, Ok(()), Tracked(Some(half)));
            },
            StateRequest::AdvanceTime(rpc) => {
                // `StateUnitDevice` devices keep no time of their own
                // (`advances_time` is false): nothing changes, and the lent
                // half goes back.
                let tracked lent = gin.tracked_unwrap();
                let (_duration, rpc) = split::<
                    std::time::Duration,
                    Result<(), RemoteError>,
                    Option<FracGhost<UnitSnapshot>>,
                >(rpc);
                complete(rpc, Ok(()), Tracked(Some(lent)));
            },
            StateRequest::QuiesceInput(rpc) => {
                complete(rpc, Ok(()), Tracked(None::<FracGhost<UnitSnapshot>>));
            },
            StateRequest::ResumeInput(rpc) => {
                complete(rpc, Ok(()), Tracked(None::<FracGhost<UnitSnapshot>>));
            },
            StateRequest::Inspect(_) => {},
        }
    }
}

/// A save error, for the reply.
#[verifier::external_body]
fn save_error(err: SaveError) -> RemoteError {
    RemoteError::new(err)
}

/// A restore error, for the reply.
#[verifier::external_body]
fn restore_error(err: RestoreError) -> RemoteError {
    RemoteError::new(err)
}

} // verus!

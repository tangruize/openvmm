// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Snapshot quiesce transactions: gating host input around the snapshot
//! vCPU boundary, bounded quiesce of every unit for save, and restart after a
//! failed save.

use super::State;
use super::StateRequest;
use super::StateUnits;
use super::state_change;
use anyhow::Context as _;
use futures::future::join_all;
use mesh::CancelContext;
use mesh::rpc::FailableRpc;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

/// A bounded quiesce failed and the VM must not be saved.
#[derive(Debug, Error)]
#[error("failed to quiesce state units for save: {failures:?}")]
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verifier::external_derive)]
pub struct QuiesceForSaveError {
    failures: Vec<(String, String)>,
    uncertain: bool,
}

// Verus: the error's view, from its fields.
#[cfg(verus_keep_ghost)]
verus! {

impl View for QuiesceForSaveError {
    type V = crate::spec::QuiesceForSaveErrorView;

    closed spec fn view(&self) -> crate::spec::QuiesceForSaveErrorView {
        crate::spec::QuiesceForSaveErrorView { uncertain: self.uncertain }
    }
}

} // verus!

impl QuiesceForSaveError {
    /// Returns whether any unit may have partially changed ownership or state.
    ///
    /// When true, the VM cannot be resumed safely and must be terminated.
    #[cfg_attr(verus_keep_ghost, verus_verify)]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        ensures
            result == self@.uncertain,
    ))]
    pub fn has_uncertain_state(&self) -> bool {
        self.uncertain
    }
}

impl StateUnits {
    /// Stops host-input producers before establishing a snapshot vCPU boundary.
    pub async fn quiesce_input_for_save(&self, timeout: Duration) -> anyhow::Result<()> {
        self.run_input_op("quiesce_input", timeout, StateRequest::QuiesceInput)
            .await
    }

    /// Resumes host-input producers before releasing a failed snapshot boundary.
    pub async fn resume_input_after_save(&self, timeout: Duration) -> anyhow::Result<()> {
        self.run_input_op("resume_input", timeout, StateRequest::ResumeInput)
            .await
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    async fn run_input_op(
        &self,
        operation: &'static str,
        timeout: Duration,
        request: impl Copy + FnOnce(FailableRpc<(), ()>) -> StateRequest,
    ) -> anyhow::Result<()> {
        let operations = {
            let inner = self.inner.lock();
            inner
                .units
                .values()
                .map(|unit| {
                    let name = unit.name.clone();
                    let operation =
                        state_change(name.clone(), unit.send.clone(), request, Some(()));
                    async move { (name, operation.await) }
                })
                .collect::<Vec<_>>()
        };
        let mut context = CancelContext::new().with_timeout(timeout);
        let results = context
            .until_cancelled(join_all(operations))
            .await
            .with_context(|| format!("{operation} timed out"))?;
        let mut failures = Vec::new();
        for (name, result) in results {
            match result {
                Ok(Some(Ok(()))) => {}
                Ok(Some(Err(error))) => failures.push(format!("{name}: {error}")),
                Ok(None) => failures.push(format!("{name}: request was not sent")),
                Err(error) => failures.push(format!("{name}: {error}")),
            }
        }
        anyhow::ensure!(
            failures.is_empty(),
            "{operation} failed: {}",
            failures.join("; ")
        );
        Ok(())
    }

    /// Stops all units in reverse dependency order within `timeout`.
    ///
    /// A timeout or communication failure after a stop request is sent leaves
    /// ownership uncertain. The returned error reports this via
    /// [`QuiesceForSaveError::has_uncertain_state`]; callers must terminate the
    /// VM rather than attempt rollback in that case.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
            old(self)@.running,
        ensures
            final(self).wf(),
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units.dom() == old(self)@.units.dom(),
            // On success the units are stopped, each has recorded its state
            // and given back its half (each received `Stop`); on failure
            // they are still marked running.
            result is Ok ==> {
                &&& !final(self)@.running
                &&& final(self)@.all_recorded()
                &&& final(self)@.holds_halves()
            },
            result is Err ==> final(self)@.running,
            // Every failure is uncertain: a unit fails only when its `Stop`
            // fails or times out (uncertain), or when a unit depending on it
            // did not stop, which in turn started with an uncertain one.
            result matches Err(e) ==> e@.uncertain,
    ))]
    pub async fn quiesce_for_save(&mut self, timeout: Duration) -> Result<(), QuiesceForSaveError> {
        assert!(self.running);

        enum UnitResult {
            Stopped,
            DependencyFailed,
            Uncertain(anyhow::Error),
        }

        let context = CancelContext::new().with_timeout(timeout);
        let mut operations = Vec::new();
        let ready_set;
        {
            let mut inner = self.inner.lock();
            ready_set = inner.ready_set(None);
            for (&id, unit) in &mut inner.units {
                assert_eq!(
                    unit.state,
                    State::Running,
                    "unit {} is not running before save quiesce",
                    unit.name,
                );
                let name = unit.name.clone();
                let dependencies = unit.dependents.clone();
                let ready_set = ready_set.clone();
                let mut context = context.clone();
                let stop = state_change(
                    name.clone(),
                    unit.send.clone(),
                    StateRequest::Stop,
                    Some(()),
                );
                operations.push(async move {
                    if !ready_set.wait("quiesce_for_save", id, &dependencies).await {
                        ready_set.done(id, false);
                        return (name, id, UnitResult::DependencyFailed);
                    }

                    let result = match context.until_cancelled(stop).await {
                        Ok(Ok(Some(()))) => UnitResult::Stopped,
                        Ok(Ok(None)) => {
                            UnitResult::Uncertain(anyhow::anyhow!("stop request was not sent"))
                        }
                        Ok(Err(error)) => UnitResult::Uncertain(error.into()),
                        Err(reason) => UnitResult::Uncertain(reason.into()),
                    };
                    ready_set.done(id, matches!(result, UnitResult::Stopped));
                    (name, id, result)
                });
                unit.state = State::Stopping;
            }
        }

        let start = Instant::now();
        let results = join_all(operations).await;
        tracing::info!(duration = ?Instant::now() - start, "save quiesce complete");

        let mut failures = Vec::new();
        let mut uncertain = false;
        let mut inner = self.inner.lock();
        for (name, id, result) in results {
            let Some(unit) = inner.units.get_mut(&id) else {
                continue;
            };
            match result {
                UnitResult::Stopped => unit.state = State::Stopped,
                UnitResult::DependencyFailed => {
                    unit.state = State::Running;
                    failures.push((
                        name.to_string(),
                        "a dependent unit did not quiesce".to_owned(),
                    ));
                }
                UnitResult::Uncertain(error) => {
                    unit.state = State::QuiesceUncertain;
                    uncertain = true;
                    failures.push((name.to_string(), format!("{error:#}")));
                }
            }
        }
        drop(inner);

        if failures.is_empty() {
            self.running = false;
            Ok(())
        } else {
            Err(QuiesceForSaveError {
                failures,
                uncertain,
            })
        }
    }

    /// Restarts every unit after a save failure that occurred after a fully
    /// successful quiesce but before publication committed.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(result =>
        requires
            old(self).wf(),
            // It holds the halves it gives back to the units (none lent).
            old(self)@.holds_halves(),
        ensures
            final(self).wf(),
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units.dom() == old(self)@.units.dom(),
            // Succeeds only from a stopped VM (it checks), and restarts it.
            result is Ok ==> !old(self)@.running && final(self)@.running,
            // On failure, units may be partly restarted; nothing more is
            // promised (the VM must be terminated).
    ))]
    pub async fn resume_after_failed_save(&mut self, timeout: Duration) -> anyhow::Result<()> {
        anyhow::ensure!(!self.running, "VM was not successfully quiesced for save");
        anyhow::ensure!(
            self.inner
                .lock()
                .units
                .values()
                .all(|unit| unit.state == State::Stopped),
            "state-unit rollback is unsafe because not every unit is cleanly stopped"
        );

        enum StartResult {
            Started,
            DependencyFailed,
            Failed(anyhow::Error),
            Uncertain(anyhow::Error),
        }

        let context = CancelContext::new().with_timeout(timeout);
        let mut operations = Vec::new();
        let ready_set;
        {
            let mut inner = self.inner.lock();
            ready_set = inner.ready_set(None);
            for (&id, unit) in &mut inner.units {
                let name = unit.name.clone();
                let dependencies = unit.dependencies.clone();
                let ready_set = ready_set.clone();
                let mut context = context.clone();
                let start = state_change(
                    name.clone(),
                    unit.send.clone(),
                    StateRequest::Start,
                    Some(()),
                );
                operations.push(async move {
                    if !ready_set.wait("snapshot_rollback", id, &dependencies).await {
                        ready_set.done(id, false);
                        return (name, id, StartResult::DependencyFailed);
                    }

                    let result = match context.until_cancelled(start).await {
                        Ok(Ok(Some(Ok(())))) => StartResult::Started,
                        Ok(Ok(Some(Err(error)))) => StartResult::Failed(error.into()),
                        Ok(Ok(None)) => {
                            StartResult::Uncertain(anyhow::anyhow!("start request was not sent"))
                        }
                        Ok(Err(error)) => StartResult::Uncertain(error.into()),
                        Err(reason) => StartResult::Uncertain(reason.into()),
                    };
                    ready_set.done(id, matches!(result, StartResult::Started));
                    (name, id, result)
                });
                unit.state = State::Starting;
            }
        }

        let results = join_all(operations).await;
        let mut failures = Vec::new();
        let mut inner = self.inner.lock();
        for (name, id, result) in results {
            let Some(unit) = inner.units.get_mut(&id) else {
                continue;
            };
            match result {
                StartResult::Started => unit.state = State::Running,
                StartResult::DependencyFailed => {
                    unit.state = State::Stopped;
                    failures.push(format!("{name}: a dependency did not restart"));
                }
                StartResult::Failed(error) => {
                    unit.state = State::QuiesceUncertain;
                    failures.push(format!("{name}: {error:#}"));
                }
                StartResult::Uncertain(error) => {
                    unit.state = State::QuiesceUncertain;
                    failures.push(format!("{name}: {error:#}"));
                }
            }
        }
        drop(inner);

        anyhow::ensure!(
            failures.is_empty(),
            "snapshot rollback could not safely restart all state units: {}",
            failures.join("; ")
        );
        self.running = true;
        Ok(())
    }
}

/// Lending the units' halves (Verus only). A stopped `StateUnits` lends all
/// its halves to whoever must know that no unit changes its state until they
/// come back: the VM worker lends them to the VM controller with a snapshot
/// (`VmRpc`'s protocol). While they are out, `StateUnits` holds none
/// (`missing_halves` is every unit), so it cannot restart, reset, restore,
/// or save the units.
#[cfg(verus_keep_ghost)]
#[verus_verify]
impl StateUnits {
    /// Lends out every unit's half. The units, stopped, keep their own
    /// halves; the view no longer knows what they record (`Unknown`); the
    /// lent halves record what it knew.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).wf(),
            !old(self)@.running,
            old(self)@.holds_halves(),
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.units == old(self)@.units.map_values(
                |snapshot: crate::spec::UnitSnapshot| crate::spec::UnitSnapshot::Unknown,
            ),
            result@.wf(),
            final(self)@.missing_halves == result@.units,
            result@.records() == old(self)@.units,
    )]
    pub fn lend_halves(&mut self) -> Tracked<crate::spec::LentHalves> {
        // PROOF TODO: replace this placeholder (`assume_new()`) with the
        // real ghost: the halves taken out of `units_ghost`, with their
        // entries in its registry.
        Tracked::assume_new()
    }

    /// Takes back lent halves: those of the units missing their half (the
    /// lent ones still collected; the halves of units removed since are
    /// dropped). Afterwards it holds every unit's half, and the returned
    /// units record what their halves record.
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self).wf(),
            !old(self)@.running,
            lent@.wf(),
            old(self)@.missing_halves.submap_of(lent@.units),
        ensures
            final(self).wf(),
            final(self)@.running == old(self)@.running,
            final(self)@.inventory == old(self)@.inventory,
            final(self)@.advances_time == old(self)@.advances_time,
            final(self)@.holds_halves(),
            final(self)@.units == old(self)@.units.union_prefer_right(
                lent@.records().restrict(
                    old(self)@.missing_halves.values().map(
                        |entry: (Seq<char>, vstd::resource::Loc, bool)| entry.0,
                    ),
                ),
            ),
    )]
    pub fn return_halves(&mut self, lent: Tracked<crate::spec::LentHalves>) {}
}

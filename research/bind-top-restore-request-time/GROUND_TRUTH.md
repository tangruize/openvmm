# Production restore request-time binding

## Result and admission boundary

Diagnostic only. The requested production decoder refinement is not established.
All required checks ran once on the unchanged intake. The specification and
executable admission checks fail, and boundary checking rejects manifest
provenance before producing an inventory. These failures are not waived by
the approved Duration package or historical verification.

No production experiment, proof edit, marker change, checker repair, package
application, or frozen-input modification was performed. The unresolved facts
below are recorded before any decoder experiment; experiments are deferred
because the current admission gates cannot support a bounded proof handoff.

## Source-supported but unproved connections

1. The production `decoded_restore_request_view` in
   `openvmm/openvmm_core/src/worker/dispatch.proof.rs:28` is still entirely
   uninterpreted. Its `has_time_adjustment` must equal the actual
   `restore_time.is_some()`. For `Some((duration, _, _))`, its `downtime_ns`
   must equal `vmcore::vmtime_duration`'s approved
   `elapsed_nanoseconds(&duration)`, including any remainder modulo 100.
   Neither binding follows from the present declaration.
2. TOP references that exact decoder in its requirement and successful
   postcondition at `dispatch.rs:3807,3817`. Its real body branches on the
   same `restore_time` argument, passes the selected `downtime` to
   `state_units.advance_time`, `partition_unit.advance_tsc`, and
   `partition.advance_snapshot_time`, and does not perform these adjustments
   when the option is absent (`dispatch.rs:3867-3900`). This supports a
   time-policy refinement but is not a body proof or component-clock equality.
3. The frozen semantic chain is
   `snapshot_restore_success` -> `snapshot_restore_result` ->
   `restore_snapshot_projection` -> `restored_virtual_time`
   (`dispatch.spec.rs:178-250`). The last function returns exact
   `request.downtime_ns` for adjusted elapsed time and zero otherwise.
   Its keeper-tick projection separately floors by 100 and wraps at 2^64.
   A proof must use the production decoder in this chain, not assume its
   missing binding or replace TOP with a history-only helper.
4. `vm/vmcore/src/vmtime_duration/observation.spec.rs:9-29` now contains
   the two sanctioned Duration-component observations and the sanctioned
   `Duration::as_nanos` specification. `elapsed_nanoseconds` is defined
   as seconds times one billion plus subsecond nanoseconds. The existing
   manifest lists exactly these three Duration declarations alongside the
   prior sanctioned load and File declarations. No further trust is needed
   merely to name the exact observation of this Duration; attachment to the
   production decoder remains unproved.
5. Snapshot payload decoding is independent debt: the request's saved-state
   View must eventually describe the actual `SavedState`. Selected VP
   correspondence, the separate load-side decoder, and prepared/loaded
   component representations are not established by the time-policy facts.
   `InitializedVm::load` uses its separate decoder in its sanctioned
   contract (`dispatch.rs:1540-1555`) and forwards its actual `restore_time`
   to TOP (`dispatch.rs:3331-3333`). That call does not prove equality of the
   two presently uninterpreted request Views.
6. Completed-restore history, cancellation/error behavior, lifecycle
   invalidation, keeper/task ownership, and `loaded_vm_representation`
   remain unproved. A successful guard acquisition at the end of TOP is
   not yet connected to a retained-Duration receipt or a loaded-state View.
   No equality of remote component clocks with keeper time is inferred.

The prior reviewed handoffs `a535358753b0` and `2548df2cad98` were read for
their retained-Duration evidence. The former establishes feasibility of
erased retention, not production attachment. The latter explicitly leaves
its projection helper conditional on request-decoding bindings. Their
historical numeric-interface obstruction is superseded by the interface
actually present here; their unproved attachment obligations are not.

## Current machine evidence

Commands used the exported `ARGUS_SKILL_PYTHON` from the repository root.
Complete current logs are under `.verus_agent/cache/checks/`.

| Required command module and arguments | Exit | Observation |
| --- | --- | --- |
| `argus_verus.tools.checks.make_verify --crate-root .` | 0 | 9.438 seconds; `0 verified, 0 errors (partial verification with --verify-*)`. The existing wrapper selects scaffolded TOP and disables lifetime checking. This is neither full-crate proof evidence nor a nonzero focused proof. |
| `argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check` | 1 | 0.011 seconds; `scope_manifest.json was last modified by an autonomous agent commit`. The reported zero marker counts are not a completed inventory. |
| `argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent` | 1 | 1.831 seconds; 137 reports: 125 BOTTOM and 12 TOP. 135 reference `research/` paths; the other two name `VmTime::view` and `VpIndex::view`. No report is reclassified as a pass. |
| `argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent` | 1 | 4.818 seconds; `ProcessorTopology::to_config: MISMATCH`. Its displayed comparison pairs frozen X86 with working Aarch64. The direct frozen-to-working diff of `dispatch.rs` contains only pre-existing datatype proof annotations and formatting, not a change to either topology method. The required check nevertheless failed. |

The prescribed call-structure command,
`argus_verus.callgraph.graph --project . build --out .verus_agent/cache/callgraph.json`,
failed because `.verus_agent/proof_state.json` is absent. The source connections
above are direct inspection, not a successful type-aware graph result. No
graph rebuild or integrity-tool repair was attempted.

The manifests parse as JSON and are present, but provenance admission fails.
Direct frozen-to-working inspection shows no difference in either manifest,
`dispatch.spec.rs`, `dispatch.proof.rs`, or the Duration interface directory.
This observation does not override either failed drift check.

No strict focused decoder/projection verification was attempted after admission
failed. In particular, there is no new result with lifetime and trait-conflict
checking enabled and no claim that the mission's native proof criterion passed.
No rlimit annotation was added or changed.

## Retained obligations and next owner

No temporary marker was added, removed, or moved. TOP's `external_body` and
the four representation bridges in `dispatch.proof.rs` remain:

- `decoded_restore_request_view`: actual payload and optional-Duration decoding;
  used by TOP, to be replaced by a source-supported definition and proved
  decoder/projection connection.
- `decoded_load_restore_request_view`: optional snapshot and VP-selection
  correspondence; used by the sanctioned load contract, unchanged here.
- `initialized_vm_representation`: prepared component Views; consumed by
  `InitializedVm::view`, requiring component representation proofs.
- `loaded_vm_representation`: restored components, virtual time, and lifecycle;
  consumed by `LoadedVm::view`, requiring real state/lifecycle attachment.

Other pre-existing admission scaffolding is unchanged; boundary failure prevents
a current complete inventory. No contract was weakened or strengthened and no
new precondition, invariant, assumption, or external declaration was introduced.

Reviewer must classify the current-baseline admission failures before this
bounded proof task can produce an admissible handoff. No freeze request or
operator decision is proposed: no evidence here requires changing the frozen
semantics, executable behavior, or sanctioned Duration interface.

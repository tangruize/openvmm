# Snapshot publication and restore: reviewable Spec/View checkpoint

This is a partial specification investigation, not a production-body, VM or filesystem proof. Snapshot production source remains identical to baseline `c9f659c36c8cb36d5aac745f7c26808296689f3f`. No new production bug is claimed; the repaired defects were in proposed specifications. Expert review is pending.

The reusable controller is [argus-system-proof-agent at `9ea9e73`](https://github.com/tangruize/argus-system-proof-agent/tree/9ea9e73452d363480f006335cbeae7249c68ec33), including pinned native Argus and the unmodified supplied checker. Its [OpenVMM guide](https://github.com/tangruize/argus-system-proof-agent/blob/9ea9e73452d363480f006335cbeae7249c68ec33/docs/OPENVMM.md) covers first-clone provisioning and scoped replay.

Start with the [module/API/observation index](specs/snapshot/meaning.md) and [formal aggregate](specs/snapshot/snapshot.rs). Definitions belong to the module; `proofs/snapshot.rs` is only a compatibility Verus entrypoint. Checker configurations now live beside the module aggregate rather than pointing a source-discovery tool at an include-only wrapper.

| Rejected meaning | Current meaning and separating case |
|---|---|
| Destination-absence failure after staging is always rollback-safe. | A source-memory alias plus failed cleanup produces `CleanupUncertain`; `destination_absence_cleanup_uncertain_witness` separates the retained alternative from the repair. Native-04's independent Reviewer found this error. |
| Any failed `stage_snapshot` is rollback-safe because its caller received no staging owner. | The callee can own staging resources internally and return `Err(staging.rollback(error))`; the caller's `?` propagates `CleanupUncertain`. `stage_cleanup_uncertain_failure_witness` separates this alternative. Follow-up source inspection found the missing callee case, and native-05 repaired it. |
| Equal lengths or arbitrary `u64` state-content IDs establish the saved-state payload observation. | The View uses `Seq<u8>` and the executable model returns `Vec<u8>` contents. `same_length_state_substitution_witness_pair` constructs lawful model inputs containing `[1]` and `[2]`: equal weak Views, different bytes. |

`StageSnapshotOutcomeView` is now an exhaustive enum. `publication_preserves_every_stage_failure` checks every failure category without assuming a separate one-hot Boolean invariant. Rollback-safe does not promise removal of independent leftover artifacts, and not committed refers to this attempt rather than absence of any pre-existing destination.

## Feedback and its exact scope

The active proposal is [snapshot-generation-access-v1](research/system-proof/candidates/snapshot-generation-access.json), revision 6 in the original private registry. Actual function-scoped correctness and the supplied checker's `abstract_determinism` question both passed for `opened_generation_access_view`, using the configured project-local Verus, Rust 2024 and matching Z3. They establish the model's stated observation and uniqueness for equal input Views, not production implementation correctness or adequacy of the whole API.

The formal aggregate reported eight verified goals and zero errors. The separately selected `publication_preserves_every_stage_failure` reported one verified goal and zero errors. These counts are not completed APIs or TOP progress. The constructive byte witness and both retired publication predicates remain in the aggregate for review.

Private trace references are `.system-proof/evidence/verification-c2c371fcf28e4de6957707e6c33728fc/correctness.json`, `.system-proof/evidence/specdet-0a2193cd62854de2/completeness.json`, and `.system-proof/evidence/verification-e3284ebcafd64057b3560960348bafce/receipt.json`. Raw execution/provider records are not published; a new clone should reproduce the selected questions rather than import these machine-local IDs.

The weaker observation was also attacked, not merely described. The mechanical supplied-checker attempt was inconclusive because its concrete counterexample catalog did not cover the value layout. Bounded live attempts then exposed Copilot terminal rendering inserting line breaks inside JSON keys at 100 columns. The reusable vertical now provides `system-proof-copilot-raw`, which preserves the supplied provider's isolation and obtains raw JSONL assistant content instead. The subsequent two-request attempt had zero provider errors and zero parse errors, but both constructor proposals were rejected by the supplied validator. The result remained inconclusive: this is not a checker-confirmed counterexample. The separately Verus-checked constructive byte case is the formal refutation evidence available here.

Those assistance attempts are retained history on the earlier byte-observation model, before the final enum/attribution cleanup; their receipts are not rebound to newer bytes. The [deferred weak proposal](research/system-proof/candidates/snapshot-generation-length-only.json) and its bounded configuration permit an explicit fresh replay. Repeating an unchanged failed attack just to refresh IDs is not a progress requirement.

## Open source relations and next work

The [portable current frontier](research/system-proof/nodes/snapshot-generation-access.json) contains thirteen reachable, unresolved obligations. Stage construction, rename preservation and restore consumption now depend on the shared representation. The saved-state source field/accessor relation is a concrete sub-obligation. Intended-capture establishment is downstream of publication, not circularly assumed to define the View. Earlier per-mission node/proposal files on run are historical patches, not the current import surface.

The numeric artifact IDs, handle flags and supplied check outcomes remain model placeholders. Actual `OpenedSnapshot` field/type correspondence, constructor guarantees, platform directory/file generation semantics, no-replace and cleanup effects, committed parent-sync failure, and the stopped-capture/caller relation remain open. Carrying an intended-capture Boolean does not establish membership. Metadata equality is not content authentication.

Historical conditional fragments and blocked attempts remain at [run checkpoint 699b3154](https://github.com/tangruize/openvmm/tree/699b3154e3a85fbcaced23a6d8280083d87f593e). The current [rollback receiver reproducer](proofs/attempts/rollback_receiver_repro.rs) has stub types, an unchecked cleanup callee and omitted logging; it is intentionally not labelled an exact source-body proof.

The [next bounded task](research/system-proof/continue-source-representation.txt) targets a real source/representation connection, not another renamed model. Native-04 and native-05 are prior investigation reviews; final publication approval binds the exact prepared diff separately. An AI-reviewed frozen snapshot or review tag is not expert approval, and continuing run work must preserve that distinction.

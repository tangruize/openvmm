# Snapshot target-binding ledger

This ledger is part of the initial specification. A model theorem is not accepted unless the corresponding real target body or a reviewed non-property-replacing implementation contract consumes it.

| Target | Role in TOP | Required checked bridge |
| --- | --- | --- |
| `VmController::handle_guest_snapshot_request` | Authorizes capture, obtains the worker boundary, writes/publishes artifacts, and decides rollback or termination | Connect the successful controller result to one committed manifest/state/RAM generation and retain failure distinctions |
| `LoadedVm::run` `VmRpc::QuiesceForSnapshot` arm | Dispatches the real capture RPC | Show the arm calls `quiesce_for_snapshot` and returns exactly its response/error classification |
| `LoadedVm::quiesce_for_snapshot` | Establishes stopped component/VP state, captures time, saves component state, and flushes RAM | Connect each ordered step to one capture cut; classify every retained-state mutation path through publication |
| `LoadedVm::save` | Produces component payloads and inventory | Connect actual `StateUnits::save` outputs and `inventory()` to the shared component projection |
| Snapshot publish functions | Stage and atomically publish manifest, state and RAM artifacts | Connect the commit point to `SnapshotArtifactsView`; callback success or staging existence is insufficient |
| `prepare_snapshot_restore_for_config` | Validates artifacts and prepares exact COW RAM, guards, state and time inputs | Establish `valid_snapshot`, compatibility and prepared-input predicates from actual validation |
| `InitializedVm::load(Some(_))` | Constructs the destination and returns the restored `LoadedVm` | Show the `Some` branch passes its actual `SavedState` to `restore_from_snapshot` and returns only after that helper succeeds |
| `LoadedVm::restore_from_snapshot` | Snapshot-specific checked restore core | Establish the success/failure contract across begin, component restore, time adjustment and stopped return |
| `LoadedVm::begin_snapshot_restore` | Validates pre-restore partition/time conditions and prepares restore policy | Connect its checks and TSC omission to the declared restore environment |
| `LoadedVm::restore` | Validates inventory and applies component payloads | Connect actual routing/consumption to stable component identity and payload relations |
| `LoadedVm::finish_snapshot_restore` | Applies time ABI adjustments and acquires the VP stop guard | Connect the exact clock transformation and protected return flags |
| `StateUnits::quiesce_for_save` | Coordinates the component capture boundary | Establish completed/stable versus rollback-safe/uncertain outcomes from real unit transitions |
| `StateUnits::save` | Collects component payloads | Establish name/payload ordering, completeness and error propagation |
| `StateUnits::restore` | Routes and consumes component payloads | Establish exact name matching, leftovers, partial effects and failure nonpublication |
| `StateUnits::validate_inventory` | Checks destination compatibility | Establish that accepted inventories match the actual registered units under the version's supported empty-inventory behavior |

## Non-acceptance rules

- Do not accept `snapshot.rs` verification counts without a checked path to these bodies.
- Do not use a ghost record populated from expected values as `project_vm`.
- Do not prove `restore_from_snapshot` while leaving the real `load(Some(_))` call unverified or merely source-anchored.
- Do not claim whole-snapshot save from `SavedState` alone; RAM generation and transactional publication are part of the TOP.
- Do not treat logging/tracing bodies as semantic obligations, but do preserve argument evaluation and every state/error/control-flow decision.

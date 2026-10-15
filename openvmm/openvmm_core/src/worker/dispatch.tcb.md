# Initial snapshot trust boundary

The supplied contracts deliberately keep the OpenVMM-owned snapshot lifecycle outside the trust boundary. The campaign must prove or explicitly account for capture-cut selection, state-unit quiesce/save/restore, inventory validation, RAM flush and backing compatibility, time adjustment, VP-count selection, error propagation, cleanup, publication ordering, and the connection from `InitializedVm::load(Some(_))` to `LoadedVm::restore_from_snapshot`.

The initial TCB may include Verus, Rust compilation semantics, the selected hypervisor and operating-system APIs at their documented interface boundaries, filesystem durability primitives once their assumptions are stated, cryptographic primitives used as specified, and hardware behavior required by the supported configuration. Opaque component payloads and external resources need explicit contracts; opacity alone does not make OpenVMM orchestration trusted.

The uninterpreted representation functions in `dispatch.proof.rs` are seed boundaries, not completed correspondence proofs. They must be refined, constrained, or replaced by source-grounded Views before an end-to-end result can claim implementation correspondence.

Non-semantic logging and diagnostic formatting may be isolated behind centralized `verus_keep_ghost` wrappers. Conditions, argument evaluation with semantic effects, state changes, result classification, cleanup, recovery, and return paths must remain visible to verification.

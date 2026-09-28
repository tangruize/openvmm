# Snapshot publication and restore specification investigation

Develop useful, source-grounded module specifications for OpenVMM's snapshot publication and restore-side artifact access. A snapshot consumer needs to understand what a completed or failed save makes available, what a successful restore-side open establishes, and under what assumptions saved state and memory belong to the intended capture. Begin with the snapshot publication TOP and use shared module Views and dependencies to connect restore access.

Use the current production source and documentation under `openvmm/openvmm_helpers/src/snapshot.rs` and `openvmm/openvmm_helpers/src/snapshot/`, following relevant callers where necessary. Investigate meaningful candidate guarantees, exercise them with actual attacks and implementation-proof feedback, and explain what should be promised rather than simply matching the easiest theorem.

This first delivery is a reviewable specification, not a complete VM, guest, filesystem-crash, or device proof. Keep original production behavior unchanged. Configuring builds and adding ghost/proof files is allowed. Propose any necessary executable or verifier change explicitly with its effects and evidence; do not quietly weaken the target. The editable project-local Verus checkout is available for genuine limitations.

Retain initial candidates, failed attempts, source correspondence, and unresolved assumptions. Produce `review.md` with an understandable comparison and next proof dependencies. Expert judgment remains pending. Do not publish raw provider logs or change unrelated branches.

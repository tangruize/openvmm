# Snapshot spec TODO

## Remove all free spec functions — resolved

There should be no free spec functions in the snapshot spec. Abstractions of
concrete types belong on the type (for example a `View` impl, so the spec
writes `x@`), not in free `spec fn`s. Free lemmas (`proof fn`) are acceptable
where they make sense.

Resolved by removing the earlier specification: all 13 free spec functions
were in the deleted `.proof.rs` files (still in git at `d326b4d6f`). The rule
stands for the new spec; see `SNAPSHOT-SPEC-DESIGN.md`.

Constraint to keep in mind: Rust's orphan rule allows `impl View for T` only in
the crate defining `T` or the crate defining `View` (`vstd`). `SavedState` and
other `openvmm_defs` types need their `View` impls in `openvmm_defs`; foreign
types need another approach.

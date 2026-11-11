# CODE-OVERVIEW.md: places found out of date while writing SPEC-DESIGN.md

(Found 2026-11-10 by the agent that drafted `SPEC-DESIGN.md`; not fixed.)

- "Lock invariants the spec needs": says the session's locks and
  `negotiation` get rules (they are still `parking_lot`, no rule); says each
  open file's `file` lock is "to decide when we work on confinement"
  (confinement was done without it); says the ~20 `get_path()` call sites
  "change to that form" (in fact `get_path()` returns a copy, and the call
  sites did not change).
- "Properties a spec could promise": says properties 1-4 and 6 "the spec
  covers", against the status table (identity, limits, snapshots not
  started).
- Open item 7 lists the placeholders in `new_lock` and `with_attr`, not the
  registry rule's in `AggregateState::new` and `with_children`.
- The ghost-maps table still phrases the pieces by node id ("inode n's
  `PointsTo` in inode n's lock"); only a later paragraph says they are keyed
  by the hidden per-inode key.

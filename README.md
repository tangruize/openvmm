# OpenVMM unified snapshot task

This directory prepares a System Proof task for the OpenVMM snapshot save/restore lifecycle. The production checkout is based on `nanvix/openvmm` revision `7f225e2cd635bbd1e1839e587e659996433396c8`; the current local task revision is `f127b8c455cabb04125eb582772c6eea326b5cc6`.

The intended end-to-end property is equivalence between the snapshot-relevant projection captured from the source VM and the restored VM projection after the allowed restore transforms. `goal.txt` gives the concise objective, `INPUT.md` is the authoritative task intent, and `targets.txt` identifies the real lifecycle entry points.

The production source contains an initial shared specification in `openvmm/openvmm_defs/src/snapshot.spec.rs` and direct initial contracts on the listed targets. The initial specification may be corrected or refined when required by source grounding, but the lifecycle objective must be preserved.

No LXD container, service, Argus project, verification worktree, or campaign has been launched. The shared specification and its composition lemmas verified with `2 verified, 0 errors`; verification of the entry and core crates currently reaches the known Verus `vir/src/prune.rs:464` frontend panic after parsing the supplied contracts.

This README and `README.zh-CN.md` are human-facing status documents, and `PROVENANCE.md` is a human-facing source record. A later experiment setup must copy only `production/`, `goal.txt`, `INPUT.md`, `targets.txt`, and the required settings from `campaign.env` into the LXD task workspace.

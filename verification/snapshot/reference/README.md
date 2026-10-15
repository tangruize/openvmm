# Reused specification material

These files are references, not active proof modules.

| File | Source and intended reuse |
| --- | --- |
| `restore-human-intent.txt` | Human/collaborator guidance for the real `InitializedVm::load(Some(_))` boundary, common logical state, prepared RAM, returned-VM representation, compatibility, time transformation and failure nonpublication |
| `previous-restore-SPEC.md` | The later detailed restore contract covering split RAM, VP identity, deferred device state, typed reconnect policy and live clock samples |
| `previous-restore-COMPONENTS.md` | Source-linked restore coverage ledger; line numbers and source shapes are historical and must be refreshed |
| `argus-1008-snapshot-correspondence.spec.rs` | The most direct earlier shared save/restore relation, used for capture/restore composition ideas |

The October 12 `snapshot_spec.rs` is intentionally not copied. It contained useful dimensions but became an oversized abstract TOP that was not consumed by the real entry points. The active seed instead keeps the direct shared state/relations and requires the target-binding ledger.

No verifier receipt from these historical files applies to the latest checkout.

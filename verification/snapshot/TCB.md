# Initial snapshot trust boundary

This is the supplied starting classification. It may be corrected, narrowed, or extended during Ground, but it is not permission to hide project-owned logic.

| Initial TCB | Location | Treatment |
| --- | --- | --- |
| Host filesystem primitives | `std::fs` operations reached from `openvmm_helpers/src/snapshot/fs.rs` and `publish.rs` | Trust documented open/read/write/flush/sync/rename/link and directory-handle effects; prove the repository's staging, generation selection, commit ordering and error propagation |
| Host memory mapping and flush primitives | OS calls beneath `membacking` and `sparse_mmap` | Trust the documented effect of a successful mapping/flush on the exact supplied handle and range; prove range enumeration, mapping mode, selected generation, call occurrence and error propagation |
| Hypervisor state access at unavoidable backend boundaries | Concrete `virt_kvm`, `virt_mshv`, `virt_whp` and partition APIs used by selected component handlers | Trust only reviewed contracts for external hypervisor reads/writes that cannot be translated; prove project-owned encoding, decoding, capability checks, VP identity, routing and state transformation |
| External clock samples | `SystemTime` and concrete host/hypervisor clock providers used by snapshot time ABI | Treat each read as a named external sample; prove provenance, ordering and the arithmetic that transforms saved state |
| External attachment observations | Host storage, filesystem and network resources reconstructed from the manifest | Trust only an explicit observer contract for content/topology/permissions at the named boundary; prove manifest matching and the selected reconnect policy |

The following are not initial TCB:

- `VmController` snapshot coordination and failure classification;
- `LoadedVm` capture, save, restore, time adjustment, stop-guard and publication control flow;
- `StateUnits` dependency ordering, inventory matching, payload routing and partial-failure behavior;
- snapshot manifest/state encoding and decoding implemented in this repository;
- RAM range selection, generation identity and publication transaction logic;
- compatibility predicates, supported-domain choices, or the final `snapshot_view` and `project_vm` correspondence.

Non-semantic diagnostics may be excluded with centralized `verus_keep_ghost` wrappers. Semantic argument evaluation, branches, state changes, result construction, cleanup and recovery remain outside the TCB.

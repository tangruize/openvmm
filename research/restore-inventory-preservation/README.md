# Destination registry across the actual TOP interval

## Result

**Diagnostic: the registered-name representation has a source-supported
ownership/frame condition on the actual construction path.** All removal
handles for this registry remain in the local `LoadedVm` during
`restore_snapshot_state`. Its callbacks receive neither those handles nor
registration authority. No reachable registry-removal path was found inside
this interval. This supports using every registered name, including names
without saved blobs and the local proxy of a failed remote component.

This is **not a Verus-proved invariant**, a definition of
`loaded_vm_representation`, or TOP completion. No proof contract, executable,
manifest, trust declaration, or accepted inventory/time proof was changed.
No runtime experiment was necessary: the unresolved question was the location
of the actual removal capabilities, not how generic handle destruction works.

Paths below are repository-relative. `dispatch` denotes
`openvmm/openvmm_core/src/worker/dispatch.rs`; `registry` denotes
`vmm_core/state_unit/src/lib.rs`; `chipset` denotes
`vmm_core/vmotherboard/src/chipset/`.

## The interval and its actual caller

`InitializedVm::load` creates this particular registry at `dispatch:1829`,
finishes chipset construction at `3177`, creates the partition unit at
`3189-3212`, awaits VP binding, and moves the owners into a local `LoadedVm`
at `3261-3329`. It directly awaits TOP at `3331-3333`, before returning
`Ok(this)` at `3343`. The no-snapshot firmware/resource-assignment branch
is not concurrent with TOP.

Both Worker entry paths wait for `load` before storing the VM and creating
the worker (`dispatch:428-450,473-489`). `VmWorker::run` only subsequently
consumes that worker and invokes `LoadedVm::run` (`493-501`). Thus the
AddVmbusDevice/PCIe hotplug/remove, reset, serialization and final teardown
arms of that control loop cannot run during this TOP invocation. Queued
control messages are not concurrently executed control handlers.

The existing LSP report `.verus_agent/cache/restore-time-callers.md` identifies
`InitializedVm::load -> restore_snapshot_state -> LoadedVm::restore`, matching
these current source calls. The maintained `argus_verus.callgraph.graph`
reader was attempted once; `.verus_agent/proof_state.json` is absent. This
setup limitation is not new graph evidence and was not repaired.

## Registration-to-owner closure

| Registered names | Construction and retained removal authority |
| --- | --- |
| `vmtime` | `dispatch:1831-1843`: `spawn` returns a `SpawnedUnit<VmTimeKeeper>`, moved to `LoadedVmInner::_vmtime` at `3282`. The future receives the keeper and request receiver, not the unit handle. |
| `input` | `dispatch:1846-1859`: the distributor/client resolver is set up before registration; the returned `SpawnedUnit` moves to `inner.input_distributor` at `3294`. Input clients have no registration handle. |
| `chipset` | `chipset/builder/mod.rs:148-149`: `build(send)` returns the root handle. `build(self):333-347` moves it to `ChipsetDevices::chipset_unit`, separately from the chipset request task. `dispatch:3281` moves the entire bundle to `inner.chipset_devices`. |
| **Every named static chipset device**, including PCI/virtio wrappers and remote-device proxies | Contrary to treating these as only one aggregate registry entry, `chipset/backing/arc_mutex/services.rs:81,136-141` unconditionally registers each finalized `dev_name` and retains its `SpawnedUnit` in `arc_mutex_device_units`. `chipset/builder/mod.rs:346` transfers that vector to `ChipsetDevices`. `device.rs:240-246` invokes this finalizer. `omit_saved_state` changes saving, not this registration. |
| `lines/<line-set-name>` | `chipset/line_sets.rs:37-54`: first use registers a line-set unit, retaining it in `LineSets::units`. `chipset/builder/mod.rs:347` transfers all such owners to `ChipsetDevices::_line_set_units`. Devices receive line interfaces/borrowed dependency handles, not these owning units. |
| `vmbus`, optional `vtl2_vmbus` | `dispatch:2772-2778,2830-2845`; `vmm_core/src/vmbus_unit.rs:41-58`: each `VmbusServerHandle` owns its `SpawnedUnit`. Both wrappers are retained at `dispatch:3287-3288`. The underlying server/control object does not own that wrapper. |
| Configured VMBus channel names `interface_name:instance_id` | `vmm_core/src/vmbus_unit.rs:224-245` registers and returns each `SpawnedUnit<ChannelUnit<_>>`; `dispatch:2996-3020` awaits and pushes every result, then moves the vector to `inner.vmbus_devices` at `3300`. A channel's own lifecycle is not destruction of this outer registration owner. |
| `partition` | `dispatch:3189-3212`; `vmm_core/src/partition_unit.rs:250-257`: the `SpawnedUnit` lives in `PartitionUnit::handle`, moved to `inner.partition_unit` at `3279`. `PartitionUnitRunner` instead holds partition/VP state and request endpoints (`partition_unit.rs:78-93`), not that handle. |

This is a closure over registrations in the **one local registry** created
by `load`, not a census of all nested hardware, channels, host resources or
other registries. Its construction-time uses are the two direct units,
the borrowed base/chipset builder, the VMBus server/channel helpers, and the
partition builder (`dispatch:1829-1859,2134,2774,2830,3011,3191-3194`).
`StateUnits` is not cloneable and its `inner` is private.

The base builder passes temporary MMIO/PIO/configuration services to resolvers,
not an owning handle or the registry
(`vmm_core/vmotherboard/src/base_chipset.rs:296-322,594-619`).
The later PCI/virtio construction uses the same chipset finalizer.
Windows proxy/kernel NIC and hvsock/redirect construction receives server
control interfaces rather than `StateUnits` or its removal handles
(`dispatch:2750-2845,2862-2887`). Such subcomponent activity is not an extra
mutation path into this registry. Runtime PCIe units are absent at TOP entry:
`pcie_hotplug_devices` is initialized empty at `dispatch:3327`.

## Mutation, callbacks, and asynchronous framing

The actual mutation sites in `registry` are:

- `UnitBuilder::build:1320-1366`: insert the unique name and unit ID; return the
  removal handle. All successful construction paths above retain that result.
- `UnitHandle::remove_if:331-337`: remove both maps. `Drop:314-317` and
  consuming `remove:322-324` call it. `detach:327-329` only clears the handle's
  weak pointer, leaving the registration; it is not removal.

`UnitHandle` has private fields and no Clone implementation. `spawn:1385-1391`
places the handle **beside**, not inside, the spawned future.
`SpawnedUnit:1396-1422` exposes only a shared handle reference or consuming
removal. `StateUnitsInspector:348-389` only inspects units and forwards
inspection requests. It exposes no registration or removal operation.
There is no name-renaming operation.

Consequently, a device task finishing, panicking, losing a channel or dropping
its device cannot itself drop its outer registration owner. Nor does it own
another local removal handle: the construction transfer above retains the
whole handle family outside the callbacks. The chipset task has the chipset
and receiver; each device task has its device and receiver; line tasks have
the line set and receiver. The VMBus and partition runners have their
underlying objects and receivers, while their wrappers retain the handles.
This is stronger than merely observing that TOP borrows `&mut StateUnits`.

`LoadedVm::restore` validates **nonempty** saved inventory before awaiting
`StateUnits::restore` (`dispatch:4765-4770`). The lock is not retained:
`registry:1001-1019,1110-1160` snapshots lookup/work, then awaits outside the
lock. `run_op:1161-1185` updates transition state but not names/membership.
`StateRequest::apply:235-242` invokes restore/advance on the unit, not on its
registration handle. The chipset device adapter delegates these operations
to its device (`chipset/backing/arc_mutex/state_unit.rs:192-193,207-209`);
it does not own any local registration authority.

TOP's later `advance_time`, VP/TSC, backend clock and final stop-guard
operations (`dispatch:3867-3905`) do not consume any owner in the table.
`PartitionUnit::{advance_tsc,temporarily_stop_vps}` sends requests and awaits
replies (`partition_unit.rs:284-291,310-324`). The runner handles them without
its outer handle (`428-448`); `StopGuard` contains just a request sender and
its destructor sends `StartVps` (`641-647`). Explicit partition teardown
instead consumes `self` and removes the handle (`267-270`), outside TOP.
Cancellation/destruction of the whole load future is not a successful TOP
return. No fairness or eventual-completion assumption is needed here.

For an arbitrary registry client, successful restore does **not** imply
preservation: after `run_op` has consumed an input blob, independent handle
removal can erase that entry; `1164-1178` even ignores receive failure for
an already-removed ID. The leftover-blob check at `1032-1041` does not reject
every such removal. With no blob, `state_change:1258-1259` skips the RPC.
Those source-level countertraces refute a generic unconditional registry
frame, but their independently held removal handle is unavailable in this
actual TOP interval. No generic handle-drop test is claimed as a TOP trace.

## Completeness, empty input, and remote failure

`inventory:507-514` enumerates all entries in the ID-ordered registry,
independently of saving. `save:954-978` filters `None` payloads.
`input` (`input_distributor.rs:211-212`), `chipset`
(`chipset/builder/mod.rs:393-395`) and line sets
(`chipset/line_sets.rs:74-78`) supply concrete no-blob cases. All retain their
registrations. Device `omit_saved_state` similarly returns `None` without
removing the device unit (`chipset/backing/arc_mutex/state_unit.rs:196-204`).

The accepted saved-side name encoding in
`openvmm/openvmm_defs/src/worker.proof.rs:9-50` is injective and includes empty
input. A destination binding to the same complete name set is therefore a
coherent candidate at state-unit granularity. It does not certify payload
coverage, active/pending component values, external resources, or clock
coherence.

Empty saved inventory still skips runtime validation and decodes to empty.
The actual constructed registry already has `vmtime`, `input`, `chipset`
and `partition`. Once this destination binding is proved, such an empty
request cannot satisfy the frozen equality precondition for that construction
(`dispatch.spec.rs:274`). No rejection or destination-name substitution is
invented. Establishing the actual caller's decoded precondition remains a
separate obligation; a runtime success alone does not establish it.

The accepted remote-TPM experiment recorded in
`.autors/openvmm-restore-v1/wiki/pages/verification/remote-tpm-time.md` is
reused, not rerun: controller loss after actual Restore can precede successful
TOP return while the configured TPM's callback still observes stale time.
It does **not** remove the local proxy's registration. Remote resolution
returns a `ResolvedChipsetDevice` through the ordinary finalizer;
`workers/chipset_device_worker/src/resolver.rs:56-104` sends remote parameters
without a local unit handle. The proxy owns request/response channels and a
worker handle, not `UnitHandle` (`proxy.rs:44-64`). Its Restore propagates
RPC/device errors (`322-325`), while its inherited `advance_time` is a no-op
(`vm/vmcore/src/device_state.rs:68-73`). Thus membership can remain stable
without time coherence; neither remote failure nor stale time permits
dropping that configured name from the representation.

## Machine evidence and next obligation

All four prescribed commands were run once from the repository/crate root
using `ARGUS_SKILL_PYTHON`, without proof/build input changes.
`checks.log` records their individual statuses; full output is in
`.verus_agent/cache/checks/{make_verify,boundary,spec_drift,exec_drift}/latest.log`.

| Check | Result |
| --- | --- |
| `argus_verus.tools.checks.make_verify --crate-root .` | Exit 0; full production selection, 1949 verified / 0 errors; 73.323 seconds. Lifetime and trait-conflict checking remain enabled. |
| `argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check` | Exit 3, INCOMPLETE; scoped scan: 0 permanent violations, 7 existing temporary locations, 0 assumptions; 1.035 seconds. |
| `argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no frozen specification drift; 1.272 seconds. |
| `argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent` | Exit 0; no executable drift; 4.914 seconds. |

No temporary marker was added, moved or removed. Boundary still reports the
TOP body cut, opaque `InitializedVm` and `LoadedVmInner`, and the four current
bridges: `decoded_saved_payload_view`, `decoded_load_restore_request_view`,
`initialized_vm_representation`, and `loaded_vm_representation`. In particular,
the latter still owes the destination registry binding and frame. The
source-supported fact above is not installed as an axiom or a new TCB promise.
The passing baseline does not verify that fact through these cuts.
The scoped boundary report is not a whole-project debt inventory:
`StateUnits` itself remains opaque at `registry:249-255`, and
`SavedStateUnit` at `391-409` still defers named-payload representation.
Neither existing cut provides a registry-stability guarantee.

**Next proof obligation:** model the registry's immutable name/ID relation
and its registration/removal authority in `state_unit`, establish at the real
`LoadedVm` construction that all local removal authority remains in the
retained owner bundle, and prove its preservation across the actual callbacks
and TOP awaits. Bind the loaded component inventory to the same encoded names
used on the saved side. This supplies the entry-to-return frame that combines
with the existing frozen equality precondition; it must not be replaced by
an unconditional `StateUnits::restore` frame or a strengthened frozen TOP
precondition. No evidenced need for a frozen-boundary change was found.

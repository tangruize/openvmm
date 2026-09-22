# Restore bridge authority

## Decision requested

Remove only the `InitializedVm::load` / `external_body` sanction from
`.verus_agent/tcb_manifest.json` and add its existing file/symbol pair to the
`goal` list in `.verus_agent/scope_manifest.json`. Preserve its source contract
as a proof obligation rather than a trusted axiom.

The two patches make exactly these same manifest edits independently against
the respective current branches. They change no executable, specification
definition, proof, marker, checker, source root, or frozen-branch designation.
The original `LoadedVm::restore_snapshot_state` goal and the entire explicit
TOP file `dispatch.spec.rs`, including `snapshot_restore_success`, remain
unchanged. No representation refinement is proposed for permanent trust.

This is the smallest authority change that both removes the conflicting trust
root and retains the wrapper's existing theorem as a protected obligation.
Removing only the sanction would leave the wrapper contract unprotected.
Keeping the sanction while excluding its dependencies would permit changes to
trusted meaning. Adding this goal deliberately extends eventual body-proof
scope to the real wrapper; it is not a claim that construction has been proved
or a change to the focused helper theorem.

The existing pending `restore-bridge-boundary` package proposes the same
manifest delta. This separately named, mission-owned package does not replace,
edit, apply, or dispose of that request or any other pending request. These
are alternative presentations of one authority decision, not two changes to
apply successively.

## What the five findings mean

The reviewed input is
`/home/ruize/.argus-skill/projects/9b3370b0cf02/handoffs/2b73696f64d0/round-0001.json`.
It establishes the production mutable-borrow proof but does not accept these
inherited boundary differences. That proof is unrelated to their cause.

`dispatch.rs:1538-1568` explicitly sanctions `InitializedVm::load` through its
`external_body` marker and the matching manifest rule. Both its `requires`
and its successful-snapshot `ensures` call
`restore_proof::decoded_load_restore_request_view`. The signature, requires,
ensures, and marker text are unchanged between frozen and live source.
Nevertheless, defining a function inside those clauses changes their trusted
meaning. A closed definition is still a definition, not an authority boundary.

The active implementation is the `argus_verus.tools.checks.spec_drift` module
loaded by `${ARGUS_SKILL_PYTHON:-python3}`, currently under
`.argus-runtime/lib/python3.12/site-packages/`.
`extract_workspace_specs` at lines 1330-1387 parses the scoped executable files
and their spec/proof files into one snapshot. `_filter_to_target_fns` at
1188-1327 starts BOTTOM at sanctioned contracts and traverses all spec bodies,
including closed bodies; TOP traversal stops before closed definitions.
Proof-function bodies do not define this closure. `diff_snapshots` at
1472 onward compares the union of selected keys. Its `is_new` means newly
selected, not necessarily newly declared in source.

In the following table, L is `InitializedVm::load`, A is
`decoded_load_restore_request_view`, B is `decoded_restore_request_view`,
and C is `decoded_saved_state_view`. All non-L nodes are in
`openvmm/openvmm_core/src/worker/dispatch.proof.rs`. Each finding's sole
protector is L; none of the five is independently sanctioned.

| Finding | Exact dependency path | Frozen versus live source |
| --- | --- | --- |
| `decoded_load_restore_request_view` (292-317) | L -> A | Selected uninterpreted declaration becomes a closed definition. The extracted difference is a signature change. |
| `decoded_restore_request_view` (97-110) | L -> A -> B | Existing uninterpreted declaration becomes closed and is newly selected through A's new body. |
| `decoded_saved_state_view` (85-95) | L -> A -> B -> C | New closed definition combines decoded payload and independent inventory. |
| `decoded_saved_payload_view` (58) | L -> A -> B -> C -> `decoded_saved_payload_view` | New uninterpreted declaration; actual payload interpretation is still owed. |
| `saved_component_inventory` (60-62) | L -> A -> B -> C -> `saved_component_inventory` | New closed definition observes the saved inventory via `saved_inventory_ids`. |

Each arrow is an actual named function call in the preceding contract or spec
body, not a proof-lemma call or an ambiguous method-name match. Thus all five
are genuine changes within one sanctioned contract's semantic dependency
closure, not a mixture of real changes and incorrect attribution. This does
not mean five independent trusted axioms were changed: one modified bridge
also exposes four previously unselected dependencies. Source declarations
and trusted selection must not be conflated.

The inventory implementation continues into
`openvmm_defs/src/worker.proof.rs:88-90`, where `saved_inventory_ids` reads
`SavedState.inventory`. That file is outside this checker's declared extraction
roots. The five-item report is not a claim of complete cross-crate semantic
dependency analysis; the path establishing each of these five is wholly
inside the current scope. No root expansion or checker repair is proposed.

There is a separate conservative-attribution issue: `view_expression` is
collected as bare `view` (1136-1141), and the index matches every same-named
spec (1209-1216, 1280-1285). Consequently the initialized/loaded VM Views and
their opaque representations retain `observe_str_pattern`, `str_strip_prefix`,
and `str_rsplit_once_dot` protectors after this candidate. Those are not any of
the five findings. This proposal does not silently remove them, correct the
active checker, or claim all future representation work is thereby unblocked.
Any later attribution correction needs its own evidence and review.

## Trust removed and proof still owed

No trusted declaration or assumption is added. The single removed sanction
means the already present `InitializedVm::load` marker becomes temporary,
unsanctioned `external_body` debt. Its source contract is neither weakened nor
proved by this reclassification. Final completion must prove the real body
and remove the marker; successful verification with it present is conditional.

Besides the five listed definitions, `pre_execution_representation` is the
only currently selected definition that leaves the protected closure. It is
closed project representation used by both retained contracts. Its intended
meaning is the concrete relation between `restored_from_snapshot`,
`restore_start_guard`, and `running` (`dispatch.proof.rs:421-428`).
The helper and wrapper must establish that relation from actual fields.
Removing BOTTOM protection is permission for honest refinement, not permission
to replace it by a vacuous condition or discard the lifecycle guarantee.

The wrapper's open semantic vocabulary remains protected through its TOP
contract, including validity, VP identity, snapshot projection, time policy,
and `snapshot_load_success`. All other sanctioned roots remain intact.
`elapsed_nanoseconds` retains the `duration_as_nanos` protector. The original
helper's open predicate and full explicit TOP file remain frozen.

Source support for the obligation is specific but not a body proof:

- `VmWorker`'s `Worker::new` calls the real wrapper at `dispatch.rs:429-436`;
  `Worker::restart` calls it at 476. Their existing compatibility/VP premises
  remain caller proof obligations; this candidate adds no precondition.
- The wrapper resolves the requested VP count, checks capacity, and truncates
  binders (1599-1612). It constructs `LoadedVm` with `running: false`, no
  restore guard, and the saved-state presence flag (3268-3284). On `Some`,
  it calls the real `restore_snapshot_state` before returning (3338-3350).
  `None` instead loads firmware and is outside its conditional guarantee.
- The helper restores state units, applies optional downtime/backend updates,
  and obtains the restore stop guard (3836-3914).
- `SavedState.inventory` contains identities independently of optional payload
  blobs. The inventory and envelope lemmas expose actual names and bytes;
  they do not decode the partition, VP, component, or virtual-time semantics.

These observations support the wrapper-composition intent, not every
construction frame fact. Proof work must relate constructed memory, resources,
compatibility, VP count and identities to the initialized View, establish the
helper precondition, and compose its successful result into the unchanged
wrapper postcondition. Every successful path and both real callers matter.

The six existing temporary locations remain: the external bodies for
`InitializedVm`, `LoadedVmInner`, and `LoadedVm::restore_snapshot_state`, plus
the uninterpreted `decoded_saved_payload_view`, `initialized_vm_representation`,
and `loaded_vm_representation`. Their obligations are concrete native storage
representations, faithful per-component decode semantics, and the real helper's
snapshot/frame/time/lifecycle theorem. None is discharged, renamed, moved, or
newly sanctioned. The only proposed debt change is the wrapper marker's
reclassification. The established `InplaceOption::as_mut` body and its two
borrow/writeback callers remain untouched; destructor and owning-extraction
proofs are not part of this boundary diagnostic.

The maintained `verus-call-structure` command cannot read this checkout's
missing `.verus_agent/proof_state.json`. The caller/body/contract evidence above
comes directly from production source; there is no maintained-graph or
verified-caller claim. No graph rebuild or infrastructure repair is proposed.

Restoring the opaque load bridge would strand the proved correspondence
lemmas and leave the required representation debt. Freezing the current
partial payload bridge would prevent its later definition while giving no
decoder proof. Neither is an acceptable alternative. The requested
trust-to-obligation change instead keeps those refinements and makes the
remaining composition proof explicit.

## Reproduce without changing the live boundary

The official interface independently checks patch applicability and compares
the proposed branch results:

```sh
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.operator.freeze_request \
  --project-root . validate restore-bridge-authority
```

That comparison uses branch commits, not the dirty working proof state.
For the latter, the following uses the active parser and selection algorithm
with isolated in-memory rule/goal sets. It checks the five protectors, unchanged
wrapper clauses, candidate comparison, and negative controls for both goal
contracts. It neither edits manifests nor installs a checker correction.
The synthetic `false` strings are checker test inputs, never Verus assumptions.

```sh
"${ARGUS_SKILL_PYTHON:-python3}" - <<'PY'
from argparse import Namespace
from copy import deepcopy
from pathlib import Path
from argus_verus.tools.checks import spec_drift as s, boundary as b

root = Path.cwd()
ctx = s._setup_drift(Namespace(crate_root=str(root), baseline_dir=".verus_agent"))
assert ctx is not None
repo, frozen, sources, baseline = ctx
scope = b.load_scope_manifest(baseline)
rules = b.load_tcb_manifest(baseline)
goals = {g.symbol for g in scope.goals}
before = s.extract_workspace_specs(sources, goals, rules, ref=frozen.commit)
after = s.extract_workspace_specs(sources, goals, rules)
expected = {
    "decoded_load_restore_request_view", "decoded_restore_request_view",
    "decoded_saved_state_view", "decoded_saved_payload_view",
    "saved_component_inventory",
}
drifts = [d for d in s.diff_snapshots(before, after).drifts if d.has_contract_drift]
assert {d.qualified_name for d in drifts} == expected
load = "InitializedVm::load"
def record(snapshot, name):
    return next(v for v in snapshot.functions.values() if v["qualified_name"] == name)
for d in drifts:
    assert d.scope == "bottom"
    assert record(after, d.qualified_name)["trust_protectors"] == [load]
for field in ("signature_text", "requires", "ensures", "sanctioned_evidence"):
    assert record(before, load)[field] == record(after, load)[field]
rule = dict(kind="symbol", value=load, marker="external_body")
assert rules.count(rule) == 1
proposed = [r for r in rules if r != rule]
goals |= {load}
before = s.extract_workspace_specs(sources, goals, proposed, ref=frozen.commit)
after = s.extract_workspace_specs(sources, goals, proposed)
assert not s.diff_snapshots(before, after).has_contract_drift
assert not s._top_spec_drift(root, repo, frozen.commit, scope.top_spec)
assert not s._declaration_drift(repo, baseline, frozen.commit)
for name in sorted(goals):
    key, = (k for k, v in after.functions.items() if v["qualified_name"] == name)
    for field in ("requires", "ensures"):
        control = deepcopy(after)
        control.functions[key][field] = ["false"]
        found = [d for d in s.diff_snapshots(before, control).drifts
                 if d.qualified_name == name and d.has_contract_drift]
        assert len(found) == 1 and found[0].scope == "top"
    print("protected TOP contract:", name)
print("isolated authority comparison complete; live boundary unchanged")
PY
```

Only Human disposition may change the sanctioned root. This package is an
unapplied authority candidate, not a wrapper proof, a waiver of the live drift,
or project completion.

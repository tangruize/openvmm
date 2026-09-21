# Verus verification

The initial verification target specifies `LoadedVm::restore_snapshot_state` through successful return with the guest still stopped.

```bash
make verify-setup
make verify
make verify MODULE=restore
make verify MODULE=vmtime
make verify-smoke
```

`make verify` (also `MODULE=` or `MODULE=all`) uses `cargo verus verify -p
openvmm_core`: it verifies the root and all Verus-enabled dependencies, without
a function selector. `MODULE=restore` uses `cargo verus focus` on
`worker::dispatch::LoadedVm::restore_snapshot_state`; its current
`external_body` means zero functions are proved by that selection.
`MODULE=vmtime` focuses the production `vmcore::vmtime` module, including
`VmTime::wrapping_add`, its modular-arithmetic lemma, and the `TimeState`
accessors. Neither focused selection is full-crate verification.

All selections retain lifetime and trait-conflict checking. Unknown modules
fail. Run `make verify-setup` separately if the pinned toolchain is missing;
verification itself does not install dependencies. The scripts check the
version against `verification/verus-version`, write output under
`target/verus/<module>.log`, and never substitute Rust compilation for Verus.
The verifier, cargo-verus, builtin crates, and `vstd` use the source revision
pinned by `verification/verus-repository` and `verification/verus-revision`.

Solver selection does not depend on the calling shell's `PATH`.
`find-z3.sh` selects `toolchain/verus-src/source/z3` by default, or an explicit
`VERUS_Z3_PATH` executable path, and checks its version against
`cargo-verus-toolchains/src/external_deps.rs` in the pinned Verus source.
Verification, setup, and smoke tests all export the validated absolute path.
An incompatible override fails before build-artifact invalidation; it is
never silently ignored. Verus's own solver-version check remains enabled.
`make verify-setup` installs or repairs the default solver even when Verus
itself is already installed. It does not replace an explicit override.

Cargo does not replay Verus result counts when artifacts are fresh. Before
each invocation, `fresh_verification.py` uses the resolved Cargo dependency
graph to run package-scoped `cargo clean --profile dev` for the selected
Verus-enabled packages. Full verification refreshes the root and its
Verus-enabled dependencies; focused verification refreshes only its root in
Cargo-Verus's separate `verus-partial` target directory. Artifacts of ordinary
dependencies and unrelated packages are not explicitly cleared; Cargo may
rebuild dependents as needed. No Rust source is touched.

Every invocation therefore obtains new verifier output, even on an unchanged
workspace. The helper reports preparation, verification, and total elapsed
time, and rejects missing counts, verification errors, and unsuccessful child
commands. Full verification and `MODULE=vmtime` additionally require nonzero
verified functions. Only the explicitly scaffolded `MODULE=restore` selection
permits a reported zero-function result; it still rejects missing evidence.
Previous logs and mere Cargo success are never used as verification evidence.

## Admission checks

Run the active Argus entry points from the repository root:

```bash
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.checks.make_verify --crate-root .
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.checks.boundary --crate-root . --baseline-dir .verus_agent check
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.checks.spec_drift --crate-root . --baseline-dir .verus_agent
"${ARGUS_SKILL_PYTHON:-python3}" -m argus_verus.tools.checks.exec_drift --crate-root . --baseline-dir .verus_agent
"${ARGUS_SKILL_PYTHON:-python3}" -m pytest -q verification/tools/test_check_readiness.py
```

The completion-mode boundary check intentionally returns 3 while temporary
proof debt remains. To check admission without claiming proof completion, use
the explicit `boundary ... admission` command instead of `check`. It still
rejects unauthorized permanent trust and manifest changes, prints every
temporary marker, and writes its separate `boundary_admission/latest.log`.
The inventory in `restore/UNINTERP.json` covers the four bridges, the TOP body,
the nine opaque project datatypes, and existing compatibility trust outside
the frozen source roots. None is silently sanctioned by admission.

The production `decoded_restore_request_view` now defines adjustment presence
and exact downtime from its actual option/Duration input. Only its
`SavedState` projection remains uninterpreted as `decoded_saved_state_view`.
In `worker::dispatch::restore_proof`, `decoded_restore_request_time_policy`
proves the binding and frozen virtual-time projection, and
`snapshot_restore_success_elapsed_time` proves the exact elapsed-time
consequence of TOP success, including sub-100ns remainders. These lemmas do not
prove the still-scaffolded async body or loaded-state representation.

The plugin source owner is the `argus-verus` package identified by the active
Python installation's `direct_url.json`; checker changes must be present in
that source and in the active installation, not just in a historical patch.
The regressions exercise that active installation and independently mutate
both production topology implementations using temporary copies. They also
reject TOP, TCB, dependency, and manifest mutations, and zero-function
verification evidence. Frozen project files are never modified by the tests.

Current status:

- the original conditional `InitializedVm::load` contract is retained, and the focused contract is attached to `LoadedVm::restore_snapshot_state`;
- production-body proof is intentionally not part of the current task;
- snapshot preparation, destination construction, component refinement, caller failure, resume, and readiness are explicit separate obligations.

The restore documents are split by boundary:

- `restore/SPEC.md`: human-reviewable restore semantics and production theorem;
- `restore/PREPARATION.md`: snapshot artifact, memory, manifest, and worker-input preparation;
- `restore/COMPONENTS.md`: stable state-unit identities and component blob interpretation;
- `restore/UNINTERP.json`: remaining View bridges, representation/body debt, and out-of-scope compatibility trust;
- `restore/LIMITATION.json`: current Verus frontend limitations and workarounds.

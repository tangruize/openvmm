# OpenVMM snapshot spec: documents

The documents about the Verus specification of OpenVMM's microVM snapshot
save and restore. The spec itself is in the code (`*.spec.rs`,
`*.proof.rs`, `*_t.rs`, and `verus_spec` attributes).

**Before checking or proving anything:** the spec needs a patched Verus;
see `verus-patches/README.md` (how to build it, on a new machine too, and
what each patch changes).

Start with the overview.

| Document | What it holds |
|---|---|
| `OVERVIEW.md` | the introduction: the goal, the software, what we prove, how the spec works |
| `OPEN-ITEMS.md` | what is still open, grouped by who or what each item waits for |
| `TRUSTED.md` | the trusted base and the deferred items |
| `CONVENTIONS.md` | the style rules of the spec: views and `wf()`, file layout, naming |
| `MESH-RPC.md` | the mesh RPC wrapper: how ghosts travel with messages |
| `MUTEX.md` | the two lock wrappers: locks with invariants, and closing |
| `CODE-CHANGES.md` | every change made to OpenVMM's code for the spec, numbered ("code change N") |
| `DESIGN.md` | the working log: every decision, its options and reasons; the exact contracts |
| `verus-concurrency-tutorial/` | a copy of the Verus concurrency tutorial (online: https://people.cs.uchicago.edu/~shanlu/verus-concurrency/): fractional ghosts, atomic and lock invariants, with examples; open `index.html` |
| `../verus-patches/README.md` | the patches to Verus and `vstd`, how to build the patched Verus, and two OpenVMM patches for OpenVMM's developers |

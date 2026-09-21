# Revoke the Error trait admission boundary

## Requested boundary change

This package exactly reverses the previously applied
`restore-error-trait-admission` package:

- frozen application commit: `fc98e2123fe59e7caa3e553e92ed58cfaabb4894`
- working application commit: `9a3e55d067d8a72f2cc094883fb6829ee810fe28`

The Human operator reviewed the result after application and revoked that
approval. The original package was narrow, but it expanded the trusted
boundary before establishing a complete strategy for the generated
`RpcError` implementation. It removed only one frontend obstruction and
immediately required another proposed trusted interface for
`thiserror::AsDynError`, while project-owned error and RPC obligations
remained.

## Exact rollback

`freeze.patch` is the exact inverse of the frozen application commit.
`run.patch` is the exact inverse of the working application commit. They
remove:

- `ExCoreError` from the sanctioned TCB;
- the `mesh_channel` source-root expansion;
- `rpc.spec.rs` and its ghost-only include;
- the `mesh_channel` Verus opt-in and `vstd` dependency;
- the corresponding frozen-side lockfile dependency edge.

No other frozen source, proof source, TOP contract, TCB entry, dependency, or
executable body is changed. The accepted reset-constructor normalization and
all unrelated proof work remain intact.

## Follow-up policy

The restore proof must not replace diagnostic or error-formatting limitations
with a chain of project-local trusted declarations. The next investigation
should separate proof-relevant error control flow from diagnostic formatting,
derive-generated `Error::source`, and logging-only behavior. It should prefer
verified projections, ghost-erased adapters, or narrowly excluded diagnostic
surfaces only when their runtime control-flow irrelevance is established.
Any new trusted boundary still requires a separate exact Human-reviewed
request.

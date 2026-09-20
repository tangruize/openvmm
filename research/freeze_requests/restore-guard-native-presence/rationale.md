# Native partition-presence proof boundary

The production guard is not known to be behaviorally incorrect. This request
is a behavior-preserving normalization of its frozen executable source, not
a TOP weakening or extension of trusted operations.

The native proof of anyhow's real negation helper and both private Bool
implementations eliminates the initial unsupported-call blocker without an
external specification. The proof uses a local copy of the exact pinned
runtime dependency with erased annotations, selected for both ordinary
execution and verification by Cargo. No guard implementation is copied or
substituted.

Two further boundaries prevent the existing spelling from completing the
field-based proof. Pinned vstd supplies no contents-equality contract for
`<String as PartialEq<&str>>::eq`; the standalone string-equality probe fails
that obligation while its slice-equality control verifies. The matching
Rust standard-library source implements the former operation by comparing
the full string slices. Its body cannot be imported as a native project
dependency: it belongs to the pinned sysroot. Adding an assumed equality
specification would enlarge the BOTTOM TCB and is not proposed.

Also, the comparison closure is inside `anyhow::ensure!`'s opaque token
tree. Verus accepts a closure-contract syntax wrapper there, but the
executable-drift checker does not erase it and reports an executable
mismatch. Moving the unchanged short-circuit condition into one local
binding makes the closure's native Verus contract visible to both the
verifier and the annotation eraser. Explicit closure argument/return types
and a block body provide an identical executable shape after erasure.

The normalization computes exactly the same Boolean condition at
the same point: time adjustment remains absent-or-partition-present, the
iterator still short-circuits, and the error message and all return paths
are unchanged. Calling `name.as_str()` selects the already specified slice
equality operation on exactly the same bytes, without mutation or allocation.
No TOP contract, sanctioned BOTTOM declaration, manifest, or verifier
semantics changes.

`freeze.patch` contains only this guard normalization.
`run.patch` independently contains the same normalization, the field-based
guard equivalence and comparison-closure contracts, native transparent
saved-state declarations, and the native anyhow proof. The accepted
declaration-admission overlay needed by the production consumer is preserved
on the working branch rather than duplicated in the request.

The real TOP consumes the guard result with `?` before restoration and time
adjustment. A guard proof only excludes omitted partition state during
time adjustment. It does not establish partition decoding, saved VP
completeness, installed VP state, or the link through time adjustment to
`restore_vp_projection`. The TOP body marker and the other representation
bridges remain separate proof obligations.

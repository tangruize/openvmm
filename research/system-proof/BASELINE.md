# Existing experiment baseline

The fresh target starts at `nanvix/openvmm` commit `c9f659c36c8cb36d5aac745f7c26808296689f3f`. The target-local Verus source is the `verus` Git submodule at `3cf18325f0fd0c3040fbdec8c0f2255c0504c91a` (`release/0.2026.09.27.3cf1832`). Its source-built binary is `verus/source/target-verus/release/verus`; do not substitute an ambient global Verus.

The local source build and vstd verification completed successfully. The latter reported 2,059 verified goals and no errors. The required solver is the official release's Z3 4.16.0, not the older system Z3.

The first manual build did not capture the reusable adapter's pre-build observations. Its selected local launcher is therefore initially associated through the explicit release-profile/full-commit/executable-hash match in `system-proof.toml`, not labelled as a tool-witnessed build. Keep that distinction; do not invent a prior build snapshot. `system-proof verus build` records the configured local source build for future changes.

The production snapshot baseline already passed all 95 selected tests with `PROTOC=/usr/bin/protoc cargo +1.98.1 test -p openvmm_helpers snapshot:: --no-default-features --quiet`. Reuse this result while production and build inputs remain unchanged; do not spend another mission rediscovering the ambient Rust/protoc mismatch.

The first native investigation produced useful publication/restore candidate meanings and retained weak-model attempts, but failed the system-proof completion gate: it did not register candidates, update the semantic dependency map, or retain evidence bound through the selected project verifier. Its global-Verus model results are historical exploration, not current implementation proof. Preserve the useful observations, but complete the source-body and tool binding before calling the next delivery reviewable.

The editable local toolchain is available for genuine frontend limitations. Any such change must remain explicit and be rebuilt before its results count. No compiler behavior patch has been applied in this baseline.

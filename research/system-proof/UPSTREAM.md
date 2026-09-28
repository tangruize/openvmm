# Experiment baseline

The source baseline is `nanvix/openvmm` commit `c9f659c36c8cb36d5aac745f7c26808296689f3f`, fetched on 2026-09-28. The publication fork is `tangruize/openvmm`; existing restore and snapshot experiment branches remain untouched.

The project-local `verus/` submodule is pinned to upstream release `0.2026.09.27.3cf1832`, source commit `3cf18325f0fd0c3040fbdec8c0f2255c0504c91a`. Its source can be edited and rebuilt; changed compiler or library code invalidates dependent evidence. No verifier semantics have been modified for this baseline.

The parent Cargo workspace excludes `verus/` so Cargo does not treat the nested verifier's separately built `vstd` as an OpenVMM package. This is build configuration, not a production-code rewrite.

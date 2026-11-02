// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! A trusted Verus specification of mesh RPC: calls and replies that carry
//! ghost values, with a per-message-type protocol (`MeshProtocol`). See
//! `snapshot-spec-docs/MESH-RPC.md` at the repository root.

#[cfg(verus_keep_ghost)]
mod external_failure_t;
mod mesh_rpc_t;

#[cfg(verus_keep_ghost)]
pub use external_failure_t::*;
pub use mesh_rpc_t::*;

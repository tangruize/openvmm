// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! A trusted Verus specification of mesh RPC: calls and replies that carry
//! ghost values, with a per-message-type protocol (`MeshProtocol`). See
//! `SNAPSHOT-SPEC-MESH-RPC.md` at the repository root.

mod mesh_rpc_t;

pub use mesh_rpc_t::*;

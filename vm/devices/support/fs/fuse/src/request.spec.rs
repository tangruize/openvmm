// Spec of `Request`, for human review.

use super::Request;
use vstd::prelude::*;

verus! {

/// The view of a `Request`: what the operations' contracts read of it.
pub struct RequestView {
    /// The node the request is about (`fuse_in_header::nodeid`).
    pub node_id: u64,
    /// The operation (`fuse_in_header::opcode`).
    pub opcode: u32,
    /// The caller's user and group ids, as the guest's kernel wrote them
    /// (`fuse_in_header::uid`, `gid`).
    pub uid: u32,
    pub gid: u32,
}

impl View for Request {
    type V = RequestView;

    closed spec fn view(&self) -> RequestView {
        RequestView {
            node_id: self.header.nodeid,
            opcode: self.header.opcode,
            uid: self.header.uid,
            gid: self.header.gid,
        }
    }
}

} // verus!

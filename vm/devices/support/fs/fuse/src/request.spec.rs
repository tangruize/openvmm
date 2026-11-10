// Spec of `Request`, for human review.

use super::Request;
use vstd::prelude::*;

verus! {

/// The view of a `Request`: what the operations' contracts read of it.
pub struct RequestView {
    /// The node the request is about (`fuse_in_header::nodeid`).
    pub node_id: u64,
}

impl View for Request {
    type V = RequestView;

    closed spec fn view(&self) -> RequestView {
        RequestView { node_id: self.header.nodeid }
    }
}

} // verus!

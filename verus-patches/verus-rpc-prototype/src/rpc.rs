//! Trusted RPC channel that carries ghost resources with each request and
//! reply.
//!
//! Everything in this file is TRUSTED. The function bodies are
//! `external_body`: they wrap a real `std::sync::mpsc` channel, and their
//! contracts state what we assume about it. Tracked values are zero-sized at
//! runtime, so nothing ghost actually crosses the channel; the contracts are
//! what make "passing" them sound. They rely on the channel delivering:
//!
//! - each request exactly once to the server, unchanged, so the server
//!   receives exactly the ghost the caller gave up (no duplication);
//! - each reply exactly once to the caller that sent the matching request
//!   (every call uses a fresh reply channel);
//! - nothing, if the server drops a request without responding: the caller
//!   gets `Err` and the ghost it sent is lost.

use std::sync::mpsc;
use vstd::prelude::*;

verus! {

/// The contract an RPC channel carries. Both ends of a channel are
/// parameterized by the same protocol, so they agree on one contract.
pub trait RpcProtocol {
    /// Per-channel constant shared by both ends, e.g. ids of ghost resources.
    type Const;
    type Request: Send + 'static;
    type Reply: Send + 'static;
    /// Ghost resources sent with a request.
    type GhostIn;
    /// Ghost resources returned with a reply.
    type GhostOut;

    /// What a caller must establish to send `req` with `gin`.
    spec fn request_ok(k: Self::Const, req: Self::Request, gin: Self::GhostIn) -> bool;

    /// What a server must establish to answer `req`/`gin` with `rep`/`gout`.
    spec fn reply_ok(
        k: Self::Const,
        req: Self::Request,
        gin: Self::GhostIn,
        rep: Self::Reply,
        gout: Self::GhostOut,
    ) -> bool;
}

/// The call failed: the server is gone, or dropped the request.
pub struct RpcError;

/// The caller's end of a channel.
#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct RpcClient<P: RpcProtocol> {
    tx: mpsc::Sender<(P::Request, mpsc::Sender<P::Reply>)>,
}

/// The server's end of a channel.
#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct RpcServer<P: RpcProtocol> {
    rx: mpsc::Receiver<(P::Request, mpsc::Sender<P::Reply>)>,
}

/// The obligation to answer one received request. Consumed by `respond`.
#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct RpcResponder<P: RpcProtocol> {
    tx: mpsc::Sender<P::Reply>,
}

impl<P: RpcProtocol> RpcClient<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    /// Another client to the same server. Sound because each call carries
    /// its own request, ghost, and reply channel.
    #[verifier::external_body]
    pub fn clone(&self) -> (result: Self)
        ensures
            result.constant() == self.constant(),
    {
        RpcClient { tx: self.tx.clone() }
    }

    /// Sends `req` with `gin` and waits for the reply.
    #[allow(unused_variables)]  // ghost-only parameters are erased at runtime
    #[verifier::external_body]
    pub fn call(&self, req: P::Request, gin: Tracked<P::GhostIn>) -> (result: Result<
        (P::Reply, Tracked<P::GhostOut>),
        RpcError,
    >)
        requires
            P::request_ok(self.constant(), req, gin@),
        ensures
            match result {
                Ok((rep, gout)) => P::reply_ok(self.constant(), req, gin@, rep, gout@),
                Err(_) => true,
            },
    {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self.tx.send((req, reply_tx)).is_err() {
            return Err(RpcError);
        }
        match reply_rx.recv() {
            Ok(rep) => Ok((rep, Tracked::assume_new())),
            Err(_) => Err(RpcError),
        }
    }
}

impl<P: RpcProtocol> RpcServer<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    /// Waits for the next request. Returns `None` when every client is gone.
    #[verifier::external_body]
    pub fn recv(&self) -> (result: Option<(P::Request, Tracked<P::GhostIn>, RpcResponder<P>)>)
        ensures
            match result {
                Some((req, gin, responder)) => {
                    &&& P::request_ok(self.constant(), req, gin@)
                    &&& responder.constant() == self.constant()
                    &&& responder.request() == req
                    &&& responder.ghost_in() == gin@
                },
                None => true,
            },
    {
        match self.rx.recv() {
            Ok((req, tx)) => Some((req, Tracked::assume_new(), RpcResponder { tx })),
            Err(_) => None,
        }
    }
}

impl<P: RpcProtocol> RpcResponder<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    pub uninterp spec fn request(&self) -> P::Request;

    pub uninterp spec fn ghost_in(&self) -> P::GhostIn;

    /// Answers the request with `rep` and `gout`.
    #[allow(unused_variables)]  // ghost-only parameters are erased at runtime
    #[verifier::external_body]
    pub fn respond(self, rep: P::Reply, gout: Tracked<P::GhostOut>)
        requires
            P::reply_ok(self.constant(), self.request(), self.ghost_in(), rep, gout@),
    {
        let _ = self.tx.send(rep);
    }
}

/// Creates a channel whose ends share the constant `k`.
#[allow(unused_variables)]  // ghost-only parameters are erased at runtime
#[verifier::external_body]
pub fn rpc_channel<P: RpcProtocol>(k: Ghost<P::Const>) -> (result: (RpcClient<P>, RpcServer<P>))
    ensures
        result.0.constant() == k@,
        result.1.constant() == k@,
{
    let (tx, rx) = mpsc::channel();
    (RpcClient { tx }, RpcServer { rx })
}

} // verus!

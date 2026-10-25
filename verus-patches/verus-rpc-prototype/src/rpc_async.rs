//! Async version of `rpc.rs`: the same trusted contracts, with `call` and
//! `recv` as `async fn`s over `futures` channels, as OpenVMM's units are async
//! tasks reached through `mesh` channels.
//!
//! Everything in this file is TRUSTED, under the same assumptions as
//! `rpc.rs` (exactly-once delivery, replies matched to requests, ghost lost on
//! failure). It reuses `RpcProtocol` and `RpcError` from `rpc.rs`.

use crate::rpc::RpcError;
use crate::rpc::RpcProtocol;
use futures::StreamExt;
use futures::channel::mpsc;
use futures::channel::oneshot;
use std::future::Future;
#[allow(unused_imports)]  // used only in specs
use vstd::future::FutureAdditionalSpecFns;
use vstd::prelude::*;

verus! {

#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct AsyncRpcClient<P: RpcProtocol> {
    tx: mpsc::UnboundedSender<(P::Request, oneshot::Sender<P::Reply>)>,
}

#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct AsyncRpcServer<P: RpcProtocol> {
    rx: mpsc::UnboundedReceiver<(P::Request, oneshot::Sender<P::Reply>)>,
}

#[verifier::external_body]
#[verifier::reject_recursive_types(P)]
pub struct AsyncRpcResponder<P: RpcProtocol> {
    tx: oneshot::Sender<P::Reply>,
}

impl<P: RpcProtocol> AsyncRpcClient<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    #[verifier::external_body]
    pub fn clone(&self) -> (result: Self)
        ensures
            result.constant() == self.constant(),
    {
        AsyncRpcClient { tx: self.tx.clone() }
    }

    /// Sends `req` with `gin` and waits for the reply.
    #[verifier::external_body]
    #[allow(unused_variables)]  // ghost-only parameters are erased at runtime
    pub async fn call(&self, req: P::Request, gin: Tracked<P::GhostIn>) -> (result: Result<
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
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.unbounded_send((req, reply_tx)).is_err() {
            return Err(RpcError);
        }
        match reply_rx.await {
            Ok(rep) => Ok((rep, Tracked::assume_new())),
            Err(_) => Err(RpcError),
        }
    }
}

impl<P: RpcProtocol> AsyncRpcServer<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    /// Waits for the next request. Returns `None` when every client is gone.
    #[verifier::external_body]
    pub async fn recv(&mut self) -> (result: Option<
        (P::Request, Tracked<P::GhostIn>, AsyncRpcResponder<P>),
    >)
        ensures
            final(self).constant() == old(self).constant(),
            match result {
                Some((req, gin, responder)) => {
                    &&& P::request_ok(old(self).constant(), req, gin@)
                    &&& responder.constant() == old(self).constant()
                    &&& responder.request() == req
                    &&& responder.ghost_in() == gin@
                },
                None => true,
            },
    {
        match self.rx.next().await {
            Some((req, tx)) => Some((req, Tracked::assume_new(), AsyncRpcResponder { tx })),
            None => None,
        }
    }
}

impl<P: RpcProtocol> AsyncRpcResponder<P> {
    pub uninterp spec fn constant(&self) -> P::Const;

    pub uninterp spec fn request(&self) -> P::Request;

    pub uninterp spec fn ghost_in(&self) -> P::GhostIn;

    /// Answers the request with `rep` and `gout`.
    #[verifier::external_body]
    #[allow(unused_variables)]  // ghost-only parameters are erased at runtime
    pub fn respond(self, rep: P::Reply, gout: Tracked<P::GhostOut>)
        requires
            P::reply_ok(self.constant(), self.request(), self.ghost_in(), rep, gout@),
    {
        let _ = self.tx.send(rep);
    }
}

/// Creates a channel whose ends share the constant `k`.
#[verifier::external_body]
#[allow(unused_variables)]  // ghost-only parameters are erased at runtime
pub fn async_rpc_channel<P: RpcProtocol>(k: Ghost<P::Const>) -> (result: (
    AsyncRpcClient<P>,
    AsyncRpcServer<P>,
))
    ensures
        result.0.constant() == k@,
        result.1.constant() == k@,
{
    let (tx, rx) = mpsc::unbounded();
    (AsyncRpcClient { tx }, AsyncRpcServer { rx })
}

/// Runs `future` to completion on the current thread. Like `.await`, it
/// yields the future's output (`vstd::future::exec_await` has the same
/// contract).
#[verifier::external_body]
pub fn block_on<F: Future>(future: F) -> (result: F::Output)
    ensures
        future.awaited(),
        result == future@,
{
    futures::executor::block_on(future)
}

} // verus!

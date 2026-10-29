// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Trusted specification of OpenVMM's mesh RPC, carrying ghost resources with
//! each request and reply.
//!
//! TRUSTED, do not attempt to prove anything in this file: Verus uses only
//! the contracts here (the bodies are `external_body`), and they are claims
//! about `mesh`, which is external (`SNAPSHOT-SPEC-TRUSTED.md`, Part 1). The
//! `tests` module at the end is ordinary test code.
//!
//! Everything in this file is TRUSTED. The exec functions are `external_body`
//! wrappers around the real `mesh` API (`mesh::channel`, `Sender::send` via
//! `RpcSend::call`, `Receiver::recv` (as `recv_msg`), `Rpc::split`, `Rpc::complete`); their
//! contracts state what we assume about it. Tracked values are zero-sized at
//! runtime, so nothing ghost crosses the channel; the contracts are what make
//! "passing" them sound.
//!
//! How a call works, and where the ghosts go:
//!
//! - The caller calls `call(&send, f, input, gin, post)`. Mesh creates a
//!   one-shot reply channel, builds an `Rpc` from `input` and the reply
//!   sender, wraps it in a message with `f` (e.g. `|rpc| Req::Save(rpc)`), and
//!   sends the message. The request ghost `gin` goes with the message.
//!   `post` is the caller's expectation of the reply; it is attached to the
//!   `Rpc` and to the returned `PendingCall`.
//! - The receiver gets the message and `gin` from `recv_msg`. The channel's
//!   protocol (`MeshProtocol::request_ok`) tells it what `gin` is and what
//!   reply the `Rpc` inside the message expects.
//! - The receiver answers with `complete(rpc, r, gout)`, which requires the
//!   `Rpc`'s expectation to hold of `r` and the reply ghost `gout`.
//! - The caller awaits `PendingCall::recv`. On success it gets `r` and `gout`
//!   with the expectation it chose.
//!
//! Ghosts attached to the ends of a channel:
//!
//! - `channel(k, gs, gr)` attaches `gs` to the sender and `gr` to the
//!   receiver; `take_sender_ghost` and `take_receiver_ghost` take them, once.
//!   This lets a ghost created with a channel reach whoever ends up holding
//!   each end, through code that only moves the ends around.
//! - A cloned sender carries no ghost.
//! - ASSUMPTION: an end whose ghost is taken was created by `channel`, not
//!   by `mesh::channel` directly (whose ends' `attached` is unknown). Verified
//!   code states this as a precondition where it takes a ghost.
//!
//! What we assume about mesh (in-process channels):
//!
//! - Each message is delivered at most once, unchanged, to the receiver of
//!   the channel it was sent on. So the receiver gets exactly the ghost the
//!   caller gave up: a ghost is never duplicated.
//! - Each `Rpc`'s reply goes only to the `PendingRpc` created with it.
//!   An `Rpc` is not `Clone`, and `complete` consumes it, so it is answered at
//!   most once.
//! - If the message or the `Rpc` is dropped without a reply, the caller gets
//!   `Err` and no ghost: the ghost it sent is lost.
//!
//! Soundness note for review: the reply ghost's type is a type parameter `G`
//! of `complete`, and `Rpc::reply_ok::<G>` is uninterpreted for every `G`.
//! The only facts about it come from `call` (for `G = M::GhostOut` of the
//! channel the `Rpc` was sent on), so a receiver cannot complete an `Rpc` with
//! a ghost of another type: it could not prove `reply_ok` for it.

use mesh::Receiver;
use mesh::Sender;
use mesh::rpc::PendingRpc;
use mesh::rpc::Rpc;
use mesh::rpc::RpcError;
use mesh::rpc::RpcSend;
use vstd::prelude::*;

verus! {

#[cfg(verus_keep_ghost)]
/// Declares `mesh::Sender` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExSender<T>(Sender<T>);

#[cfg(verus_keep_ghost)]
/// Declares `mesh::error::RemoteError` (an `Rpc`'s remote error) to Verus,
/// as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRemoteError(mesh::error::RemoteError);

#[cfg(verus_keep_ghost)]
/// Declares `mesh::Receiver` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExReceiver<T>(Receiver<T>);

#[cfg(verus_keep_ghost)]
/// Declares `mesh::rpc::Rpc` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(I)]
#[verifier::reject_recursive_types(R)]
pub struct ExRpc<I, R>(Rpc<I, R>);

#[cfg(verus_keep_ghost)]
/// Declares `mesh::rpc::RpcError` to Verus, as an opaque type.
#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(E)]
pub struct ExRpcError<E>(RpcError<E>);

/// The contract a mesh channel carries, implemented by its message type.
/// Both ends of a channel have the same constant, so they agree on it.
pub trait MeshProtocol: Sized {
    /// Per-channel ghost constant, e.g. the ids of the receiver's ghost
    /// resources.
    type Const;
    /// Ghost resources sent with a message.
    type GhostIn;
    /// Ghost resources returned with the reply to an `Rpc` in a message.
    type GhostOut;
    /// Ghost attached to a channel's sender when it is created.
    type SenderGhost;
    /// Ghost attached to a channel's receiver when it is created.
    type ReceiverGhost;

    /// What a sender must establish to send `msg` with `gin` on a channel
    /// with constant `k`. For a message holding an `Rpc`, this also says what
    /// the `Rpc` expects of its reply (via `RpcAdditionalSpecFns::reply_ok`),
    /// so the receiver knows what it must answer.
    spec fn request_ok(k: Self::Const, msg: Self, gin: Self::GhostIn) -> bool;
}

/// Spec functions of a `mesh::Sender`.
pub trait SenderAdditionalSpecFns<M: MeshProtocol> {
    /// The channel's constant.
    spec fn constant(&self) -> M::Const;

    /// The ghost attached to this sender, if not yet taken.
    spec fn attached(&self) -> Option<M::SenderGhost>;
}

impl<M: MeshProtocol> SenderAdditionalSpecFns<M> for Sender<M> {
    uninterp spec fn constant(&self) -> M::Const;

    uninterp spec fn attached(&self) -> Option<M::SenderGhost>;
}

/// Spec functions of a `mesh::Receiver`.
pub trait ReceiverAdditionalSpecFns<M: MeshProtocol> {
    /// The channel's constant.
    spec fn constant(&self) -> M::Const;

    /// The ghost attached to this receiver, if not yet taken.
    spec fn attached(&self) -> Option<M::ReceiverGhost>;
}

impl<M: MeshProtocol> ReceiverAdditionalSpecFns<M> for Receiver<M> {
    uninterp spec fn constant(&self) -> M::Const;

    uninterp spec fn attached(&self) -> Option<M::ReceiverGhost>;
}

/// Spec functions of a `mesh::rpc::Rpc`.
pub trait RpcAdditionalSpecFns<I, R> {
    /// The request's input.
    spec fn spec_input(&self) -> I;

    /// The caller's expectation of the reply, with reply ghost type `G`.
    spec fn reply_ok<G>(&self, r: R, gout: G) -> bool;
}

impl<I, R> RpcAdditionalSpecFns<I, R> for Rpc<I, R> {
    uninterp spec fn spec_input(&self) -> I;

    uninterp spec fn reply_ok<G>(&self, r: R, gout: G) -> bool;
}

/// Creates a channel whose ends share the constant `k`, with `send_ghost`
/// attached to the sender and `recv_ghost` to the receiver.
#[verifier::external_body]
#[allow(unused_variables)]  // ghost-only parameters are erased at runtime
pub fn channel<M: MeshProtocol>(
    k: Ghost<M::Const>,
    send_ghost: Tracked<M::SenderGhost>,
    recv_ghost: Tracked<M::ReceiverGhost>,
) -> (result: (Sender<M>, Receiver<M>))
    ensures
        result.0.constant() == k@,
        result.1.constant() == k@,
        result.0.attached() == Some(send_ghost@),
        result.1.attached() == Some(recv_ghost@),
{
    mesh::channel()
}

/// Takes the ghost attached to `send`.
#[verifier::external_body]
#[allow(unused_variables)]  // the ghost is erased at runtime
pub fn take_sender_ghost<M: MeshProtocol>(send: &mut Sender<M>) -> (result: Tracked<
    Option<M::SenderGhost>,
>)
    ensures
        final(send).constant() == old(send).constant(),
        final(send).attached() is None,
        result@ == old(send).attached(),
{
    // TRUSTED: keep this `assume_new()`. This trusted function's
    // contract hands out the ghost; its body has none to give.
    Tracked::assume_new()
}

/// Takes the ghost attached to `receiver`.
#[verifier::external_body]
#[allow(unused_variables)]  // the ghost is erased at runtime
pub fn take_receiver_ghost<M: MeshProtocol>(receiver: &mut Receiver<M>) -> (result: Tracked<
    Option<M::ReceiverGhost>,
>)
    ensures
        final(receiver).constant() == old(receiver).constant(),
        final(receiver).attached() is None,
        result@ == old(receiver).attached(),
{
    // TRUSTED: keep this `assume_new()`. This trusted function's
    // contract hands out the ghost; its body has none to give.
    Tracked::assume_new()
}

/// Another sender to the same receiver, with no ghost attached. Sound
/// because each message carries its own ghost.
#[verifier::external_body]
pub fn clone_sender<M: MeshProtocol>(send: &Sender<M>) -> (result: Sender<M>)
    ensures
        result.constant() == send.constant(),
        result.attached() is None,
{
    send.clone()
}

/// Sends the one-way message `msg` on `send`, with the ghost `gin`
/// (TRUSTED, like `call`: the receiver gets `msg` and `gin` through
/// `recv_msg`).
#[verifier::external_body]
#[allow(unused_variables)]  // the ghost is erased at runtime
pub fn send_msg<M: MeshProtocol + 'static + Send>(send: &Sender<M>, msg: M, gin: Tracked<M::GhostIn>)
    requires
        M::request_ok(send.constant(), msg, gin@),
{
    send.send(msg)
}

/// The caller's end of one call: awaits the reply.
#[verifier::external_body]
#[verifier::reject_recursive_types(M)]
#[verifier::reject_recursive_types(R)]
pub struct PendingCall<M: MeshProtocol, R> {
    pending: PendingRpc<R>,
    _protocol: std::marker::PhantomData<M>,
}

impl<M: MeshProtocol, R> PendingCall<M, R> {
    /// The caller's expectation of the reply, as passed to `call`.
    pub uninterp spec fn reply_ok(&self, r: R, gout: M::GhostOut) -> bool;
}

/// Sends the message `f(rpc)` on `send`, where `rpc` carries `input`, with
/// the ghost `gin`. `post` is what the caller expects of the reply.
#[verifier::external_body]
#[allow(unused_variables)]  // ghost-only parameters are erased at runtime
pub fn call<M, F, I, R>(
    send: &Sender<M>,
    f: F,
    input: I,
    gin: Tracked<M::GhostIn>,
    post: Ghost<spec_fn(R, M::GhostOut) -> bool>,
) -> (pending: PendingCall<M, R>) where
    M: MeshProtocol + 'static + Send,
    F: FnOnce(Rpc<I, R>) -> M,
    R: 'static + Send,

    requires
        forall|rpc: Rpc<I, R>| #[trigger] rpc.spec_input() == input ==> f.requires((rpc,)),
        // Whatever message `f` builds from an `Rpc` carrying `input` and
        // expecting `post`, it may be sent with `gin`.
        forall|rpc: Rpc<I, R>, msg: M|
            rpc.spec_input() == input && (forall|r: R, gout: M::GhostOut|
                #[trigger] rpc.reply_ok(r, gout) == post@(r, gout)) && f.ensures((rpc,), msg)
                ==> M::request_ok(send.constant(), msg, gin@),
    ensures
        forall|r: R, gout: M::GhostOut| #[trigger] pending.reply_ok(r, gout) == post@(r, gout),
{
    PendingCall { pending: send.call(f, input), _protocol: std::marker::PhantomData }
}

impl<M: MeshProtocol, R: 'static + Send> PendingCall<M, R> {
    /// Waits for the reply. On success, the reply and its ghost satisfy the
    /// caller's expectation.
    #[verifier::external_body]
    pub async fn recv(self) -> (result: Result<(R, Tracked<M::GhostOut>), RpcError>)
        ensures
            result matches Ok((r, gout)) ==> self.reply_ok(r, gout@),
    {
        match self.pending.await {
            // TRUSTED: keep this `assume_new()`. This trusted function's
            // contract hands out the ghost; its body has none to give.
            Ok(r) => Ok((r, Tracked::assume_new())),
            Err(err) => Err(err),
        }
    }
}

impl<M: MeshProtocol, T: 'static + Send, E: 'static + Send> PendingCall<M, Result<T, E>> {
    /// Waits for the reply of a call whose reply is a `Result`, as
    /// `RpcSend::call_failable` does: a remote error becomes
    /// `RpcError::Call`. On success, `Ok(t)` and its ghost satisfy the
    /// caller's expectation. (On `RpcError::Call` the reply's ghost is
    /// dropped.)
    #[verifier::external_body]
    pub async fn recv_failable(self) -> (result: Result<(T, Tracked<M::GhostOut>), RpcError<E>>)
        ensures
            result matches Ok((t, gout)) ==> self.reply_ok(Ok(t), gout@),
    {
        match self.pending.await {
            // TRUSTED: keep this `assume_new()`. This trusted function's
            // contract hands out the ghost; its body has none to give.
            Ok(Ok(t)) => Ok((t, Tracked::assume_new())),
            Ok(Err(e)) => Err(RpcError::Call(e)),
            Err(RpcError::Channel(e)) => Err(RpcError::Channel(e)),
        }
    }
}

/// Waits for the next message. Returns `None` when every sender is gone.
#[verifier::external_body]
pub async fn recv_msg<M: MeshProtocol>(receiver: &mut Receiver<M>) -> (result: Option<
    (M, Tracked<M::GhostIn>),
>)
    ensures
        final(receiver).constant() == old(receiver).constant(),
        final(receiver).attached() == old(receiver).attached(),
        result matches Some((msg, gin)) ==> M::request_ok(old(receiver).constant(), msg, gin@),
{
    match receiver.recv().await {
        // TRUSTED: keep this `assume_new()`. This trusted function's
        // contract hands out the ghost; its body has none to give.
        Ok(msg) => Some((msg, Tracked::assume_new())),
        Err(_) => None,
    }
}

/// Splits `rpc` into its input and an input-less `Rpc` with the same
/// expectation (for reply ghost type `G`).
#[verifier::external_body]
pub fn split<I, R: 'static + Send, G>(rpc: Rpc<I, R>) -> (result: (I, Rpc<(), R>))
    ensures
        result.0 == rpc.spec_input(),
        forall|r: R, gout: G| #[trigger] result.1.reply_ok(r, gout) == rpc.reply_ok(r, gout),
{
    rpc.split()
}

/// Answers `rpc` with `r` and the ghost `gout`.
#[verifier::external_body]
#[allow(unused_variables)]  // ghost-only parameters are erased at runtime
pub fn complete<I, R: 'static + Send, G>(rpc: Rpc<I, R>, r: R, gout: Tracked<G>)
    requires
        rpc.reply_ok(r, gout@),
{
    rpc.complete(r)
}

} // verus!

// In a normal build the trait's spec function is erased, so every type
// satisfies it, with no ghosts.
#[cfg(not(verus_keep_ghost))]
impl<T> MeshProtocol for T {
    type Const = ();
    type GhostIn = ();
    type GhostOut = ();
    type SenderGhost = ();
    type ReceiverGhost = ();
}

#[cfg(test)]
mod tests {
    // OUT OF SCOPE: keep the `assume_new()`s in this module (tests, not part
    // of verification).
    use super::*;
    use pal_async::DefaultDriver;
    use pal_async::async_test;
    use pal_async::task::Spawn;

    enum TestMsg {
        Double(Rpc<u32, u32>),
        Note(u32),
    }

    #[cfg(verus_keep_ghost)]
    verus! {

    impl MeshProtocol for TestMsg {
        type Const = ();
        type GhostIn = ();
        type GhostOut = ();
        type SenderGhost = ();
        type ReceiverGhost = ();

        open spec fn request_ok(k: (), msg: TestMsg, gin: ()) -> bool {
            true
        }
    }

    } // verus!

    #[async_test]
    async fn call_round_trip(driver: DefaultDriver) {
        let (mut send, mut recv) = channel::<TestMsg>(
            Ghost::assume_new(),
            Tracked::assume_new(),
            Tracked::assume_new(),
        );
        let _ = take_sender_ghost(&mut send);
        let send2 = clone_sender(&send);
        let server = driver.spawn("server", async move {
            let _ = take_receiver_ghost(&mut recv);
            while let Some((msg, _)) = recv_msg(&mut recv).await {
                if let TestMsg::Double(rpc) = msg {
                    let (input, rpc) = split::<u32, u32, ()>(rpc);
                    complete(rpc, input * 2, Tracked::<()>::assume_new());
                }
            }
        });
        let (r, _) = call(
            &send2,
            TestMsg::Double,
            21,
            Tracked::assume_new(),
            Ghost::assume_new(),
        )
        .recv()
        .await
        .unwrap();
        assert_eq!(r, 42);
        drop(send);
        drop(send2);
        server.await;
    }

    #[async_test]
    async fn send_msg_arrives(_driver: DefaultDriver) {
        let (send, mut recv) = channel::<TestMsg>(
            Ghost::assume_new(),
            Tracked::assume_new(),
            Tracked::assume_new(),
        );
        send_msg(&send, TestMsg::Note(7), Tracked::assume_new());
        drop(send);
        let Some((TestMsg::Note(n), _)) = recv_msg(&mut recv).await else {
            panic!("expected the one-way message");
        };
        assert_eq!(n, 7);
        assert!(recv_msg(&mut recv).await.is_none());
    }
}

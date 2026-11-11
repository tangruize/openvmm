// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Host identity of guest filesystem requests (`--mount-owner`).
//!
//! In caller mode, a worker thread performs each request as the host UID and
//! GID of its guest caller, as the guest kernel reports them in the FUSE
//! request header. Guest UID 0 and GID 0 are squashed to the owner of the
//! export root, so the guest can create neither root-owned nor setuid-root
//! files on the host. A request that cannot run as its caller fails with
//! `EPERM`; it never falls back to the VMM's own identity.

use fuse::protocol::FUSE_BATCH_FORGET;
use fuse::protocol::FUSE_DESTROY;
use fuse::protocol::FUSE_FORGET;
use fuse::protocol::FUSE_INIT;
use fuse::protocol::FUSE_INTERRUPT;
use fuse::protocol::FUSE_RELEASE;
use fuse::protocol::FUSE_RELEASEDIR;
#[cfg(verus_keep_ghost)]
use fuse::spec::FuseGhost;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

/// Verus specification of property 3, identity: as whom each request runs.
#[cfg(verus_keep_ghost)]
pub(crate) mod spec {
    include!("owner.spec.rs");
}

/// Maps guest callers to the host identities that perform their requests.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[cfg_attr(verus_keep_ghost, verifier::external_derive)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CallerIdentity {
    /// The owner of the export root, which replaces guest UID 0.
    squash_uid: lx::uid_t,
    /// The group of the export root, which replaces guest GID 0.
    squash_gid: lx::gid_t,
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl CallerIdentity {
    /// Builds the mapping for an export root owned by `uid` and `gid`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        ensures
            // A root owned by root is refused; else guest root becomes the
            // root's owner.
            uid == 0 || gid == 0 ==> r is Err,
            r matches Ok(ci) ==> ci.wf() && ci.squash() == (uid, gid),
    ))]
    pub(crate) fn for_export_root_owner(uid: lx::uid_t, gid: lx::gid_t) -> anyhow::Result<Self> {
        anyhow::ensure!(
            uid != 0 && gid != 0,
            "microVM virtio-fs caller ownership squashes guest root to the owner of the export root, which must not be UID 0 or GID 0"
        );
        Ok(Self {
            squash_uid: uid,
            squash_gid: gid,
        })
    }

    /// Returns the host UID and GID that perform a request from guest `uid`
    /// and `gid`.
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        ensures
            r == self.maps(uid, gid),
    ))]
    pub(crate) fn host_identity(&self, uid: lx::uid_t, gid: lx::gid_t) -> (lx::uid_t, lx::gid_t) {
        (
            if uid == 0 { self.squash_uid } else { uid },
            if gid == 0 { self.squash_gid } else { gid },
        )
    }
}

/// Returns whether handling a request with `opcode` may access the host
/// filesystem.
///
/// Negotiating the session and releasing host objects never check
/// permissions, so these requests run as the VMM. The guest can therefore
/// always mount, unmount, and release what it opened, even when its other
/// requests fail closed.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verus_spec(r =>
    ensures
        r == CallerIdentity::touches_host_files(opcode),
))]
fn accesses_host_files(opcode: u32) -> bool {
    !matches!(
        opcode,
        FUSE_INIT
            | FUSE_DESTROY
            | FUSE_FORGET
            | FUSE_BATCH_FORGET
            | FUSE_INTERRUPT
            | FUSE_RELEASE
            | FUSE_RELEASEDIR
    )
}

/// Dispatches `request` to `session`, performing it as the host identity of
/// its caller when `caller_identity` is set.
///
/// Verus: lends the request's ghost state `g` and the thread's credential
/// token `cred` (its own, before and after) on to the session.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verifier::allow(undeclared_external_trait))]
#[cfg_attr(verus_keep_ghost, verus_spec(
    with Tracked(g): Tracked<&mut crate::spec::VirtioFsGhost>,
        Tracked(cred): Tracked<&mut lx::cred::ThreadCred>
    requires
        session.owns_ghost(old(g)),
        old(g).wf(),
        old(cred)@ == lx::cred::ThreadCredView::Own,
    ensures
        session.owns_ghost(final(g)),
        final(g).wf(),
        final(cred)@ == lx::cred::ThreadCredView::Own,
))]
pub(crate) fn dispatch(
    caller_identity: Option<&CallerIdentity>,
    session: &fuse::Session<crate::spec::VirtioFsGhost>,
    request: fuse::Request,
    sender: &mut impl fuse::ReplySender,
    mapper: Option<&dyn fuse::Mapper>,
) {
    dispatch_with(
        caller_identity,
        session,
        request,
        sender,
        mapper,
        &mut HostIdentity::default(),
    );
}

/// Dispatches `request` like [`dispatch`], but performs a request that runs
/// as its caller through `identity`, which switches the thread to the
/// caller's host UID and GID, or fails, and switches it back.
///
/// Verus: the proof of this body shows property 3. It must call
/// `session.dispatch` with `runs_as` = `CallerIdentity::runs_as(
/// caller_identity, request@)` (SPEC: that ghost argument is part of the
/// spec), which `Session::dispatch` requires the credential token to be; the
/// token is `Switched` only between `identity.switch` and `identity.restore`.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[cfg_attr(verus_keep_ghost, verifier::allow(undeclared_external_trait))]
#[cfg_attr(verus_keep_ghost, verus_spec(
    with Tracked(g): Tracked<&mut crate::spec::VirtioFsGhost>,
        Tracked(cred): Tracked<&mut lx::cred::ThreadCred>
    requires
        session.owns_ghost(old(g)),
        old(g).wf(),
        old(cred)@ == lx::cred::ThreadCredView::Own,
    ensures
        session.owns_ghost(final(g)),
        final(g).wf(),
        final(cred)@ == lx::cred::ThreadCredView::Own,
))]
pub(crate) fn dispatch_with(
    caller_identity: Option<&CallerIdentity>,
    session: &fuse::Session<crate::spec::VirtioFsGhost>,
    request: fuse::Request,
    sender: &mut impl fuse::ReplySender,
    mapper: Option<&dyn fuse::Mapper>,
    identity: &mut impl SwitchIdentity,
) {
    match caller_identity {
        Some(caller_identity) if accesses_host_files(request.opcode()) => {
            let unique = request.unique();
            let (uid, gid) = caller_identity.host_identity(request.uid(), request.gid());
            match identity.switch(uid, gid) {
                Ok(()) => {
                    session.dispatch(request, sender, mapper);
                    identity.restore();
                }
                Err(error) => {
                    tracelimit::warn_ratelimited!(
                        uid,
                        gid,
                        error = error.value(),
                        "virtio-fs request cannot run as its caller; failing it with EPERM"
                    );
                    if let Err(error) = sender.send_error(unique, lx::Error::EPERM.value()) {
                        tracelimit::error_ratelimited!(
                            unique,
                            error = &error as &dyn std::error::Error,
                            "Failed to send reply",
                        );
                    }
                }
            }
        }
        _ => session.dispatch(request, sender, mapper),
    }
}

/// Switches the calling thread's filesystem identity for a request, and back
/// ([`HostIdentity`]; the tests use their own).
///
/// Verus (for verification only; `virtiofs-spec-docs/CODE-CHANGES.md`): this
/// replaces a `run_as` closure that took the operation as `&mut dyn FnMut()`,
/// which Verus cannot specify, so that `dispatch_with` switches, dispatches,
/// and restores in straight-line code.
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub(crate) trait SwitchIdentity {
    /// Switches the thread to host `uid` and `gid`, or fails (then the thread
    /// keeps its own credentials).
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        with Tracked(cred): Tracked<&mut lx::cred::ThreadCred>
        requires
            old(cred)@ == lx::cred::ThreadCredView::Own,
        ensures
            r is Ok ==> final(cred)@ == (lx::cred::ThreadCredView::Switched { uid, gid }),
            r is Err ==> final(cred)@ == lx::cred::ThreadCredView::Own,
    ))]
    fn switch(&mut self, uid: lx::uid_t, gid: lx::gid_t) -> lx::Result<()>;

    /// Switches the thread back to its own credentials.
    #[cfg_attr(verus_keep_ghost, verus_spec(
        with Tracked(cred): Tracked<&mut lx::cred::ThreadCred>
        requires
            old(cred)@ is Switched,
        ensures
            final(cred)@ == lx::cred::ThreadCredView::Own,
    ))]
    fn restore(&mut self);
}

/// The host's identity switching (`lxutil::switch_fs_identity`), which the
/// profile accepts only on Linux.
#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
#[derive(Default)]
pub(crate) struct HostIdentity {
    /// The guard of the current switch, which restores the thread's
    /// credentials (also when dropped, on a panic).
    #[cfg(target_os = "linux")]
    guard: Option<lxutil::FsIdentityGuard>,
}

#[cfg_attr(verus_keep_ghost, verus_verify)]
impl SwitchIdentity for HostIdentity {
    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(cred): Tracked<&mut lx::cred::ThreadCred>))]
    fn switch(&mut self, uid: lx::uid_t, gid: lx::gid_t) -> lx::Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.guard = Some(lxutil::switch_fs_identity(lxutil::FsIdentity { uid, gid })?);
            Ok(())
        }
        // Fails, because the profile accepts caller ownership only on Linux.
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (uid, gid);
            Err(lx::Error::EPERM)
        }
    }

    #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
    #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(cred): Tracked<&mut lx::cred::ThreadCred>))]
    fn restore(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(guard) = self.guard.take() {
            guard.restore();
        }
    }
}

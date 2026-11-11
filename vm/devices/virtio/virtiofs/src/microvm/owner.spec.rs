// Spec of property 3, identity, for human review: as whom the host performs
// each request (written 2026-11-11; `virtiofs-spec-docs/notes-20261111/
// DESIGN-identity.md`).
//
// THE PROPERTY (`CallerIdentity::runs_as`). In the default owner mode
// (`Vmm`, `caller_identity` is `None`), every request runs as OpenVMM, with
// the device thread's own credentials. In `Caller` mode (`--mount-owner
// caller`, Linux only), a request that touches host files runs as its
// caller, mapped: guest UID 0 becomes the share root's owner, and guest
// GID 0 its group; every other id passes through, so the host id is never
// 0 (`CallerIdentity::wf`). The seven requests that touch no host file
// (`touches_host_files`) run as OpenVMM. "Its caller" means the ids in the
// request's header, which the guest's kernel writes: the security content
// is "never host root, never OpenVMM's capabilities or groups", not "the
// right user".
//
// CHECKED: the mapping (`maps`, `wf`, the contracts of
// `for_export_root_owner`, `host_identity` and `accesses_host_files`).
//
// TRUSTED, in words (the identity's trusted part, as with confinement):
// (T1) `lxutil::with_fs_identity` (`lxutil/src/unix/credentials.rs`)
// changes the calling thread's credentials only: `setfsuid`, `setfsgid`,
// `setgroups`, and `capset` of the thread. (T2) While its operation runs,
// the thread's filesystem ids are the identity, its supplementary groups at
// most that gid, and it has no effective capability; if it cannot take the
// identity, the operation does not run and it fails with `EPERM`. (T3)
// After the operation, the thread's credentials are what they were (or the
// process panics). (T4) The operation runs synchronously on that thread.
// And: every host call of a request is made inside `session.dispatch`,
// which `dispatch_with` runs whole inside `run_as` (`owner.rs`, lexically),
// and the seven requests that touch no host file make no `lxutil` call
// (`release` closes a file descriptor).
//
// HOW IT CAN BECOME A CONTRACT. `run_as` takes its operation as
// `&mut dyn FnMut()`, which Verus cannot specify; once it takes a generic
// closure (the change the keeper's proof already needs, `dispatch`'s doc),
// `with_fs_identity` gets a trusted contract: the operation runs with a
// thread-credential token whose view is the identity, and `dispatch_with`
// ensures that `session.dispatch` ran only as `runs_as` says, or not at all
// (and the reply is `EPERM`). Checking every `lxutil` call against the
// token (each would take it) adds little beyond that one wrap point until
// the host model also says what an identity may do (decided 2026-11-11).
//
// NOT CLAIMED: what the kernel then permits that identity (mode bits, ACLs,
// the owner of new files); that a write through a handle opened earlier by
// another caller is refused (a descriptor keeps its opener's rights); and
// that restore's reopening of handles, which runs as OpenVMM, is no more
// powerful than the original open (Pedro documents it).

use super::CallerIdentity;
use vstd::prelude::*;

verus! {

/// As whom the host performs a request.
pub(crate) enum RunAs {
    /// As OpenVMM: the device thread's own credentials.
    Vmm,
    /// As host user `uid` and group `gid`, with no other group and no
    /// capability.
    Caller { uid: u32, gid: u32 },
}

impl CallerIdentity {
    /// The share root's owner and group, which replace guest UID 0 and GID 0.
    pub(crate) closed spec fn squash(&self) -> (u32, u32) {
        (self.squash_uid, self.squash_gid)
    }

    /// Neither replacement is 0: the mapped identity is never host root.
    pub(crate) closed spec fn wf(&self) -> bool {
        self.squash_uid != 0 && self.squash_gid != 0
    }

    /// The host identity of guest `uid` and `gid`: guest root becomes the
    /// share root's owner (and group), any other id passes through.
    pub(crate) closed spec fn maps(&self, uid: u32, gid: u32) -> (u32, u32) {
        (
            if uid == 0 { self.squash_uid } else { uid },
            if gid == 0 { self.squash_gid } else { gid },
        )
    }

    /// The request may make a host call: all but `INIT`, `DESTROY`,
    /// `FORGET`, `BATCH_FORGET`, `INTERRUPT`, `RELEASE` and `RELEASEDIR`.
    pub(crate) open spec fn touches_host_files(opcode: u32) -> bool {
        !(opcode == fuse::protocol::FUSE_INIT || opcode == fuse::protocol::FUSE_DESTROY
            || opcode == fuse::protocol::FUSE_FORGET || opcode == fuse::protocol::FUSE_BATCH_FORGET
            || opcode == fuse::protocol::FUSE_INTERRUPT || opcode == fuse::protocol::FUSE_RELEASE
            || opcode == fuse::protocol::FUSE_RELEASEDIR)
    }

    /// Property 3: as whom the host performs `request`, with the device's
    /// mapping `caller_identity` (`None` in the default `Vmm` mode).
    pub(crate) open spec fn runs_as(caller_identity: Option<CallerIdentity>, request: fuse::RequestView) -> RunAs {
        match caller_identity {
            Some(ci) if Self::touches_host_files(request.opcode) => {
                let (uid, gid) = ci.maps(request.uid, request.gid);
                RunAs::Caller { uid, gid }
            },
            _ => RunAs::Vmm,
        }
    }
}

} // verus!

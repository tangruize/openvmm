// Spec of property 3, identity, for human review: as whom the host performs
// each request (written 2026-11-11; `virtiofs-spec-docs/notes-20261111/
// DESIGN-identity.md`).
//
// THE PROPERTY (`CallerIdentity::runs_as`, as the thread credential token's
// view, `lx/src/cred_t.rs`). In the default owner mode
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
// IN THE SPEC: the mapping (`maps`, `wf`, the contracts of
// `for_export_root_owner`, `host_identity` and `accesses_host_files`); and
// that the request runs as `runs_as`: `Session::dispatch` requires the
// thread's credential token (`lx::cred::ThreadCred`) to be `runs_as`, and
// `dispatch_with` calls it with `CallerIdentity::runs_as(caller_identity,
// request@)` (its proof shows the token is that: `Own` in `Vmm` mode and for
// the seven exempt requests, `Switched` with the mapped ids between
// `SwitchIdentity::switch` and `restore` otherwise). This needed a change for
// verification only (`CODE-CHANGES.md`): `run_as`'s closure became the
// `SwitchIdentity` trait, and `lxutil` exposes `switch_fs_identity`'s two
// halves.
//
// TRUSTED: the token's meaning, as a contract on `lxutil`'s
// `switch_fs_identity` and `FsIdentityGuard::restore` (`lx/src/cred_t.rs`,
// `lxutil/src/unix/credentials.rs`): (T1) they change only the calling
// thread's credentials (`setfsuid`, `setfsgid`, `setgroups`, `capset`); (T2)
// after a switch, the thread's filesystem ids are the identity, its
// supplementary groups at most that gid, and it has no effective capability,
// or it fails with `EPERM` and nothing changed; (T3) `restore` (or a drop,
// on a panic) brings the thread's own credentials back. In words only:
// every host call of a request is made inside `Session::dispatch` (it is the
// one place a request's operation runs), and the seven exempt requests make
// no `lxutil` call (`release` closes a file descriptor). Checking every
// `lxutil` call against the token (each would take it) adds little until the
// host model also says what an identity may do (decided 2026-11-11).
//
// NOT CLAIMED: what the kernel then permits that identity (mode bits, ACLs,
// the owner of new files); that a write through a handle opened earlier by
// another caller is refused (a descriptor keeps its opener's rights); and
// that restore's reopening of handles, which runs as OpenVMM, is no more
// powerful than the original open (Pedro documents it).

use super::CallerIdentity;
use vstd::prelude::*;

verus! {

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
    /// mapping `caller_identity` (`None` in the default `Vmm` mode): with
    /// the thread's own credentials (OpenVMM's), or switched to the caller's
    /// mapped ids.
    pub(crate) open spec fn runs_as(caller_identity: Option<CallerIdentity>, request: fuse::RequestView) -> lx::cred::ThreadCredView {
        match caller_identity {
            Some(ci) if Self::touches_host_files(request.opcode) => {
                let (uid, gid) = ci.maps(request.uid, request.gid);
                lx::cred::ThreadCredView::Switched { uid, gid }
            },
            _ => lx::cred::ThreadCredView::Own,
        }
    }
}

} // verus!

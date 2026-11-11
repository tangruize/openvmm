// The keeper of the file system's ghost state between requests.
//
// TRUSTED, do not attempt to prove anything in this file: Verus uses only
// the contracts below. Nothing here exists at runtime (these are proof
// functions, which a normal build removes), so the executable is unchanged.
//
// WHAT THIS IS FOR. Every FUSE operation of virtio-fs takes the file
// system's ghost state `g` (`VirtioFsGhost`: the whole-map pieces that tie
// the guest's state to the data behind the locks; `lib.spec.rs`). Some
// operations change it (a `create` adds a node). Between two requests, `g`
// has to be kept somewhere, so that the next request gets the `g` that the
// previous one left. The natural place would be a field of the worker that
// handles requests; but there are two workers, one per queue (the
// high-priority queue and the request queue), each its own task, and both
// handle requests, so both need `g`; and a tracked value, like any value, can
// be in only one place. Sharing it between them in a way that Verus checks
// would need a lock, which would change the executable (an extra lock and
// unlock per request). So the keeping is trusted instead, here.
//
// HOW IT IS USED.
//
// 1. Once, when the file system is created: `VirtioFs::new_microvm` creates
//    the ghost pieces together with the inode table and the file table (half
//    of each `FracGhost` into its table's lock, the rest into `g`), so that
//    `g` matches the locks, and hands `g` out; its contract demands it (a `g`
//    that the new file system owns, well formed, and new), so its proof must
//    create it. The device's setup (`VirtioFsDevice::new_microvm_hostfs`)
//    then hands `g` to the keeper: `keep_fs_ghost(fs, g)`.
// 2. For each request (`process_virtiofs_request`, which both queue workers
//    call): `take_fs_ghost(session)` gives out `g` (and `take_thread_cred()`
//    the thread's credential token, its own, which goes back with
//    `give_back_thread_cred` the same way); the request lends it down
//    to its operation (`owner::dispatch` -> `Session::dispatch` -> the
//    handler, e.g. `VirtioFs::create`); then `give_back_fs_ghost(session, g)`
//    takes it back, as the operation left it.
//
//    request worker:  take_fs_ghost  -> g
//                     create(.., g)     (may change g: a new node)
//                     give_back(g)   -> the worker no longer has g
//    hiprio worker:   take_fs_ghost  -> g   (as create left it)
//                     forget(.., g)
//                     give_back(g)
//
// WHAT IS ASSUMED (not checked by Verus):
//
// A1. A take is always followed by its give-back before the next take, so at
//     any moment at most one request has `g`. True because requests never
//     overlap: both queue workers are tasks on one thread (virtio-fs's driver
//     is `driver_source.simple()`, the shared device thread), and
//     `process_virtiofs_request` is a plain function, with no `await`, so a
//     request runs to its end before the thread switches tasks
//     (`virtiofs-spec-docs/CODE-OVERVIEW.md`, "Every request runs alone").
//     This breaks if virtio-fs gets a driver of its own (a target CPU) or
//     several threads, or if request handling becomes `async`.
// A2. Only `process_virtiofs_request` takes and gives back, and the keeper
//     holds one ghost state per file system: the one its creation kept. So
//     what a take gives out is what the last give-back (or the creation) left,
//     and it is this session's file system's. Not yet true across a restore:
//     `restore_microvm_state` (`microvm/fs.rs`) replaces both tables but not
//     the `g` kept here (virtio-fs's save and restore, property 6, are not
//     specified yet; the plan, in `virtiofs-spec-docs/notes-20261111/
//     DESIGN-snapshot.md`, is that restore borrows `g` from the keeper, and
//     this assumption then names restore too).
//
// The contracts say only what the operations need: the `g` given out is this
// session's file system's (`owns_ghost`) and well formed (`wf`); a
// give-back requires the same, which every operation's contract keeps.

use crate::VirtioFs;
use crate::spec::VirtioFsGhost;
use fuse::Session;
use fuse::spec::FuseGhost;
use fuse::spec::FuseOwns;
use vstd::prelude::*;

verus! {

/// Once, at the file system's creation: the keeper now holds `g`.
pub axiom fn keep_fs_ghost(fs: &VirtioFs, tracked g: VirtioFsGhost)
    requires
        fs.owns(&g),
        g.wf(),
;

/// At the start of a request: the keeper gives out the file system's ghost
/// state (assumptions A1 and A2).
pub axiom fn take_fs_ghost(session: &Session<VirtioFsGhost>) -> (tracked g: VirtioFsGhost)
    ensures
        session.owns_ghost(&g),
        g.wf(),
;

/// At the end of a request: the keeper takes the ghost state back, as the
/// request's operation left it.
pub axiom fn give_back_fs_ghost(session: &Session<VirtioFsGhost>, tracked g: VirtioFsGhost)
    requires
        session.owns_ghost(&g),
        g.wf(),
;

/// At the start of a request: the keeper also gives out the device thread's
/// credential token (`lx::cred::ThreadCred`, property 3), which is the
/// thread's own between requests: a request that switched the thread's
/// identity switched it back (`owner::dispatch` ensures it). A1 holds for
/// the token as for `g`: one request at a time has it.
pub axiom fn take_thread_cred() -> (tracked cred: lx::cred::ThreadCred)
    ensures
        cred@ == lx::cred::ThreadCredView::Own,
;

/// At the end of a request: the keeper takes the token back, the thread's
/// own again.
pub axiom fn give_back_thread_cred(tracked cred: lx::cred::ThreadCred)
    requires
        cred@ == lx::cred::ThreadCredView::Own,
;

} // verus!

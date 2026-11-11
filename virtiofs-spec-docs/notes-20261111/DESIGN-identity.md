# Design prep: property 3, identity (each request runs as its caller)

(2026-11-11. Read-only study of branch `system-proof/virtiofs-spec-20261108`.
Author of the code: Pedro, `8c01aabc1` "lxutil: switch filesystem
credentials per thread on Linux" and `df92e9fda` "virtiofs: perform microVM
share requests as the guest caller", both 2026-10-05.)

## 0. First fact: identity is opt-in

`MicroVmOwnerMode` (`microvm/profile.rs:95-104`): **`Vmm` is the default**
(`profile.rs:493, 524`), every request runs as OpenVMM. `Caller`
(`--mount-owner caller`, `resolver.rs:28-34`) is refused off Linux
(`profile.rs:535-537`, `UnsupportedOwnerMode`). So "nothing runs as
OpenVMM" holds only in `Caller` mode on Linux; in `Vmm` mode and on Windows
everything runs as OpenVMM.

## 1. What the code does, for one request

**Setup (once, as OpenVMM).** `new_microvm` opens the root and builds the
mapping from its owner: `CallerIdentity::for_export_root_owner(root_stat.uid,
root_stat.gid)` (`microvm/fs.rs:295-301`), which refuses UID 0 or GID 0
(`owner.rs:38-47`); an aggregate needs all children to have one owner
(`fs.rs:358-374`). The device copies it to each queue worker (`device.rs:62`,
`virtio.rs:314, 348`).

**The mapping** (`owner.rs:51-56`), UID and GID independently:

```rust
( if uid == 0 { self.squash_uid } else { uid },
  if gid == 0 { self.squash_gid } else { gid } )
```

The squash values are nonzero, so the host identity is never 0; every other
ID passes through.

**Per request.** `process_virtiofs_request` (`virtio.rs:467`) ->
`owner::dispatch` (`virtio.rs:518`) -> `dispatch_with(.., run_as)`
(`owner.rs:106, 120-147`): if a mapping exists and
`accesses_host_files(opcode)`, it maps the header's `uid`/`gid`
(`fuse/src/request.rs:111-118`: whatever the guest kernel wrote) and runs
the **whole** `session.dispatch(..)` inside `run_as` (`owner.rs:123-129`);
otherwise it dispatches directly, as OpenVMM (`owner.rs:146`). Both queues
go through this.

**Not wrapped:** `INIT, DESTROY, FORGET, BATCH_FORGET, INTERRUPT, RELEASE,
RELEASEDIR` (`owner.rs:66-77`). None makes a permission-checked host call:
`release` drops the `LxFile` (close, `lib.rs:602-605`), `destroy` clears the
tables (`lib.rs:821-830`), `init` sets `max_write` (`microvm/fs.rs:93-100`).

**`run_as`** (`owner.rs:150-160`): Linux, `lxutil::with_fs_identity(
FsIdentity { uid, gid }, operation)`; elsewhere `Err(EPERM)` (defense in
depth: the profile refuses `Caller` there, and restore needs the same mode,
`microvm/state.rs:357`).

**`with_fs_identity`** (`lxutil/src/unix/credentials.rs:78-83`):
`switch(identity).map_err(|_| EPERM)?; operation(); drop(guard)`. `switch`
(`:101-140`): refuses `u32::MAX` (`:102`); saves fs IDs, caps, groups; raw
`setgroups([])` if any group differs from `gid` (`:113-117`); `setfsgid`,
`setfsuid`, then reads them back, since neither reports failure (`:118-126`);
raw `capset` clears effective caps (`:127-138`).

**Failure.** Any step fails: the operation does not run, the guard undoes
the partial change, and `dispatch_with` logs and replies `EPERM`
(`owner.rs:130-143`); never a fallback to OpenVMM. Host errors inside the
operation are returned as usual.

**Restore.** `Drop for SwitchedCredentials` (`:143-169`): caps, fs IDs,
groups, caps again (fsuid to/from 0 adjusts caps); **panics** if it cannot.
The guard is a local, so it also restores when unwinding from a panic.

**Why per-thread is enough.** Raw syscalls, not glibc's broadcasting
wrappers (`credentials.rs:4-10`); a plain `FnMut()` closure, no `await` in a
request; so nothing else (including other devices' tasks on the shared
device thread) runs while switched.

**Ownership of new files.** `create`/`mkdir`/`mknod`/`symlink` pass the
guest's **raw, unsquashed** `request.uid()/gid()` (`lib.rs:427, 463, 492,
521`) into `LxCreateOptions` (`inode.rs:406, 428, 448, 470`), but those
fields are dead code on Linux (`lxutil/src/lib.rs:1215-1218`). The owner
comes from the fs identity, so the squash works through the switch.
`set_attr`'s `thread_uid` is ignored on Linux (`lxutil/src/lib.rs:174,
217`); a guest `chown` runs without `CAP_CHOWN` and the kernel refuses it.

**Host access outside a request, as OpenVMM:** setup (root, `lstat` of pinned
paths, `microvm/fs.rs:207-215`); save (`lstat` of aliases, `fstat` of handles,
`fs.rs:524-531, 584-586`); restore (re-`lstat`, and **reopening every saved
handle**, `fs.rs:752-760, 799-803`). Pedro's commit says so ("Capture and
restore still inspect and reopen the guest's files as OpenVMM"), and in
`Caller` mode OpenVMM holds `CAP_SETUID`/`CAP_SETGID`. Closing fds
(`FORGET`/`RELEASE`/`DESTROY`) needs no permission.

**The Guide's warning** (`Guide/.../virtio-fs.md:295-304`): the guest kernel
chooses the header IDs, so guest root or a hostile kernel can act as **any
nonzero host UID/GID** inside the share, leaving setuid files owned by it.

## 2. The property, precisely

What the code supports, for a `VirtioFs` whose `caller_identity` is
`Some(ci)` (`Caller` mode, Linux):

> **P3.** Every host call that `lxutil` makes while handling a request `r`
> whose opcode is not in the exempt list is made on the device thread while
> that thread's filesystem identity is `ci.host_identity(r.uid, r.gid)`. At
> that point the thread has no supplementary group except that GID and no
> effective capability. That identity is never UID 0 or GID 0. A request
> whose identity cannot be taken makes no host call and fails with `EPERM`.
> Exempt requests make no `lxutil` call. After every request the thread's
> credentials are what they were before.

In `Vmm` mode: every host call runs with the thread's own credentials.

What it **cannot** say without a host model:
- What the kernel then permits for that identity: mode bits, ACLs, the
  sticky bit, the owner given to new files (beyond "the fs identity").
- That a write through a handle opened earlier by another caller is
  refused. Unix fds keep the opener's rights, so P3 is about the
  identity of each call, not about who may write.
- That save/restore's reopen is no more powerful than the original open.
- That the header's UID is the "real" guest process. It is attacker input.
  The security content of P3 is "never host root, never OpenVMM's caps or
  groups", not "the right user".

## 3. How to specify it in the existing structure

Same pattern as confinement (open item 9): a checked part plus trusted
words.

**Checked, cheap: the mapping** (`owner.rs`; `RequestView` in
`fuse/src/request.spec.rs` gains `uid`, `gid`, `opcode`, spec only):

```rust
impl CallerIdentity {
    pub open spec fn maps(&self, uid: u32, gid: u32) -> (u32, u32) {
        (if uid == 0 { self.squash_uid } else { uid },
         if gid == 0 { self.squash_gid } else { gid })
    }
    pub open spec fn wf(&self) -> bool { self.squash_uid != 0 && self.squash_gid != 0 }
}
// for_export_root_owner: ensures r is Ok ==> r->Ok_0.wf() && squash == (uid, gid)
// host_identity:         ensures r == self.maps(uid, gid), self.wf() ==> r.0 != 0 && r.1 != 0
```

**Trusted: `with_fs_identity`**, with a linear token for the thread's
credentials (`lxutil/src/unix/credentials_t.rs`):

```rust
pub enum HostCred { Own, Switched(FsIdentity) }   // Own = the thread's original
pub tracked struct ThreadCred { .. }               // not Copy; minted only by trusted fns
pub fn with_fs_identity<T, G>(identity: FsIdentity, op: F) -> (r: lx::Result<T>)
  where F: FnOnce(Tracked<&mut G>, Tracked<&ThreadCred>) -> T
  with Tracked(own): Tracked<&mut ThreadCred>, Tracked(g): Tracked<&mut G>
  requires old(own)@ is Own,
           forall|c: ThreadCred| c@ == HostCred::Switched(identity) ==> op.requires((old(g), &c)),
  ensures  final(own)@ is Own,                                        // restored
           r is Err ==> r->Err_0.code() == lx::EPERM && final(g) == old(g), // op did not run
           r is Ok ==> exists|c: ThreadCred| c@ == HostCred::Switched(identity)
                         && op.ensures((old(g), &c), r->Ok_0),
           identity.uid == u32::MAX || identity.gid == u32::MAX ==> r is Err,
```

In words at the top of the file: (T1) the raw `setfsuid`, `setfsgid`,
`setgroups`, `capset(pid 0)` change only the calling thread; (T2) while `op`
runs, the thread's fs IDs are `identity`, its groups within `{gid}`, no
effective caps; (T3) Drop restores or panics; (T4) `op` runs synchronously on
this thread. `G` passes through only because Verus cannot capture `&mut g`
in a closure (CODE-OVERVIEW, "Passing the ghost state down"). The `Own`
token comes from the keeper: `take_fs_ghost` also hands it, `give_back`
requires `Own` again.

**Option A (full): every `lxutil` call needs the token.** The 16 `LxVolume`
methods and the `LxFile` methods virtio-fs uses get `with Tracked(cred):
Tracked<&ThreadCred>`, no requirement on its value; in words, "the method's
host calls run on the calling thread, whose credentials are `cred@`". The
property is stated once, at `Session::dispatch`, as a new FUSE question:

```rust
spec fn runs_as(&self, r: RequestView, c: HostCred) -> bool;   // fuse; virtio-fs answers:
open spec fn runs_as(&self, r: RequestView, c: HostCred) -> bool {
    match self.caller_identity_spec() {
        None => c is Own,
        Some(ci) => if accesses_host_files(r.opcode) { c == HostCred::Switched(ci.maps(r.uid, r.gid).into()) }
                    else { c is Own },
    }
}
// Session::dispatch:  with Tracked(cred)  requires self.fs_runs_as(request@, cred@)
```

The `Fuse` methods take `cred` next to `g`, except the seven exempt ones,
which take none: Verus then checks they make no `lxutil` call. Since the
token is linear, the one reaching each `lxutil` call is the one `dispatch`
got, so P3 follows. Wrinkle: `fuse` depends only on `lx`, so the token type
must live in `lx` or be a second generic (`Fuse<G, C>`). Setup, save and
restore need an `Own` token from a trusted `ThreadCred::outside_request()`
("only these three call it", like the keeper's A2).

**Option B (light): check the dispatch, trust the rest.** Verify only
`owner::dispatch_with` (with the mapping and `with_fs_identity`):

```rust
// ensures (shape): caller_identity is Some(ci) && accesses_host_files(request@.opcode) ==>
//   session.dispatch ran only with cred@ == Switched(ci.maps(request@.uid, request@.gid))
//   || (it did not run && the reply is EPERM)
```

Trusted in words: every host call of a request happens inside
`session.dispatch` (lexically true, `owner.rs:125-129`), and the exempt
handlers make no `lxutil` call; both reviewable by grep.

**Not in `VirtioFsGhost`:** every handler would then have to ensure
`final(g).cred == old(g).cred` at the FUSE level, which does not know the
field. **Relation to the keeper:** P3 needs less than A1. Credentials are
per thread, so several threads would still be correct; only T4 (no `await`
inside the closure) is needed, so the identity proof survives the change
that would break the keeper.

## 4. Decisions for the lead

**D1. What does property 3 claim in `Vmm` mode, the default?** (a) Only
`Caller` mode, Linux; (b) both modes, via `runs_as` (`Vmm`: always the
thread's own credentials). **Recommend (b)**: one `match` arm, and it states
honestly that the default runs everything as OpenVMM.

**D2. How much is checked?** (a) Option A, a token on every `lxutil` call;
(b) Option B, only `dispatch_with`, the mapping and `with_fs_identity`'s
contract. **Recommend (b) now, (a) with the host model**: the single wrap
point covers the whole dispatch, so (a)'s ~90 call sites would check little
beyond a grep; with a host model the token becomes the identity the model's
permission checks use, and pays for itself.

**D3. Exempt opcodes and save/restore run as OpenVMM.** (a) Accept, in
words (they close fds only; save/restore are not requests); (b) ask Pedro to
reopen as the original caller (needs the opener's identity in the saved
state: a code change). **Recommend (a)**, with restore-as-OpenVMM recorded
in `BUGS.md` as a known limit: Pedro documents it, and the guest cannot
reach it during a request.

**D4. How strong is "caller"?** (a) "The header's IDs, mapped": the guest
chooses them; (b) also assume no other host user's files are in the share.
**Recommend (a)** in the contract, the Guide's warning in words; (b) is a
deployment rule the code does not enforce.

**D5. Where is the trusted contract?** (a) `lxutil/src/unix/credentials_t.rs`
with T1-T4 at its top (CONVENTIONS' `_t.rs`); (b) inline `external_body`.
**Recommend (a)**: the second trusted boundary after the keeper, to be read
as one.

## 5. Code changes, risks, size

**Code changes.** Option B: one, already needed by the keeper's proof:
`run_as` and `with_fs_identity` take a generic closure receiving
`Tracked<&mut G>` (and, under A, `Tracked<&ThreadCred>`) instead of
`&mut dyn FnMut()` (`owner.rs:118, 128`); `with_fs_identity` gains a
ghost-only type parameter `G`, runtime unchanged. Option A: no runtime
change, but ghost parameters on ~22 `lxutil` methods and every virtio-fs
helper that reaches them.

**Risks.**
- Verus with a closure of two tracked arguments, one `&mut`, in
  `op.requires`/`op.ensures`: only one tracked argument was tested. Test first.
- T2 is the substance and is trusted (e.g. Linux drops fs caps when fsuid
  leaves 0, handled at `:163-166`); the ignored tests
  (`credentials.rs:422-537`) are its evidence.
- For Pedro (not bugs): the raw `LxCreateOptions` uid/gid are unused on
  Linux, so a backend that honoured them would bypass the squash; restore
  reopens handles as OpenVMM.

**Size.** Option B: about 1 day (spec ~80-120 lines: `owner.rs` spec,
`RequestView` fields, `credentials_t.rs`; proof of `dispatch_with` and two
small functions). Option A: 3-5 more days, mostly mechanical threading.

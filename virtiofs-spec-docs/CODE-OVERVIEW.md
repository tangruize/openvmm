# virtio-fs: what the code does

An overview of OpenVMM's virtio-fs device, as a starting point for its
spec, in Pedro Henrique Penna's code: `microsoft/openvmm`, branch
`feature/microvm/dev`. This branch, `system-proof/virtiofs-spec-20261108`,
starts from that branch's commit `db42fde04` (2026-11-06) unchanged, with
only our Verus setup, scripts and documents added on top, so the code we
specify is exactly Pedro's at that commit. The spec's style rules and lessons are in
`snapshot-spec-docs/CONVENTIONS.md` and `snapshot-spec-docs/LESSONS.md`.

## What it is for

virtio-fs lets the guest use a host directory as its own filesystem. For
example, the host shares `/srv/data`, the guest mounts it at `/mnt`, and
when a guest program opens `/mnt/a.txt`, it is really `/srv/data/a.txt` on
the host.

## How a request travels

A guest program reads `/mnt/a.txt`:

1. The guest kernel turns the read into **FUSE requests**: "look up `a.txt`
   in the root", then "open it", then "read 4096 bytes".
2. It puts each request on a **virtio queue** (memory shared by guest and
   host).
3. OpenVMM takes the request off the queue, **performs it on the host**
   (opens `/srv/data/a.txt`, reads it), and puts the reply on the queue.

## The layers

Following that path, from the queue up (with the details of each, and of
what Pedro changed in it, in `CODE-OVERVIEW-DETAILS.md`):

0. **The generic virtio layer** (`vm/devices/virtio/virtio`): shared by
   every virtio device. It holds the ring buffer (the virtio queue in guest
   memory), the MMIO transport (the device's registers, which the guest
   reads and writes), and the `VirtioDevice` trait that each device
   implements.
1. **virtio glue** (`vm/devices/virtio/virtiofs/src/virtio.rs`,
   `virtio_util.rs`): the virtio-fs device, implementing `VirtioDevice`; its
   loop takes each request from layer 0's queue, passes it up, and gives
   the reply back. Mostly upstream code.
2. **FUSE layer** (`vm/devices/support/fs/fuse`, about 3,800 lines): decodes
   each request ("this is a `lookup` of `a.txt` in node 1"), agrees with the
   guest on the FUSE version and limits (`session.rs`), and calls the
   matching method of the `Fuse` trait.
3. **The filesystem** (`VirtioFs`, `virtiofs/src/lib.rs`): implements the
   `Fuse` trait's operations: `lookup`, `open`, `read`, `write`, `create`,
   `mkdir`, `rename`, `unlink`, `link`, `symlink`, the xattr operations,
   about 35 in all. It remembers what the guest holds:
   - **inodes**: "node 5 is `a.txt`" (`inode.rs`);
   - **open files**: "handle 9 is node 5, opened for reading" (`file.rs`).
4. **Host access** (`vm/devices/support/fs/lxutil`): performs the operation
   on the host, with Linux semantics. On Linux it opens paths with
   `openat2`, which refuses any path that would leave the shared directory
   (through `..` or a symlink).

## What Pedro added for the microVM

In `virtiofs/src/microvm/` and `aggregate.rs`, about 6,000 lines:

- **A fixed profile** (`profile.rs`): exactly one share slot, a fixed mount
  tag, a fixed FUSE version, a fixed number of queues, and a largest
  request size. The profile never contains host paths.
- **An access policy** (`policy.rs`): three lists of paths inside the share.
  - **Denied** paths hide themselves and everything below them. Deny
    `secrets`, and the guest cannot see `secrets/key`.
  - **Allowed** paths show part of a denied path again. Deny `home` but
    allow `home/pub`.
  - **Writable** paths, if a read-write share has any, are the only places
    the guest may change; the rest is read-only.

  A hidden directory on the way to an allowed path is **traverse-only**: in
  the example, the guest can look up `home` and sees only `pub` in it.
- **The caller's identity** (`owner.rs`): each request runs on the host as
  the UID and GID of the guest process that made it. Guest root (UID 0) is
  mapped to the owner of the shared directory, so the guest cannot create
  root-owned or setuid-root files on the host. A request that cannot run as
  its caller fails with `EPERM`; it never runs as OpenVMM itself.
- **Symlink checks** (`fs.rs`): before the guest creates a symlink on a
  writable share, its target is checked.
- **Limits** (`limits.rs`): caps on the number of inodes, aliases and open
  handles, and on path lengths, so that the guest cannot use up the host's
  memory.
- **Aggregates** (`aggregate.rs`): one share whose root is a synthetic,
  read-only directory, holding several host directories or files as named
  children, each with its own policy.
- **Snapshots** (`saved_state.rs`, `state.rs`, `fs.rs`): the saved state
  records relative names and the host objects' identities, never host paths
  or open handles. On restore, each host object is checked to be still the
  same one.
- **The Windows wrapper** (`lxutil/src/windows/`, about 7,600 lines): host
  access with Linux semantics on Windows hosts (NTFS), which the microVM
  uses on WHP (Pedro, 2026-11-09).

## Async, locks and RPC

- **Async: only in the virtio glue.** `virtio.rs` has an async loop per
  queue (`run`), and async start, stop, reset, `quiesce_input` and
  `resume_input`; `microvm/device.rs` saves and restores asynchronously,
  and `microvm/resolver.rs` builds the device asynchronously. The
  filesystem itself is synchronous: the loop takes a request, calls
  `process_virtiofs_request`, which runs the FUSE operation to the end,
  and completes it before taking the next. The `Fuse` trait's operations
  and `VirtioFs` have no `async`.
- **Little concurrency in the microVM.** It has two queues, the
  high-priority queue and one request queue (`MICROVM_REQUEST_QUEUES = 1`,
  `microvm/profile.rs`), and each has a worker; both workers run on one
  thread, so requests are handled one at a time ("Every request runs
  alone", below). The one place where two things meet is a save
  against running requests: each request first takes a `RequestGuard`
  from an admission gate (`self.admission.accept()`), and a save closes
  the gate, so no request is taken off the queue during a save
  (`virtio.rs`, the comment in `run`).
- **Locks: many, mostly `RwLock`**, because `VirtioFs` is shared (`Arc`)
  and written for upstream's several queues: `VirtioFs`'s `inodes`,
  `files` and `negotiation` (`lib.rs`); each inode's `path` and `aliases`
  (`inode.rs`); each open file's `file` and `directory_snapshot`
  (`file.rs`); the FUSE session's (`session.rs`). The inodes' lookup
  counts are atomics.
- **RPC: none.** `mesh` only encodes the saved state (`#[mesh(...)]` on
  the saved-state structs); no messages are sent.
- **Threads and the caller's identity.** Linux keeps credentials per
  thread, so the device thread switches its own filesystem UID and
  GID with raw system calls, performs the operation, and switches back
  (`lxutil/src/unix/credentials.rs`). The identity property rests on
  trusted facts about those system calls.

## Lock invariants the spec needs (proposed)

The guest's state is behind locks, and Verus knows what a lock holds only
through its rule (its invariant), so each lock gets one:

- **The inode table (`inodes`), each inode's `path` and `aliases`, and the
  file table (`files`)**: each holds a piece of the ghost state, and its
  rule says that its data matches that piece ("The guest's state in the
  spec", below).
- **Each open file's `file: RwLock<LxFile>`**: no rule for integrity (the
  handle's entry in `files` says whether it was opened for writing);
  what confinement needs of it is to decide when we work on confinement.
- **The session's locks and `negotiation`**: what was agreed with the guest
  (the FUSE version and limits) stays fixed after the start.

What makes this simpler than the snapshot spec: a share's policy never
changes once its volume is created (when the device is built, or by the
restore that attaches a share to a dormant device), and requests run one
at a time ("Every request runs alone", below; the admission gate between
save and requests is the main exception).

The locks: vstd's own verified `RwLock<V, Pred>` (`vstd/rwlock.rs`),
replacing `parking_lot::RwLock` where a lock needs an invariant (decided
2026-11-09), rather than a trusted wrapper over `parking_lot` like the
snapshot spec's `mutex_t.rs` (vstd has no `Mutex`, so that one had to be a
wrapper). Each lock gets its rule as a ghost value when it is created
(`RwLock::new(val, Ghost(pred))`), so the rule can name that lock's own
ghost ids, and nothing about the lock is trusted. The cost, in Pedro's
code: vstd's lock spins instead of sleeping (in the microVM it never
waits, since requests run one at a time), its handles are released by
an explicit `release_write` / `release_read`, not when dropped, and Verus
does not check that they are, and the call sites of `get_path()` (about
20) change to that form. A panic while one of these locks is held leaves it held (a
`parking_lot` guard was released as the panic unwound), so any later
request that takes the lock spins forever; accepted (2026-11-10), since a
panic in a request is already a device failure. Each is an entry in `CODE-CHANGES.md`.

**Who writes what in the locks' rules** (agreed 2026-11-10):

- *Spec (written here, reviewed by people):* the swap of each lock to
  vstd's `RwLock` and the ghost field next to its data (a code change); the
  rule's type, with the fields that `owns` needs (the ghost pieces' ids);
  and each rule's **core**: the ghost piece in the lock is this lock's
  entry of its ghost map, and it equals the data in the lock (for the path
  lock: `piece.id() == map_id`, `piece.key() == key`, and
  `piece.value() == v.path@`). The core is spec
  because it is the only thing that ties `g@` to the real data: without
  it, a contract such as `write`'s ("every name of the file is writable")
  would be about ghost names, not the paths the code hands to the host.
- *TODO for the proof agent:* everything else in the rules (extra facts and
  fields, e.g. how an inode's ghost key is chosen), the internal facts in
  `VirtioFsGhost::wf`, the real creation of the ghost pieces and rules
  (now `assume_new()`, marked PLACEHOLDER, e.g. in `VirtioFsInode::with_attr`),
  and all proofs. It must keep each rule's core. Each rule's comment marks
  its core (SPEC) and what is left (TODO).

The rules, with their cores: `PathRule` and `AliasesRule`
(`inode.spec.rs`: the piece is this inode's entry of `paths` / `aliases`,
and equals its path / names), `InodeTableRule` and `FileTableRule`
(`lib.spec.rs`: the half is the ghost table's, and its entries are the real
table's: each node's inode view; each handle's inode view and writability,
from its open flags). The two inode rules are written in full, as worked
examples; the proof may revise all but the cores. Left as TODO in the table
rules: the links across locks (each table inode's locks' rules have the
recorded key and the maps' ids; each handle's recorded node is one whose
inode is the file's), and that each lock holds a half.

## The guest's state in the spec: real data and ghost state (agreed 2026-11-09)

**The real data.** One `VirtioFs` is the whole filesystem the guest sees
(one share, or, in an aggregate, several shares under a synthetic root).
It owns one inode table and one file table, shared by all its shares:

```
VirtioFs
└── inner: Arc<VirtioFsInner>
    │
    ├── inodes: RwLock<InodeMap>                 LOCK 1: the whole inode table
    │   │                                        ghost: FracGhost half of `inodes`
    │   │
    │   └── node id ──► Arc<VirtioFsInode>
    │         │
    │         ├── path: RwLock<PathBuf>                    LOCK 2 (one per inode)
    │         │     its main path in the share             ghost: GhostPointsTo(n -> path),
    │         │                                                   entry n of `paths`
    │         │
    │         ├── aliases: RwLock<BTreeSet<PathBuf>>       LOCK 3 (one per inode)
    │         │     every name the code knows              ghost: GhostPointsTo(n -> names),
    │         │     (main path included)                          entry n of `aliases`
    │         │
    │         ├── lookup_count: AtomicU64                  atomic, left out for now
    │         │
    │         └── inode_nr, file_type, guest_inode_nr      plain, never change
    │             volume: Arc<VirtioFsVolume> ────────┐    (recorded in its `inodes` entry)
    │                                                 │
    ├── files: RwLock<HandleMap<Arc<VirtioFsFile>>>   │  LOCK 4: the whole file table
    │   │                                             │  ghost: FracGhost half of `files`
    │   └── handle ──► VirtioFsFile                   │
    │         ├── file: RwLock<LxFile>                │  the open host file
    │         ├── inode: Arc<VirtioFsInode>           │  the node it was opened on
    │         ├── open_flags                          │  plain: read-only / for writing
    │         └── directory_snapshot: RwLock<...>     │  a directory's listing
    │                                                 │
    ├── mode: Direct | Aggregate(...)                 │
    ├── microvm_profile, caller_identity              │  plain
    └── negotiation: RwLock<FuseNegotiation>          │  agreed FUSE version and limits
                                                      ▼
                                   VirtioFsVolume  (one per share; all fields plain, fixed)
                                     ├── volume: Arc<LxVolume>    host access (lxutil)
                                     ├── id, readonly
                                     ├── policy: SubtreePolicy    denied / allowed / writable
                                     └── pinned_identities, strict_paths, exposed_file


VirtioFsGhost  (not in any lock: kept by the trusted keeper, lent to every request handler)
├── inodes:  FracGhost half                      (other half in LOCK 1)
├── paths:   GhostMapAuth<node id, path>         (entry n's GhostPointsTo in node n's LOCK 2)
├── aliases: GhostMapAuth<node id, set of names> (entry n's GhostPointsTo in node n's LOCK 3)
└── files:   FracGhost half                      (other half in LOCK 4)
```

An inode's `volume` (`Arc<VirtioFsVolume>`) is its share: the shared host
directory, with the share's id, `readonly` flag and `policy`. All its
fields are plain, and none changes after the volume is created. Node ids
are unique across all shares (`InodeMap`).

**Why the spec needs ghost state.** The handlers take `&self`, and a
contract cannot mention what a lock holds: Verus knows a lock's content
only through its rule, and only while holding the lock, and a handler
holds no lock at its start or end. So the spec keeps a ghost copy of what
is behind the locks, split into pieces so that it always matches the real
data.

**The ghost state: one map per kind of lock.** For each, one piece is for
the whole map, and the rest sit where the code protects the data:

| Ghost map | Key -> value | Mirrors | Kind | Pieces |
|---|---|---|---|---|
| `inodes` | node id -> the inode's view: its object, kind, and its volume's `readonly` flag and policy | which inodes are in the table (lock 1), and their plain fields | `FracGhost` | two halves: one in lock 1, one in `VirtioFsGhost` |
| `paths` | node id -> main path | each inode's `path` (lock 2) | `GhostMapAuth` + `PointsTo` | the `Auth` in `VirtioFsGhost`; inode n's `PointsTo` in inode n's lock 2 |
| `aliases` | node id -> every name the code knows | each inode's `aliases` (lock 3) | `GhostMapAuth` + `PointsTo` | the `Auth` in `VirtioFsGhost`; inode n's `PointsTo` in inode n's lock 3 |
| `files` | handle -> the open file's view: its node, and whether it was opened for writing | the file table (lock 4) | `FracGhost` | two halves: one in lock 4, one in `VirtioFsGhost` |

**Inside `VirtioFsGhost`, `paths` and `aliases` are keyed by a ghost key;
the contracts never see it** (decided 2026-11-10). The table above is
what the contracts see (`VirtioFsGhostView`, keyed by node id). Inside
`VirtioFsGhost`, the two `GhostMapAuth`s are keyed by a per-inode ghost
key instead, for two reasons:

- an inode's locks are created, with their rules, in
  `VirtioFsInode::with_attr`, before the inode gets its node id (the inode
  table assigns it when the inode is inserted), and a lock's rule never
  changes; so the rule names its entry by a key fixed at creation
  (`PathRule.key`, `inode.spec.rs`), taken from a ghost counter
  `next_key`;
- an inode may be built and then dropped: `InodeMap::insert` drops the new
  inode when the table already has one for the same host file (a second
  name of it), and the entry made for it stays in the ghost map (its
  piece was in the dropped lock).

The inode table's ghost value records each node's key (`InodeTable.keys`),
and the view is computed through it: node n's path is the entry of n's
key. So entries of no node's key (dropped inodes, forgotten ones) and
`next_key` are invisible to the contracts, which therefore read exactly as
if the maps were keyed by node id. What only the proof needs (each table
node has a key with entries, keys are distinct, and every key in use is
below `next_key`) is in `VirtioFsGhost::wf`. Rejected: letting the view
show the keys, which made `lookup`'s contract unprovable when the host file
already had a node (the dropped inode's entry and the moved `next_key`
showed in the view, and the contract said they do not change).

`GhostMapAuth` + `PointsTo` is used only where each entry has its own
lock: the `Auth` is like a collection of half permissions, one per entry,
so its holder can read the whole map, and changing entry n also needs
entry n's `PointsTo` (`vstd/resource/impls/map.rs`: `insert` adds an entry
and returns its `PointsTo`; `agree` shows that a `PointsTo` and the `Auth`
agree; `update` changes an entry with both). Where the whole map is under
one lock, a `FracGhost` in two halves is enough.

`VirtioFsGhost` is the bundle of the four whole-map pieces, defined in
virtiofs with virtiofs's own types; its view is `VirtioFsGhostView`, the
four maps (each piece's value: `FracGhost` and `GhostMapAuth` have a view,
`@`). How the `Fuse` trait, in the `fuse` crate, can take it and state the
rules without naming virtiofs's types: "Where the contracts are", below.

**What an `inodes` entry holds: the inode's view.** `VirtioFsInode`'s
view is what the spec can read of it through `&self`, its plain fields:
its object (`(volume id, inode_nr)`, as the inode table keys it,
`DedupKey::Ino`, `inode.rs:567`), its kind (`file_type`, an `lx::mode_t`,
the code's own type), and its volume's `readonly` flag and policy (the
policy's view, `SubtreePolicyView`). All are set when the inode is made
and never change. Its locked parts (`path`, `aliases`) and its count are
not in it: they are the `paths` and `aliases` maps. "Is node n's path +
`x` writable?" and "are all of node n's names writable?" are then spec
functions of node n's view (`!readonly && policy.can_write(p)`) and of
`paths[n]` and `aliases[n]`. A handle's entry in `files` is likewise
`VirtioFsFile`'s view: its node and whether it was opened for writing
(`open_flags`). The root is the exception for paths and
names: its path is always empty and its only name is that path, so it has
no `PointsTo`, and its locks' rules say just that ("Removed inodes and
node ids", below); the spec's path of a node is the empty path for the root
and `paths[n]` for any other node, and likewise for its names.

**How the ghost state keeps matching.** Each lock's rule says that the
real data in it matches the ghost piece in it (the path in inode n's lock 2
is `paths[n]`, the table in lock 1 is `inodes`, ...). When the code changes
data under a lock, the proof changes the ghost value at the same moment:
it takes the lock's piece out, updates the value with both pieces (the
lock's and the one in `VirtioFsGhost`), and puts the piece back before the
lock is released, when Verus checks the rule. E.g. `lookup` adds inode 7 to
the table, and the proof adds 7 to `inodes` (with its view) and to
`paths` and `aliases` (whose new `PointsTo`s go into inode 7's locks);
`rename` changes the path of every inode at or below the old name
(`rename_alias_prefix`), each under its own lock 2 and 3, and the proof
updates each one's `paths` and `aliases` entries.

The ids pin the pieces (LESSONS F): each lock's rule, given when the lock
is created (`RwLock::new(val, Ghost(pred))`), names its piece's id. The
current ids of the `paths` and `aliases` maps are recorded in the `inodes`
value (in lock 1), and lock 1's rule says that every inode in the table,
but the root, pins those ids; so `DESTROY`, which starts fresh maps,
updates the recorded ids under lock 1's write lock ("Removed inodes and
node ids", below). (`VirtioFsInner::wf()` cannot say any of this: the
table is behind its lock.)

**Every request runs alone, in the microVM** (checked 2026-11-09). This is
what lets a request trust `VirtioFsGhost` for its whole length, and why the
two reads of a path in `create` agree: the FUSE handler `create`
(`lib.rs:289`) computes node 7's path + `x` with `child_path` and checks it
(`check_writable_entry`), and then `VirtioFsInode::create`
(`inode.rs:327`) computes it again and hands that copy to the host. The
microVM has two queues, and both run the same worker code: `start_queue`
creates the same `VirtioFsWorker` for each (`virtio.rs:300-353`), and each
dispatches whatever request it finds. Nothing makes the high-priority
queue take only `FORGET`: the guest chooses the queue, so a malicious
guest can send a `rename` on it. But the two workers are tasks on one
thread, and a request runs to its end before the thread switches tasks:

- The VM worker creates one thread for devices (`new_device_thread`,
  `openvmm_core/src/worker/dispatch.rs:195`) and gives its executor to
  `ThreadDriverBackend` (`dispatch.rs:382`). virtio-fs asks for
  `driver_source.simple()` (`virtio.rs:241`), with no target CPU, so it
  gets that thread, not a thread of its own (`vmcore/src/vm_task.rs:434-460`);
  both queue tasks run on it (`virtio.rs:329`).
- `process_virtiofs_request` is a plain function, not an `async fn`
  (`virtio.rs:443, 459`), so there is no `await` in a request.

(Upstream, with several request workers, a rename can run between the two
reads; it is harmless there only because upstream has no per-path rules,
just a read-only flag for the whole volume.)

**Why the request holds the whole-map pieces, and the locks keep the
others** (decided 2026-11-09).

- *In the microVM, the locks protect nothing*: no two requests overlap, so
  no lock is ever contended (they are there for upstream's several
  workers). But the data is inside them: the inodes are shared (`Arc`,
  `&self`), so the code reaches a path only through its lock. So the proof
  needs a link between what the code reads from a lock and the ghost
  state; that is the only job of the locks' rules and their ghost pieces
  here. The link is **checked**: each lock holds its piece, and Verus
  checks at every release that the data matches it. (The other way, a
  trusted promise that whatever the code reads from a lock is what the
  ghost state says, needs no pieces, but Verus would no longer check that
  the ghost state follows the code's writes; rejected, since the checked
  way covers more.)
- *The request holds the `Auth`s (and the table halves) for its whole
  length, not only while it holds a lock.* Handing them out only with a
  lock (e.g. keeping the `paths` `Auth` in the inode table's lock, which
  the code holds for most path changes: `lookup`, `rename`, `unlink`; not
  `link`, `lib.rs:429`) would be the ideal, but does not work, for two
  reasons. A contract must name node 7's path at the start and end of a
  call, when the handler holds no lock. And the code itself relies on
  running sequentially: in `create`, the FUSE handler reads node 7's path
  (copy 1, `lib.rs:301`) and checks it, releases the lock, and then
  `VirtioFsInode::create` reads it again (copy 2, `inode.rs:336`) and hands
  it to the host; between the two reads no lock is held, and the two are
  equal only because nothing else runs in between. With the `Auth` only
  under a lock, the proof could not show copy 1 == copy 2, and rightly so.
  So the keeper's assumption ("How a request gets the ghost state",
  below) is what this design rests on.

**The contracts it allows.** Every FUSE handler takes the ghost state,
`Tracked(g): Tracked<&mut VirtioFsGhost>`, passed down from
`process_virtiofs_request` through the FUSE dispatch (below). The ones
that change names (`create`, `mkdir`, `mknod`, `symlink`, `unlink`,
`rmdir`, `rename`, `link`) follow one rule; for `create` of `name` in node
`n`, with p = node n's path + `name`: if node n may not write p
(`!readonly && policy.can_write(p)` of node n's view), it returns an
error, and the ghost state is unchanged; if it succeeds, it may. Each also says how the ghost state changes:
`create`, `mkdir`, `mknod` and `symlink` add a node; `link` adds a name to
`aliases`; `unlink`, `rmdir` and `rename` remove or change entries of
`paths` and `aliases`.

**Where the contracts are: on the `Fuse` trait, with a spec-function
trait** (decided 2026-11-10, tested on a small example). The `fuse` crate
defines no data for the ghost state, only the questions its rules ask, as
a trait of spec functions without bodies, and the `Fuse` trait takes the
ghost state's type as a parameter:

```rust
// fuse crate
pub trait FuseState: Sized {
    spec fn is_dir(&self, n: u64) -> bool;
    spec fn child_path(&self, parent: u64, name: Seq<u8>) -> PathView;
    spec fn can_write(&self, n: u64, p: PathView) -> bool;
    spec fn after_mkdir(&self, n: u64, name: Seq<u8>, m: u64) -> Self;
    ...
}
pub trait Fuse<G: View>: Send + Sync where G::V: FuseState {
    fn mkdir(...) -> (r: lx::Result<fuse_entry_out>)
        with Tracked(g): Tracked<&mut G>
        ensures /* the rules, in terms of FuseState's functions */;
    ...
}
```

virtiofs gives the bodies (`impl FuseState for VirtioFsGhostView`), from its
own types: an inode's view, `SubtreePolicyView::can_write`, and so on. So
nothing is copied: the rules are stated once, on the trait, and their
meaning comes from virtiofs, so a reader checks the trait's contracts (when
an operation may succeed, and that it changes the state as an `after_`
function says) and virtiofs's bodies (what each function means). The trait
level is needed: `Session::dispatch` calls the handlers through
`Box<dyn Fuse<G>>`, and through a trait object Verus knows only the
trait's contract. The small example (4 verified, 1 rejected, as wanted):
`Session` learns "Ok ⇒ `can_write`" through `dyn Fuse<G>` for any `G`; the
worker, with virtiofs's `G`, learns its meaning (the policy's writable
set); a correct `mkdir` meets the contract; an implementation that returns
Ok without the rule is rejected. `FuseState` needs `Sized`, so that
functions such as `after_mkdir` can return the state.

*Why a type parameter, and what it costs* (decided 2026-11-10). The
handlers take `&self` and the state is behind locks, so no view of a Rust
data structure can say what a call changes: the spec needs a ghost
variable, passed into each handler. The question is only where its type
is defined:

- **(a), chosen: in virtiofs, passed to the trait as `G`** (`Fuse<G>`). The
  `fuse` crate never knows what `G` is made of (that `VirtioFsGhost` holds
  `GhostMapAuth`s and `FracGhost` halves is virtiofs's business, for its
  proof): its contracts use only what the bound promises, that `G` has a
  view (`old(g)@`, `final(g)@`) and that the view answers `FuseState`'s
  questions; virtiofs fixes `G = VirtioFsGhost` and gives the answers. No
  copies of virtiofs's types.
- **(b): in the `fuse` crate, as a fixed type** (`FuseGhost`), named only in
  the ghost argument (`with Tracked(g): Tracked<&mut FuseGhost>`), which a
  normal build removes. But the `fuse` crate cannot name virtiofs's types,
  so its contents would be copies built from types it can name (a node's
  write rule as a set of paths instead of virtiofs's policy, ...), kept in
  step with virtiofs by hand. Rejected for that.
  (An associated type, `type Ghost`, is refused through `dyn Fuse`; a type
  parameter per method makes the trait unusable as `dyn Fuse`; Rust cannot
  add a type parameter only in Verus builds.)

The cost of (a), in normal builds (`cargo build`, without Verus): `G`
appears in the code itself, not only in Verus attributes. So:
1. signatures in Pedro's and upstream code change: `Fuse` becomes
   `Fuse<G>`, `Session` becomes `Session<G>`, and every place that names
   them (`virtio.rs`, `owner.rs`, `SectionFs`, the tests' `Session::new`);
   a `CODE-CHANGES.md` entry;
2. the compiler must see the ghost state's type, and every type its fields
   name, in normal builds too (a normal build keeps a ghost struct's
   fields; tested): so the spec files that define them are included
   without `cfg(verus_keep_ghost)`: `fuse/src/lib.spec.rs`,
   `virtiofs/src/lib.spec.rs`, `microvm/policy.spec.rs` (for
   `SubtreePolicyView`, a field of an inode's view), and `lx`'s
   `string.spec.rs` (for consistency). Only their type definitions are
   left in a normal build: Verus's macro removes their spec functions, and
   their `.proof.rs` files stay under `cfg(verus_keep_ghost)`.

Nothing changes at runtime: no value of these types is ever created,
stored or passed (the ghost argument itself is removed in normal builds);
the compiler only checks that the types exist, as for any type named in a
signature. Tested on two small crates built both ways
(`cargo build`, and Verus: a wrong `mkdir` in the other crate is rejected
through the trait's contract).

Separately, Verus processes a trait as a whole (a trait's method cannot be
left out), so every type that the `Fuse` trait's 45 methods name is
declared to Verus: about 30, most opaque (`verus_verify(external_body)`),
a few with their fields visible for the contracts (`fuse_entry_out`,
`fuse_attr`, `fuse_open_out`, `CreateOut`). This cost comes with any
contract on the trait, (a) or (b).

*Status* (2026-11-10): every operation of the trait (44) has its contract
(`fuse/src/lib.rs`), with `FuseState` in `fuse/src/lib.spec.rs` and
virtio-fs's answers in `virtiofs/src/lib.spec.rs` (an inode's view in
`inode.spec.rs`); Verus checks the spec of `lx`, `fuse` and `virtiofs`
with 0 errors, every body still `external_body`, and the normal builds
pass. What the contracts do not say yet: "Open items of the FUSE-level
spec", below. `SectionFs` (Windows only, not compiled for Verus on Linux)
implements `setup_mapping` and `remove_mapping`, which then need the
ghost argument too.

`VirtioFs` implements the trait, and its proof
is where the code's policy checks are verified. Every implementor must
meet the contract: the other one is `SectionFs`
(`virtiofs/src/section.rs`), and the trait's default bodies return
`ENOSYS`; so the contract allows "not supported" as an error, which it
can, since the rules only limit when an operation may succeed.

**No model of the host, for now** (decided 2026-11-10). Integrity's rules
are all about paths (the path being changed, the node's names), which the
request and the ghost state give, and the policy decides; the one host fact
it relies on ("an `lxutil` method changes only the path it is given") is
trusted, not modeled. Confidentiality's path rules (a lookup succeeds only
for a visible path; a traverse-only directory lists only the entries that
lead to allowed paths) are the same. The exception is the identity-based
hiding (the volume's pinned identities): it is about which object is at a
path, which only the host knows. So until the spec has a model of the host,
`lookup`'s contract says "some object" at the path, and does not enforce
the pinned rule. The model, when it comes (for the pinned identities, or
for functional correctness): a map path -> (object, kind) per volume, as
ghost state lent with the rest (it changes with `mkdir`, `rename`,
`unlink`), tied to the host by `lxutil`'s trusted contracts (`lstat(p)`
returns what the map has at `p`; `mkdir(p)` adds a directory at `p`; ...),
which take it as a ghost argument.

**How a request gets the ghost state: the trusted keeper** (decided
2026-11-10; `virtiofs/src/fs_ghost_t.rs`). Every operation takes the file
system's ghost state `g`, and some change it, so between two requests `g`
has to be kept somewhere, for the next request to get what the previous one
left. A field of the worker that handles requests would be the natural
place, but there are two workers, one per queue, each its own task, and
both handle requests; a tracked value, like any value, is in only one
place. Sharing it between them in a way Verus checks would need a lock, an
extra lock and unlock per request in the executable. So the keeping is
trusted instead: three trusted proof functions (none exists at runtime, so
the executable is unchanged):

| Function | When | Contract |
|---|---|---|
| `keep_fs_ghost(fs, g)` | once, at creation (the device's setup, with the `g` that `VirtioFs::new_microvm` hands out) | requires `fs.owns(g)` and `g.wf()`: the keeper now holds `g` |
| `take_fs_ghost(session) -> g` | at the start of each request (`process_virtiofs_request`) | ensures `session.owns_ghost(g)` and `g.wf()` |
| `give_back_fs_ghost(session, g)` | at the end of that request | requires `session.owns_ghost(g)` and `g.wf()`, which every operation keeps |

```
request worker:  take_fs_ghost  -> g
                 create(.., g)     (may change g: a new node)
                 give_back(g)   -> the worker no longer has g
hiprio worker:   take_fs_ghost  -> g   (as create left it)
                 forget(.., g)
                 give_back(g)
```

`g` itself is created, not by the keeper, but by `VirtioFs::new_microvm`,
together with the inode table (the root alone) and the empty file table,
and handed out (a ghost argument, `Tracked<&mut Option<VirtioFsGhost>>`).
Its contract demands it, so that the proof must create it: on success, a
`g` that the new file system owns, well formed, and new (`is_new()`: the
root alone, no names, no handles). Its proof creates the `paths` and
`aliases` `Auth`s and the two tables' `FracGhost`s with a lemma, puts one
half of each table's into that table's lock, and keeps the rest as `g`;
`owns` holds only if the locks hold the matching halves, so the creation
cannot be skipped. The device's setup
(`VirtioFsDevice::new_microvm_hostfs`) then hands `g` to the keeper. Only
the keeping is trusted. What is assumed, written at
the top of the file:

1. A take is always followed by its give-back before the next take, so at
   any moment at most one request has `g`: requests never overlap ("Every
   request runs alone", above). This breaks if virtio-fs gets a driver of
   its own (a target CPU) or several threads, or if request handling
   becomes `async`.
2. Only `process_virtiofs_request` takes and gives back, and the keeper
   holds one ghost state per file system, the one its creation kept; so a
   take gives out what the last give-back (or the creation) left, and it is
   the session's file system's.

`process_virtiofs_request` and `new_microvm` carry comments saying what
their proofs do (their bodies are not verified yet, and ghost code can only
be in a verified body). `Session::dispatch` and `owner::dispatch` take `g`
and require and keep `owns_ghost(g)` (`Session::owns_ghost(g)`: its file
system owns `g`) and `g.wf()`, with `external_body` bodies.
`owner::dispatch_with` is not given to Verus yet: its `run_as` takes the
operation as `&mut dyn FnMut()`, which Verus does not support; its proof
needs `run_as` to take a generic closure that takes `g` as an argument
(below), a code change that comes with that proof.

**Passing the ghost state down: what Verus supports** (tested 2026-11-09, on
small examples). From `process_virtiofs_request` to `VirtioFs::create`:
`owner::dispatch` (`microvm/owner.rs:75`), the `run_as` closure that runs
the request as its caller (`owner.rs:101`), `fuse::Session::dispatch`
(`fuse/src/session.rs:76`), `self.fs.create(...)` on a `Box<dyn Fuse>`
(`session.rs:309`), and the handler.
- A ghost argument through `Box<dyn Fuse>` works, and Verus checks the
  contract through it.
- `Box<dyn Fuse + Send + Sync>` (`session.rs:48`) is refused ("dyn with
  more than one trait"); it works with `Send + Sync` as supertraits of
  `Fuse` (`trait Fuse: Send + Sync`) and the field `Box<dyn Fuse>`.
- `Fuse` cannot name virtiofs's ghost type as an associated type (`dyn
  Fuse<G = ...>` is refused), but can as a generic parameter
  (`trait Fuse<G>`), which is the design chosen ("Where the contracts are",
  above).
- `run_as`'s `&mut dyn FnMut()` (`owner.rs:128`) is refused, and so is a
  closure that captures a `&mut` to the ghost state; a generic closure
  that takes it as an argument (`FnOnce(Tracked<&mut VirtioFsGhost>)`) works.

So the chain is feasible, with code changes in the `fuse` crate (the trait,
its type parameter, and `Session`) and in `run_as`; each is an entry in
`CODE-CHANGES.md`.

**Where the policy is, and where it is not (decided 2026-11-09).** The
integrity rule is stated at the FUSE handlers, as above, and they are
specified first. `lxutil` is the trusted boundary (LESSONS D1; on Linux a
thin wrapper, one `*at` system call per method): its methods get trusted
`external_body` contracts, written after the FUSE handlers' and only what
their proofs need, such as "a method that changes the host changes only
the path it is given".

`lxutil`'s methods take the path as `impl AsRef<Path>` (anything that can be
turned into a path: `&Path`, `PathBuf`, a string), and vstd has no spec of
`as_ref`. Decided (2026-11-09): add a trusted fact to our vstd patch (for
`&Path`, `PathBuf` and `&PathBuf`, `as_ref()` returns the same path),
rather than change `lxutil` to take `&Path`, which would change every
caller that passes a string (plan9 and many tests).

**Removed inodes and node ids (decided 2026-11-09).**
- `FORGET` removes an inode from the table, but leaves its entries in the
  `paths` and `aliases` maps: the inode's `PointsTo`s stay in its locks,
  and an open file that still holds the inode (through its `Arc`) uses
  them; its handle's entry in `files` still names the node. (`rename`
  then no longer updates that inode, since it updates only the inodes in
  the table, and neither the ghost entry nor the lock changes, so the two
  still agree.) A node id that is gone is harmless: `HandleMap::insert` (`lib.rs:966`) never hands
  out an id again (it only counts up, and refuses every insert once it
  reaches the end, `can_insert`, `lib.rs:1018`).
- The exception is the guest's FUSE `DESTROY` (`lib.rs:638`, on unmount):
  it clears the inode table except the root, and starts handing out ids
  from the beginning again (`lib.rs:1042`). It runs, like every request,
  with the ghost state lent to it, and replaces the `paths` and `aliases`
  `Auth`s with fresh ones (new ids, recorded in the `inodes` map's value),
  so the stale entries go with the old ones. It also closes every open file
  (`files.write().clear()`), so no handle keeps a removed inode. The root
  inode stays in the table with its old locks, whose rules were fixed when
  it was made; so **the root gets no `PointsTo`**: its path is always the
  empty path (the share's root; `rename` and `unlink` act only on
  children), and its only name is that path, so its locks' rules are just
  "the path is empty" and "the names are {the empty path}". The spec's
  path of a node is then the empty path for the root and `paths[n]` for any
  other node, and likewise for its names.

**Changes to a file itself, and its other names (decided 2026-11-09).**
Some requests change a name in a directory (`create`, `mkdir`, `mknod`,
`symlink`, `unlink`, `rmdir`, `rename`): they need only that name to be
writable. Others change the file itself, its contents or attributes
(`set_attr`, `open` with truncate or write access, `set_xattr`,
`remove_xattr`, and a handle's `write` and `set_attr`): a file can have
several names (hard links), and a change through one name shows through
all of them.

- **The rule: every name the code knows for the file must be writable.**
  This is the rule the code follows: an inode is read-only unless the
  policy lets the guest write through its main path and every one of its
  aliases (`policy_permits_writes`, `microvm/inode.rs:166-179`, used by
  `readonly()`, `inode.rs:186`); and `link` refuses to add a name to a
  read-only file (`lib.rs:421-426`), so that no new name makes a read-only
  file writable.
- **In the spec**: "every name of node n is writable" is a spec function of
  node n's view and `aliases[n]` ("The guest's state in the spec"
  above).
- **The code knows only the names the guest reached**, by `lookup` or by
  its own `link`; it never asks the host for a file's other names
  (`policy_permits_writes`'s comment: "every path that it knows for the
  object"). So a hard link that the host made is not covered. Example: a
  hard link on the host makes `/a/x` and `/b/y` two names of one file (not
  two copies: a write through either name changes what both show); `/a` is
  writable, `/b` is visible but read-only; the guest looks up only `/a/x`,
  so the code knows only that name, finds it writable, and allows a write,
  which changes `/b/y` too. The guest cannot set this up itself (its `link`
  is refused), only the host can; so it falls under the theme of setting
  aside what the host does, and the spec states it as an assumption (see
  "The host assumption" below). Pedro's code handles the like case for hidden
  paths (`pinned_paths`, `microvm/policy.rs:265-270`, pins the objects of
  denied paths by identity, so that no hard link shows them), but not for
  read-only ones: to ask him whether that is intended.

**Open items of the FUSE-level spec** (2026-11-10), to come back to:
1. **When an operation fails** (started 2026-11-10; Jay's style). Each
   operation gets a set of rules, each a condition and its code, at the
   FUSE level (`FuseState`, `fuse/src/lib.spec.rs`: e.g. `entry_errors`,
   shared by the operations that remove or make an entry),
   and its contract says `fails_with(r, rules)`: if any rule applies, the
   operation fails, with the code of one of the rules that apply, or with a
   host error. The order in which the code checks does not matter. The
   host is one more rule, which may always apply (`host_error`: virtio-fs
   answers "any code", having no host model), so the codes are pinned only
   in the spec's terms; what the contract does guarantee is that the
   operation fails. New FUSE-level question: `is_synthetic(n)` (virtio-fs:
   an aggregate's root, which returns `EROFS`; its view now records whether
   the file system is an aggregate). Codes are compared as numbers
   (`e.code() == lx::EROFS`, `lx/src/error.spec.rs`): Verus refuses
   `lx::Error::EROFS` (a public constant built from a private field).
   Done: `unlink`, `rmdir`, `rename`, `create`, `mkdir`, `mknod`,
   `symlink`, `link`, `open`, `open_dir`, `lookup`, `get_attr`, `set_attr`,
   `read_link`, `statfs`, `read`, `write`, `fsync`; `release` and
   `release_dir` always succeed (`r is Ok`). No rules for the operations
   virtio-fs leaves to the trait's default (`ENOSYS`): `flush`,
   `fallocate`, `lseek`, `copy_file_range`, and the like. More questions:
   `buffer_too_large` (virtio-fs: over 1 MiB, `E2BIG`); running out of
   memory for the buffer (`ENOMEM`) is left under "host". Shared rule sets: `entry_errors`, `handle_errors` (a
   request through a handle), `node_errors`. `is_hidden(n, p)` (virtio-fs:
   the share's policy hides `p`; `child_path` refuses it with `EACCES`
   before the host is asked) is in every rule set that makes a child's
   path. More FUSE-level questions: `bad_open_flags`
   (`util::open_flags`: `O_CREAT`, `O_EXCL`, or a flag `create` refuses), `bad_create_flags` (flags
   that `util::create_flags` refuses, from lx's constants, which differ by
   CPU architecture) and `bad_symlink_target` (microVM only: an empty
   target, `ENOENT`; over 4095 bytes, `ENAMETOOLONG`); for it the view
   records whether the file system is a microVM's, which `owns` ties to its
   profile. Left under "host": `create` on an existing file (bug 3's check:
   whether a file exists is the host's to say). Still to do: the other
   operations.
   Left out of the rules, for now (failures the code checks that no rule
   names; harmless while `host_error` is "any code", but needed as rules if
   a host model narrows it): the microVM's extra name rules (`\` or `:` in
   a name, `EINVAL`; a path too long, `E2BIG`), the limits (`ENOSPC`, `E2BIG`), and a single-file share
   (`ENOTDIR`, `EACCES`). Not stated: when an operation must succeed
   (needs a host model).
2. **The lookup counts** (`lookup_count`, atomics): `forget` says only that
   it may remove the node, and the limits property needs them. Decided
   (2026-11-10): added with the limits property, not before, since
   integrity and confidentiality do not need them; then each node's count
   is a ghost piece tied to the atomic (`atomic_with_ghost!`), every
   operation that hands out a node says "count + 1", and `forget` says that
   the node is removed exactly when its count reaches 0.
3. **The host model**, for the pinned identities ("No model of the host,
   for now", above). Also the root: a lookup whose object is the root's own
   (found by the inode table's dedup) gives the root's node, and changes the
   root's main path in the code (`VirtioFsInode::lookup`), while the spec
   keeps the root's path empty; only the host can make a name lead to the
   root's object.
4. **What a directory listing shows** (`read_dir`, `read_dir_plus`): the
   reply is an encoded buffer, not yet decoded by the spec; the
   traverse-only listing rule (only the entries that lead to allowed paths)
   is part of confidentiality.
5. **The aggregate's synthetic root**: `lookup` in it, `open_dir`'s
   sentinel handle, `get_attr` on it. Known wrong until then: the success
   clauses of `open_dir`, `lookup`, `get_attr` and `statfs` say the guest
   holds the node (`lookup`: a directory), but on an aggregate's root (not
   in the inode table) each succeeds (`open_dir` with a sentinel handle,
   `SYNTHETIC_ROOT_FH`, not in the file table either). Their error rules
   already allow for it: they leave a synthetic node out (`is_synthetic`),
   or, for `lookup` and `get_attr`, say nothing of it yet.
6. **Visibility at the time of an operation on a node.** Every operation on
   a node the guest holds first re-checks, at that moment, that none of the
   node's names is hidden and that none passes through a symbolic link
   (`get_inode` -> `validate_confined_object`, `microvm/inode.rs:181`): a
   node can come to have a hidden name after it was handed out, as when a
   `rename` moves its directory onto a denied path inside a writable one.
   The contracts of the operations on a node (the reads, `set_xattr`,
   `open`, ...) say only that the guest holds the node, which is weaker
   than the code: a confidentiality rule to add ("succeeds only if none of
   the node's names is hidden"). Not an invariant of the state, since the
   code does not keep it between operations; it checks it at each one.
7. **`owns`'s body in virtio-fs** (done 2026-11-10). Every operation
   requires and keeps `self.owns(g)` (the ghost state is this file
   system's) and `g.wf()`, both declared at the FUSE level (`FuseOwns`,
   `FuseGhost`). `owns` now says that the ids of `g`'s two table halves are
   the ones the file system's two table locks' rules pin
   (`inner.inodes.pred().id`, `inner.files.pred().id`), and that `g`
   records whether the file system has the microVM profile; the inode table
   records the ids of the `paths` and `aliases` maps (`wf`), and each
   inode's locks hold its entries of them. Left to the proof (TODO): the
   locks' creation (`InodeTableLocked::new_lock`, `FileTableLocked::new_lock`,
   `VirtioFsInode::with_attr`) still uses `assume_new()` placeholders, which
   the proof of `new_microvm` and of the inode builders replaces.
8. **Pedro's questions** (`BUGS.md`): device nodes from `mknod`; host hard
   links between a writable and a read-only path.

## The theme of the spec: protecting the host from the guest

The microVM is a sandbox: NVX uses it to run workloads that the host does
not trust, such as code it was asked to run. So the spec treats the guest
as an attacker that wants to read or change host files outside what it was
given, to make the host keep unbounded state, or to crash OpenVMM.

This is what separates Pedro's microVM virtio-fs from upstream OpenVMM's
`--virtio-fs`. Upstream shares a whole host directory with a guest that is
usually under the user's control (a development or test VM). The microVM
gives the guest exactly what the workload needs: it may read its inputs,
write only where it is allowed, and never see what is hidden, such as
credentials stored next to the data. Nearly every change Pedro made to
virtio-fs protects the host from the guest: the size bounds, the bounded
tables, the access policy, path confinement, running requests as their
caller, and DMA restricted to guest RAM.

So the spec's first properties are security properties (confinement,
policy, identity, limits, below), and one rule applies throughout: every
byte the guest can write (the virtio queues, the shared interrupt-status
word, the FUSE requests, the node ids and handles in them) is untrusted
input, and each property must hold whatever the guest sends.

## Where a spec would attach

- the `Fuse` trait's operations (its contracts, on the trait itself:
  "Where the contracts are", above);
- building the device (`new_microvm`, `new_microvm_aggregate`);
- the device's save and restore.

## Properties a spec could promise

Properties 1-4 are the security properties and 6 is snapshots, which the
spec covers. 5 is deferred. Pedro summarizes his additions to virtio-fs in
three parts (2026-11-09): snapshot and restore (property 6), the access
policy (property 2), and the Windows wrapper in `lxutil` ("Layer 4 in the
spec", below).

1. **Confinement**: every host object the device touches lies inside the
   share.
2. **Policy**: the guest sees only visible paths, and changes only writable
   ones. Precisely, two properties (agreed 2026-11-08):
   - **Confidentiality**: everything the guest gets back (a node id, a
     file's attributes, a directory entry, a file's bytes) is about a host
     object the guest may see.
   - **Integrity**: a request that would change the host (a file's bytes
     or attributes, a name created, removed or renamed) succeeds only
     through a path the guest may write; otherwise it returns an error.
     Stated at the FUSE handlers ("The contracts it allows", above): the
     spec has no model of the host, so it does not state the host's
     changes themselves; that `lxutil` changes only the path it is given
     is trusted.

   The guest **may see** a host object if some path in the share leads to
   it and that path is visible under the policy (a traverse-only
   directory: only that it exists, and only its entries that lead to
   allowed paths); how the spec knows which object a path leads to is to
   decide when we work on confidentiality. The guest **may write** through
   a path if the policy makes it writable; a change to a file itself needs
   every name the code knows for the file to be writable ("Changes to a
   file itself, and its other names" above, with the host assumption's
   part about hard links, below).

   The code's enforcement
   rules (`CODE-OVERVIEW-DETAILS.md`, layer 3, change 4: every name change
   checked on all its paths; a file's change needs all its names writable;
   no hard link that makes a read-only file writable; hidden directories
   pinned by identity; filtered listings) are not specified one by one: they
   are how each operation meets these two properties, which is proof work.
3. **Identity**: each operation runs as its caller, guest root is mapped,
   and nothing runs as OpenVMM.
4. **Limits**: the caps are never exceeded.
5. **The operations' meaning**: `read` returns the file's bytes, `write`
   replaces them, and so on. This rests on trusted facts about the host's
   filesystem. Deferred, read and write included (decided 2026-11-08): it
   has nothing to do with Pedro's changes. Integrity needs only one trusted fact
   about what a call does: an `lxutil` method that changes the host
   changes only the path it is given ("Where the policy is" above).
6. **Snapshots**: save records no host paths, and restore accepts only the
   same host objects.

## The plan (revised 2026-11-08)

The generic virtio layer first, since virtio-net, virtio-fs and
virtio-blk, the three devices Pedro named as next, all sit on it, and a
spec of it is written once, at the trait level, for all of them. Then
virtio-fs's own layers.

**Layer 0: the generic virtio layer.** It is code to verify, not to
trust: it parses descriptor chains that the guest writes (chains that loop,
indices that wrap, lengths and addresses that overflow, buffers outside
RAM). The snapshot spec already has views of the MMIO transport and of a
queue's configuration and progress (`transport/mmio.spec.rs`:
`VirtioMmioView`, `VirtioQueueView`), for save and restore only; they are
extended, not replaced.

- **The queue** (`VirtioQueue`), viewed as one sequence of requests, in the
  order the guest made them available, each with its own state
  (`Available`, `Taken`, or `Completed(bytes_written)`), and the index up
  to which the device has taken them. Each request carries its own state
  because a device may complete requests out of order (the virtio standard
  allows it; virtio-fs's spec adds "in order" on top, since each queue's worker
  finishes each request before taking the next). The model is of requests,
  not of descriptor slots, which the guest reuses. Contracts:
  - `next()` takes the next available request, which is well formed (no
    loop, bounded length, every buffer inside the RAM the device may
    access), marks it `Taken`, and moves the index; or reports an error;
  - `complete()` marks one taken request `Completed` with the bytes written;
  - progress only moves forward.
- **The `VirtioDevice` trait**, at trait level: `start_queue` resumes from
  the saved progress and `stop_queue` reports it; `save_device` and
  `restore_device` round-trip the device's private state; `reset` clears it.
  And quiescing (agreed 2026-11-08): after `quiesce_input` returns `Ok`, the
  device takes no new request from any of its queues until `resume_input`:
  while quiesced, each queue's index of taken requests does not move, and
  requests the guest makes available meanwhile stay `Available`. For this,
  the device's view records whether it is quiesced. Each device meets the
  contract its own way, which is proof work, not spec: virtio-fs with its
  admission gate (`CODE-OVERVIEW-DETAILS.md`, layer 1, change 3), and
  virtio-net and virtio-blk with their own.
- **The MMIO transport**: feature negotiation, with the mask of hidden
  features; device DMA only within the memory map's RAM ranges; the shared
  interrupt-status word (`CODE-OVERVIEW-DETAILS.md`, "What this means for
  the spec").

**Concurrency: what can run at the same time as a request** (discussed
2026-11-08). Four sources, for a request such as `rename` on the request
queue's worker:

1. **The high-priority queue's worker.** A normal guest sends it only
   `FORGET`, `BATCH_FORGET` and `INTERRUPT`, but it dispatches any request,
   and a malicious guest can send it any. It runs on the same thread as the
   request queue's worker, and a request runs to its end, so the two never
   overlap (see "Every request runs alone" above).
2. **Other request workers.** Not in the microVM, which has one request
   queue.
   Upstream OpenVMM can have several (by default two, at most eight), which
   is why `VirtioFsInner` has so many `RwLock`s. The spec covers only the
   microVM (decided 2026-11-08).
3. **Other host processes.** Assumed not to change the shared folder
   (see "The host assumption" below).
4. **Save and restore.** They run only with the admission gate closed, no
   request in flight, and the queues stopped, so never during a request.
   The security properties do not need to consider them; the snapshot
   properties do, in the style of the snapshot spec.

The spec stays simple; the proof carries the reasoning about the locks
and ghost pieces (e.g. that `rename`'s update of the paths and aliases of
every inode below the old name keeps each inode's lock rule).

**The host assumption: the host does not change the shared folder**
(agreed 2026-11-08 and 2026-11-09). One assumption, with two parts: no other host process changes the shared folder while the
device runs; and when the device starts, the folder has no hard link that
joins a writable path and a visible read-only path (see "Changes to a file
itself, and its other names" above: the guest cannot make such a link,
only the host can). With this assumption, the share changes only through
OpenVMM's own calls, so what the guest was granted when it reached an
object stays true until the guest itself changes it. It is the intended
model (Pedro, 2026-11-09, on a call with Shan): if the host changes a file
while the guest is working on it, the behavior is undefined. Two
conditions:

- **It is stated as an assumption**, in this document and later in the
  trusted list, worded as above, so that everyone can see which results depend on
  it. NVX's benchmark `virtfs_live_roundtrip` (`nvx/doc/benchmarks.md`)
  has the guest observe a host rewrite; by Pedro's statement, what the
  guest sees then is not something the device promises.
- **The parts that do not need it stay independent of it.** Confinement
  rests on `openat2` refusing anything outside the share's root, which the
  host's kernel guarantees even while other processes change the folder,
  so its proof does not use the assumption.

**virtio-fs's layers.** Three steps, from the bottom:

1. **The policy as an abstract model** (written 2026-11-08: `virtiofs/src/microvm/policy.spec.rs`, and the contracts of `SubtreePolicy::new`, `visibility` and `is_writable` in `policy.rs`). `SubtreePolicy` gets a view,
   `SubtreePolicyView` (the denied, allowed and writable path sets), with
   closed spec functions `visibility(p)` (visible, traverse-only, hidden)
   and `can_write(p)`, written from the policy's documentation and then
   compared with the code. `SubtreePolicy::new`'s checks become its `wf()`.
2. **The guest's state as ghost state** (revised 2026-11-09): `VirtioFsGhost`,
   with its four maps (`inodes`, `paths`, `aliases`, `files`), lent to
   each request ("The guest's state in the spec", above). Example, after
   the guest looks up `work/a.txt` and opens it for writing: `inodes: 5 -> {
   object: (volume 1, inode 4711), kind: S_IFREG, readonly: false, policy:
   the share's }`, `paths: 5 -> work/a.txt`, `aliases: 5 ->
   {work/a.txt}`, `files: 9 -> { node: 5, writable: true }`. It changes with the guest's requests: `open`
   and `opendir` add a handle, `release` and `releasedir` remove it;
   `lookup` adds a node; `forget` may remove one (when its count reaches
   zero; the lookup counts, atomics, are left out for now, with
   `atomic_with_ghost!` when a property needs them, such as limits);
   `create`, `mkdir`, `mknod` and `symlink` add a node (`create` a handle
   too); `link` adds a name; `unlink`, `rmdir` and `rename` remove or
   change names. Its invariant needs no host facts, since whether a path is
   writable depends only on the path and the node's policy: every handle
   opened for writing belongs to a node whose names are all writable.
3. **The operations' contracts**, on the `Fuse` trait's operations, in terms
   of `VirtioFsGhostView`: an operation that changes something succeeds only on
   a path the guest may write ("The contracts it allows", above); `lookup`
   succeeds only for a name the guest may see and returns that path's node;
   a traverse-only directory lists only the entries that lead to allowed
   paths. The code's checks are not in the contracts: they are how the code
   meets them. For confidentiality (`lookup`, listings), the inode's view
   already has its volume's policy, so `visibility` is there too; what is
   left to decide when we work on it is how the contract knows which host
   object a path leads to (and the volume's pinned identities, from the
   volume's plain fields, in the inode's view).

   **Down to the host** (revised 2026-11-09): layer 3's operations are
   proved against the contracts of the `LxVolume` methods they call, and
   those contracts are trusted (see "Where the policy is, and where it is
   not" above).

Confinement and the caller's identity then build on step 3, with
`lxutil`'s trusted contracts.

**Layer 4 in the spec: the trusted boundary.** `lxutil` is trusted ("Where
the policy is, and where it is not" above): its methods get trusted
`external_body` contracts that state only what our properties use, written
after the FUSE handlers' contracts. Integrity uses one fact: a method that
changes the host changes only the path it is given. Other properties will
add theirs when we work on them (confinement: what `openat2` with
`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` guarantees; identity: that the
credential calls change only the calling thread; buffer bounds: `pread`
returns at most the bytes asked for). File contents are left out (property
5, deferred).

On Linux, `lxutil` is a thin wrapper, one `*at` system call per method. Its
two pieces of real logic are Pedro's additions (`CODE-OVERVIEW-DETAILS.md`,
layer 4): the confinement walk (`confine_paths`), on which property 1
rests, and the per-thread credential switching (`unix/credentials.rs`), on
which property 3 rests. Their proofs are deferred (decided 2026-11-09),
until we work on properties 1 and 3; until then they are trusted like the
rest of `lxutil`.

**The Windows side is Pedro's** (Pedro, 2026-11-09, at the spec design
meeting): `lxutil/src/windows/`, about 7,600 lines that emulate Linux
behavior on NTFS, which the microVM uses on WHP. Pedro summarizes his
additions to virtio-fs in three parts: snapshot and restore, the access
policy, and this Windows wrapper. (Our branch's history starts at
2026-08-01, after the wrapper was written, so its authorship did not show
there.) Unlike the Linux side, it is not a thin wrapper: each method
emulates a Linux call with several Windows calls. So trusting it as a
whole would put 7,600 lines of logic in the trusted base; how to specify
it (its contracts as `LxVolume`'s on Linux, proved against trusted facts
about the Windows calls it makes) is to decide when we work on it.

**virtio-fs's saved state, and the snapshot spec.** The snapshot spec
covers the MMIO transport, which saves the inner device's private state as
one blob and hands exactly that blob back on restore
(`vm/devices/virtio/virtio/src/transport/mmio.spec.rs`, `VirtioMmioView`'s
`device_state`), but not what the blob holds, nor that the inner device
puts its state back: a recorded TODO, "the inner devices' layer"
(`snapshot-spec-docs/DESIGN.md`, the table of layers, "virtio-MMIO → its
device task → the inner device (net, console, fs, block)"). The virtio-fs
spec fills it in for virtio-fs: the blob gets a view holding the FUSE
session's state (layer 2, `SessionState`) and the filesystem's saved state
(layer 3: the inodes with their aliases and identities, the open handles,
the policy), and `save_device` / `restore_device` get contracts on
`VirtioDevice` (layer 0's plan above): after a restore, the device's state
is the one that was saved. Open, for when we work on snapshots: the
guest-visible state is ghost state (`VirtioFsGhost`, kept by the trusted
keeper), not a view of the device, and a restore rebuilds the inodes and
their locks, so how a restore's contract states it, and how the keeper
takes the restored ghost state, is still to work out.

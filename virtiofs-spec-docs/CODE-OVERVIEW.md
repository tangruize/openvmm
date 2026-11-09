# virtio-fs: what the code does

An overview of OpenVMM's virtio-fs device, as a starting point for its
spec, in Pedro Henrique Penna's code: `microsoft/openvmm`, branch
`feature/microvm/dev`. This branch, `system-proof/virtiofs-spec-20261108`,
starts from that branch's commit `db42fde04` (2026-11-06) unchanged, with
only our Verus setup, scripts and documents added on top, so the code we
specify is exactly Pedro's at that commit. (This document was first
written from `bdea9ab19`, 2026-11-05; virtio-fs, the FUSE layer and
`lxutil` are the same in both.) The spec's style rules and lessons are in
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

## Async, locks and RPC

- **Async: only in the virtio glue.** `virtio.rs` has an async loop per
  queue (`run`), and async start, stop, reset, `quiesce_input` and
  `resume_input`; `microvm/device.rs` saves and restores asynchronously,
  and `microvm/resolver.rs` builds the device asynchronously. The
  filesystem itself is synchronous: the loop takes a request, calls
  `process_virtiofs_request`, which runs the FUSE operation to the end,
  and completes it before taking the next. The `Fuse` trait's operations
  and `VirtioFs` have no `async`.
- **Little concurrency in the microVM.** It has one request queue
  (`MICROVM_REQUEST_QUEUES = 1`, `microvm/profile.rs`), so requests are
  handled one at a time. The one place where two things meet is a save
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
  thread, so the queue's worker thread switches its own filesystem UID and
  GID with raw system calls, performs the operation, and switches back
  (`lxutil/src/unix/credentials.rs`). The identity property rests on
  trusted facts about those system calls.

## Lock invariants the spec needs (proposed)

The guest's state is behind those locks, and Verus knows what a lock holds
only through its invariant, so properties 1-3 need one per lock:

- **`VirtioFs.inodes: RwLock<InodeMap>`**: every node id the guest holds
  maps to an inode, and node ids are unique.
- **Each inode's `path: RwLock<PathBuf>`**: the lock's rule says that next
  to the path, the lock holds one ghost piece, `PointsTo(node -> path)`,
  for this inode's node id. The request worker holds the other piece, the
  `GhostMapAuth` of the whole `paths` map. Changing a path needs both
  pieces, so the path in the lock always equals `paths[node]` in the
  worker's `Auth`. See "Knowing an inode's path" below (agreed
  2026-11-09). This is where `rename` changes things: the delicate one.
- **Each inode's `aliases`**: the same for each of its other names (hard
  links).
- **`VirtioFs.files: RwLock<HandleMap<...>>`**: every open handle refers to
  a live inode; one opened for writing, to a writable path.
- **Each file's `file: RwLock<LxFile>`**: the host file is that inode's,
  which, with trusted facts about `openat2`, lies inside the share.
- **The session's locks and `negotiation`**: what was agreed with the guest
  (the FUSE version and limits) stays fixed after the start.

What makes this simpler than the snapshot spec: the policy never changes
after the device is built, so each invariant can mention it without
depending on another lock (one invariant per lock, about that lock's own
content); and with one queue, little happens at the same time (the
admission gate between save and requests is the main exception).

The locks: vstd's own verified `RwLock<V, Pred>` (`vstd/rwlock.rs`),
replacing `parking_lot::RwLock` where a lock needs an invariant (decided
2026-11-09), rather than a trusted wrapper over `parking_lot` like the
snapshot spec's `mutex_t.rs` (vstd has no `Mutex`, so that one had to be a
wrapper). Each lock gets its rule as a ghost value when it is created
(`RwLock::new(val, Ghost(pred))`), so the rule can name that lock's own
ghost ids, and nothing about the lock is trusted. The cost, in Pedro's
code: vstd's lock spins instead of sleeping (in the microVM the only
overlap is `FORGET`'s short read of a path), its handles are released by
an explicit `release_write` / `release_read`, not when dropped, and Verus
does not check that they are, and the call sites of `get_path()` (about
20) change to that form. Each is an entry in `CODE-CHANGES.md`.

## Knowing an inode's path: the ghost `paths` map (agreed 2026-11-09)

**The problem.** The guest names a directory by its node id, not by its
path: "create `x` in node 7". The integrity rule for `create` is about the
path, node 7's path + `x`: if it is not writable, `create` fails and changes
nothing; if `create` succeeds, it was writable. Node 7's path is in its
inode's `path` lock, and a spec cannot mention what a lock holds, since
another thread could change it at any time. The code also reads it twice:
the FUSE handler `create` (`lib.rs:289`) computes `/a/x` with `child_path`
and checks it (`check_writable_entry`), and then `VirtioFsInode::create`
(`inode.rs:327`) computes it again with `child_path` and hands that copy
to the host. The proof must know that both reads give the same path.

**Why they do, in the microVM.** Only the request worker changes a path
(`rename`, `unlink` and `link`, `inode.rs:239, 514, 537, 553`). The other
thread, the high-priority worker, runs `FORGET`, which changes the inode
*table* (`inodes.write()`, `lib.rs:267`) but only reads a path (for
`dedup_key`, `inode.rs:570`, on volumes keyed by path). So nothing renames
node 7 while the request worker handles `create`. (Upstream, with several
request workers, a rename can run between the two reads; it is harmless
there only because upstream has no per-path rules, just a read-only flag
for the whole volume.)

**How Verus learns it: vstd's `GhostMapAuth` and `GhostPointsTo`**
(`vstd/resource/impls/map.rs`).

*The idea.* A `GhostMapAuth` is like a collection of half permissions, one
for every entry in the map. Whoever owns it can read the whole map. To
update an entry, it also needs the other half for that entry, a
`GhostPointsTo` (key -> value). Each piece has an id, and every operation
requires the ids to match; that id is what the spec pins.

*The operations used here:*

| Operation | What it does | Used when |
|---|---|---|
| `auth.insert(n, p)` | adds key `n`, which must be new, and returns `PointsTo(n -> p)` | a new inode (`lookup`, `create`, ...): the `PointsTo` goes into its lock |
| `points_to.agree(&auth)` | proves `auth@[n] == p` | the worker opens n's lock: its path is the map's |
| `points_to.update(&mut auth, q)` | changes entry `n` to `q` in both pieces | `rename`, with n's lock taken for writing |
| `auth.delete_points_to(points_to)` | removes key `n`; needs the `PointsTo` back | an inode is removed |

*Applied to the inodes.* A ghost map `paths`, node id -> path, is split
into pieces:

| Piece | Says | Where it is |
|---|---|---|
| `GhostMapAuth` | the whole map | held by the request worker |
| `GhostPointsTo(n -> p)` | node n's path is p | inside inode n's `path` lock, with the path |

- Changing node n's path needs both the `Auth` and n's `PointsTo`, which
  only a writer of n's lock can take out. Only the request worker has the
  `Auth`, so only it can change a path.
- While the request worker holds the `Auth`, `paths[n]` is node n's path,
  without holding n's lock. So the spec of `create` can say "let p =
  `paths[7]` + `x`".
- `FORGET` takes n's lock for reading and reads the concrete path. It
  could also look at n's `PointsTo` (a shared reference is enough to read
  a token), but cannot change it, and does not need it.
- The lock's rule pins the ids (LESSONS F): "holds `PointsTo(node ->
  path@)`, for this inode's node id and the `Auth`'s map id", both given
  as the lock's ghost rule when it is created; `VirtioFsInner::wf()` says
  that every inode in the table uses the `Auth`'s map id.

The same pattern as `FracGhost` (the pieces together are needed to change
the value, so a holder of one piece can trust it), packaged for a map:
one `Auth` for all inodes, with one id, instead of one `FracGhost` per
inode.

**The contracts it allows.** The FUSE handlers that change names
(`create`, `mkdir`, `mknod`, `symlink`, `unlink`, `rename`, `link`) take
the `Auth` as a tracked argument, passed down from the request worker's
loop through the FUSE dispatch (ghost parameters on trait methods, patch
0005). `may_change(p)` is the ghost field of `lxutil`'s `LxVolume`
(agreed 2026-11-08): set once, right after the policy is attached, to
`!read_only && policy@.can_write(p)`, and required by every `LxVolume`
method that changes the host. For `create` of `name` in node `n`, with p =
`paths[n]` + `name`:
if p is not writable (`!may_change(p)`), it fails and changes nothing; if
it succeeds, p was writable. `rename` and `unlink` also say how `paths`
changes.

**Removed inodes and node ids (decided 2026-11-09).**
- `FORGET` removes inodes on the high-priority worker, which has no `Auth`,
  so the inode's `PointsTo` disappears with the inode and the `Auth` keeps
  a stale entry. Harmless: `HandleMap::insert` (`lib.rs:966`) never hands
  out an id again (it only counts up, and refuses every insert once it
  reaches the end, `can_insert`, `lib.rs:1018`).
- The exception is the guest's FUSE `DESTROY` (`lib.rs:638`, on unmount):
  it clears the inode table except the root, and starts handing out ids
  from the beginning again (`lib.rs:1042`). It runs on the request worker,
  which then replaces its `Auth` with a fresh one (a new id), so the stale
  entries go with the old one. The root inode stays in the table with its
  old lock, whose rule names the old id; so **the root gets no
  `PointsTo`**: its path is always the empty path (the share's root;
  `rename` and `unlink` act only on children), and its lock's rule is just
  "the path is empty". Inodes that `DESTROY` removed are no longer in the
  table, and the spec only speaks of the inodes in the table.

**Open, for later:**
- "Nothing else on the host changed" (on success) needs a ghost view of the
  share; until then the `may_change` requirement on `LxVolume`'s changing
  methods carries it.
- Changes to a file itself (kind 2), which need all of its names, and
  `link`.

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

- the `Fuse` trait's operations, as the FUSE layer calls them;
- building the device (`new_microvm`, `new_microvm_aggregate`);
- the device's save and restore.

## Properties a spec could promise

Properties 1-4 are the security properties and 6 is snapshots: Pedro's two
themes, which the spec covers. 5 is deferred.

1. **Confinement**: every host object the device touches lies inside the
   share.
2. **Policy**: the guest sees only visible paths, and changes only writable
   ones. Precisely, two properties (agreed 2026-11-08):
   - **Confidentiality**: everything the guest gets back (a node id, a
     file's attributes, a directory entry, a file's bytes) is about a host
     object the guest may see.
   - **Integrity**: every change the guest causes on the host (a file's
     bytes or attributes, a name created, removed or renamed) is to an
     object the guest may write, in a directory the guest may write.

   Both rest on two definitions, written once from the policy and the
   namespace model (layer 4's facts, in the plan below): the guest **may
   see** a host object if some path in the share leads to it and that path
   is visible under the policy (a traverse-only directory: only that it
   exists, and only its entries that lead to allowed paths); the guest
   **may write** a host object if every name the guest has used for it is
   writable, and the guest cannot add a writable name to an object it may
   not write.

   **"May write" goes by name, not by object** (decision (b), 2026-11-08,
   to revisit after reading more code). The two differ only when the host
   has hard-linked one file across the policy's boundary. Example: an
   administrator links `/srv/data/tools/run` (read-only area) and
   `/srv/data/work/run` (writable area), one file; the guest looks up only
   `work/run` and writes to it. By object (reading (a)), the file is
   read-only, because one of its paths is; by name (reading (b)), it is
   writable, because the name the guest used is. The code does (b): a
   change needs every name the guest has used for the object (its aliases)
   to be writable. The guest itself cannot create such a link: linking an
   object it may not write into a writable directory fails with `EXDEV`.
   Reading (a) would need every host path to the file, which the code
   cannot learn cheaply. Note the asymmetry: for hiding, the code goes by
   object (denied identities, `CODE-OVERVIEW-DETAILS.md`, layer 3, change
   3); for writing, by name.

   **When permission is judged** (agreed 2026-11-08). Other host processes
   can change the share (rename, link, delete) at any time, without
   OpenVMM knowing. So the spec does not assume that only OpenVMM changes
   the share, and keeps no ghost copy of the host's namespace. Instead:
   - Permission is judged when the guest **reaches an object by name**: at
     a lookup, an open, or a create. At that moment, the object is checked
     against the policy, under the namespace as it is then (the trusted
     facts about each host call say what was true when the call happened).
   - Later operations through a node id or a handle the guest already holds
     act on that same object, known by its identity. A change made on the
     host in between does not take away what was already granted, as on
     Linux, where permission is checked when a file is opened and an open
     file stays writable after a rename.

   Example: the guest looks up `work/a.txt` (writable) and opens it for
   writing; an administrator then renames it on the host to `tools/a.txt`
   (read-only); the guest's later writes through its handle are allowed,
   and the code allows them too. So the two properties read: every object
   the guest reaches by name was one it may see at that moment; every
   change acts on an object the guest was allowed to change when it reached
   it. The code's enforcement
   rules (`CODE-OVERVIEW-DETAILS.md`, layer 3, change 4: every name change
   checked on all its paths; a file's change needs all its names writable;
   no hard link that makes a read-only file writable; hidden directories
   pinned by identity; filtered listings) are not specified one by one: they
   are how each operation meets these two properties, which is proof work.
3. **Identity**: each operation runs as its caller, guest root is mapped,
   and nothing runs as OpenVMM.
4. **Limits**: the caps are never exceeded.
5. **The operations' meaning**: `rename` renames, `read` returns the file's
   bytes, and so on. This rests on trusted facts about the host's
   filesystem. Deferred, read and write included (decided 2026-11-08): it
   has nothing to do with Pedro's changes.
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
  allows it; virtio-fs's spec adds "in order" on top, since its one worker
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

1. **The high-priority queue's worker.** It handles only `FORGET`,
   `BATCH_FORGET` and `INTERRUPT`: it lowers nodes' reference counts and
   removes a node from the inode table when its count reaches zero. It never
   changes an inode's path, and an operation that already took its own
   reference to an inode (`get_inode`'s `Arc`) keeps that inode alive. The
   microVM has it, so the spec considers it.
2. **Other request workers.** Not in the microVM, which has one request
   queue, so `lookup`, `rename`, `create` and the rest run one at a time.
   Upstream OpenVMM can have several (by default two, at most eight), which
   is why `VirtioFsInner` has so many `RwLock`s. The spec covers only the
   microVM (decided 2026-11-08).
3. **Other host processes,** which can change the shared folder at any time
   without OpenVMM knowing. See the first stage below.
4. **Save and restore.** They run only with the admission gate closed, no
   request in flight, and the queues stopped, so never during a request.
   The security properties do not need to consider them; the snapshot
   properties do, in the style of the snapshot spec.

The spec stays simple; the proof carries the concurrency reasoning (e.g.
that `rename`'s update of the aliases keeps the inode table's rule).

**First stage: assume that no other host process changes the shared
folder** (agreed 2026-11-08). With this assumption, `LxVolume`'s view is the
real tree of files, and it changes only through OpenVMM's own calls, so
each host call's contract says exactly how it changes the view (after
`rename(a, b)`, the object that was at `a` is at `b`), and the proof follows
the tree from call to call. This removes the open question of how to write
a view that changes between calls. Two conditions:

- **It is stated as an assumption**, in this document and later in the
  trusted list, worded as "no other host process changes the shared folder
  while the device runs", so that everyone can see which results depend on
  it. It is not true in general: NVX itself relies on the host changing a
  share while the VM runs (its benchmark `virtfs_live_roundtrip`, in
  `nvx/doc/benchmarks.md`, has the guest observe a host rewrite).
- **The parts that do not need it stay independent of it.** Confinement
  rests on `openat2` refusing anything outside the share's root, which the
  host's kernel guarantees even while other processes change the folder,
  so its proof does not use the assumption, and still holds when the
  assumption is dropped.

A later stage drops the assumption, using "When permission is judged"
(B'), and checks which proofs break: exactly the places where a host-side
change could matter.

**virtio-fs's layers.** Three steps, from the bottom:

1. **The policy as an abstract model** (written 2026-11-08: `virtiofs/src/microvm/policy.spec.rs`, and the contracts of `SubtreePolicy::new`, `visibility` and `is_writable` in `policy.rs`). `SubtreePolicy` gets a view,
   `SubtreePolicyView` (the denied, allowed and writable path sets), with
   closed spec functions `visibility(p)` (visible, traverse-only, hidden)
   and `can_write(p)`, written from the policy's documentation and then
   compared with the code. `SubtreePolicy::new`'s checks become its `wf()`.
2. **The filesystem's state as a view** (agreed 2026-11-08). `VirtioFsView`
   records what the guest currently holds:
   - `policy`: the share's policy (denied, allowed and writable paths, and
     the denied identities); it never changes after the device is built;
   - `nodes`: for each node id the guest holds, the host object's identity
     (device and inode number), its kind (directory, file, symlink), its
     names (the paths in the share through which the guest reached it, its
     aliases), and the guest's reference count;
   - `handles`: for each handle the guest holds, its node, and whether it
     was opened for writing.
   - `volume`: the view of the share's `LxVolume` (layer 4's object, one per
     share, which `VirtioFs` holds and through which every host operation
     goes): the share's contents as a tree, for each directory its entries
     (name -> object), for each object its identity and kind (agreed
     2026-11-08). `lookup`'s contract can then say directly that the node's
     object is the object `volume` has at the looked-up path.

   A node says which file (from a lookup; enough to ask about the file,
   rename it or delete it); a handle says an open file (from an open; reads
   and writes go through it); one node can have several handles. Example,
   after the guest looks up `work/a.txt` and opens it for writing:
   `nodes: 5 -> { object: (dev 8, ino 4711), kind: file, names:
   {work/a.txt}, refs: 1 }`, `handles: 9 -> { node: 5, writable: true }`.

   The view changes with the guest's requests: `open` and `opendir` add a
   handle, `release` and `releasedir` remove it; `lookup` adds a node or a
   reference, `forget` drops references; `create`, `mkdir`, `mknod` and
   `symlink` add a node (`create` a handle too); `link` adds a name;
   `unlink`, `rmdir` and `rename` remove or change names. Changes made on
   the host by other processes do not change it: it records what the guest
   was granted when it reached each object (see "When permission is
   judged").

   Whether a path is visible or writable depends only on the path and the
   policy, not on the host's current namespace, so `wf()` needs no host
   facts: every name in `nodes` is visible under the policy, or is a
   traverse-only directory; no node's object is a denied identity; every
   handle opened for writing belongs to a node whose names are all
   writable.
3. **The operations' contracts**, on the `Fuse` trait's operations, in terms
   of that view: `lookup` succeeds only for a name the guest may see and
   returns that path's node; an operation that changes something succeeds
   only on a path the guest may write; a traverse-only directory lists only
   the entries that lead to allowed paths. The conditions are spec
   functions of `VirtioFsView` (agreed 2026-11-08), and the contracts only
   call them. For `lookup`:

   ```rust
   impl VirtioFsView {
       /// The path, in the share, of `name` in the directory `parent`.
       closed spec fn child_path(self, parent: u64, name: Name) -> PathView;
       /// The guest may reach `name` in `parent`: its path is visible under
       /// the policy (or is a traverse-only directory), and `object` is not
       /// a denied identity.
       closed spec fn may_look_up(self, parent: u64, name: Name, object: HostObject) -> bool;
       /// The view after a lookup of `name` in `parent` returned `node`, for
       /// `object`: `node` has the child's path among its names, and one more
       /// reference.
       closed spec fn after_lookup(self, parent: u64, name: Name, node: u64, object: HostObject) -> VirtioFsView;
   }
   ```

   and the contract: on `Ok(entry)`, with `object` the host object the
   lookup reached, `old(self)@.may_look_up(parent, name, object)` and
   `final(self)@ == old(self)@.after_lookup(parent, name, entry.node_id,
   object)`; on `Err`, the view is unchanged; and `wf()` before and after.
   The code's checks (traverse-only filtering, the denied-identity check,
   ...) are not in the contract: they are how the code meets it. The
   policy's own functions, `visibility(p)` and `can_write(p)`, stay on
   `SubtreePolicyView` (step 1), and `may_look_up` calls them. `object` is
   the object that the view's `volume` has at the looked-up path.

   **Proving down to the host** (agreed 2026-11-08): layer 3's operations
   are proved against the contracts of the `LxVolume` methods they call;
   `LxVolume`'s methods (our code, in `lxutil`) are proved against the
   host's system calls; only the system calls are trusted, as a model of
   what each does to the tree (e.g. `renameat2` moves an entry, `openat2`
   with these flags resolves only beneath the root). So `LxVolume`'s view
   is what ties the spec to the host's filesystem.

   **Open, to decide later**: other host processes can change the share's
   tree between two calls, without OpenVMM ("When permission is judged"),
   so the volume's view can change even when OpenVMM does not call it.
   Each `LxVolume` method's contract describes the tree at the moment of its
   own call, and nothing promises that it stays the same between calls; how
   to write that in Verus (the view of an object that changes outside our
   code's calls) is still to work out.

Confinement and the caller's identity then build on step 3, with trusted
facts about the host's `openat2` and credentials.

**Layer 4 in the spec: what we assume about the host** (discussed
2026-11-08). Layer 4 calls the host's filesystem (Linux's system calls,
or Windows's), which is outside our code, so the spec must state, as
trusted facts, what those calls do. We state only what our properties
use, not everything the calls do:

- **Which object a call acts on, and how names change.** Each host object
  has an identity (device and inode number) and a kind (directory, file,
  symlink), and each directory maps names to objects. The facts say how
  the namespace calls change that (`mkdirat`, `linkat`, `unlinkat`,
  `renameat2`, including `RENAME_EXCHANGE`: after `renameat2(a, b)`
  succeeds, the object that was at `a` is at `b`), what `lstat` and a
  directory listing report, and what the confinement calls guarantee
  (`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` returns an
  object beneath the root, reached without symlinks, or fails;
  `O_NOFOLLOW` never follows a symlink). Layer 3's proofs need these: its
  alias bookkeeping after a rename or unlink, its policy checks on paths,
  its filtering of traverse-only directories.
- **What the caller's identity calls change**: the raw credential calls
  change only the calling thread's filesystem UID, GID, groups and
  capabilities.
- **Bounds on what calls return**: e.g. `pread` returns at most the bytes
  asked for, `getxattr` fits the buffer given. Layer 3's buffer bounds rely
  on them.
- **File contents: not needed for now.** Read and write's meaning ("the
  guest reads back what it wrote") would need facts about file contents
  (`pread` returns the content's slice from the offset; `pwrite` replaces
  it). Decided (2026-11-08, revised the same day): not in the spec for now,
  since it has nothing to do with Pedro's changes; the spec covers his two
  themes, snapshot and restore, and security against the guest.

Left out: file contents, timestamps, permissions, extended attributes'
meaning and the rest of what the calls do, unless a property needs them. On Linux, layer 4
is otherwise a thin wrapper, one `*at` system call per operation, so those
calls are the trusted facts; its two pieces of real logic, the
component-by-component confinement walk and the credential switching, are
proved against them. The Windows side (`lxutil/src/windows/`, about 7,600
lines that emulate Linux behavior on NTFS, nearly all upstream code; Pedro
changed 5 lines of it) needs its own facts, and is deferred, to decide
later, since the microVM runs on WHP.

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
is the one that was saved.

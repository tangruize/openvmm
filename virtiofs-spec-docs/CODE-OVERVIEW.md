# virtio-fs: what the code does

An overview of OpenVMM's virtio-fs device, as a starting point for its
spec, in Pedro Henrique Penna's latest code: `microsoft/openvmm`, branch
`feature/microvm/dev`, commit `bdea9ab19` (2026-11-05). Our branch,
`system-proof/virtiofs-spec-20261106`, is older; compare a function with
that commit before specifying it. The spec's style rules and lessons are
in `snapshot-spec-docs/CONVENTIONS.md` and `snapshot-spec-docs/LESSONS.md`.

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
- **Each inode's `path: RwLock<PathBuf>`**: the path lies inside the share
  and is visible under the policy; for a writable inode, under a writable
  path. This carries properties 1 and 2, and is where `rename` changes
  things: the delicate one.
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

What we need to build: a wrapper that gives `RwLock` an invariant, in the
style of `mutex_t.rs`. vstd's verified `RwLock` (`vstd/rwlock.rs`,
`RwLockPredicate`) shows the interface; the production code uses Rust's own
`RwLock`, so the wrapper is trusted, like our `Mutex` wrapper. These
invariants come after the policy's spec (layer 1 of the plan), since each
mentions its `visibility` and `can_write`.

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

Properties 1-4 are the security properties, the spec's first goal; 5 and 6
are about correct behavior.

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
   **may write** a host object if every path in the share that leads to it
   is writable (this "every path" is what handles hard links). Both are
   judged on the namespace at the time of each request, since renames and
   links, by the guest or on the host, change it. The code's enforcement
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
   filesystem. Read and write's meaning is in the spec from the start
   (decided 2026-11-08); the rest (attributes, extended attributes,
   timestamps) later.
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

**virtio-fs's layers.** Three steps, from the bottom:

1. **The policy as an abstract model.** `SubtreePolicy` gets a view,
   `SubtreePolicyView` (the denied, allowed and writable path sets), with
   closed spec functions `visibility(p)` (visible, traverse-only, hidden)
   and `can_write(p)`, written from the policy's documentation and then
   compared with the code. `SubtreePolicy::new`'s checks become its `wf()`.
2. **The filesystem's state as a view.** `VirtioFs`'s inodes and open files
   as share-relative paths ("node 5 is `a/b`"; "handle 9 is node 5, open for
   writing"). Its `wf()`: every inode and handle the guest holds has a path
   the guest may see.
3. **The operations' contracts**, on the `Fuse` trait's operations, in terms
   of that view: `lookup` succeeds only for a name the guest may see and
   returns that path's node; an operation that changes something succeeds
   only on a path the guest may write; a traverse-only directory lists only
   the entries that lead to allowed paths.

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
- **File contents, only for read and write.** A file's content is a
  sequence of bytes; `pread` returns its slice from the offset, at most the
  length asked for, and `pwrite` replaces that slice. Layer 3's `read` and
  `write` only pass bytes between the guest and these calls, so their
  contracts say exactly that, and "the guest reads back what it wrote"
  follows. Decided (2026-11-08): read and write's meaning is in the spec
  from the start, together with confinement, policy, identity and limits.

Left out: timestamps, permissions, extended attributes' meaning and the
rest of what the calls do, unless a property needs them. On Linux, layer 4
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

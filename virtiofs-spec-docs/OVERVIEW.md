# The microVM's virtio-fs spec: overview

This is the place to start. It says what the spec is about and how it is
built, in plain words; the other documents (section 6) have the details.

The spec is written in Verus, for the microVM's virtio-fs: the device that
lets a guest use a folder of the host as a file system. The code is Pedro
Henrique Penna's: his branch `feature/microvm/dev` at `db42fde04`, which
builds on OpenVMM's existing virtio-fs. The spec is ours, on the branch
`system-proof/virtiofs-spec-20261108`, which is his code plus the spec
(and the code changes `CODE-CHANGES.md` lists). It is a spec: the
functions' bodies are not proved yet (`external_body`), and a proof agent
will prove them.

## 1. The goal

A guest may be controlled by an attacker. The guest sees a host folder,
the **share**, and sends requests: look up a name, open, read, write,
create, rename, delete, and about 30 more. The spec says what the device
promises, whatever the guest sends:

1. **Confinement.** The guest reaches only objects inside the share, never
   anything else on the host.
2. **Policy: integrity and confidentiality.** Inside the share, a profile
   may hide paths (*denied*), show some of them again (*allowed*), and make
   the rest read-only (only the *writable* paths may change). Integrity:
   the guest changes only what it may change, through any name.
   Confidentiality: it sees only what it may see.
3. **Identity.** Each request runs on the host as the guest's caller
   (mapped, so never as host root), or as OpenVMM in the default mode.
4. **Limits.** The device's tables stay within fixed caps (4096 nodes,
   4096 open handles, and so on), whatever the guest does.

Two more are not specified. **5. File contents and attributes**, by
decision, since they are not about security: what `read` returns and
`write` changes in a file's bytes, the attributes `getattr` reports, the
values of extended attributes, and a listing's full contents. (Much of
what the operations mean is specified: when each succeeds or fails, and
how it changes the guest's names, nodes and handles, section 3.) **6.
Snapshots**, the device's own save and restore, stopped until this spec is
verified (section 5).

## 2. The software

A request travels from the guest's kernel to the host's disk through five
layers:

```
  guest kernel
      │  a FUSE request in a virtio queue (guest memory)
      ▼
  0. virtio transport      vm/devices/virtio/virtio        the queue, the MMIO registers
  1. virtio-fs device      virtiofs/src/virtio.rs          takes each request off the queue
      │  owner::dispatch: switch to the caller's identity (Caller mode)
      ▼
  2. FUSE session          support/fs/fuse                 decodes it, calls a `Fuse` method
  3. the file system       virtiofs/src/lib.rs             `VirtioFs`: lookup, open, write, ...
      │  remembers what the guest holds: nodes ("node 5 is a.txt") and handles
      ▼
  4. host access           support/fs/lxutil               `LxVolume`: one host call per method
      │  openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS) on Linux
      ▼
  host kernel, host disk
```

Pedro's microVM additions sit in layer 3 (`virtiofs/src/microvm/`,
`aggregate.rs`): a fixed **profile** (one share, or an *aggregate* of
several shares under a synthetic root), the **access policy**, the
**caps**, **owner modes** (`Vmm`, the default, or `Caller`), and save and
restore.

The device handles **one request at a time**: two queues, but both
workers run on one thread, and a request runs to its end. Yet the code
guards its tables with locks (`RwLock`s), as if requests could run at
once. We decided to follow the locks, not to rely on the order: each lock
gets a rule that says what its data is (section 4.2; for this, vstd's
locks replace `parking_lot`'s, a change for verification only). Only the
keeper, which hands the ghost state to each request, relies on requests
running one at a time (section 4.3).

## 3. What we prove

The promises are stated once, on the `Fuse` trait's methods
(`fuse/src/lib.rs`), and virtio-fs meets them. For each operation:

- **When it may succeed.** For example, `write` succeeds only through a
  handle the guest holds, opened for writing, on a file whose every name
  the guest may write. `create` makes a name only at a path the guest may
  write.
- **What it changes.** For example, after `rename(a, b)` the names below
  `b` are gone and those below `a` are below `b`; after a failure,
  nothing changed.
- **When it fails, and with which code.** Each operation has a set of
  rules, each a condition and an error code: "the node is not held:
  `EINVAL`", "the path is hidden: `EACCES`", "no room for a node:
  `ENOSPC`". If a rule applies, the operation fails, with the code of one
  of the rules that apply, or with an error of the host. The order in
  which the code checks them does not matter.

Behind them, facts that hold before and after every request (`inv`,
`wf`), for example:

- every name the device stores is a plain path inside the share;
- every name of a node names that node's object on the host;
- a hidden object recorded when the share was attached has no other name;
- no object has both a writable name and a visible read-only one;
- the caps hold.

Writing the spec found **9 bugs** in the code (`BUGS.md`), each fixed with
a test, and a patch for Pedro (`patches/`). For example: a `rename` of a
directory could move a hidden file into view, where the guest could
delete it (bug 5); a `link` to a deleted file linked another file (bug
4).

## 4. How the spec works

### 4.1 Two levels: questions and answers

The promises in section 3 need words like "the guest may write this
path" or "this node is a directory". The FUSE level does not know
virtio-fs, so it declares these as **questions** (`FuseState`,
`fuse/src/lib.spec.rs`): `can_write(n, p)`, `is_dir(n)`, `is_hidden(n, p)`,
and so on. The rules are written with them. virtio-fs **answers** each
question from its own state (`virtiofs/src/lib.spec.rs`): `can_write`
reads the share's policy, `is_dir` the node's type.

So the trait's contracts read like the FUSE protocol, and the answers say
what virtio-fs means by each word. Another file system could answer them
differently.

### 4.2 The ghost state `g`

What the guest holds (its nodes, each node's names, its open handles) is
spread over the code's data, behind five locks. The spec gathers it into
one **ghost state**, `g` (`VirtioFsGhost`), which exists only for Verus.
Every operation takes `g`, and its contract says how `g` changes. `g`'s
**view** is what the contracts read: the nodes, their names, the handles.

`g` has to match the real data. The trick is to split each piece of ghost
state in two:

```
           g (the request's)                       the locks (the code's data)
  ┌─────────────────────────────┐       ┌───────────────────────────────────────┐
  │ half of the inode table ────┼── = ──┼─ half, in the inode table's lock      │
  │ half of the file table  ────┼── = ──┼─ half, in the file table's lock       │
  │ the names' map (Auth)   ────┼── = ──┼─ each inode's piece, in its own locks │
  │ the shares' view, the disks │       │   (path lock, aliases lock)           │
  └─────────────────────────────┘       └───────────────────────────────────────┘
```

Each lock's **rule** says its data matches its piece ("the table in this
lock is what this half records"). Changing the data needs both pieces,
the lock's and `g`'s, so the code can change the tables only while it
holds `g`. That is how `g` always tells the truth.

### 4.3 The keeper

Between requests, `g` must be kept somewhere, so that the next request
gets what the last one left. The keeper (`virtiofs/src/fs_ghost_t.rs`)
does that: a request takes `g` at its start and gives it back at its end.
It is trusted, with two assumptions: one request at a time has `g` (true,
since requests run one at a time), and the keeper holds this file
system's `g`. It also hands out the thread's credential token (4.5).

### 4.4 The host model

Some promises are about which object a name reaches, which only the host
knows. The **host model** (`lxutil/src/host_t.rs`) describes, for each
share, the object at each path on the host's disk (its device and inode
numbers, and its type). Each `lxutil` method's contract says how it reads
or changes that: `lstat` reports the object at the path, `unlink` removes
the path, `rename` moves a subtree, and so on. `g` keeps each volume's
**disk** value, and `wf` ties each node's names to its object there.

This is what lets the spec say that a hidden object stays hidden at other
names, that hard links do not mix writable and read-only names, that the
share's root is the intended folder, and that no name passes through a
symbolic link.

### 4.5 Identity

The thread's credentials are a ghost token (`lx/src/cred_t.rs`): its view
is `Own` (OpenVMM's) or `Switched { uid, gid }`. `lxutil`'s
`switch_fs_identity` and `restore` change it, and `Session::dispatch`
requires it to be the request's identity (`CallerIdentity::runs_as`,
`microvm/owner.spec.rs`). So a request in `Caller` mode runs only as its
mapped caller.

## 5. What is trusted, and what is not done

**Trusted** (Verus does not check it; each is written where it is used,
and listed in `SPEC-DESIGN.md`, section 11):

- **The host assumptions:** no other host process changes a share while
  the device runs; no mount point inside a share (Linux); no two volumes
  share a folder; an object's inode number is the same on every call; on
  Windows, different names are different objects (no other case, no short
  names).
- **Facts about the share as attached** (one axiom, `attached_disk`): no
  host hard link joins a writable and a visible read-only name, a folder
  has one name, and the share is a tree.
- **`lxutil`'s methods meet their contracts** (their bodies are not
  verified), and the host kernel's guard keeps them inside the share.
  That a call changes nothing outside the share is said in words; it
  could become a contract (`lxutil/src/lib.spec.rs` says how).
- **The keeper** and its two assumptions, and the credential token's
  meaning.
- **Facts about Rust's standard library** in our patched `vstd` (a path's
  view, `AsRef`).

**Not done:**

- **The proof.** Every body is `external_body`. Where the proof must
  create ghost values at start-up, the spec marks it `PLACEHOLDER`.
- **Snapshots** (property 6): virtio-fs's own save and restore, stopped by
  decision until this spec is verified. Combining with the VM's snapshot
  spec (its own branch) needs `g` to leave the keeper while the VM is
  stopped, as a device's snapshot ghost does (`SPEC-DESIGN.md`, open
  items).
- **File contents and attributes** (property 5), by decision: not about
  security.
- Smaller gaps, each in `SPEC-DESIGN.md`'s open items: the lookup counts,
  the listings' caches, a byte cap on `rename`.

Some code changes exist only to make verification possible, and are not
for Pedro: for example vstd's locks in place of `parking_lot`'s, the
`Fuse` trait's ghost type parameter, and the identity switch as two calls
instead of a closure. The bug fixes, and one change that makes listings
simpler to specify, are patches for Pedro. `CODE-CHANGES.md` lists every
change and says which is which.

## 6. The other documents

All in `virtiofs-spec-docs/`:

| Document | What it is |
|---|---|
| `OVERVIEW.md` | this one: start here |
| `SPEC-DESIGN.md` | the spec's design, for the proof agent: every file, the FUSE level, `g`, the locks, the keeper, confinement, what is trusted, the rules for the proof, the open items |
| `BUGS.md` | the bugs found, each with what goes wrong, the fix and its patch; and the questions for Pedro |
| `CODE-CHANGES.md` | every change to what a normal build compiles, and why |
| `patches/` | the bug fixes as patches on Pedro's code (they apply in order) |
| `CODE-OVERVIEW.md` | the working record: how the code works, every decision with its date, the status of each property |
| `CODE-OVERVIEW-DETAILS.md`, `DATA-STRUCTURES.md` | the code's layers and data structures, in detail |
| `notes-20261111/` | design notes and the review of 2026-11-11 (identity, limits, snapshots, the decisions made) |

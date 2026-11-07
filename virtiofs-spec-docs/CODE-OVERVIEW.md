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

## The four layers

Following that path:

1. **virtio glue** (`vm/devices/virtio/virtiofs/src/virtio.rs`,
   `virtio_util.rs`): takes requests off the virtio queues and puts replies
   on. Mostly upstream code.
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

## Where a spec would attach

- the `Fuse` trait's operations, as the FUSE layer calls them;
- building the device (`new_microvm`, `new_microvm_aggregate`);
- the device's save and restore.

## Properties a spec could promise

1. **Confinement**: every host object the device touches lies inside the
   share.
2. **Policy**: the guest sees only visible paths, and changes only writable
   ones.
3. **Identity**: each operation runs as its caller, guest root is mapped,
   and nothing runs as OpenVMM.
4. **Limits**: the caps are never exceeded.
5. **The operations' meaning**: `rename` renames, `read` returns the file's
   bytes, and so on. This rests on trusted facts about the host's
   filesystem.
6. **Snapshots**: save records no host paths, and restore accepts only the
   same host objects.

## The plan (proposed 2026-11-06)

Three layers, from the bottom:

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

Confinement and the caller's identity then build on layer 3, with trusted
facts about the host's `openat2` and credentials.

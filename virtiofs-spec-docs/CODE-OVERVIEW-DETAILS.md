# virtio-fs: the code in detail

The details behind `CODE-OVERVIEW.md`: each of the four layers on its own,
plus the generic virtio layer under them (layer 0, where the ring buffer
is),
what it does with an example, and then what Pedro Henrique Penna added to
it, change by change. The code is Pedro's latest (`microsoft/openvmm`,
branch `feature/microvm/dev`, commit `bdea9ab19`, 2026-11-05). Each change
names its commit; its commit message has the full story
(`git log -1 <commit>`). The dates are those of the rebased chain, not
always when the work was first done; a change with a pull-request number
(`#4594`) has landed upstream.

## Words used below

- **The guest** is the virtual machine; **the host** is the real machine
  that runs OpenVMM. **The share** is the host directory that the guest
  mounts (in the examples, `/srv/data`, mounted in the guest at `/mnt`).
- **A FUSE request** is one filesystem operation that the guest kernel sends
  for a path under the share: `FUSE_LOOKUP`, `FUSE_OPEN`, `FUSE_READ`, and
  so on. Its header carries the operation, a **node id** (which file or
  directory it is about), and the **UID and GID** of the guest process that
  made it.
- **A node id** is the number by which the guest names a file or directory,
  like an inode number. OpenVMM hands one out in its reply to a lookup
  ("`a.txt` is node 5"); the guest uses it in later requests ("open node
  5"). Node 1 is always the share's root.
- **A lookup count**: each time OpenVMM hands out a node id, the guest's
  count of references to it goes up by one; when the guest no longer needs
  the node, it sends `FUSE_FORGET` with the number of references it drops.
  When the count reaches zero, OpenVMM may forget the node.
- **A handle** is the number by which the guest names an open file or
  directory, handed out by `FUSE_OPEN` or `FUSE_OPENDIR` ("node 5 is open
  as handle 9") and given back by `FUSE_RELEASE`.
- **`FUSE_INIT`** is the first request of a session: the guest and OpenVMM
  agree on the protocol version, the optional features, and limits such as
  the largest write. `FUSE_DESTROY` ends the session.

## Layer 0: the generic virtio layer (where the ring buffer is)

`vm/devices/virtio/virtio` (about 7,500 lines), shared by every virtio
device in OpenVMM (virtio-blk, virtio-net, virtio-fs, the console, ...).
It is not part of virtio-fs, but every virtio-fs request passes through it,
under layer 1.

### What it does

It has three parts.

**1. The ring buffer: the virtio queue** (`queue.rs`, `queue/split.rs`,
`common.rs`). A queue lives in guest memory, which the guest's driver
allocates, and OpenVMM reads and writes there. It has three parts (the
"split" queue of the virtio standard, the kind the microVM uses):

- **the descriptor table**: an array of descriptors, each naming one buffer
  in guest memory: its guest address, its length, whether the device may
  write it (a reply buffer) or only read it (a request buffer), and the
  next descriptor of the same chain. A request is a chain: for a FUSE
  request, first the request's buffers, then the buffers for the reply;
- **the available ring**: written by the guest. Each entry is the first
  descriptor of a chain that is ready for the device, and an index says how
  far the guest has filled it;
- **the used ring**: written by the device. Each entry is a chain the
  device has finished and how many bytes it wrote into the reply buffers,
  and an index says how far the device has filled it.

OpenVMM remembers how far it has read the available ring
(`last_avail_index`). `VirtioQueue` (`common.rs`) is the queue as a device
sees it: `next()` waits for and returns the next ready chain, as a
`VirtioQueueCallbackWork` from which the device reads the request and into
which it writes the reply, and `complete(work, bytes_written)` puts the
chain on the used ring. Layer 1's loop uses exactly these two. (The
"packed" queue, `queue/packed.rs`, is another layout of the standard; the
microVM disables it.)

**2. The transport: how the guest finds and talks to a device**
(`transport/`). The microVM uses **virtio-MMIO** (`transport/mmio.rs`):
each device is a window of registers at a fixed guest-physical address
(virtio-fs's is `0xd0001000`), which the guest learns from its kernel
command line. The guest's driver reads there the device type and its
features, writes the guest addresses of each queue's three parts, writes
the `QueueNotify` register to say "a queue has new requests" (the
doorbell), and reads and acknowledges the interrupt status when the device
interrupts it. (OpenVMM also has a PCI transport, `transport/pci.rs`,
which the microVM does not use.) Behind the registers, a device task
(`transport/task.rs`) turns the guest's register writes into commands for
the device: start the queues, stop them, reset.

**3. The device interface: the `VirtioDevice` trait** (`device.rs`). What
each virtio device implements, and so what layer 1 is: its features and
queue count (`traits`), its configuration registers
(`read_registers_u32`, `write_registers_u32`), `start_queue` and
`stop_queue` (with the queue's progress, so a stopped queue can resume
where it stopped), `reset`, `quiesce_input` and `resume_input`, and
`save_device` and `restore_device`.

Example: the guest's driver wants to send a lookup. It writes the request
into one buffer and leaves an empty buffer for the reply; it fills two
descriptors (the request buffer, readable; the reply buffer, writable,
chained after it); it adds the first descriptor's index to the available
ring and raises the ring's index; and it writes `QueueNotify`. The write
traps to OpenVMM, whose queue sees a new chain: `next()` returns it to
layer 1. When layer 1 has the reply, `complete(work, bytes)` adds the chain
to the used ring, and the transport interrupts the guest, whose driver
reads the used ring and finds its reply.

### What Pedro added

1. **Device-private saved state** (`3d7b3af1c`). The transports saved only
   their own state: the negotiated features, the queues' configuration and
   progress. A device whose state is not in its queues (virtio-fs's inode
   table, a console's buffered data) had nowhere to save it. Now
   `VirtioDevice` has `save_device` and `restore_device`, called after all
   queues have stopped, and the transport saves the device's blob with its
   own state. Restore also stopped trusting the saved queues: it checks
   exactly two feature banks, queue sizes within each queue's maximum,
   rings that are aligned, inside guest RAM and not overlapping, and
   progress consistent with the rings, before it changes anything.
2. **Quiescing input through the transports** (`77d460b89`). A snapshot
   first stops host input from reaching devices, and resumes it if the
   save fails. `VirtioDevice` gains `quiesce_input` and `resume_input`, and
   the transports forward them to the device task, ordered with its other
   commands. virtio-fs's admission gate (layer 1) is its implementation.
3. **Masking features** (`faf2edaac`). A fixed microVM device must control
   which features it offers; the microVM supports only split queues. The
   MMIO transport can now hide features from the guest
   (`new_with_disabled_features`), and a saved state that negotiated a
   hidden feature fails to restore.
4. **Shared interrupt status** (`8b06bebcd`). Each virtio-MMIO interrupt
   cost the guest two trapping register accesses: reading
   `INTERRUPT_STATUS` to learn why, and writing `INTERRUPT_ACK` to clear it.
   In the new mode, the status lives in a word of guest memory, set by the
   device with an atomic compare-and-exchange; the device raises the
   interrupt only when the word goes from zero to nonzero, and the guest
   reads and clears it without trapping. (The NVX guest kernel's patch for
   "the shared virtio-mmio interrupt status" is the guest's side of this.)
5. **Restricting device DMA** (`0efa13a45`, `7b666d1e7`). A descriptor
   could name any guest-physical address. Now a device's view of guest
   memory admits only a set of ranges, the RAM ranges of the microVM's
   memory map (`e820`), and a queue refuses a descriptor whose buffer lies
   elsewhere; the transport marks such a device as needing a reset.
6. **Keeping queue progress after a failed start** (`d8e8f271c`,
   2026-11-06, after the commit the rest of this file describes). If one of
   a device's queues failed to start, the queues that had started were
   stopped, and their progress was lost, so a later start repeated the
   requests they had completed, such as a file system operation. Now that
   progress is kept.

## Layer 1: the virtio glue

`vm/devices/virtio/virtiofs/src/virtio.rs` (682 lines) and `virtio_util.rs`.

### What it does

This layer is the virtio-fs **device**: what the guest's driver sees as a
piece of hardware. It implements layer 0's `VirtioDevice` trait, and uses
layer 0's queue (the ring buffer) and transport. It does four things:

- **It describes itself.** When the guest's driver reads the device's
  configuration, it answers with the share's tag (the name the guest mounts,
  `mount -t virtiofs microvm /mnt`) and the number of request queues.
- **It starts and stops its queues** when the guest's driver asks, and
  resets the device.
- **It runs a worker loop per queue** (`run`): wait for the guest's
  notification, take the next request off the queue, give its bytes to
  layer 2, write the reply bytes into the guest's reply buffer, and tell the
  guest the request is done. One request is handled completely before the
  next is taken.
- **It quiesces and resumes its input** around a snapshot (see the admission
  gate below).

Example: the guest's driver puts a `FUSE_LOOKUP` request for `a.txt` on the
queue and writes the device's notification register. The worker wakes up,
takes the request, passes it to layer 2, gets back a reply ("node 5"),
writes it into the guest's reply buffer, and interrupts the guest.

`git blame` on our older copy gives 193 of its 643 lines to Pedro, the rest
to OpenVMM's original code.

### What Pedro added

1. **A bound on request sizes** (`96ffa21cb`). Several paths trusted sizes
   that the guest sends. A `read`, `getxattr` or `listxattr` allocated a
   buffer of whatever size the guest asked for, so one request could make
   OpenVMM allocate up to 4 GiB; `write` and `setxattr` accepted payloads
   of any size; the code that walks the guest's buffers computed lengths and
   addresses with unchecked arithmetic, so a malformed request could
   underflow or panic. Now:
   - a guest-requested buffer is capped at 1 MiB, and a larger request
     fails with `E2BIG`;
   - memory is reserved fallibly, and a failure is `ENOMEM`, not a crash;
   - offsets, lengths and guest addresses use checked arithmetic, and an
     inconsistent request is reported as invalid data or `EIO`;
   - the warnings a guest can trigger at will (unknown node ids and
     handles, too many forgets, ...) are rate-limited, so a guest cannot
     flood the host's log.
2. **The fixed microVM device** (`5333cd695`). `VirtioFsDevice::new_microvm`
   builds a device with a fixed shape: one request queue, no "packed"
   virtio rings, and no shared-memory region (so no DAX). Two hardening
   changes apply to every virtio-fs device: a request larger than the
   largest write plus its header is reported as corruption, and a
   configuration read past the end returns zero instead of panicking.
3. **The admission gate** (`82776b0cf`), for snapshots. Before the worker
   takes a request off the queue, it must get a `RequestGuard` from the
   gate (`self.admission.accept()`). Quiescing for a snapshot closes the
   gate, and a save requires the gate closed, no request in flight, and the
   queues stopped. So a request that the guest put on the queue during the
   save stays on the queue, instead of being taken off and answered with an
   empty reply; after restore, the worker takes it.
4. **A dormant slot** (`358de8f5c`). A microVM has a fixed list of devices,
   which a snapshot records, and a restore cannot add a device. So a microVM
   started without a share could never get one after a restore. Now every
   microVM has the virtio-fs device: without a share it is **dormant** (no
   host directory attached, `new_microvm_dormant`), and a later restore can
   attach a share to it.
5. **Running each request as its caller** (`df92e9fda`). With the option
   `--mount-owner caller`, the worker performs each request that touches
   host files as the UID and GID in the request's header (layer 4 does the
   switch). `INIT`, `DESTROY`, `FORGET`, `BATCH_FORGET`, `INTERRUPT`,
   `RELEASE` and `RELEASEDIR` touch no host file and run unchanged, so the
   guest can always mount, unmount, and release what it opened. Details in
   layer 3.

## Layer 2: the FUSE layer

`vm/devices/support/fs/fuse` (about 3,800 lines): `request.rs`,
`session.rs`, `reply.rs`, `protocol.rs`.

### What it does

This layer knows the FUSE protocol, and nothing about the host's files. It
plays the part that the library `libfuse` plays for a classic FUSE daemon:

- **It decodes a request** (`Request::new`, `request.rs`): it copies the
  header out of the guest's buffer (operation, node id, UID, GID, a
  request number) and then the operation's arguments (for a lookup, the
  name; for a write, the offset and the data). Copying first means that the
  guest cannot change a request after it has been checked.
- **It runs the session** (`Session`, `session.rs`). The first request must
  be `FUSE_INIT`; the session answers with the protocol version and
  features it accepts and the limits (largest write, ...), and remembers
  them. Until then, any other request is refused. `FUSE_DESTROY` ends the
  session.
- **It dispatches** (`Session::dispatch`): for each request, it calls the
  matching method of the `Fuse` trait, implemented by layer 3:
  `FUSE_LOOKUP` calls `lookup(request, name)`, `FUSE_READ` calls
  `read(request, arguments)`, and so on.
- **It encodes the reply** (`reply.rs`): the method's result, or an error
  number such as `ENOENT`, as the FUSE reply bytes that layer 1 writes back.

Example: the bytes of a lookup become `Request { opcode: FUSE_LOOKUP,
nodeid: 1, uid: 1000, gid: 1000, name: "a.txt" }`; the session checks that
`FUSE_INIT` has happened and calls `lookup` on layer 3; its answer ("node 5,
these attributes") is encoded as a `fuse_entry_out` reply.

### What Pedro added

1. **Saving and restoring the session** (`8b3255cb7`). What the session
   agreed with the guest at `FUSE_INIT` lived only in memory, so a restored
   device would start uninitialized while the guest's driver went on with
   the agreed protocol. Now `SessionState` records whether the session is
   initialized and what was agreed, with no file handles or other host
   resources; `Session::save_state` captures it and
   `Session::restore_state` restores it after checking it: an uninitialized
   state carries no agreed values; an initialized one has a supported
   protocol version (7.27 up to the implementation's), only features the
   guest offered, consistent extended flags, and nonzero limits. Two fixes
   make the saved values well defined: the agreed minor version is now the
   lower of the guest's and the implementation's (before, the guest's,
   which could claim a newer protocol than the code implements), and
   `FUSE_DESTROY` resets what was agreed.
2. **Rate-limited diagnostics** (`926827446`, upstream `#4581`): malformed
   or unsupported requests, which the guest controls, are logged at a
   limited rate.

## Layer 3: the filesystem (`VirtioFs`)

`vm/devices/virtio/virtiofs/src/lib.rs`, `inode.rs`, `file.rs`,
`aggregate.rs`, and Pedro's `microvm/` directory (about 6,000 lines with
`aggregate.rs`).

### What it does

`VirtioFs` implements the `Fuse` trait: one method per FUSE operation,
about 35 (`lookup`, `get_attr`, `set_attr`, `open`, `create`, `read`,
`write`, `release`, `mkdir`, `mknod`, `symlink`, `link`, `unlink`, `rmdir`,
`rename`, `open_dir`, `read_dir`, `read_dir_plus`, `statfs`, `fsync`, the
xattr operations, `forget`, ...). Because the guest names everything by
numbers, it keeps two tables:

- **The inode table** (`inodes`, an `InodeMap`): node id → inode. An inode
  records its path in the share (`a.txt`), its other names (see aliases
  below), and its lookup count. Repeated lookups of the same host file
  return the same node id: the table is also keyed by the host file's
  identity (its device and inode number), or by its path on host
  filesystems whose inode numbers are not stable (FAT).
- **The file table** (`files`): handle → open file. An open file records
  its inode, the host file it opened (an `LxFile`), and its open flags.

Each method looks up the numbers in the request, turns them into a path in
the share, and asks layer 4 to perform the operation on the host.

Example, reading `/mnt/a.txt`:

1. `lookup(node 1, "a.txt")`: node 1 is the root; layer 4 checks that
   `a.txt` exists and returns its attributes; the inode table gets node 5 =
   `a.txt`, lookup count 1; the reply says "node 5".
2. `open(node 5, read-only)`: layer 4 opens the host file; the file table
   gets handle 9 = node 5, read-only; the reply says "handle 9".
3. `read(handle 9, offset 0, 4096 bytes)`: reads from handle 9's host file.
4. Later, `release(handle 9)` removes handle 9, and `forget(node 5, 1)`
   drops the reference, so node 5 can be removed.

A request that names an unknown node id or handle fails with an error.

### What Pedro added

1. **The fixed microVM profile** (`5333cd695`). A snapshot must describe the
   device completely, and what the guest can make the host keep must stay
   bounded. The profile fixes:
   - the device's identity: attachment `fs:microvm0`, tag `microvm`;
   - the protocol: FUSE 7.31 or newer, writes of at most 1 MiB, direct I/O
     (no page cache in the guest), and entry and attribute cache lifetimes
     of zero, so the guest always asks again;
   - the share's root, named by its host identity (device and inode
     number on Linux), not by its path. `VirtioFs::new_microvm` checks the
     host path against that identity before and after it opens the
     directory, so the directory cannot be swapped in between; the path is
     not kept;
   - confinement: each operation checks again that the inode's path has no
     symbolic-link component; a name may not contain `\` or `:`; paths are
     bounded in length;
   - bounded tables: the inode, alias and handle tables have fixed limits,
     checked before anything is created on the host, failing with `ENOSPC`
     or `E2BIG` instead of creating state that cannot be tracked
     (`microvm/limits.rs`).
2. **Aliases** (`6e106bddc`). An inode used to record one host path, which
   went stale. Example: the guest creates a hard link `b.txt` to `a.txt`,
   then unlinks `a.txt`; the inode still pointed at `a.txt`, which no
   longer exists, although the file is still reachable as `b.txt`.
   Similarly, renaming a directory left every inode below it pointing at the
   old place. Now each inode keeps the set of paths through which the guest
   knows it (`aliases`): a lookup or a link adds one; an unlink drops the
   paths at or below the removed name from every inode, and an inode whose
   main path was removed switches to a remaining alias; a rename rewrites
   the paths at or below the old name.
3. **Denied paths, enforced by identity** (`3c673f20b`). A share often
   contains things the sandbox must not see, such as credentials next to
   the workload's data. Hiding a path by its name is not enough: the guest
   could reach the same file through a hard link, or look it up again after
   a rename. So when the share is attached, the device records the host
   identity of each denied path, and every lookup, every reopen after a
   restore, and every confined operation refuses a denied identity, or a
   path below a denied path, with `EACCES`. At most 128 denied paths, each
   canonical (no `..`, no separators, no whitespace, ...), sorted, and none
   inside another.
4. **The access policy** (`2f468534c`), which extends 3. A share has three
   sets of paths (`microvm/policy.rs`):
   - **denied** paths hide themselves and everything below them;
   - **allowed** paths show a part of a denied path again. Example: deny
     `home`, allow `home/pub`: the guest sees `home/pub` and nothing else
     in `home`;
   - **writable** paths, when a read-write share has any, are the only
     places the guest may modify; the rest is read-only.

   The nearest denied or allowed path above a path decides whether it is
   visible. A hidden directory on the way to an allowed path (`home` in the
   example) is **traverse-only**: the guest can look it up, but only as a
   directory; listing it shows only the directories that lead to allowed
   paths; any other name in it fails with `EACCES`; and neither it nor its
   entries can be modified. Enforcement:
   - every change of the namespace (`create`, `mkdir`, `mknod`, `symlink`,
     `link`, `unlink`, `rmdir`, `rename`, including `RENAME_EXCHANGE`)
     checks its directories and every path it creates, removes or replaces,
     so a writable directory cannot remove or swap away a traverse-only
     directory and expose what it hides;
   - every change of a file (`write`, opening for writing or truncation,
     `setattr`, `setxattr`, `removexattr`) requires every name the guest
     has used for that file to be writable, and fails with `EROFS` before
     the host is touched;
   - a hard link, from a writable directory, to a file the guest may not
     modify fails with `EXDEV`, because it would make the file writable
     through the new name;
   - traverse-only directories are pinned by identity too, so a rename,
     bind mount, junction or hard link on the host cannot expose their
     hidden entries under another name;
   - it also fixed `RENAME_EXCHANGE`, which swaps two names: it was treated
     as a one-way rename, so the destination's node kept a path naming the
     other object, and later policy checks used the wrong name.
5. **Symbolic links** (`0962b79c4`). Build tools need symlinks, but the
   microVM profile refused them. Now a read-write share accepts `symlink`,
   storing the target exactly as the guest gave it; a read-only share still
   fails with `EROFS`. This is safe because the host never follows a link
   while resolving a guest path (layer 4's confinement): the guest resolves
   links in its own namespace, so a target such as `/etc/passwd` or
   `../../secret` cannot reach host data outside the policy. Operations on
   a link itself are allowed, a lookup through a link is not, and an open
   that reaches a link fails with `ELOOP`.
6. **The caller's identity** (`df92e9fda`). Before, every request ran as
   OpenVMM's own user, so files a workload created were owned by OpenVMM,
   and a directory it created (owned by OpenVMM, mode 0755) refused the
   workload's own writes in it (`microsoft/nvx#297`). With `--mount-owner
   caller`:
   - each request runs as the UID and GID of its guest caller, without
     OpenVMM's supplementary groups or capabilities (layer 4);
   - guest UID 0 and GID 0 are mapped, each on its own, to the owner and
     group of the share's root, so the guest can create neither root-owned
     nor setuid-root files on the host; a share whose root is owned by UID 0
     or GID 0 is refused;
   - a request that cannot run as its caller fails with `EPERM`; it never
     falls back to OpenVMM's identity;
   - Windows has no per-request identity to switch to, so `caller` is
     refused there;
   - the mode is part of the snapshot, and a restore must use the same
     mode.
7. **Aggregates** (`ee1321dd3`, `e31c56071`). A sandbox may need several
   host directories with different modes. (Pedro first added a second
   virtio-fs slot, `9a893eafb`, then replaced it with this.) One share can
   be an **aggregate**: a synthetic, read-only root whose named children
   are host directories, each with its own mode and its own denied, allowed
   and writable paths (`--mount-aggregate`, `--mount-child
   NAME,HOST_PATH[,ro|rw]`): 1 to 256 children, with names of 1 to 64
   letters, digits, `.`, `_` or `-`; children may not overlap by path, root
   identity, or mount. A child can also be a single host file (`e31c56071`):
   its volume opens the file's parent directory but a built-in policy
   denies the directory and allows only the file's name, so the guest never
   gets a node of the directory. Inode numbers are mixed with a per-child
   key (`namespace_ino`, `inode.rs`), so two children's files do not show
   the same inode number.
8. **Snapshots** (`82776b0cf`, later `9841285ee`). Before, the device could
   not be saved: its session, the node ids and handles the guest holds, and
   its directory positions existed only in memory. Now it saves:
   - the profile and policy, the session (layer 2), and the next node id
     and handle;
   - each inode: node id, aliases (relative paths), lookup count, the inode
     number the guest saw, and the host object's identity and kind;
   - each open handle: its node, open flags, the object's identity, and,
     for a directory, a snapshot of its entries.

   It saves relative names and identities only: no host path, no open host
   file, no file contents. A directory handle takes a bounded snapshot of
   its entries at its first read and serves later reads from it, so a
   position (a "cookie") the guest holds in a listing stays valid after a
   restore even if the host directory has changed. Saving checks every
   alias and handle again (no link component, same identity, within the
   bounds). Restore checks the state against the profile, then the root's
   identity, then reopens every alias and handle and checks that each is
   still the same host object; a handle is reopened without `O_CREAT`,
   `O_EXCL` or `O_TRUNC`, and `O_TMPFILE` is refused; the session is
   restored last. `9841285ee` merged the saved state's six versions into
   one, version 1. A platform snapshot whose slot is dormant can now get an
   aggregate at restore (`23be33cd4`).
9. **Robustness, upstream** (`389e4e57a`, `#4594`; `ca6b2b177`, `#4836`).
   Allocating node ids and handles now fails cleanly (`ENOSPC`) instead of
   panicking when the counter runs out or an id is still in use, and an
   unknown node id from the guest no longer reaches an assertion. A
   `READDIRPLUS` that failed partway used to drop entries whose lookups had
   already counted a reference; the guest never forgets nodes it did not
   receive, so those inodes stayed until the session ended. Now the entries
   already gathered are returned, and the error is reported on the next
   read.

## Layer 4: host access (`lxutil`)

`vm/devices/support/fs/lxutil` (Linux: `src/unix/`; Windows:
`src/windows/`).

### What it does

`lxutil` performs the operations on the host with Linux semantics, on
Linux and on Windows hosts alike: `LxVolume` is an opened share root, and
its methods (`lookup`, `open`, `mkdir`, `rename`, `set_attr`, ...) take a
path relative to it. On Windows it maps Linux behavior (permissions,
symlinks, extended attributes) onto NTFS. Layer 3 never calls the host
directly; it always goes through `lxutil`.

Example: layer 3's `lookup(node 1, "a.txt")` becomes
`volume.lstat("a.txt")` on the share's `LxVolume`, which returns the host
file's attributes.

### What Pedro added

1. **Path confinement that never follows symbolic links** (`5515f8843`), the
   option `LxVolumeOptions::confine_paths`, which the microVM uses. A guest
   that can create symlinks could otherwise make the host follow one out of
   the share. On Linux, a confined volume opens the parent directory of
   every path with `openat2` and the flags `RESOLVE_BENEATH` (refuse
   anything outside the root) and `RESOLVE_NO_SYMLINKS` (refuse any
   symlink on the way), or, where `openat2` is missing, one `O_NOFOLLOW`
   open per path component. It then applies the operation to the last
   component through that pinned directory, without following it. `chmod`
   pins the object with `O_PATH` and refuses a link; `statfs` and the
   extended-attribute calls use the pinned directory, not a host path. So a
   rename or a new link made at the same time cannot redirect an operation
   outside the share. On Windows, a confined volume creates WSL-style
   links, which Windows never follows.

   Upstream later added a similar protection for every share, on by
   default: `29f0adceb`, "lxutil: prevent host symlink traversal" (John
   Starks, `#4764`), which uses `openat2` on Linux and keeps rename and
   hard-link operations on one opened directory handle. Pedro's branch
   has both: upstream's protection, and his `confine_paths` option on top
   (`lxutil/src/lib.rs`, `LxVolumeOptions::confine_paths`). How the two
   overlap is to check when we specify confinement.
2. **Per-thread credentials** (`df92e9fda`, `src/unix/credentials.rs`), for
   the caller's identity. Linux keeps credentials per thread, but the C
   library's `setgroups` and `set*id` change every thread of the process.
   So this module issues the raw system calls (`setfsuid`, `setfsgid`,
   `setgroups`, `capset`), which change only the calling thread: the worker
   thread takes the caller's identity, runs the operation, and restores its
   own. If it cannot take the identity, the operation is not run, and the
   result is `EPERM`.

## What this means for the spec

The properties in `CODE-OVERVIEW.md` are about virtio-fs (layers 1-4).
Layer 0, the generic virtio layer, adds properties of its own, which every
microVM virtio device shares:

- **Everything the guest can write is untrusted input**: the queues'
  descriptors and rings, the shared interrupt-status word, and the FUSE
  requests in the buffers. The device checks it before using it, and
  copies it out first where the guest could change it meanwhile.
- **Device DMA stays in guest RAM**: a microVM device reads and writes only
  the RAM ranges of the memory map (`e820`), never the shared
  interrupt-status page, the ISA hole or the MMIO aperture; it refuses a
  ring or a descriptor that lies even partly outside them, touching none of
  it, and marks itself `DEVICE_NEEDS_RESET` (change 5).
- **The shared interrupt-status word**: the device's own state never
  depends on what the guest writes into the word, only on whether it was
  zero (whether to raise the interrupt); the device only sets bits in it,
  atomically (change 4).
- **Quiescing input**: after a device's `quiesce_input` returns `Ok`, it
  takes no new request from its queues until `resume_input`: each queue's
  index of taken requests does not move. A contract of the `VirtioDevice`
  trait, stated once at layer 0; virtio-fs meets it with its admission gate
  (layer 1, change 3), which needs no spec of its own, only a proof that it
  meets this contract. Before change 2,
  every virtio device returned `Ok` without stopping anything. This is the
  same property as the snapshot spec's open item 1.1
  (`snapshot-spec-docs/OPEN-ITEMS.md` on the snapshot branch, "host input
  is gated during the boundary"), seen from the device: one property,
  tracked there for the snapshot spec.

The virtio-fs properties map onto these additions:

- **Confinement**: layer 4's `confine_paths`, and layer 3's checks that no
  path has a link component and that the root is the right object.
- **Policy**: layer 3's denied, allowed and writable paths, enforced by
  identity, with the traverse-only rules and the checks on every namespace
  and file change.
- **Identity**: layer 3's caller mode and root mapping, on layer 4's
  per-thread credentials.
- **Limits**: layer 1's size bound and layer 3's table bounds.
- **Snapshots**: layer 1's admission gate, layer 2's session state, and
  layer 3's saved state and restore checks.

The host-side facts each rests on (what `openat2` with these flags refuses,
what the raw credential calls change) are trusted facts about the host:
group D1 in `snapshot-spec-docs/LESSONS.md`.

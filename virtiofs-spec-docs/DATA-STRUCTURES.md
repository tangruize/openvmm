# virtio-fs: the key data structures

The structures that layer 3 (`VirtioFs`) keeps, in the order that makes
each one easy to follow, with one running example: the guest runs `cat
/mnt/work/a.txt`, on a share of the host directory `/srv/data`. The code is
Pedro Henrique Penna's, as on this branch (`feature/microvm/dev` at
`db42fde04`). Words such as "node id" and "lookup count" are introduced
where they first appear.

Where everything hangs:

```rust
pub struct VirtioFs {
    inner: Arc<VirtioFsInner>,
}

struct VirtioFsInner {
    inodes: RwLock<InodeMap>,                        // 1. the inode table
    files: RwLock<HandleMap<Arc<VirtioFsFile>>>,     // 3. the file table
    mode: VirtioFsMode,                              // a single share, or an aggregate
    microvm_profile: Option<MicroVmVirtioFsProfile>, // the microVM's fixed settings
    caller_identity: Option<microvm::owner::CallerIdentity>, // caller mode
    negotiation: RwLock<FuseNegotiation>,            // what FUSE_INIT agreed
}
```

## 1. The inode table (`InodeMap`, `virtiofs/src/lib.rs`)

**What it is for.** The guest refers to files by number. The inode table
is how OpenVMM turns a number into its record of the file (an inode,
section 2).

```rust
struct InodeMap {
    inodes_by_node_id: HandleMap<Arc<VirtioFsInode>>,            // node id -> inode
    inodes_by_key: HashMap<DedupKey, (Arc<VirtioFsInode>, u64)>, // host file -> (inode, node id)
    ...
}
```

- **A node id** is the number OpenVMM gives the guest for one file or
  directory, in its reply to a lookup ("`a.txt` is node 3"); the guest sends
  it back in later requests ("open node 3"). Node 1 is always the share's
  root.
- **`inodes_by_node_id`** maps each node id that the guest may use to the
  inode.
- **`inodes_by_key`** goes the other way, from a host file (by its device and
  inode number on the host) to its inode and node id, so that one host file
  gets one node id even if the guest reaches it under two names.

**Numbers are handed out by a counter and never reused.** `HandleMap` is a
`HashMap<u64, T>` plus the next number (`next_handle`). Nodes are also
removed (section 2, the lookup count), so a `Vec` would fill with holes; and
a number is never given to another file, so a stale number from the guest
is simply not found.

**The table has one lock**, `inodes: RwLock<InodeMap>`: worker A (the
request queue) adds entries, and worker B (the high-priority queue, for
`FORGET`) removes them.

Example:

1. At the start: `1 -> the root ("")`.
2. The guest looks up `work` in node 1: `2 -> "work"` is added.
3. The guest looks up `a.txt` in node 2: `3 -> "work/a.txt"` is added.
4. If the guest later reaches the same host file under another name (a hard
   link `work/b.txt`), `inodes_by_key` finds that it already has node 3: the
   guest gets node 3 again, and `work/b.txt` is added to node 3's names. No
   node 4 is made.

## 2. An inode (`VirtioFsInode`, `virtiofs/src/inode.rs`)

**What it is.** OpenVMM's record of one file or directory that the guest
knows, one per host file. It is always held through an `Arc` once it is in
the table: by the table, by each open file on it, and by each request that
uses it, so that it lives until the last of them lets go.

```rust
pub struct VirtioFsInode {
    pub(crate) volume: Arc<VirtioFsVolume>,        // which share it is in (5)
    pub(crate) path: RwLock<PathBuf>,              // its main path in the share
    pub(crate) aliases: RwLock<BTreeSet<PathBuf>>, // every name the guest used
    pub(crate) lookup_count: AtomicU64,            // node ids handed to the guest
    inode_nr: lx::ino_t,                           // its inode number on the host
    file_type: lx::mode_t,                         // file, directory, symlink, ...
    guest_inode_nr: lx::ino_t,                     // the inode number the guest sees
}
```

- **Fixed for the record's life**: `volume` (which gives the policy),
  `inode_nr` (its identity on the host), `file_type`, and `guest_inode_nr`
  (for an aggregate, mixed with the child's key, so that two children's files
  do not show the same number).
- **Changing**, so protected:
  - `path`, behind a lock: the path OpenVMM uses for host calls;
  - `aliases`, behind a lock: every name through which the guest reached
    it;
  - `lookup_count`, an atomic counter.

**Why the path and the aliases have locks.** An operation on one file can
change other inodes' fields. After a `rename docs papers`, `rename` takes
the table's lock and rewrites, in every inode of the share, the path and the
aliases at or below `docs` (`InodeMap::rename_alias_prefix`,
`VirtioFsInode::rename_alias_prefix`): `docs/report.txt` becomes
`papers/report.txt`. After an `unlink` or `rmdir` of `work/a.txt`, it
removes the aliases at or below `work/a.txt` from every inode, and an inode
whose main path was removed switches to one of its remaining aliases
(`remove_alias_prefix`). Also, the inode is shared through `Arc`, so everyone
holds `&VirtioFsInode`, and Rust lets a field change through a shared
reference only if it is protected (a lock or an atomic). In the microVM,
only worker A changes paths and aliases, so these locks are never contended
there; upstream OpenVMM, with several request workers, needs them.

**The lookup count.** How many times OpenVMM has handed this node id to the
guest. It goes up by one for each reply that gives the guest the node id: an
entry from `lookup`, `create`, `mkdir`, `mknod`, `symlink`, `link`, and each
entry of a `READDIRPLUS`. `open`, `read` and `write` do not change it (`open`
gives a handle, not a node id). The guest's kernel keeps the same count, and
sends `FORGET(node, n)` to give back `n` of them. `FORGET` (worker B,
`VirtioFs::forget`), while holding the table's lock for writing:

1. lowers the count by `n`;
2. if it reaches 0, removes the node from the table (both its entries); from
   then on, a request naming the node fails with `EINVAL`, and the number is
   not reused;
3. the record itself is freed once nothing else holds it (no open file, no
   running request).

`FORGET` changes nothing on the host, and no path or alias; it does not
affect open files or running requests, which hold their own `Arc`. A
request that names a node (`setattr`, `unlink` in a directory) starts with
`get_inode`, which finds the node under the table's lock (so a `FORGET`
cannot remove it in the middle) and returns an `Arc` (which keeps it alive
afterwards). If the guest needs the file again after a `FORGET`, it looks it
up again and gets a new node.

Example, node 3 after the lookup of `work/a.txt`:

```
volume:         the share /srv/data (policy: writable "work")
path:           work/a.txt
aliases:        {work/a.txt}
lookup_count:   1
inode_nr:       4711 (on the host)
file_type:      regular file
guest_inode_nr: 4711 (the same, for a single share)
```

After a hard link `work/b.txt` to it: `aliases` is `{work/a.txt,
work/b.txt}` and `lookup_count` is 2; `path` stays `work/a.txt`.

## 3. The file table (`files`, `virtiofs/src/lib.rs`)

**What it is for.** When the guest opens a file, it gets a handle number;
later reads and writes name the handle. The file table turns a handle into
the open file (section 4).

```rust
files: RwLock<HandleMap<Arc<VirtioFsFile>>>,    // handle -> open file
```

The same `HandleMap` as the inode table: handles start at 1 and are never
reused; behind one lock.

- `open`, `opendir` and `create` (which creates the file and opens it) add a
  handle;
- `release` and `releasedir` remove it (`remove_file`), when the guest
  closes the file;
- `read`, `write`, `readdir`, `fsync` and the like use a handle without
  changing the table.

There is no count: one open gives one handle, one release removes it.
`FORGET` does not touch this table.

Example: `open(node 3, read-only)` adds `1 -> the open file of node 3,
read-only`, and the reply says "handle 1"; `read(handle 1, offset 0, 4096
bytes)` reads through it; `release(handle 1)` removes it.

## 4. An open file (`VirtioFsFile`, `virtiofs/src/file.rs`)

**What it is.** OpenVMM's record of one file or directory that the guest
has open, one per handle.

```rust
pub struct VirtioFsFile {
    pub(crate) file: RwLock<LxFile>,                          // the open host file
    pub(crate) inode: Arc<VirtioFsInode>,                     // which file it is
    open_flags: u32,                                          // how it was opened
    pub(crate) directory_snapshot: RwLock<DirectorySnapshot>, // for a directory
}
```

- **`file`**: the open file on the host (an `LxFile`, essentially a host file
  descriptor). Reads and writes go straight to it (`read` calls `pread`,
  `write` calls `pwrite`), not through a path, so a later rename does not
  change which host file they reach.
- **`inode`**: the inode it was opened on, which it keeps alive while the
  file is open.
- **`open_flags`**: how the guest opened it (read-only, for writing, ...),
  fixed for the file's life. The `writable` field of a handle's view comes
  from it.
- **`directory_snapshot`**: for a directory only, a copy of its entries taken
  at the first read, from which later reads of the listing are served, so
  that a position the guest holds in the listing stays valid across a
  snapshot and restore. Behind a lock, because the first read fills it in.

Example, handle 1 after `open(node 3, read-only)`: `file` is the host file
`/srv/data/work/a.txt`, open read-only; `inode` is node 3's; `open_flags` is
read-only; `directory_snapshot` is empty.

## 5. The volume (`VirtioFsVolume` and `LxVolume`)

Two structures, one inside the other.

**`VirtioFsVolume`** (`virtiofs/src/inode.rs`): virtio-fs's record of one
share, or of one child of an aggregate.

```rust
pub(crate) struct VirtioFsVolume {
    pub(crate) volume: Arc<LxVolume>,         // layer 4: the opened share root
    pub(crate) id: u32,                       // which share (0 for a single share)
    pub(crate) readonly: bool,                // a read-only share?
    stable_file_ids: bool,                    // stable inode numbers on the host?
    pub(crate) strict_paths: bool,            // the microVM's confinement checks on?
    pub(crate) policy: SubtreePolicy,         // the access policy
    pub(crate) pinned_identities: HashMap<(u64, u64), Option<PathBuf>>,
                                              // the hidden objects, by host identity
    pub(crate) exposed_file: Option<PathBuf>, // for a single-file aggregate child
}
```

All its fields are fixed once the share is set up, so it has no locks.
Every inode of the share holds an `Arc` to it; that is how an inode finds its
policy. Its methods only answer questions (`id`, `readonly`,
`path_writable`, `ensure_path_allowed`, `ensure_object_allowed`,
`entry_listable`, ...): none changes the host.

**`LxVolume`** (`lxutil/src/lib.rs`): `lxutil`'s object for the share's root
directory on the host. On Linux it holds an open handle to that directory and
the volume's options (owner, permission masks, confinement). Its methods
(`lstat`, `open`, `mkdir`, `unlink`, `rename`, ...) take a path relative to the
root and call the host. Its fields are fixed too.

**How they relate.** `VirtioFsVolume.volume` is the `LxVolume`, and `Deref`
lets the inode call `self.volume.unlink(path)` as if `VirtioFsVolume` had the
method. For example, `unlink`:

```
lib.rs:    VirtioFs::unlink_helper         checks the path against the policy
inode.rs:    VirtioFsInode::unlink(name)   computes the path again
             self.volume.unlink(path)      Deref passes the call through
lxutil:        LxVolume::unlink(path)      calls the host
```

**Order of creation.** `VirtioFsVolume::new(volume: LxVolume, ...)` receives
an `LxVolume` that already exists, and starts with an empty policy
(`SubtreePolicy::default()`); the microVM's setup attaches the real policy
afterwards (`microvm/inode.rs`, `new_with_strict_paths`). So the `LxVolume`
exists before the policy is known.

## How the guest's requests change these structures

| Request | Inode table | Inode | File table |
|---|---|---|---|
| `lookup` | adds a node, or finds the existing one | adds a name; count + 1 | |
| `create` | adds a node | count + 1 | adds a handle |
| `mkdir`, `mknod`, `symlink` | adds a node | count + 1 | |
| `link` | | adds a name; count + 1 | |
| `unlink`, `rmdir` | (under its lock) | removes names at or below the path, in every inode of the share | |
| `rename` | (under its lock) | rewrites names at or below the old path, in every inode of the share | |
| `open`, `opendir` | | | adds a handle |
| `release`, `releasedir` | | | removes the handle |
| `FORGET` | removes the node when its count reaches 0 | count - n | |
| `read`, `write`, `getattr`, `readdir`, ... | | | |

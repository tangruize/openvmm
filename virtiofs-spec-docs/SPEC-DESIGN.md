# virtio-fs spec: design, for the proof

The Verus specification of the microVM's virtio-fs device, as it stands,
for whoever proves the production code against it. It states only the
current design. Start with `OVERVIEW.md` for the big picture. The history,
the alternatives and the reasons in full are in `CODE-OVERVIEW.md`; the
changes to the executable code are in `CODE-CHANGES.md`; bugs found and
questions for Pedro are in `BUGS.md`. Style rules:
`snapshot-spec-docs/CONVENTIONS.md`.

The code is Pedro Henrique Penna's, on `feature/microvm/dev` at
`db42fde04`, which builds on OpenVMM's existing virtio-fs. This branch
(`system-proof/virtiofs-spec-20261108`) adds the spec, the code changes
listed in `CODE-CHANGES.md`, and these documents.

## 1. What is specified

The spec protects the host from the guest, which may send anything. It
states, at the FUSE level (the `Fuse` trait's contracts):

1. **Confinement**: every host object the device touches is inside the
   share (sections 7 and 8).
2. **Policy: integrity and confidentiality.** Integrity: a request that
   changes the host (a file's bytes or attributes, a name created, removed
   or renamed) succeeds only through names the guest may write, all of
   them. Confidentiality: what the guest gets back (a node, attributes, a
   directory entry, a file's bytes) is about a path the guest may see.
3. **Identity**: each request runs as its caller, mapped so that it is
   never host root, or as OpenVMM in the default owner mode (section 9).
4. **Limits**: in the microVM, the device's tables stay within their caps
   (section 10).

And for every operation, **when it fails**: its error rules, each a
condition and its code (section 3).

Not specified:

5. **File contents and attributes**, by decision, since they are not about
   security: what `read` returns and `write` changes in a file's bytes,
   the attributes `getattr` reports, extended attributes' values, a
   listing's full contents. (When each operation succeeds or fails, and how
   it changes the guest's names, nodes and handles, is specified.)
6. **Snapshots**: virtio-fs's own save and restore, stopped by decision
   until this spec is verified (section 13).

**The highest-level guarantee: names, then objects.** The guest names host
objects by paths relative to the share. The policy is about names: an
operation succeeds only through names the policy allows. Confinement ties
names to objects: virtio-fs hands `lxutil` only relative paths of plain
names (checked), `lxutil` and the host's kernel resolve each beneath the
share's root (trusted), and the host model (section 7) says which object
each name reaches. The policy alone would not be enough: if the volume
followed a host link `logs/gateway` -> `/etc`, a read of the visible name
`logs/gateway/passwd` would meet every policy rule and still read the
host's `/etc/passwd`.

**Host assumptions.** The spec holds only if:

- no other host process changes the share while the device runs;
- a share contains no mount point (on Linux; on Windows mount points are
  reparse points, refused, section 8);
- no two volumes share a host directory;
- an object's inode number is the same on every call (always on Linux; on
  Windows, a share's filesystem must report stable file ids);
- on Windows, paths that differ name for name are different objects: no
  name reaches an object in another case (`SECRET` for `secret`) or by an
  8.3 short name (`SECRE~1`). lxutil opens every path case-insensitively,
  so this holds only for directories that are case-sensitive and volumes
  without short names (a question for Pedro, `BUGS.md`);
- as the share was attached: no host hard link joined a path the guest may
  write and a visible path it may not (Pedro's setup rule), a directory had
  one path, and the share was a tree. These are about what the disk held,
  not how the host behaves, so they are an axiom (`attached_disk`, section
  7); the others are built into `lxutil`'s trusted contracts.

**One request at a time.** The microVM runs one request at a time: two
queues, but both workers are tasks on one thread, and a request runs to
its end. The code still guards its tables with `RwLock`s, and the spec
follows the locks, each with a rule (section 5), rather than the order.
Only the keeper relies on the order (section 6).

**Every byte the guest writes is untrusted.** No contract requires
anything of a request's contents (names, flags, node ids, handles): an
operation must meet its contract whatever the guest sends.

## 2. The files

| File | What it holds |
|---|---|
| `vm/devices/support/fs/fuse/src/lib.rs` | the `Fuse<G>` trait: the 44 operations, each with its contract |
| `vm/devices/support/fs/fuse/src/lib.spec.rs` | `FuseGhost`, `FuseOwns`, `FuseState` (questions and rules), `ListedEntry` |
| `vm/devices/support/fs/fuse/src/session.rs`, `session.spec.rs` | `Session<G>` and `dispatch`'s contract (the ghost state and the credential token); `owns_ghost` |
| `vm/devices/support/fs/fuse/src/request.spec.rs` | `RequestView`: the node id, opcode, uid and gid |
| `vm/devices/support/fs/fuse/src/protocol.rs` | the FUSE structs and constants, declared to Verus |
| `vm/devices/support/fs/lx/src/string.spec.rs`, `error.spec.rs` | `LxStr`'s view (its bytes); `Error::code` |
| `vm/devices/support/fs/lx/src/cred_t.rs` | the thread's credential token, `ThreadCred` (TRUSTED, section 9) |
| `vm/devices/support/fs/lx/src/lib.rs` | `Stat`, `StatEx`, `Timespec` and the `O_*`, `S_IF*`, `DT_*` constants, visible to Verus |
| `vm/devices/support/fs/lxutil/src/lib.spec.rs` | `LxVolume::is_share_path`, `host_name` (Windows escaping), and the trusted text: confinement and the host assumptions |
| `vm/devices/support/fs/lxutil/src/host_t.rs` | the host model (TRUSTED, section 7): `HostDisk`, `VolumeDisk`, the axiom `attached_disk` |
| `vm/devices/support/fs/lxutil/src/lib.rs` | the contracts of the 16 `LxVolume` methods virtio-fs calls, of `LxFile::fstat` and of `LxVolumeOptions::new_volume`, in the host model's terms |
| `vm/devices/support/fs/lxutil/src/unix/credentials.rs` | the contracts of `switch_fs_identity` and `FsIdentityGuard::restore` (TRUSTED, section 9) |
| `vm/devices/virtio/virtiofs/src/lib.spec.rs` | virtio-fs's ghost state and its view, `wf`, `owns`, the table locks' rules, and virtio-fs's answers to `FuseState` |
| `vm/devices/virtio/virtiofs/src/inode.spec.rs` | the path and aliases locks' rules; `VirtioFsInode`'s view; `VirtioFsVolume::share_view`, `pinned_view` |
| `vm/devices/virtio/virtiofs/src/aggregate.spec.rs` | the aggregate registry lock's rule |
| `vm/devices/virtio/virtiofs/src/fs_ghost_t.rs` | the keeper (TRUSTED, section 6) |
| `vm/devices/virtio/virtiofs/src/microvm/policy.spec.rs` | the access policy's view (`SubtreePolicyView`), `can_write`, `visibility`, `hides_below` |
| `vm/devices/virtio/virtiofs/src/microvm/profile.spec.rs` | which host object a profile's root identity names |
| `vm/devices/virtio/virtiofs/src/microvm/owner.rs`, `owner.spec.rs` | the microVM's `dispatch`, `dispatch_with`, `SwitchIdentity`; property 3 (`CallerIdentity::runs_as`) |
| `vm/devices/virtio/virtiofs/src/microvm/fs.rs` | `VirtioFs::new_microvm`'s contract |

**Commands** (from the repository root; the memory cap keeps the machine
usable):

```
systemd-run --user --scope -p MemoryMax=12G --quiet env CARGO_BUILD_JOBS=4 \
    scripts/verify-snapshot-spec.sh lx lxutil fuse virtiofs
systemd-run --user --scope -p MemoryMax=12G --quiet env CARGO_BUILD_JOBS=4 \
    cargo test -p lx -p lxutil -p fuse -p virtiofs
```

Verus must be the patched build described in `verus-patches/README.md`
(Verus `426d8b0` with patches `0001`-`0005`; `vstd` from
`verus-patches/vstd`, with patches `vstd-0001`-`vstd-0005`).

**Spec files and the normal build.** Some spec files are included in every
build, because a normal build needs their types (a ghost struct is a type
parameter of a lock, or a field of the ghost state):
`fuse/src/lib.spec.rs`, `virtiofs/src/lib.spec.rs`, `inode.spec.rs`,
`aggregate.spec.rs`, `microvm/policy.spec.rs`, `lxutil/src/host_t.rs`. The
others are under `cfg(verus_keep_ghost)`.

## 3. The FUSE level

### The trait

```rust
pub trait Fuse<G: FuseGhost>: Send + Sync + FuseOwns<G>
where
    G::V: FuseState,
```

`G` is the file system's ghost state. The `fuse` crate knows nothing of
it but three traits:

- `FuseGhost::wf(&self)`: `G` is well formed (its view's `inv()` and the
  consistency of what the view does not show).
- `FuseOwns::owns(&self, g)`: `g` is this file system's.
- `FuseState`, on `G::V`: the questions and rules below.

`G = ()` is the ghost state of a file system with none (the crate's
tests): it owns `()`, which is always well formed, and its `FuseState`
answers permit nothing.

### The ghost argument

Each operation takes the ghost state as a `verus_spec` `with` argument:

- `Tracked<&mut G>` for the 16 operations that change the state (`lookup`,
  `forget`, `create`, `mkdir`, `mknod`, `symlink`, `link`, `unlink`,
  `rmdir`, `rename`, `open`, `open_dir`, `release`, `release_dir`,
  `read_dir_plus`, `destroy`);
- `Tracked<&G>` for the 26 that only read it;
- none for `init` and `remove_mapping`.

Every operation that takes `g` requires `self.owns(g)` and `g.wf()`; the
`&mut` ones ensure both of `final(g)`. Example, `write`:

```rust
with Tracked(g): Tracked<&G>
requires
    self.owns(g),
    g.wf(),
ensures
    r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh) && g@.opened_for_writing(_arg.fh) && g@.file_can_write(_arg.fh),
    G::V::fails_with(r, g@.write_errors(_request@.node_id, _arg.fh, _data@.len())),
```

`Session::dispatch` takes `g`, and also the thread's credential token and
the identity the request must run as (section 9).

### Questions and rules

`FuseState` (`fuse/src/lib.spec.rs`) has two kinds of spec function:

- A **question** is declared without a body; each file system answers it.
  Example: `holds_node(n)`, "the guest holds node `n`".
- A **rule** has its body in the trait, built from questions, the same for
  every file system. Example: `is_entry(parent, name)`.

The trait is in sections: the state's invariant; questions about nodes and
paths, about handles, about how operations change the state, about what
the file system refuses (including the caps: `no_room_for_node`,
`no_room_for_name`, `no_room_for_handle`); then rules about names and
flags, about when an operation may succeed, and about when it fails.

The rules the success clauses lean on most:

- `uses_node(n)`: the guest holds `n`, and none of its names is hidden now
  (`names_visible`). The code re-checks a node's names before every
  operation on it (`get_inode` -> `validate_confined_object`).
- `uses_handle(n, fh)`: `uses_node(n)`, the guest holds `fh`, and `fh` is
  `n`'s.
- `is_entry(parent, name)`: `parent` is a directory whose names are
  visible, and `name` is one plain name.
- `may_add_name(parent, name)`: `is_entry`, `parent` may be changed, and
  the new name's path is writable.

A path in the spec is in the host's form: `child_path` pushes
`LxVolume::host_name(name)`, the name itself on Linux, lxutil's escaping on
Windows.

### Success clauses

A success clause says what holds if the operation succeeds, and what the
state is after it. For an `&mut` operation, an error leaves the state
unchanged (`Err(_) => final(g)@ == old(g)@`), with one exception:
`read_dir_plus` keeps the lookups it did before an error
(`looked_up_in`).

State changes are exact: `final(g)@ == s.after_reach(n, p, m, kind)`,
`s.unlinked(n, p, final(g)@)`, `s.renamed(n, p, q, exchange, final(g)@)`,
`s.after_open(fh, n, writable)`, and so on. The proof must make the ghost
state follow the code exactly. A rename of a path onto itself leaves the
state as it is.

Some clauses state more than a state change. For example, `create` that
opens an existing node the guest holds, with names, for writing or
truncating, needs every name of it writable (bug 3's rule); `link` needs
its target to have a name (`has_names`); `rename` needs no hidden path
below what it moves (`hides_below`).

### Error rules

Each operation's rules are a set of codes (`*_errors`, an `ISet<i32>`):
each rule is a condition and its code, and the set holds the codes of the
rules that apply. The contract says, for example,

```rust
G::V::fails_with(r, old(g)@.mkdir_errors(_request@.node_id, _name@)),
```

and `fails_with` means: if any rule applies, the operation fails, with the
code of one of the rules that apply, or with a host error (`host_error`).
The order in which the code checks does not matter. virtio-fs answers
`host_error` "any code" (the host model does not say which errors the host
gives), so what the proof must show is that the operation fails whenever a
rule applies.

Shared sets: `node_errors`, `handle_errors`, `entry_errors`, and
`hidden_names` (a held node with a hidden name: `EACCES`). Error codes are
compared as numbers: `e.code() == lx::EROFS` (`lx/src/error.spec.rs`).

`release` and `release_dir` always succeed (`r is Ok`). `forget`,
`batch_forget` and `destroy` return nothing. The operations virtio-fs
leaves to the trait's default body (which returns `ENOSYS`) have no error
rules.

### Listings

`read_dir` and `read_dir_plus` say `lists_only_allowed(dir, buf, plus)`:
the reply is the encoding of some entries, each `.`, `..`, or one that
`may_list` allows now. The encoding is `ListedEntry::records`
(`fuse/src/lib.spec.rs`), read off `DirEntryWriter` (`reply.rs`): for each
entry, in a `read_dir_plus` reply the lookup's reply (`fuse_entry_out`,
128 bytes), then the header (`fuse_dirent`: inode number, offset, name
length, type, little-endian), the name, and zeros up to a multiple of 8.

## 4. virtio-fs's ghost state

### The ghost state and its view

`VirtioFsGhost` (`virtiofs/src/lib.spec.rs`) is the bundle that a request
holds:

```rust
pub tracked struct VirtioFsGhost {
    pub tracked inodes: FracGhost<InodeTable>,          // half of the ghost inode table
    pub tracked paths: GhostMapAuth<int, PathView>,     // every inode's main path, by key
    pub tracked aliases: GhostMapAuth<int, Set<PathView>>, // every inode's names, by key
    pub tracked files: FracGhost<Map<u64, VirtioFsFileView>>, // half of the ghost file table
    pub ghost next_key: int,
    pub ghost microvm: bool,
    pub ghost shares: Map<Seq<u8>, VirtioFsShareView>,
    pub tracked disks: Map<u32, lxutil::host::VolumeDisk>,  // each volume's disk (section 7)
}
```

Its view, `VirtioFsGhostView`, is what the contracts read: by node id, the
inodes (`VirtioFsInodeView`: volume, kind, read-only flag, policy, exposed
file, pinned objects), their main paths and their names; the open handles
(`VirtioFsFileView`: node, inode view, writable); the flags `aggregate` and
`microvm`; and the `shares`. The disks are not in the view: the host may
change in an operation that then fails, and an error promises that the
view is unchanged.

The ghost inode table (`InodeTable`) records, beside the inodes' views:
each node's ghost key (`keys`, below), each node's host object by its
inode number (`host_inos`), each volume's `LxVolume` (`lx_volumes`), the
ids of the `paths` and `aliases` maps, and the aggregate flag.

### Hidden keys

Inside `VirtioFsGhost`, `paths` and `aliases` are keyed by a per-inode
**ghost key**, not by node id. An inode's locks are created, with their
rules, in `VirtioFsInode::with_attr`, before the inode table gives it a
node id, and a lock's rule never changes; so each rule names its entry by
a key fixed at creation (`PathRule.key`).

- Keys come from the counter `next_key`: a new inode takes it, and it
  grows. Keys are never reused.
- The ghost inode table records each node's key (`InodeTable.keys`).
- The view is computed through it (`impl View for VirtioFsGhost`): node
  `n`'s path is the entry of `n`'s key.

So entries of no node's key are not seen: an inode built and then dropped
(`InodeMap::insert` drops a new inode when the table already has the same
host file), or a forgotten one. The contracts therefore read as if the
maps were keyed by node id.

### `inv`, `wf`, `owns`

- `inv` (on the view):
  - every table node but the root has a main path and names, its main path
    among its names when it has any;
  - every main path and name, and every share's root path, is a share path
    (`LxVolume::is_share_path`, which each `lxutil` method requires);
  - an aggregate's shares have plain names;
  - in the microVM, the caps hold (`within_limits`, section 10).
- `wf` (on `VirtioFsGhost`):
  - `inv`; the recorded map ids are the maps'; every table node but the
    root has a key with entries in both maps; no two such nodes share a
    key; every key in use is below `next_key`;
  - the host model's facts (section 7): each volume's disk is its
    `LxVolume`'s; every name of a node names, on its volume's disk, the
    node's object; a pinned object has a name only at its own path, and
    stays on the disk; no object has both a writable path and a visible
    read-only one; each disk is a tree; a directory has one path; at each
    traverse-only path, the disk holds a directory.
- `owns` (on `VirtioFs`): `g`'s two table halves are the other halves of
  those this file system's table locks hold (the ids their rules pin); `g`
  records whether the file system is a microVM's and an aggregate; an
  aggregate's shares are its registry's (microVM), and a direct share has
  none; and the share's root is the intended one (section 7).

`DormantMicrovmFs` owns the empty ghost state; `SectionFs` (Windows only)
owns any (`true`: it is not part of the microVM).

### The aggregate's synthetic root

An aggregate's node 1 is made up: its entries are the shares, and it is
not in the inode table. The FUSE contracts treat it through questions:
`is_synthetic(n)` and `is_synthetic_handle(fh)`. virtio-fs's answers have
two cases, node 1 of an aggregate and every other node:

- node 1 is held, a directory, never writable;
- `may_reach(1, p, kind)`: `p` is one share's name (and a single-file
  share's root is a regular file);
- `after_reach(1, ...)`: the node reached is that share's root, in that
  share, with the root's path there (`""`, or the exposed file), not
  `[name]`;
- node 1's own handle (`SYNTHETIC_ROOT_FH`, `u64::MAX`), which `open_dir`
  hands out every time, is held, and its node is 1 (the file table never
  hands out `u64::MAX`: bug 7);
- `open_dir`'s success clause has a case for it: its own handle, nothing
  changes.

The shares are tied to the code by the registry lock's rule (section 5),
for a microVM aggregate, whose children never change. A non-microVM
aggregate (children added and removed by `add_child`/`remove_child`
outside any request) is not covered: its root's contracts cannot be
proved.

## 5. The locks

Each lock that holds guest state is vstd's verified `RwLock<V, Pred>`,
with its rule (`Pred`) given at creation and never changed. vstd's locks
replace `parking_lot`'s for this (a change for verification only,
`CODE-CHANGES.md`). There are five:

| Lock (field) | Holds | Rule | Rule file |
|---|---|---|---|
| `VirtioFsInode::path` | `InodePath`: the main path, and its `paths` entry (`GhostPointsTo`) | `PathRule { key, map_id, is_root }` | `inode.spec.rs` |
| `VirtioFsInode::aliases` | `InodeAliases`: the names, and their `aliases` entry | `AliasesRule { key, map_id, is_root }` | `inode.spec.rs` |
| `VirtioFsInner::inodes` | `InodeTableLocked`: the `InodeMap`, and the lock's half of the ghost inode table | `InodeTableRule { id }` | `lib.spec.rs` |
| `VirtioFsInner::files` | `FileTableLocked`: the `HandleMap`, and the lock's half of the ghost file table | `FileTableRule { id }` | `lib.spec.rs` |
| `AggregateState::registry` | `AggregateRegistry`: the children | `RegistryRule { fixed, shares }` | `aggregate.spec.rs` |

The other locks are still `parking_lot` and have no rule: each open file's
`file` and `directory_snapshot`, and the `negotiation`. No contract needs
them.

**Each rule has a SPEC core and a TODO part.** The core is part of the
spec; the proof must keep it. Each rule's comment marks both.

- `PathRule` core: the piece is an entry of the `paths` map
  (`piece.id() == map_id`), this inode's (`piece.key() == key`), and equals
  the real path (`piece.value() == v.path@`). For the root: the path is
  empty and there is no piece.
- `AliasesRule` core: the same for the names (`piece.value()` is the set of
  the real names' views). For the root: its only name is the empty path.
- `InodeTableRule` core: the half is the ghost inode table's, its inodes
  are the real table's inode views, its aggregate flag is the real
  table's, and each node's host object (`host_inos`) and its volume's
  `LxVolume` (`lx_volumes`) are the real inode's (`host_ino`,
  `lx_volume`).
- `FileTableRule` core: the half is the ghost file table's, it has the
  real table's handles, and each handle's inode view and writability (from
  its open flags) are the real file's.
- `RegistryRule` core: with `fixed`, the children are `shares`, by name
  (`encode_utf8` of each child's name).

**TODO for the proof**, in the rules: the cross-lock links (each table
inode's own locks' rules have the key recorded in `InodeTable.keys` and
the maps' ids; the path and aliases keys are the same; each handle's
recorded node is one whose inode is the file's, while the node is in the
table); the dedup index (`inodes_by_key`) against `host_inos`: a key
`Ino(v, ino)` finds the node of that host object; and that each table lock
holds exactly a half. The proof may add facts and fields to any rule.

**How the ghost state follows the code.** When the code changes data under
a lock, the proof changes the ghost piece in it at the same moment, with
the whole-map piece in `g`, before the lock is released (when Verus checks
the rule):

- `FracGhost` (the two tables): one half in the lock, one in `g`; both
  halves together change the value.
- `GhostMapAuth` + `GhostPointsTo` (paths, names): the `Auth` in `g`, each
  inode's entry in its lock; `insert` makes an entry, `agree` reads it
  against the `Auth`, `update` changes it.

**PLACEHOLDERs.** Where a lock is created, the ghost piece and the rule
are `Tracked::assume_new()` and `Ghost::assume_new()`; the proof replaces
each with the real piece and rule:

- `VirtioFsInode::with_attr`: the path and aliases pieces (fresh entries
  at key `next_key`) and rules;
- `InodeTableLocked::new_lock` and `FileTableLocked::new_lock`: the halves
  and rules;
- `AggregateState::new` and `with_children`: the registry rule.

These need a ghost argument to reach `g` (or the pieces made with it).
Adding ghost arguments is fine: they change no executable behaviour.

**Release is explicit.** vstd's handles are released by `release_write` /
`release_read`, not when dropped. Every critical section ends with one,
including those that return early: they compute their result, release,
then return.

## 6. The keeper

Between requests the ghost state `g` is kept by trusted axioms
(`virtiofs/src/fs_ghost_t.rs`; proof functions, nothing at runtime):

- `keep_fs_ghost(fs, g)`, once, when the file system is created;
- `take_fs_ghost(session) -> g`, at the start of each request;
- `give_back_fs_ghost(session, g)`, at its end;
- `take_thread_cred() -> cred` and `give_back_thread_cred(cred)`: the
  device thread's credential token (section 9), its own (`Own`) between
  requests.

A request lends `g` and the token down: `process_virtiofs_request` ->
`owner::dispatch` -> `owner::dispatch_with` -> `Session::dispatch` -> the
handler. The dispatchers' contracts require and ensure `owns_ghost(g)` and
`g.wf()`, and that the token is `Own` before and after.

Assumptions (not checked):

- **A1.** A take is followed by its give-back before the next take. True
  because requests never overlap in the microVM: both queue workers are
  tasks on one thread (`driver_source.simple()`), and
  `process_virtiofs_request` has no `await`.
- **A2.** Only `process_virtiofs_request` takes and gives back, and the
  keeper holds one ghost state per file system. Not yet true across a
  restore, which replaces both tables but not `g` (section 13).

**Creating `g`.** `VirtioFs::new_microvm`'s contract demands, on success,
a `g` that the new file system owns, well formed, and new (`is_new`: the
root alone, no names, no handles). Its proof creates the maps and the
table halves, puts one half of each into its lock, takes each volume's
disk from `new_volume`, calls `attached_disk` for it (section 7), and
hands the rest out; the device's setup then calls `keep_fs_ghost`.

## 7. The host model

`lxutil/src/host_t.rs` (TRUSTED; in every build, since `VolumeDisk` is a
field of `g`) describes what each share holds on the host's disk:

- `HostObject { dev, ino, kind }`: one object, by its device and inode
  numbers, and its file type;
- `HostDisk { objects: IMap<PathView, HostObject> }`: the object at each
  path. Two paths may hold the same object (hard links). Helpers:
  `renamed`, `exchanged` (a subtree moved or swapped), `nothing_below`,
  `parent_is_dir`, `is_tree`, `is_new_ino`, `unchanged_or_made`,
  `no_mixed_links`, `directories_have_one_path`, `as_attached`;
- `VolumeDisk`: one volume's disk, declared only, with `volume()` (its
  `LxVolume::id`), `untouched()` (no call has changed it yet) and its view
  (`HostDisk`). Only `LxVolumeOptions::new_volume` hands one out, untouched,
  with its volume;
- `LxFile::object()`: the host object a file is open on.

**`lxutil`'s contracts in its terms** (`lxutil/src/lib.rs`, trusted, since
`lxutil`'s bodies are not verified). Each of the 16 methods takes its
volume's disk, by `&` to read and by `&mut` to change it, and requires
`disk.volume() == self.id()`:

- `lstat` reports the object at the path (its device, inode number, type);
- `open` opens the object at the path if there is one; else `O_CREAT`
  makes a regular file there, with a new inode number;
- `mkdir_stat`, `mknod_stat`, `symlink_stat` make a new object at the path,
  whose parent is a directory, with a new inode number; the stat is the new
  object's;
- `link_stat` gives the object at one path a second path (never a
  directory's);
- `unlink` removes a path (a directory only when empty);
- `rename` moves or swaps a subtree (or, with `RENAME_WHITEOUT`, leaves a
  character device behind);
- an error leaves the disk unchanged, or, for a call that makes a path,
  with that one new path;
- `LxFile::fstat` reports the object the file is open on.

**The axiom** `attached_disk(disk, writable, visible)`: for a disk no call
has changed, the share as attached satisfies `as_attached`: no host hard
link joins a writable path and a visible read-only one, a directory has
one path, and the share is a tree. Its doc says why it is an axiom: it is
about what the disk held, which no contract states and no code checks.
The proof calls it once per volume, at creation.

**What `wf` says with it** (section 4): each name of a node names the
node's object on its volume's disk; a pinned object (a hidden one,
recorded when the share was attached, `pinned_identities`) has a name only
at its own path, and stays on the disk; no object has a writable path and
a visible read-only one; each disk is a tree, so no name passes through a
symbolic link; a directory has one path, so no other name leads to the
root's object; at each traverse-only path, the disk holds a directory (so
a listed entry of unknown type there, `DT_UNK`, is one).

**What `owns` says with it**: on each volume's disk, the object at the
root's path is the one the profile's root identity names
(`microvm/profile.spec.rs`: on Linux the device and inode numbers; on
Windows the file id, the volume's name being checked by path): volume 0
for a single share, volume `i + 1` for an aggregate's `i`-th child.

## 8. Confinement

**Checked: what virtio-fs hands to `lxutil`.** The 16 `LxVolume` methods
virtio-fs calls (`lstat`, `statx`, `set_attr_stat`, `open`, `mkdir_stat`,
`mknod_stat`, `symlink_stat`, `link_stat`, `read_link`, `unlink`,
`rename`, `stat_fs`, `get_xattr`, `set_xattr`, `list_xattr`,
`remove_xattr`) each require `LxVolume::is_share_path` of their path (both
paths for `rename` and `link_stat`): every component of the path's view is
a plain name (not empty, no `/`, no NUL, not `.` or `..`); the empty path is
the root. The path's view keeps the root as `/` and `.` and `..` as
themselves (vstd patch `vstd-0003`), so the precondition rules them out.
`inv` says every path virtio-fs stores is a share path, so the proof can
show it at every call site.

**In the contracts, with the host model**: every object a call reads,
opens or changes is one on the share's disk; and the paths virtio-fs hands
over pass through no symbolic link (each disk is a tree).

**Trusted, in words: what `lxutil` then does.** A call changes nothing
outside the share, and follows no link on the way (but a final one, for
`lstat` and `read_link`), whatever the path. The kernel guarantees it:

- Linux: every path is resolved relative to the root's descriptor by
  `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` (`unix/util.rs`).
- Windows: every path is opened relative to the root's handle with
  `OBJ_DONT_REPARSE`, which refuses any reparse point on the way (NT
  links, junctions, mount points; `windows/util.rs`,
  `open_relative_file`); `rename` and `link` name their target as one
  component of a parent opened that way.

What is trusted is that `lxutil` always uses these flags. That a call
changes nothing outside the share could become a contract (a declared
`HostOutside` that every `lxutil` method takes by `&`); it is not, for now,
since the trust would be the same (`lxutil/src/lib.spec.rs` says when it
would be worth doing).

**virtio-fs's own walk** (the microVM, before each operation:
`validate_confined_paths`, `ELOOP` on a link on the way;
`reject_opened_symlink`) is defense in depth. With the host model it never
fails (each disk is a tree).

**In the contracts, from virtio-fs's checks** (each a question virtio-fs
answers): hidden paths (`is_hidden`, `EACCES`), names with `\` or `:`
(`refuses_name`, `EINVAL`, microVM), paths over 4096 bytes
(`path_too_long`, `E2BIG`, microVM on Linux only, since on Windows the code
measures UTF-16 bytes), traverse-only objects that are not directories and
single-file shares (`may_reach`, `is_hidden`).

## 9. Identity

`microvm/owner.spec.rs`. **The property**, `CallerIdentity::runs_as(
caller_identity, request)`: in the default owner mode (`Vmm`,
`caller_identity` is `None`), every request runs as OpenVMM (`Own`). In
`Caller` mode (Linux only), a request that touches host files runs as its
caller, mapped (`maps`: guest UID 0 becomes the share root's owner, GID 0
its group, every other id passes through; `wf`: never host root); the
seven that touch no host file (`touches_host_files`: `INIT`, `DESTROY`,
`FORGET`, `BATCH_FORGET`, `INTERRUPT`, `RELEASE`, `RELEASEDIR`) run as
OpenVMM. "Its caller" means the ids in the request's header, which the
guest's kernel writes: the security content is "never host root, never
OpenVMM's capabilities or groups".

**The token.** `lx::cred::ThreadCred` (TRUSTED, `lx/src/cred_t.rs`,
declared only): its view is `Own` or `Switched { uid, gid }`.
`Session::dispatch` requires it to be `runs_as`. `dispatch_with` switches
through `SwitchIdentity` (`HostIdentity` in production), dispatches, and
restores, in straight-line code; its proof shows the token is right.

**Checked**: the mapping (contracts of `for_export_root_owner`,
`host_identity`, `accesses_host_files`), and `dispatch_with`'s use of the
token. **Trusted**: the token's meaning, as the contracts of `lxutil`'s
`switch_fs_identity` (the thread's filesystem ids become the identity, no
other group, no capability; or `EPERM` and nothing changed) and
`FsIdentityGuard::restore` (its own again); and the keeper's two token
axioms. **In words**: every host call of a request is made inside
`Session::dispatch`, and the seven exempt requests make no `lxutil` call.

Not claimed: what the kernel then permits that identity; that a write
through a handle opened earlier by another caller is refused; that
restore's reopening of handles, which runs as OpenVMM, is no more powerful
than the original open.

## 10. Limits

In the microVM, `inv` says the caps hold (`within_limits`): at most 4096
nodes and 4096 open handles, 256 names per node, 16 Ki names in all
(`name_count`, a `Set::fold`), and 1 MiB of name bytes (`name_bytes`; on
Linux only, as `path_too_long`). Every contract's `final(g).wf()` promises
them, so no success clause changed.

The error rules say when an operation fails for lack of room:
`no_room_for_node` (`mkdir`, `mknod`, `symlink`: `ENOSPC` for a full table
or name total, `E2BIG` for the bytes), `no_room_for_name` (`link`), and
`no_room_for_handle` (`open`, `open_dir` but the synthetic root, `create`:
`ENOSPC`). `mkdir` has its own set, `mkdir_errors`, so that `unlink` and
`rmdir`, which share `entry_errors`, do not get the rule. No node rule for
`lookup` or `create`: whether they add a node is the host's to say.

Not covered: the listings' caches (not in the view; their totals are
checked only at save, a question for Pedro), and `rename`'s byte cap (no
rule: weaker, not false; `inv` still bounds the bytes after a successful
rename).

## 11. What is trusted

- **The keeper**: its axioms (the ghost state's three, the credential
  token's two) and assumptions A1 and A2 (section 6).
- **The host model** (section 7): `VolumeDisk`'s view, the contracts of
  `lxutil`'s methods in its terms, and the axiom `attached_disk`; with the
  host assumptions (section 1) it rests on.
- **Identity** (section 9): the credential token's meaning, as the
  contracts of `switch_fs_identity` and `FsIdentityGuard::restore`.
- **`lxutil`'s method bodies** (`external_body`), and confinement's part in
  words (section 8).
- **`std` facts in the patched vstd**: a path's view (`vstd-0003`,
  uninterpreted, what `Path::components` yields), `AsRef::as_ref` for
  `&T`, `Path`, `PathBuf` (`vstd-0004`), `Arc`'s view (`vstd-0001`).
- **Every `external_body` function and type**: all production bodies (the
  proof's to replace), and the declarations of external types by name
  (`ExRwLock`, `ExBorrowedFd`, `ExFuseOperation`, `ExAnyhowError`, `lx`'s
  `StatFs`, `lxutil`'s `LxVolume`, `LxFile`, `LxCreateOptions`,
  `SetAttributes`, `VolumeError`, and others).
- **The answer that stands for missing knowledge**: `host_error` is "any
  code".

## 12. Rules for the proof

- **Do not change what a contract means.** The trait's contracts, the
  `FuseState` questions' meanings and rules, `owns`, `is_new`, and the
  lock rules' SPEC cores are the spec. A proof that needs a stronger fact
  may add it to `wf`, to a lock rule's TODO part, or to the view's `inv`
  if the code keeps it, and says why.
- **Do not change executable code.** If the proof seems to need a code
  change, ask Shan: the spec's authors make it, with an entry in
  `CODE-CHANGES.md`. Ghost arguments and ghost fields are not executable
  changes.
- **Replace every PLACEHOLDER** (section 5), and no new `assume_new`.
- **`dispatch_with` calls `Session::dispatch` with `runs_as` =
  `CallerIdentity::runs_as(caller_identity, request@)`**: that ghost
  argument is part of the spec (section 9).
- **At creation, call `attached_disk`** once per volume
  (`VirtioFs::new_microvm`), with the share's policy, for `wf`'s facts on
  hard links, directories' paths and trees; after that, the operations
  keep them.
- **Trusted code stays trusted**: do not add axioms beyond those listed in
  section 11.

Spots that look hard:

- `read_dir_plus`: `looked_up_in` is an existential over a sequence of
  lookups folded with `after_reach` (`Seq::fold_left`); the proof must
  build the sequence as the loop goes, including on an error.
- `lookup` when `InodeMap::insert` finds the host file already in the
  table (dedup): the new inode and its fresh ghost entries are dropped; the
  view must not see them (it does not, through the keys), and the existing
  node's path and names change (`after_reach`'s first case).
- `rename` and `RENAME_EXCHANGE`: every inode of the share in the table has
  its names rewritten (`map_names`, `moved`, `exchanged`), each under its
  own aliases and path locks; `unlinked` first drops the names at or below
  the new path; on the disk, `HostDisk::renamed` or `exchanged` moves the
  subtree, and `wf`'s host facts must follow.
- The limits: the code sums over `HashMap` iterations
  (`microvm_alias_usage`, `preflight_microvm_rename`), the spec over sets
  (`Set::fold`); a lemma must show they agree. The rename's simulation
  (`preflight_microvm_rename`) must give exactly the renamed names.
- `forget`: the node leaves the table exactly when its count reaches zero,
  which the spec does not track; the contract allows either outcome.
- The microVM's listing cache is re-judged at each reply (`listable_now`),
  which is what `lists_only_allowed` needs.

## 13. Open items

Each is also in `CODE-OVERVIEW.md`'s open items or status table.

- **virtio-fs's save and restore** (property 6): not specified, by
  decision, until this spec is verified; what the rework needs is below.
- **File contents and attributes** (property 5): not specified, by
  decision (not about security).
- **The lookup counts**: not needed by the limits; they wait for
  `forget`'s correctness, liveness and snapshots.
- **`read_dir_plus`**: its lookups and the names it lists are not tied
  together (functional only).
- **Error rules left out**: `rename`'s byte cap (`E2BIG`), a listing's
  first read at an offset other than 0 (`EINVAL`), `ENOMEM`. Not stated:
  when an operation must succeed.
- **Confinement's last part**: that a call changes nothing outside the
  share stays in words (section 8).
- **Questions for Pedro** (`BUGS.md`): host hard links (whether to check
  them at attach), mount points inside a share, names in another case or
  short names on Windows, the listings' totals checked only at save, and
  the listing note.

**Property 6 and the snapshot spec: what the later rework needs.**
Combining virtio-fs's save and restore with the snapshot spec (on
`system-proof/snapshot-spec-review-20261023`: `snapshot-spec-docs/`,
`virtio/src/transport/mmio.spec.rs`) needs:

- **`g` guarded as a device's snapshot ghost is.** The snapshot spec
  guards every change to a device's state with the device's `FracGhost`,
  half of which goes up to `StateUnits` at `Stop`, so nothing changes while
  stopped. virtio-fs's state is guarded by `g` instead (every table and name
  change needs it), but `g` stays in the keeper while the device is
  stopped, so nothing stops a request then. The plan: at stop, `g` leaves
  the keeper and goes up with the transport's stop reply (with the
  transport's half); while stopped the keeper is empty; at start, or after
  a restore that rebuilds `g` from the blob, it goes back. This changes the
  keeper's axioms (A2) and the transport's stop and start path. `g` is
  compatible: it already guards every table and name change, so it moves
  rather than changes.
- **The saved state that `g` does not cover yet:** the nodes' lookup
  counts, each directory handle's cached listing, the next node and handle
  ids, and the FUSE negotiation (`parking_lot`). For "state equals ghost"
  while stopped, they join `g`'s view or are shown unchanged otherwise.
- **The transport code.** The snapshot spec relies on two transport
  changes (`verus-patches/openvmm-0001`, `0002`: save keeps the captured
  device state; restore sets it). Pedro's code restructured that part
  (`TransportRestore`) without them: here a second save fails, and a save
  right after a restore fails ("device state was not captured"). They need
  porting to `TransportRestore`.
- **The restore contract** the snapshot spec needs: after a successful
  restore, the device's state is the one the blob describes (the full
  form, `DESIGN-snapshot.md`'s D1(a)), on top of the policy core that holds
  for any blob (its D1(c), D2(a)). Restore must also re-establish `wf`'s
  host facts, which its own checks support (each name reopened, each
  object checked).

Last night's analysis, with the code walk, the trust question (the blob is
untrusted: policy facts hold for any blob, "same host objects" only for an
authentic one), the decisions and the sizes:
`notes-20261111/DESIGN-snapshot.md`.

# virtio-fs spec: design, for the proof

The Verus specification of OpenVMM's virtio-fs device, as it stands, for
whoever proves the production code against it. It states only the current
design. The history, the alternatives and the reasons in full are in
`CODE-OVERVIEW.md`; the changes to the executable code are in
`CODE-CHANGES.md`; bugs found and questions for Pedro are in `BUGS.md`.
Style rules: `snapshot-spec-docs/CONVENTIONS.md`.

The code is Pedro Henrique Penna's, on `feature/microvm/dev` at
`db42fde04`. This branch (`system-proof/virtiofs-spec-20261108`) adds the
spec, a few code changes listed in `CODE-CHANGES.md`, and these documents.

## 1. What is specified

**Scope.** The spec protects the host from the guest, at the FUSE level:

- **Integrity**: a request that changes the host (a file's bytes or
  attributes, a name created, removed or renamed) succeeds only through a
  path the guest may write.
- **Confidentiality**: what the guest gets back (a node, attributes, a
  directory entry, a file's bytes) is about a path the guest may see.
- **When an operation fails**: each operation's error rules (a condition
  and its code).
- **Confinement, its first half**: virtio-fs hands `lxutil` only relative
  paths of plain names.

Not specified: a model of the host's contents, the caller's identity
(each request runs as its caller), the limits (bounded tables), virtio-fs's
own save and restore, and what `read` and `write` move.

**The highest-level guarantee: names, then objects.** The guest names host
objects by paths relative to the share. The guarantee has two parts:

1. *The policy is about names.* An operation succeeds only through names
   the policy allows. This part is in the contracts.
2. *Confinement ties names to objects.* virtio-fs hands `lxutil` only
   relative paths of plain names (checked), and `lxutil`, with the host's
   kernel, resolves each beneath the share's root without following a link
   (trusted, in words). So each allowed name is an object inside the share.

Part 1 alone is not enough. If the volume followed a host link
`logs/gateway` -> `/etc`, a read of the visible name `logs/gateway/passwd`
would meet every contract and still read the host's `/etc/passwd`.

**What the host model adds** (`lxutil/src/host_t.rs`, started 2026-11-11):
which object a name reaches. With it, the spec says that every name of a
node names the node's object on the disk, and that a hidden object is not
reached through another name (pinned identities), that no object has both a writable path and a visible
read-only one (host hard links), that the share's root is the intended
directory (its identity, in `owns`), and that no other name leads to the
root's object. It does not yet say that no path passes through a link.

**Host assumptions.** The spec holds only if:

- no other host process changes the share while the device runs;
- a share contains no mount point (on Linux; on Windows mount points are
  refused, see section 7);
- no two volumes share a host directory (so only a volume's own calls
  change what its disk holds: the host model);
- an object's inode number is the same on every call (always on Linux; on
  Windows, a share's filesystem must report stable file ids);
- as the share was attached: no host hard link joined a path the guest may
  write and a visible path it may not (Pedro's setup rule), and a
  directory had one path (no directory hard links on Linux or NTFS, and no
  mount point), and the share was a tree (every object's parent a
  directory). These are about what the disk held, not how the host
  behaves, so they are an axiom (`attached_disk`, `lxutil/src/host_t.rs`);
  the others are built into `lxutil`'s trusted contracts;
- on Windows, paths that differ name for name are different objects: no
  name of a share reaches an object in another case (`SECRET` for
  `secret`) or by an 8.3 short name (`SECRE~1`). lxutil opens every path
  case-insensitively, so this holds only for directories that are
  case-sensitive and volumes without short names (a question for Pedro,
  `BUGS.md`).

**Every byte the guest writes is untrusted.** No contract requires
anything of a request's contents (names, flags, node ids, handles): an
operation must meet its contract whatever the guest sends.

## 2. The files

| File | What it holds |
|---|---|
| `vm/devices/support/fs/fuse/src/lib.rs` | the `Fuse<G>` trait: the 44 operations, each with its contract |
| `vm/devices/support/fs/fuse/src/lib.spec.rs` | `FuseGhost`, `FuseOwns`, `FuseState` (questions and rules), `ListedEntry` |
| `vm/devices/support/fs/fuse/src/session.rs`, `session.spec.rs` | `Session<G>` and `dispatch`'s contract; `owns_ghost` |
| `vm/devices/support/fs/fuse/src/request.spec.rs` | `RequestView` (the node id) |
| `vm/devices/support/fs/fuse/src/protocol.rs` | the FUSE structs and constants, declared to Verus |
| `vm/devices/support/fs/lx/src/string.spec.rs`, `error.spec.rs` | `LxStr`'s view (its bytes); `Error::code` |
| `vm/devices/support/fs/lxutil/src/lib.spec.rs` | `LxVolume::is_share_path`, and the confinement fact (trusted, in words) |
| `vm/devices/support/fs/lxutil/src/lib.rs` | the preconditions on 16 `LxVolume` methods |
| `vm/devices/virtio/virtiofs/src/lib.spec.rs` | virtio-fs's ghost state and its view, `wf`, `owns`, the table locks' rules, and virtio-fs's answers to `FuseState` |
| `vm/devices/virtio/virtiofs/src/inode.spec.rs` | the path and aliases locks' rules; `VirtioFsInode`'s view; `VirtioFsVolume::share_view` |
| `vm/devices/virtio/virtiofs/src/aggregate.spec.rs` | the aggregate registry lock's rule |
| `vm/devices/virtio/virtiofs/src/fs_ghost_t.rs` | the keeper: trusted axioms (section 6) |
| `vm/devices/virtio/virtiofs/src/microvm/policy.spec.rs` | the access policy's view (`SubtreePolicyView`) |
| `vm/devices/virtio/virtiofs/src/microvm/fs.rs` | `VirtioFs::new_microvm`'s contract |
| `vm/devices/virtio/virtiofs/src/microvm/owner.rs` | the microVM's `dispatch`'s contract |

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
parameter of a lock, or a field): `fuse/src/lib.spec.rs`,
`virtiofs/src/lib.spec.rs`, `inode.spec.rs`, `aggregate.spec.rs`,
`microvm/policy.spec.rs`. The others are under `cfg(verus_keep_ghost)`.

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
`&mut` ones ensure both of `final(g)`. Example, `write`
(`fuse/src/lib.rs`):

```rust
with Tracked(g): Tracked<&G>
requires
    self.owns(g),
    g.wf(),
ensures
    r is Ok ==> g@.uses_handle(_request@.node_id, _arg.fh) && g@.opened_for_writing(_arg.fh) && g@.file_can_write(_arg.fh),
    G::V::fails_with(r, g@.write_errors(_request@.node_id, _arg.fh, _data@.len())),
```

### Questions and rules

`FuseState` (`fuse/src/lib.spec.rs:133`) has two kinds of spec function:

- A **question** is declared without a body; each file system answers it.
  Example: `holds_node(n)`, "the guest holds node `n`".
- A **rule** has its body in the trait, built from questions, the same for
  every file system. Example: `is_entry(parent, name)`.

The trait is in sections: the state's invariant; questions about nodes and
paths, about handles, about how operations change the state, about what
the file system refuses; then rules about names and flags, about when an
operation may succeed, and about when it fails.

The rules the success clauses lean on most:

- `uses_node(n)`: the guest holds `n`, and none of its names is hidden now
  (`names_visible`). The code re-checks a node's names before every
  operation on it (`get_inode` -> `validate_confined_object`): a rename can
  give a held node a hidden name.
- `uses_handle(n, fh)`: `uses_node(n)`, the guest holds `fh`, and `fh` is
  `n`'s.
- `is_entry(parent, name)`: `parent` is a directory whose names are
  visible, and `name` is one plain name.
- `may_add_name(parent, name)`: `is_entry`, `parent` may be changed, and
  the new name's path is writable.

### Success clauses

A success clause says what holds if the operation succeeds, and what the
state is after it. For an `&mut` operation, an error leaves the state
unchanged (`Err(_) => final(g)@ == old(g)@`), with one exception:
`read_dir_plus` keeps the lookups it did before an error
(`looked_up_in`).

State changes are exact: `final(g)@ == s.after_reach(n, p, m, kind)`,
`s.unlinked(n, p, final(g)@)`, `s.renamed(n, p, q, exchange, final(g)@)`,
`s.after_open(fh, n, writable)`, and so on. The proof must make the ghost
state follow the code exactly.

### Error rules

Each operation's rules are a set of codes (`*_errors`, an `ISet<i32>`):
each rule is a condition and its code, and the set holds the codes of the
rules that apply. The contract says

```rust
G::V::fails_with(r, old(g)@.entry_errors(_request@.node_id, _name@)),
```

and `fails_with` (`fuse/src/lib.spec.rs:383`) means: if any rule applies,
the operation fails, with the code of one of the rules that apply, or with
a host error (`host_error`). The order in which the code checks does not
matter. virtio-fs answers `host_error` "any code" (the host model does not
say which errors the host gives), so
what the proof must show is that the operation fails, whenever a rule
applies.

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
(`fuse/src/lib.spec.rs:84`), read off `DirEntryWriter` (`reply.rs`): for
each entry, in a `read_dir_plus` reply the lookup's reply
(`fuse_entry_out`, 128 bytes), then the header (`fuse_dirent`: inode
number, offset, name length, type, little-endian), the name, and zeros up
to a multiple of 8.

## 4. virtio-fs's ghost state

### The ghost state and its view

`VirtioFsGhost` (`virtiofs/src/lib.spec.rs:121`) is the bundle that a
request holds:

```rust
pub tracked struct VirtioFsGhost {
    pub tracked inodes: FracGhost<InodeTable>,          // half of the ghost inode table
    pub tracked paths: GhostMapAuth<int, PathView>,     // every inode's main path, by key
    pub tracked aliases: GhostMapAuth<int, Set<PathView>>, // every inode's names, by key
    pub tracked files: FracGhost<Map<u64, VirtioFsFileView>>, // half of the ghost file table
    pub ghost next_key: int,
    pub ghost microvm: bool,
    pub ghost shares: Map<Seq<u8>, VirtioFsShareView>,
}
```

Its view, `VirtioFsGhostView` (`:81`), is what the contracts read: by node
id, the inodes (`VirtioFsInodeView`: volume, kind, read-only flag, policy,
exposed file), their main paths and their names; the open handles
(`VirtioFsFileView`: node, inode view, writable); and the flags
`aggregate` and `microvm`, and the `shares`.

### Hidden keys

Inside `VirtioFsGhost`, `paths` and `aliases` are keyed by a per-inode
**ghost key**, not by node id. An inode's locks are created, with their
rules, in `VirtioFsInode::with_attr`, before the inode table gives it a
node id, and a lock's rule never changes; so each rule names its entry by
a key fixed at creation (`PathRule.key`).

- Keys come from the counter `next_key`: a new inode takes it, and it
  grows. Keys are never reused.
- The ghost inode table records each node's key (`InodeTable.keys`,
  `:112`).
- The view is computed through it (`impl View for VirtioFsGhost`, `:143`):
  node `n`'s path is the entry of `n`'s key.

So entries of no node's key are not seen: an inode built and then dropped
(`InodeMap::insert` drops a new inode when the table already has the same
host file), or a forgotten one. The contracts therefore read as if the
maps were keyed by node id.

### `wf`, `inv`, `owns`

- `inv` (`:380`, on the view): every table node but the root has a main
  path and names, its main path among its names when it has any; every
  main path and name, and every share's root path, is a share path
  (`LxVolume::is_share_path`, which each `lxutil` method requires: so the
  paths virtio-fs hands to `lxutil` meet its precondition); an aggregate's
  shares have plain names.
- `wf` (`:305`, on `VirtioFsGhost`): `inv`; the recorded map ids are the
  maps'; every table node but the root has a key with entries in both
  maps; no two such nodes share a key; every key in use is below
  `next_key`.
- `owns` (`:331`): `g`'s two table halves are the other halves of those
  this file system's table locks hold (the ids their rules pin); `g`
  records whether the file system is a microVM's and an aggregate; an
  aggregate's shares are its registry's (microVM), and a direct share has
  none.

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
  hands out every time, is held, and its node is 1;
- `open_dir`'s success clause has a case for it: its own handle, nothing
  changes.

The shares are tied to the code by the registry lock's rule (section 5),
for a microVM aggregate, whose children never change. A non-microVM
aggregate (children added and removed by `add_child`/`remove_child`
outside any request) is not covered: its root's contracts cannot be
proved.

## 5. The locks

Each lock that holds guest state is vstd's verified `RwLock<V, Pred>`,
with its rule (`Pred`) given at creation and never changed. There are
five:

| Lock (field) | Holds | Rule | Rule file |
|---|---|---|---|
| `VirtioFsInode::path` (`inode.rs:179`) | `InodePath`: the main path, and its `paths` entry (`GhostPointsTo`) | `PathRule { key, map_id, is_root }` | `inode.spec.rs:33` |
| `VirtioFsInode::aliases` (`inode.rs:180`) | `InodeAliases`: the names, and their `aliases` entry | `AliasesRule { key, map_id, is_root }` | `inode.spec.rs:72` |
| `VirtioFsInner::inodes` (`lib.rs:157`) | `InodeTableLocked`: the `InodeMap`, and the lock's half of the ghost inode table | `InodeTableRule { id }` | `lib.spec.rs:256` |
| `VirtioFsInner::files` (`lib.rs:158`) | `FileTableLocked`: the `HandleMap`, and the lock's half of the ghost file table | `FileTableRule { id }` | `lib.spec.rs:282` |
| `AggregateState::registry` (`aggregate.rs:87`) | `AggregateRegistry`: the children | `RegistryRule { fixed, shares }` | `aggregate.spec.rs:25` |

The other locks are still `parking_lot` and have no rule: each open file's
`file` and `directory_snapshot` (`file.rs:29`, `:32`) and the
`negotiation` (`lib.rs:163`). No contract needs them.

**Each rule has a SPEC core and a TODO part.** The core is part of the
spec; the proof must keep it. Each rule's comment marks both.

- `PathRule` core: the piece is an entry of the `paths` map
  (`piece.id() == map_id`), this inode's (`piece.key() == key`), and equals
  the real path (`piece.value() == v.path@`). For the root: the path is
  empty and there is no piece.
- `AliasesRule` core: the same for the names (`piece.value()` is the set of
  the real names' views). For the root: its only name is the empty path.
- `InodeTableRule` core: the half is the ghost inode table's, its inodes
  are the real table's inode views, and its aggregate flag is the real
  table's.
- `FileTableRule` core: the half is the ghost file table's, it has the
  real table's handles, and each handle's inode view and writability (from
  its open flags) are the real file's.
- `RegistryRule` core: with `fixed`, the children are `shares`, by name
  (`encode_utf8` of each child's name).

**TODO for the proof**, in the rules: the cross-lock links (each table
inode's own locks' rules have the key recorded in `InodeTable.keys` and
the maps' ids; the path and aliases keys are the same; each handle's
recorded node is one whose inode is the file's, while the node is in the
table) and that each table lock holds exactly a half. The proof may add
facts and fields to any rule.

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

- `VirtioFsInode::with_attr` (`inode.rs:199`): the path and aliases
  pieces (fresh entries at key `next_key`) and rules;
- `InodeTableLocked::new_lock` (`lib.rs:110`) and
  `FileTableLocked::new_lock` (`lib.rs:141`): the halves and rules;
- `AggregateState::new` and `with_children` (`aggregate.rs:91`, `:102`):
  the registry rule.

These need a ghost argument to reach `g` (or the pieces made with it).
Adding ghost arguments is fine: they change no executable behaviour.

**Release is explicit.** vstd's handles are released by `release_write` /
`release_read`, not when dropped. Every critical section ends with one,
including those that return early: they compute their result, release,
then return.

## 6. The keeper

Between requests the ghost state `g` is kept by trusted axioms
(`virtiofs/src/fs_ghost_t.rs`; proof functions, nothing at runtime):

- `keep_fs_ghost(fs, g)` (`:72`), once, when the file system is created;
- `take_fs_ghost(session) -> g` (`:80`), at the start of each request;
- `give_back_fs_ghost(session, g)` (`:88`), at its end.

A request lends `g` down: `process_virtiofs_request` -> `owner::dispatch`
-> `Session::dispatch` -> the handler. Both dispatchers' contracts require
and ensure `owns_ghost(g)` and `g.wf()`.

Assumptions (not checked):

- **A1.** A take is followed by its give-back before the next take. True
  because requests never overlap in the microVM: both queue workers are
  tasks on one thread (`driver_source.simple()`), and
  `process_virtiofs_request` has no `await`.
- **A2.** Only `process_virtiofs_request` takes and gives back, and the
  keeper holds one ghost state per file system.

**Creating `g`.** `VirtioFs::new_microvm`'s contract (`microvm/fs.rs:264`)
demands, on success, a `g` that the new file system owns, well formed, and
new (`is_new`: the root alone, no names, no handles). Its proof creates the
maps and the table halves, puts one half of each into its lock, and hands
the rest out; the device's setup then calls `keep_fs_ghost`.

Not covered: virtio-fs's own save and restore, which replace the tables
and so need a new `g`.

## 7. Confinement

**Checked: what virtio-fs hands to `lxutil`.** The 16 `LxVolume` methods
virtio-fs calls (`lstat`, `statx`, `set_attr_stat`, `open`, `mkdir_stat`,
`mknod_stat`, `symlink_stat`, `link_stat`, `read_link`, `unlink`,
`rename`, `stat_fs`, `get_xattr`, `set_xattr`, `list_xattr`,
`remove_xattr`) each require:

```rust
#[cfg_attr(verus_keep_ghost, verus_spec(requires LxVolume::is_share_path(path.as_ref_spec()@)))]
```

(both paths for `rename` and `link_stat`). `is_share_path`
(`lxutil/src/lib.spec.rs:34`): every component of the path's view is a
plain name (not empty, no `/`, no NUL, not `.` or `..`); the empty path is
the root. The path's view keeps the root as `/` and `.` and `..` as
themselves (vstd patch `vstd-0003`), so the precondition rules them out.
The proof must show it at every call site; the paths come from
`child_path`, which checks each name.

**Trusted, in words: what `lxutil` then does.** A call with share paths
touches only objects beneath the volume's root, and follows no link on the
way (but a final one, for `lstat` and `read_link`). No contract states it:
the spec has no notion of a host object. The kernel guarantees it:

- Linux: every path is resolved relative to the root's descriptor by
  `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` (`unix/util.rs`).
- Windows: every path is opened relative to the root's handle with
  `OBJ_DONT_REPARSE`, which refuses any reparse point on the way (NT
  links, junctions, mount points; `windows/util.rs`,
  `open_relative_file`); `rename` and `link` name their target as one
  component of a parent opened that way.

What is trusted is that `lxutil` always uses these flags.

**Not required: virtio-fs's own walk.** Before each operation, the
microVM walks each name of the node and refuses a link on the way (`ELOOP`,
`validate_confined_paths`), and refuses an opened link
(`reject_opened_symlink`). It is defense in depth; the claim rests on the
kernel's guard. With the host model (2026-11-11), `wf` says each disk is a
tree, so every step but the last of a node's name is a directory: no name
passes through a link, and the walk never fails.

**In the contracts, from virtio-fs's checks** (each a question virtio-fs
answers): hidden paths (`is_hidden`, `EACCES`), names with `\` or `:`
(`refuses_name`, `EINVAL`, microVM), paths over 4096 bytes
(`path_too_long`, `E2BIG`, microVM on Linux only, since on Windows the code
measures UTF-16 bytes), traverse-only objects that are not directories and
single-file shares (`may_reach`, `is_hidden`).

## 8. What is trusted

- The keeper's three axioms, and assumptions A1 and A2 (section 6).
- `lxutil`'s method bodies (`external_body`), and the confinement fact in
  words (section 7).
- The host model (`lxutil/src/host_t.rs`, 2026-11-11): what a share holds
  on disk (`HostDisk`: the object at each path), the disk value per volume
  (`VolumeDisk`, declared only; `LxVolumeOptions::new_volume` hands it out
  with its volume), and the contracts of `lxutil`'s methods in terms of it
  (`lib.rs`; `LxFile::fstat` too). It rests on the host assumptions. Its
  one axiom, `attached_disk`, states the assumptions about the share as
  attached (hard links, one path per directory, a tree) for a disk no call
  has changed (`untouched`).
  virtio-fs keeps each volume's disk in `g` (`VirtioFsGhost::disks`, not in
  the view), and `wf` says every name of a node names, on its volume's
  disk, the node's object (`InodeTable::host_inos`, which the inode table's
  rule ties to each inode's `inode_nr`).
- The host assumptions (section 1).
- `std` facts in the patched vstd: a path's view (`vstd-0003`,
  uninterpreted, what `Path::components` yields), `AsRef::as_ref` for
  `&T`, `Path`, `PathBuf` (`vstd-0004`), `Arc`'s view (`vstd-0001`).
- Every `external_body` function and type: all production bodies (the
  proof's to replace), and the declarations of external types by name
  (`ExRwLock`, `ExBorrowedFd`, `ExFuseOperation`, `ExAnyhowError`, `lx`'s
  `Stat`, `StatEx`, `StatFs` (`Stat` and `Timespec` are now visible to Verus,
  for the postconditions above), `lxutil`'s `LxVolume`, `LxFile`,
  `LxCreateOptions`, `SetAttributes`, and others).
- The answers that stand for missing knowledge: `host_error` is "any code";
  a listed entry of unknown type (`DT_UNK`) in a traverse-only directory
  is a directory if its record says so (the code asks the host).

## 9. Rules for the proof

- **Do not change what a contract means.** The trait's contracts, the
  `FuseState` questions' meanings and rules, `owns`, `is_new`, and the
  lock rules' SPEC cores are the spec. A proof that needs a stronger fact
  may add it to `wf`, to a lock rule's TODO part, or to the view's `inv`
  if the code keeps it, and says why.
- **Do not change executable behaviour.** A code change needs Shan's
  approval, and an entry in `CODE-CHANGES.md`. Ghost arguments and ghost
  fields are not executable changes.
- **Replace every PLACEHOLDER** (section 5), and no new `assume_new`.
- **At creation, call `attached_disk`** once per volume
  (`VirtioFs::new_microvm`), with the share's policy, for `wf`'s facts on
  hard links and on directories' paths; after that, the operations keep
  them.
- **Trusted code stays trusted**: do not add axioms beyond those listed in
  section 8.

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
  the new path.
- `forget`: the node leaves the table exactly when its count reaches zero,
  which the spec does not track; the contract allows either outcome.
- The microVM's listing cache is re-judged at each reply (`listable_now`),
  which is what `lists_only_allowed` needs.

## 10. Open items

Each is in `CODE-OVERVIEW.md`'s open items or status table.

- **The rest of the host model** (items 3, 8, 9): `DT_UNK` entries. (Confinement's last part, that a call changes
  nothing outside the share, stays in words; it could be a contract, a
  declared `HostOutside` taken by shared reference: `lxutil/src/lib.spec.rs`
  says why it is not, for now.) (Done: the host
  model itself, `lxutil/src/host_t.rs`, the disks in `g`, `wf`'s tie of
  each node's names to its object, bug 3's rule in `create`'s contract,
  the pinned identities, host hard links, one path per directory, and the
  root's identity in `owns`, from `microvm/profile.spec.rs`, and the
  symbolic-link walk: each disk is a tree, in `wf`.)
- **Limits** (property 4), with the lookup counts (item 2).
- **Identity** (property 3): each request runs as its caller.
- **virtio-fs's save and restore** (property 6), including a new `g` on
  restore.
- **`read_dir_plus`**: its lookups and the names it lists are not tied
  together (functional only).
- **Error rules left out**: the limits (`ENOSPC`, `E2BIG`), a listing's
  first read at an offset other than 0 (`EINVAL`), `create` on an existing
  file, `ENOMEM`. Not stated: when an operation must succeed.
- **Questions for Pedro** (`BUGS.md`): host hard links; mount points inside
  a microVM share; the listing note.

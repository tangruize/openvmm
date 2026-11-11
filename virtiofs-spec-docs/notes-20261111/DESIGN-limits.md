# Design prep: property 4, limits (2026-11-11)

Paths are under `vm/devices/virtio/virtiofs/src/` unless they start with `fuse/`
(`vm/devices/support/fs/fuse/src/`). The constants are in
`microvm/saved_state.rs:20-30`. All caps apply to the microVM only (`is_microvm()`).

## 1. Every limit in the code

"Before host" means the check runs before any call that changes the host.

| What is bounded | Cap | Checked at | Error | Before host? |
|---|---|---|---|---|
| nodes in the inode table | `MAX_INODES` 4096 | `microvm/limits.rs:41` (`preflight_microvm_new_inode`) | ENOSPC | yes (mkdir `lib.rs:461`, mknod `:489`, symlink `:519`, create `:422`); checked again after the host call (`InodeReservation::insert`, `lib.rs:1118-1131`) |
| names of one node | `MAX_ALIASES_PER_INODE` 256 | `limits.rs:69` (`preflight_microvm_alias_add`) | ENOSPC | yes (link `lib.rs:555`, then `add_alias` at `:557` with no check) |
| names, all nodes | `MAX_ALIASES` 16 Ki | `limits.rs:46`, `:73`, `:125` | ENOSPC | yes |
| bytes of all names | `MAX_ALIAS_BYTES` 1 MiB | `limits.rs:50-54`, `:77-81`, `:133` | E2BIG | yes |
| one path's bytes | `MAX_PATH_BYTES` 4096 | `limits.rs:158`, `microvm/inode.rs:247-249` | E2BIG | yes (already in the spec: `path_too_long`) |
| names after a rename | the three caps above | `limits.rs:86-139` (`preflight_microvm_rename`, simulates the rename) | ENOSPC / E2BIG | yes (`lib.rs:693`, before `inode.rename`) |
| open handles | `MAX_HANDLES` 4096 | `microvm/fs.rs:159-166` (`validate_file_insert`), again at `lib.rs:1152-1167` | ENOSPC | yes (open `lib.rs:396`, create `:423`) |
| id space of node ids and handles | `u64` counter | `HandleMap::can_insert` `lib.rs:1281`, `reserve` `:1240` | ENOSPC | yes (`reserve_*` comes before the host call) |
| guest buffers (read, getxattr, listxattr) | `MAX_GUEST_BUFFER_SIZE` 1 MiB (`lib.rs:71`) | `guest_buffer` `lib.rs:1189-1192`; read_dir `microvm/file.rs:23` | E2BIG | yes (already in the spec: `buffer_too_large`) |
| write data, setxattr value | 1 MiB | `lib.rs:592`, `:775` | E2BIG | yes (in the spec) |
| a whole FUSE request | `MAX_FUSE_REQUEST_BYTES` = 1 MiB + 4 KiB (`microvm/mod.rs:32`) | `virtio.rs:474` | no reply: `notify_corruption` | yes (before parsing) |
| a string inside a request | same | `virtio_util.rs:113-115` | E2BIG | yes |
| one listing's cache: entries | `MAX_DIRECTORY_ENTRIES_PER_HANDLE` 4096 | `microvm/file.rs:124` | E2BIG | no host change at all (host readdir) |
| one listing's cache: name bytes | `MAX_DIRECTORY_SNAPSHOT_BYTES` 64 KiB | `microvm/file.rs:142` | E2BIG | same |
| entry name length | 255 | `microvm/file.rs:128` | EINVAL | same |
| all listings' caches, total | `MAX_DIRECTORY_ENTRIES` 16 Ki, `MAX_DIRECTORY_BYTES` 4 MiB | **only at save** (`microvm/fs.rs:596-612`) and restore (`microvm/state.rs:570-583`) | save fails | n/a |
| saved state | every cap above | save: `microvm/fs.rs:509-576`; restore: `microvm/state.rs:397-582` | anyhow error | n/a |

Point 1. **Pedro's "checked before anything is created on the host" holds.** Every
table check runs before the host call. The checks after the host call
(`InodeReservation::insert`, `FileReservation::insert`) cannot fail in the microVM,
because nothing runs between the first check and the second: there is one request
worker, and the ghost `&mut g` is the request's. Even the rare dedup case passes. For
example, a `mknod` whose host inode number is reused from an unlinked node the guest
still holds dedups onto that node. That node has no names, so the second check asks for
exactly what the first one did.

Point 2. **A rename never adds names.** It maps each name set through `moved` or
`exchanged`, after a filter. So the `ENOSPC` branches of `preflight_microvm_rename`
(`limits.rs:119`, `:125`) can never fire. Under CONVENTIONS ("a check that can never
fail") this is a finding, and a harmless one. Only `E2BIG` is live for a rename: the
total bytes, or one moved name longer than 4096 bytes (`limits.rs:129`).

Point 3. **Gap A: the dedup type check skips the node cap.** `preflight_microvm_insert`
(`limits.rs:33`) treats any existing key as "add a name". `dedup_entry` (`lib.rs:1424-1436`)
treats it differently: when the file type differs, it evicts the key, and `insert`
then adds a **new** node.

```rust
// limits.rs:33: no file-type test
if let Some((existing, _)) = self.inodes_by_key.get(&inode.dedup_key()) {
    return self.preflight_microvm_alias_add(existing, &path);   // no MAX_INODES check
}
// lib.rs:1428: the insert does test the type, and evicts
.is_some_and(|(existing, _)| existing.file_type() != inode.file_type())
```

So a `lookup` (or one of `read_dir_plus`'s lookups) that reaches this branch with 4096
nodes leaves 4097 nodes. I found no way to reach it under the host assumption: every
host object the guest creates goes through an insert, which repoints the key. But the
file type comes from the host's `lstat`, so without a host model the proof cannot rule
the branch out. **The invariant cannot be proved without a fix** (decision D3).

Point 4. **Gap B: the listing totals are only checked at save.** At run time each
handle may cache 4096 entries and 64 KiB, and there can be 4096 handles. Five open
directories of 4096 entries each, each read once, are enough to make
`save_microvm_state` fail ("saved directory entries exceed the aggregate bound"). The
guest can block snapshots this way. Nothing in `BUGS.md` covers it (decision D4).
A related point for the snapshot notes: save also fails for a held node with no names
(`microvm/fs.rs:516`). That is not a limit.

## 2. What the spec could promise

**(a) An invariant of the state.** All of it is in the view already: node ids, names
(`names_of`), and handles. No host model is needed:

- `|inodes| <= 4096`, and `|files| <= 4096`;
- for each node `n`, `|names_of(n)| <= 256`;
- the sum over nodes of `|names_of(n)|` is at most 16 Ki (the root counts its one name,
  `""`, as `microvm_alias_usage` does);
- the sum over nodes and names of `encoded_len` is at most 1 MiB. Linux only, as with
  `path_too_long`: on Windows the code counts UTF-16 bytes (`state.rs:660-668`).

With these facts in `inv`, save's table checks (`microvm/fs.rs:509-576`) pass for these
caps. Restore's checks (`state.rs:397-484`) establish them, so restore's contract can
promise `wf`. The listing caches are not in the view (open item 4), so they are outside
the invariant.

**(b) Error rules, with no host model.** Each rule is "condition ⇒ fails".

- mkdir, mknod, symlink: full table, or full name total → ENOSPC; total bytes plus
  `|p|` over 1 MiB → E2BIG. These always run the new-node check (`preflight_new_inode_path`).
- link: target `m` does not already have the name `p`, and `m` has 256 names or the
  total is full → ENOSPC; bytes → E2BIG.
- rename: the names after the rename exceed 1 MiB in total, or one exceeds 4096 bytes
  → E2BIG.
- open, create, open_dir (not on the synthetic root): 4096 handles → ENOSPC.
- **No node rule for `lookup`, `read_dir_plus` or `create`.** The lookup may reach an
  existing node (dedup) and succeed with a full table. `create` first asks the host
  whether the file exists (`preflight_create_inode`, `microvm/fs.rs:429-443`). Which
  case applies is the host's to say.

**(c) Lookup counts (open item 2).** **The limits do not need them.** Every cap is a
size of a table or of a name set (`limits.rs:41`: `values.len()`). A count only decides
*when* `forget` removes a node (`lib.rs:382-384`), and removal only shrinks the tables.
What the counts would add is a different kind of property:

- that `forget` removes a node exactly when its count reaches 0 (functional correctness);
- that room comes back when the guest forgets (liveness);
- that saved and restored counts agree (snapshots: `SavedInode::lookup_count`).

If the counts are added anyway, here is what they need:

- the view gets `counts: Map<u64, nat>`, and `g` gets a `GhostMapAuth<int, u64>` keyed
  like `paths`;
- `VirtioFsInode::lookup_count` becomes vstd's `atomic_ghost::AtomicU64` with a
  `GhostPointsTo` piece. This is a **code change**. `forget`'s `.update` closure
  (`inode.rs:321-331`) has no vstd form, so it becomes a load-and-CAS loop. The
  `fetch_add`s (`inode.rs:303`, `:317`) wrap at 2^64, which the proof must handle or
  assume away.
- contracts: `after_reach` and `after_link` say count + 1 (one per `read_dir_plus`
  step); a new node starts at 1 (`inode.rs:226`); `forget(n, k)` gives
  `count' = count -sat k` and removes the node iff `count' == 0` (never the root).

**Bounded replies** (optional, no host model). `read` may promise
`r.len() <= arg.size`: `truncate` never grows (`lib.rs:584`). `read_dir`'s bound rests
on `DirEntryWriter` checking `capacity()` (`fuse/reply.rs:144`). `try_reserve_exact`
may reserve more than asked, so either the bound uses the capacity, or the code clamps.

## 3. How it fits the existing structure

**The invariant goes in virtio-fs's `inv` body** (`lib.spec.rs:380`). Every contract
already ensures `final(g).wf()`, which includes `self@.inv()`. So with no contract
changed, every success also promises that the table had room. For example, `lookup`'s
`final(g)@ == s.after_reach(..)` together with `final(g).wf()` forces `after_reach(..)`
to be within the caps.

```rust
// VirtioFsGhostView, lib.spec.rs
closed spec fn inv(&self) -> bool {
    &&& /* existing facts */
    &&& self.microvm ==> self.within_limits()
}
/// The microVM's caps (saved_state.rs:20-25), which keep the saved state bounded.
closed spec fn within_limits(self) -> bool {
    &&& self.inodes.dom().finite() && self.files.dom().finite()
    &&& self.inodes.len() <= 4096                       // MAX_INODES
    &&& self.files.len() <= 4096                        // MAX_HANDLES
    &&& forall|n: u64| #[trigger] self.inodes.contains_key(n) ==>
            self.names_of(n).finite() && self.names_of(n).len() <= 256
    &&& self.name_count(self.inodes.dom()) <= 16 * 1024 // MAX_ALIASES
    &&& self.name_bytes_ok()                            // cfg(unix): <= 1 MiB
}
/// The number of names of the nodes in `ns`, one node at a time.
closed spec fn name_count(self, ns: Set<u64>) -> nat decreases ns.len() {
    if ns.finite() && ns.len() > 0 {
        let n = ns.choose();
        self.names_of(n).len() + self.name_count(ns.remove(n))
    } else { 0 }
}
// name_bytes(ns): the same, summing Self::encoded_len over each node's names.
```

**Hidden keys do not count, by construction.** The sums run over
`self.inodes.dom()`, the view's node ids, and use `names_of`, which reads
`aliases@[t.keys[n]]` only for table nodes. Two kinds of entry are left out:

- the ghost entries of an inode that `lookup_child` built and `InodeMap::insert` then
  dropped (dedup);
- the entries of a forgotten node that an open file still keeps.

The code skips both too: it counts `inodes_by_node_id.values` (`limits.rs:144`). The
one thing to watch is that a sum over the `GhostMapAuth` domain would be wrong; the sum
must be over the view. Handles are different: a handle of a forgotten node still counts,
in both the code (`files.values.len()`) and the view (`files`).

**Locks.** The preflight reads under a read lock and releases it; the insert comes
later, under a write lock. The proof carries the result across because `g` holds the
other half of each table's `FracGhost` and the `aliases` `Auth`. Nobody can change the
table or a name set without `g`, which is the request's `&mut`. This needs no new rule.

**New FUSE-level questions** (`fuse/lib.spec.rs`, in the same shape as
`bad_symlink_target`):

```rust
/// Making a new node at `p` would pass one of the file system's caps (`code`).
spec fn no_room_for_node(&self, p: PathView, code: i32) -> bool;
/// Adding the name `p` to node `m` would pass a cap.
spec fn no_room_for_name(&self, m: u64, p: PathView, code: i32) -> bool;
/// The names after renaming `p` (node `n`'s share) to `q` would pass a cap.
spec fn no_room_after_rename(&self, n: u64, p: PathView, q: PathView, exchange: bool, code: i32) -> bool;
/// One more open handle would pass a cap.
spec fn no_room_for_handle(&self) -> bool;
```

virtio-fs's answers (`lib.spec.rs`), for example:

```rust
closed spec fn no_room_for_node(&self, p: PathView, code: i32) -> bool {
    self.microvm && {
        ||| self.inodes.len() >= 4096 && code == lx::ENOSPC
        ||| self.name_count(self.inodes.dom()) >= 16 * 1024 && code == lx::ENOSPC
        ||| self.name_bytes(self.inodes.dom()) + Self::encoded_len(p) > 1024 * 1024
            && code == lx::E2BIG                          // cfg(unix) only
    }
}
closed spec fn no_room_for_name(&self, m: u64, p: PathView, code: i32) -> bool {
    self.microvm && !self.names_of(m).contains(p) && {
        ||| self.names_of(m).len() >= 256 && code == lx::ENOSPC
        ||| /* the total, and the bytes, as above */
    }
}
// no_room_after_rename: the byte total of every node's names after the rename
// (exchange: xs.map(exchanged); otherwise xs.filter(!q-prefix).map(moved)),
// or one such name with encoded_len > 4096, gives E2BIG. Never ENOSPC (point 2).
```

**Rule sets that change:**

- `mkdir`, `mknod`, `symlink`: `entry_errors` gains
  `holds_node(n) && no_room_for_node(child_path(n, name), c)`. Better: a new set
  `make_errors`, so that `unlink` and `rmdir`, which share `entry_errors`, do not get it.
- `link_errors`: gains `no_room_for_name`.
- `rename_errors`: gains `no_room_after_rename`.
- `open_errors`, `create_errors`, and `open_dir_errors` (when `!is_synthetic(n)`):
  gain `no_room_for_handle() && c == lx::ENOSPC`.

No success clause changes. The FUSE level does not mention aliases: "names" there means
`names_visible`'s names.

## 4. Decisions for the lead

- **D1. How much to promise?** (a) the invariant only; (b) the invariant plus the error
  rules of section 2(b); (c) also node rules for lookup and create (needs the host
  model). **Recommend (b).** (c) is not possible yet.
- **D2. Lookup counts?** (a) leave them out of limits and re-scope open item 2 to
  forget-correctness, liveness and snapshots; (b) add them now, as decided on
  2026-11-10. **Recommend (a).** The premise of that decision ("the limits property
  needs them") does not hold, and the counts cost a code change (vstd atomics, the
  `forget` rewrite, overflow).
- **D3. Gap A, the dedup type check?** (a) a code change: in
  `preflight_microvm_insert`, a key whose node has another file type goes to the
  new-node check, as `dedup_entry` does (about 3 lines and a test); (b) a trusted
  assumption that a key's node has the host object's type (host-model territory);
  (c) leave the cap on nodes out of the invariant. **Recommend (a)**, and tell Pedro.
- **D4. Gap B, listing totals only at save?** (a) record it as a finding for Pedro
  (BUGS.md) and list it among save's failure reasons, with the caches staying out of
  the spec; (b) a code change that checks the totals when a cache is built (E2BIG),
  with each handle's cache size added to the view later. **Recommend (a) now**; ask
  Pedro whether he wants (b).
- **D5. The byte cap on Windows?** (a) Linux only, like `path_too_long`; (b) an
  uninterpreted `utf16_len`. **Recommend (a).**
- **D6. Where the caps live?** (a) in virtio-fs's `inv` (no contract text changes);
  (b) explicit "had room" success clauses. **Recommend (a)**, plus a one-line doc
  comment on `wf`. Use literals with comments, as `path_too_long` does, until the
  `saved_state` constants are declared to Verus.
- **D7. The request bound** (`virtio.rs:474`, which drops the request)? (a) state it
  in layer 0/1 with the queue spec; (b) leave it out. **Recommend (a)**, later. It is
  not a FUSE reply.

## 5. Code changes, risks, size

**Code changes.**

- Needed: D3(a), about 3 lines in `microvm/limits.rs` plus a test.
- Optional: D4(b); D2(b) (the counts: about 30 lines in `inode.rs` and `lib.rs`).
- None for the spec text itself.

**Risks.**

- *Sums over `HashMap` iteration.* `microvm_alias_usage` and `preflight_microvm_rename`
  iterate in any order, so a lemma must show that an order-free sum over a set equals
  the loop's sum. This is the main proof cost.
- *The rename's simulation.* The proof must show that the simulation in
  `limits.rs:96-118` gives exactly the `renamed` state, with set collisions merging.
- *Checked additions.* The code's `checked_add` and `is_none_or` must line up with
  `nat` arithmetic.
- *Finiteness.* `inv` must state that the maps are finite.
- *Windows bytes.* The byte cap is left out there (D5).

**Size.**

- Spec: `within_limits`, `name_count` and `name_bytes` (~45 lines); 4 FUSE questions
  (~20); answers (~50); rule-set edits (~20). About 135 lines in all.
- Counts, if chosen: about 80 more spec lines.
- Proof, later: contracts and loop invariants for the 5 preflight functions and 2
  reservation inserts, plus the sum lemmas, roughly 400-600 lines.

# Review of the virtio-fs FUSE spec against the code (2026-11-11)

Branch `system-proof/virtiofs-spec-20261108`, read-only review. I judged
every claim against the code. I used the design documents only for
vocabulary, and to leave out gaps they already list (open items 1-7, the
pinned identities, the root-dedup case of item 3, non-microVM aggregates,
restore). Paths are relative to the repository root.
`fuse/` = `vm/devices/support/fs/fuse/src/`,
`vfs/` = `vm/devices/virtio/virtiofs/src/`,
`lxutil/` = `vm/devices/support/fs/lxutil/src/`.

Findings are ordered by severity.

---

## 1. `rename` of a name onto itself: the contract demands that names be dropped, and the code (after bug fix 2) keeps them

*Status (2026-11-11): confirmed; fixed in the spec (`renamed`: `p == q` leaves the state as it is).*

- Spec: `fuse/lib.rs:434-447` (`s.renamed(n, p, q, is_exchange(flags), final(g)@)`),
  `vfs/lib.spec.rs:504-511` (`renamed`, the non-exchange branch:
  `unlinked(n, q, mid)`, then `map_names(moved p->q)`).
- Code: `vfs/lib.rs:686-691` (`if old_path == new_path { return Ok(()); }`,
  which is code change 16).
- Scenario: `lookup(1, "a")` gives node 2 with `aliases[2] = {[a]}`. Then
  `rename(1, "a", 1, "a", flags = 0)`. The code passes every check and returns
  `Ok(())` without touching the table, so `final(g)@ == old(g)@`. The
  contract needs a `mid` with `unlinked(1, [a], mid)`, which forces
  `mid.aliases[2] = {}`, and then `final == mid.map_names(moved([a], [a]))`,
  which is the identity map. So `final.aliases[2] = {}`, but the code leaves
  `{[a]}`. Bug fix 2 changed the code to match the intent, but the spec still
  describes the old, buggy behaviour. (The exchange branch is fine:
  `exchanged(x, p, p) == x`.)
- Class: **FALSE**. Confidence: high.
- Fix: in `renamed`, make the non-exchange case with `p == q` mean
  `after == *self` (or define it as `exchanged` when `p == q`). Equivalently,
  in the contract's `Ok` branch: `p == q ==> final(g)@ == s`.

## 2. `link` to a node that has no names left breaks `inv`, so `final(g).wf()` is false (and the code links the wrong object)

*Status (2026-11-11): confirmed; fixed in the code (code change 26, `BUGS.md` bug 4: `link` refuses a target with no names, `ENOENT`) and the spec (question `has_names`, in `link`'s rules and success).*

- Spec: `fuse/lib.rs:480-489` (`final(g)@ == s.after_link(target, ...)`,
  plus `final(g).wf()`). `vfs/lib.spec.rs:480-482` (`after_link` adds `p` to
  `aliases[m]` and leaves `paths[m]` alone). `vfs/lib.spec.rs:381-385`
  (`inv`: `!aliases[m].is_empty() ==> aliases[m].contains(paths[m])`).
- Code: `vfs/lib.rs:534-570` (`link`), `vfs/inode.rs:575-579` (`add_alias`
  inserts the alias and never sets the main path), `vfs/inode.rs:582-594`
  (`remove_alias_prefix` keeps a stale main path once the last alias is
  gone).
- Scenario: guest actions only, no host race, Linux or microVM:
  1. `lookup(1, "a")` gives node 2 (object A): `paths[2] = [a]`,
     `aliases[2] = {[a]}`.
  2. `unlink(1, "a")`: `aliases[2] = {}`, `paths[2] = [a]` (kept).
  3. `create(1, "a")` makes a new object X, which gets a new node because
     its inode number differs.
  4. `link(1, "b", target = 2)`:
     - `get_inode(2)` passes, because `lstat("a")` now finds X.
     - `readonly()` passes.
     - `link_stat(target.get_path() = "a", "b")` links **X** as "b".
     - `add_alias("b")` runs.

  Now `aliases[2] = {[b]}` and `paths[2] = [a]`. `after_link` computes
  exactly this, but this state violates `inv`, so `final(g).wf()` cannot
  hold. There is also a real code bug: node 2 (object A, whose reply carries
  A's guest inode number) now has a name of object X. Step 3 can also be
  `rename(1, "x", 1, "a")`, which replaces "a".
- Class: **FALSE** (`wf` in the ensures), plus a code bug. Confidence: high.
- Fix: in the code, refuse `link` from a target with no names (`ENOENT`, as
  Linux does for an unlinked file), or set the main path when the first
  alias is added. Make `after_link` match whichever you choose (for
  example, `aliases[m].is_empty() ==> paths[m] = p`).

## 3. A rename of a directory moves (and lets the guest then delete) a hidden path below it: the spec allows it and so does the code

*Status (2026-11-11): confirmed by a test against the code; fixed in the code (code change 27, `BUGS.md` bug 5: `rename` refuses a path with a denied path below it, `EROFS`) and the spec (question `hides_below`, in `rename`'s rules, which now take the flags, and its success).*

- Spec: `fuse/lib.rs:434-447`. The rename succeeds when `can_write(n, p)`
  and `can_write(new_dir, q)` hold, and nothing constrains the paths
  *below* `p`. `microvm/policy.spec.rs` (`can_write`'s doc: "a denied path
  inside a writable one hides part of it, and the guest must not change
  what it cannot see"). The integrity property (CODE-OVERVIEW, property 2:
  "a name created, removed or renamed succeeds only through a path the
  guest may write").
- Code: `vfs/lib.rs:663-710` (only `old_path` and `new_path` are checked).
  The pinned-identity check runs only when a node is made
  (`vfs/microvm/inode.rs:69-86`, through `VirtioFsInode::new`). It does not
  run in `unlink` (`vfs/lib.rs:997-1010`) or in listings
  (`vfs/microvm/file.rs:106-112`).
- Scenario: a read-write microVM share whose policy denies `d/secret`
  (`denied_ok` accepts it; no writable list).
  1. `rename(1, "d", 1, "e", 0)`. Both paths are Visible and writable. The
     host moves `d/` together with `secret`.
  2. `read_dir` of `e` lists `secret` with its inode number: `e/secret` is
     Visible, and `listable_now` passes.
  3. `unlink(node_e, "secret")`. `can_write([e, secret])` holds, and the
     host deletes the hidden file.

  `lookup("e/secret")` is still refused, because the object is pinned. But
  the hidden object's name leaks, and the hidden object is moved and then
  removed. `RENAME_EXCHANGE` of `d` with an empty writable directory does
  the same.
- Class: **TOO WEAK** (the spec permits what the integrity property
  forbids; the code does it). Confidence: high on the mechanics, medium on
  whether this is intended.
- Fix: require in `rename` (and in the spec's success clause) that no
  policy path that is hidden or not writable lies strictly below `p` or `q`
  (for example `forall d in denied: !p.is_prefix_of(d) || ...`). This needs
  only the policy, not the host.

## 4. Windows: `child_path` ignores lxutil's escaping, so every path equality is false for names with escaped characters

- Spec: `vfs/lib.spec.rs:400-402` (`child_path = path_of(parent).push(name)`).
  Every `after_reach`, `may_reach`, `can_write`, `unlinked` and `renamed`
  uses it. `lxutil/lib.spec.rs` says Windows is "in the claim".
- Code: `vfs/inode.rs:670-671` (`path.push_lx(name)`).
  `lxutil/windows/path.rs:15-48` (`path_from_lx`) turns every character
  that is not legal on NTFS (`* ? " < > |`, controls, and so on) into a
  private-use code point (U+F000 + c).
- Scenario: on Windows (outside the microVM, which refuses only `\` and
  `:`), `lookup(1, "a*b")` succeeds with the code's path view
  `[a\u{F02A}b]`. The contract states
  `final(g)@ == after_reach(1, [a*b], ...)`, so the stored path differs
  from the spec's. Two different guest names ("*" and "\u{F02A}") also
  reach one host path, while the spec counts them as two paths.
- Class: **FALSE** on Windows. Confidence: high.
- Fix: have the spec's `child_path` apply the escaping (a spec function for
  `path_from_lx`, with `push_lx`'s contract stating it), or refuse every
  escapable byte in the microVM, as is already done for `\` and `:`, and
  restrict the Windows claim to the microVM.

## 5. Windows: case-insensitive (and 8.3) host lookup lets a writable path name a hidden object

- Spec: the path-level integrity and confidentiality claim, and the trusted
  text in `lxutil/lib.spec.rs` ("touches only objects beneath the volume's
  root"). The policy is matched component-wise and case-sensitively:
  `microvm/policy.rs:208-238`.
- Code: `lxutil/windows/util.rs:110` (`OBJ_CASE_INSENSITIVE` is always
  set). `vfs/microvm/inode.rs:47-61` (`ensure_path_allowed` compares paths
  exactly).
- Scenario: a read-write Windows microVM share denies `secret` (and the
  directory is not case-sensitive). `unlink(1, "SECRET")`: `[SECRET]` is
  Visible and writable to both the code and the spec, and NTFS deletes
  `secret`. Short names such as `SECRE~1` behave the same way on volumes
  with 8.3 names enabled. `lookup` is still caught by the pinned
  identities, but `unlink`, `rename` and `create` with `O_EXCL` are not.
- Class: **TOO WEAK / DOC MISMATCH**: the contracts hold literally, but the
  trusted assumption that distinct paths are distinct objects is false on
  Windows, and the documents do not list it. Confidence: medium (it depends
  on the per-directory case-sensitivity setting).
- Fix: add this to the trusted assumptions (and to the questions for
  Pedro), or have the microVM open its volumes case-sensitively and refuse
  names that are not canonical.

## 6. `write` (and `fallocate`/`copy_file_range`/`setup_mapping`, defaults): `opened_for_writing(fh)` is never checked by the code

- Spec: `fuse/lib.rs:565` (`r is Ok ==> uses_handle && opened_for_writing(fh) && file_can_write(fh)`).
- Code: `vfs/lib.rs:591-598`. The code checks the size, `get_file` and
  `check_writable(file.inode())`, then calls `file.write` → `pwrite`.
  `open_flags` is read nowhere on this path (`grep open_flags`: only the
  saved state uses it).
- Scenario: `open(n, O_RDONLY)` gives `fh`, then `write(n, fh, data)`.
  Only the host's fd mode makes `pwrite` fail, and `lxutil` has no
  postcondition, so the proof cannot rule out `Ok`.
- Class: **UNPROVABLE**. Confidence: high.
- Fix: add `if !opens_for_writing(file.open_flags()) { return Err(EBADF) }`
  to `write` (one comparison), or drop the conjunct from the contract.

## 7. `mkdir` and `symlink` fix the new node's kind (`S_IFDIR`, `S_IFLNK`), but the code takes it from a by-name stat after creation

- Spec: `fuse/lib.rs:333` (`after_reach(..., lx::S_IFDIR)`) and
  `fuse/lib.rs:257` (`..., lx::S_IFLNK`). `after_reach` writes `kind` into
  the new inode's view.
- Code: `vfs/inode.rs:426-431` and `vfs/inode.rs:467-473` (`with_attr`
  uses `stat.mode & S_IFMT`). `lxutil/unix/mod.rs:116-120` and `150-159`:
  `mkdirat` / `symlinkat`, then `stat_at` by name, which the doc says is
  "susceptible to a race".
- Scenario: the host replaces the new directory with a file between
  `mkdirat` and `fstatat`. The inode table then gets a node with kind
  `S_IFREG`, while the contract says `S_IFDIR`. Even without a race, no
  lxutil postcondition says the stat is a directory, so this is
  unprovable. `mknod` and `create` already take the reply's kind.
- Class: **UNPROVABLE** (and FALSE under a host race). Confidence: high.
- Fix: use `entry.attr.mode & lx::S_IFMT` as `mknod` does, or have the code
  check the stat's type (`EIO` otherwise) before inserting.

## 8. `rename` with `RENAME_WHITEOUT` still makes a 0:0 character device in the microVM

- Spec/doc: `virtiofs-spec-docs/BUGS.md:332-346` and `CODE-CHANGES.md` row
  24 ("The check closes it": a whiteout that would hide a lower overlayfs
  file). The rename contract (`fuse/lib.rs:434-447`) says nothing about
  the flags beyond `is_exchange`.
- Code: `vfs/lib.rs:692-694` passes `flags` through unchanged.
  `lxutil/unix/mod.rs:236-265` calls `renameat2(..., flags)`. Nothing in
  `microvm/` filters it.
- Scenario: `rename(1, "a", 1, "b", RENAME_WHITEOUT = 4)` on a writable
  share. Linux (5.8 and later) leaves a whiteout, a 0:0 character device,
  at `a` without `CAP_MKNOD`. That is the very device that
  `validate_mknod_mode` was added to refuse.
- Class: **DOC MISMATCH / TOO WEAK** (a gap in code change 24, which the
  spec does not catch). Confidence: medium-high (it relies on the
  unprivileged-whiteout behaviour of current kernels).
- Fix: in the microVM, refuse any rename flag other than
  `RENAME_NOREPLACE | RENAME_EXCHANGE` (`EINVAL`), and add a `rename_errors`
  rule for it.

## 9. `create`'s contract does not carry bug fix 3 (truncating an existing node through one writable name)

- Spec: `fuse/lib.rs:932-940`. On success it requires only
  `may_add_name(n, name)`. Nothing requires that `node_can_write(m)` holds
  when an existing `m` is opened for writing or with `O_TRUNC`.
- Code: `vfs/lib.rs:421` and `vfs/lib.rs:860-892`
  (`check_create_existing`: `EROFS` if the held node has a name that is not
  writable).
- Scenario: node 5 has the names `rw/f` and `ro/f`. `create(rw, "f", O_TRUNC)`
  is refused by the code, but the contract allows success, and so allows
  truncating a file with a read-only name. This is the integrity property
  that bug 3 was about.
- Class: **TOO WEAK** (security relevant). Confidence: high that it is
  missing. Note that a strengthened clause would be hard to prove, because
  the check uses an `lstat` by path, before the host create, so it needs
  the host model.
- Fix: add `(opens_for_writing(flags) || truncates(flags)) && s.holds_node(m) ==> s.node_can_write(m)`
  once the host model can tie the pre-check to `m`. Until then, record it
  as an open item.

## 10. `SYNTHETIC_ROOT_FH = u64::MAX` can be handed out by the file table

- Spec: `fuse/lib.rs:526`, `790` and `937` (`!s.holds_file(out.fh)`).
  `vfs/lib.spec.rs:516-518` and `587-589` (`u64::MAX` is always held in an
  aggregate).
- Code: `vfs/lib.rs:1281-1283` (`can_insert` allows `u64::MAX`; the test
  `handle_map_stops_after_max_handle` hands it out). `vfs/aggregate.rs:35-39`
  claims "can't collide". `vfs/microvm/fs.rs:785` and
  `microvm/state.rs:587` (after a restore, `next_handle_id` is bounded only
  from below).
- Scenario: in an aggregate whose file `next_handle` is `u64::MAX` (after
  2^64-2 opens, or a saved state with that value), `open(n)` returns
  `fh = u64::MAX`, which is already held, so `!holds_file(out.fh)` is false.
- Class: **UNPROVABLE** (FALSE in reachable edge cases). Confidence: high.
  Practical severity: low.
- Fix: have the aggregate's file map refuse `u64::MAX` (stop at
  `u64::MAX - 1`), and correct the comment.

## 11. lxutil's preconditions need "every stored path is a share path", which no core or `inv` states

- Spec: `lxutil/lib.rs:141` and the other `requires LxVolume::is_share_path(...)`.
  `vfs/inode.spec.rs` (the `PathRule`/`AliasesRule` cores fix only the id,
  key and value). `vfs/lib.spec.rs:381-387` (`inv`).
- Code: `get_path()` is passed to `lstat`, `open`, `stat_fs`, `link_stat`
  and others. `renamed_path` and `exchanged_path` build new paths.
- I found no call site that passes a path that is not plain (on Unix,
  `child_path` refuses `/`, NUL, `.` and `..`, and `push` adds one
  component; restore uses `validate_relative_path(.., true)`). But the
  proof of every host call needs the fact, and it is stated nowhere.
- Class: **UNPROVABLE as stated** (the TODOs allow it to be added).
  Confidence: medium. Severity: low.
- Fix: add `LxVolume::is_share_path(v.path@)` (and the same for every
  alias) to the two lock rules' cores, or to `inv` over `paths` and
  `aliases`.

## 12. Smaller documentation mismatches

- `fuse/lib.spec.rs:236-238`: `after_release` "is no longer held". For
  virtio-fs's synthetic handle (`release_dir(1, u64::MAX)`, or `release`
  of `u64::MAX`), `holds_file` stays true. The contracts are fine. Reword
  it as "removed from the open files".
- `vfs/aggregate.rs:37-38` comment: see finding 10.
- `vfs/lib.rs:689-691` (STYLE/behaviour): the early return for
  `old_path == new_path` also skips flag checks, so `RENAME_NOREPLACE`
  returns `Ok` where Linux returns `EEXIST`, and unknown flags return `Ok`
  where Linux returns `EINVAL`. No contract is affected.

## Other observations (code, not contract)

- `vfs/microvm/file.rs:81-87`: in a microVM `read_dir_plus`, a lookup error
  (with `?`, including `ENOENT` from a deletion after the cache was built),
  the `ino` mismatch `EIO`, and the `dir_entry_plus` `EIO` all discard a
  buffer whose earlier entries already took lookup references. Those
  references are never forgotten. The non-microVM path
  (`vfs/file.rs:233-240`) carefully avoids this. The spec allows it
  (`looked_up_in` also on an error), but it leaks toward `MAX_INODES`.
- `restore_microvm_state` replaces both tables without touching the ghost
  state, so the keeper's assumption A2 does not hold after a restore. This
  is known (property 6, not started). I list it because `fs_ghost_t.rs`
  does not mention it.

---

## What I checked and found consistent

- **`lookup`**:
  - The direct and microVM paths: `child_path`'s checks against
    `is_entry`, `refuses_name`, `path_too_long` and `is_hidden`, and the
    order of the error codes.
  - `may_reach`: Hidden through `ensure_path_allowed` (unconditional), the
    traverse-only ⇒ directory rule and the exposed ⇒ regular-file rule
    through `ensure_object_allowed`.
  - The reply kind against `file_type`.
  - Dedup (`InodeMap::insert`/`insert_reserved`): `existing.lookup(p)`
    matches `after_reach`'s existing branch. Eviction on a type change
    gives a new node, and a failed insert changes nothing in the view.
  - The synthetic root: the shares, `ENOENT`, and `q = share.root` with
    `share_view` against the inode's view.
- **`forget`, `destroy`**: the root is never removed. `clear()` keeps the
  direct root and empties the aggregate, matching `after_destroy`.
- **`get_attr`/`get_statx`**:
  - The `FH` and synthetic handle cases.
  - `handle_errors` against `get_file`'s order (`EBADF`, `EINVAL`, `EACCES`,
    and `EBADF` for another node's handle).
  - `u64::MAX` with a node other than 1.
- **`set_attr`**: the `changes` mask; `readonly()` against `node_can_write`
  and `file_can_write` (`policy_permits_writes` with `restricts_writes`:
  with deny-only policies every visible path is writable, and hidden names
  are caught first by `names_visible`).
- **`open`/`open_dir`**:
  - `util::open_flags` against `bad_open_flags`/`bad_create_flags`.
  - `check_open_readonly` against `EROFS`, and access mode 3.
  - The file's `open_flags` keeps the access mode.
  - The synthetic `open_dir` changes nothing.
- **`create`, `mknod`, `mkdir`, `symlink`**: the error order;
  `may_add_name`; `refuses_mknod`; `bad_symlink_target`. In the dedup case,
  `after_reach` then `after_open` uses the existing inode's view, and
  `FileTableRule` uses `arg.flags` for `create`. A failure after the inode
  insert is impossible given the preflight checks and the stable file
  table.
- **`unlink`/`rmdir`**: `remove_alias_prefix` against `unlinked` (a main
  path under `p` takes an arbitrary remaining name, or stays; other volumes
  are untouched; the root's `""` alias is unaffected).
- **`rename`**: `rename_alias_prefix` and `exchange_alias_prefixes` against
  `moved`/`exchanged`, including `renamed_path`'s empty suffix and alias
  collisions; `EXDEV`/`EROFS`/`ENOTDIR` (`ENOTDIR` through `child_path`).
- **`link`**: other than finding 2, the error rules, `entry.nodeid == target`
  and the synthetic target.
- **`read`, `write` (size), `fsync`, `fsync_dir`, `statfs`, `read_link`**:
  the error rules and the synthetic cases.
- **xattr operations**: `ENODATA`/`EROFS` for the synthetic root, `E2BIG`
  ordering, and the `size == 0` dispatch in `Session`.
- **Listings**:
  - The `ListedEntry::record` layout against `DirEntryWriter`:
    `fuse_direntplus` puts `entry_out` (128 bytes) first, then the header
    fields in order, with padding.
  - The plus-mode kind comes from the lookup's attr.
  - `may_list` against `listable_now`/`entry_listable` (traverse-only
    entries are dirs, and `DT_UNK` is trusted, as documented).
  - Outside the microVM the policy is always `SubtreePolicy::default`
    (`new_with_strict_paths` is the only setter), so everything is visible.
  - Listings of the synthetic root.
- **The ghost state**:
  - `InodeTableRule`/`FileTableRule` cores against what the locks hold.
  - `owns` (microVM ⇔ every volume strict, through `open_microvm_volume`).
  - The `wf` keys and the view excluding node 1.
  - `is_new` for `new_microvm`.
  - The `RegistryRule` (names are unique and plain through the profile).
  - `DormantMicrovmFs` with the default bodies.
- **lxutil call sites**: every path comes from `child_path`, `get_path`,
  `root_entry` or a component-wise prefix. On Unix none is absolute,
  contains `.` or `..`, or is empty except for the root.

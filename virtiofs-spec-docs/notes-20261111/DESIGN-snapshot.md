# virtio-fs snapshot (save and restore): design-prep notes (2026-11-11)

These notes prepare property 6 of `CODE-OVERVIEW.md` ("save records no host
paths, and restore accepts only the same host objects"). They cover Pedro's
code at `db42fde04`. Nothing in the code was changed. Paths are relative to
`vm/devices/virtio/virtiofs/src/` unless they begin with another crate.

## 0. Who makes the saved state, who reads it, and whether it is trusted

- **Who makes it.** The transport stops the device, and the device task calls
  `save_device` (`virtio/src/transport/task/restore.rs:52`). That is
  `microvm/device.rs:187-218`, which refuses while a worker still has state
  (`:196`) or a request is still admitted (`:201-204`), then calls
  `save_microvm_state` (`microvm/fs.rs:490-677`). The transport keeps the blob
  (`transport/core/restore.rs:63-75`), and MMIO `save` puts it into its own
  saved state (`transport/mmio.rs:508`), which goes into the VM's snapshot.
- **Who reads it.** MMIO `restore` (`mmio.rs:513-533`) first runs
  `restore_common`, which calls the device's validator
  (`core/restore.rs:212` -> `microvm/device.rs:257-295`), so
  `validate_microvm_state` runs before any transport state changes. Then the
  `DeviceCommand::Restore` command reaches `DeviceTask::restore_device`,
  which **requires that no queue has started**
  (`transport/task/restore.rs:26-29`), and calls `microvm/device.rs:220-255`.
  That validates again (`:247`) and calls `restore_microvm_state`
  (`microvm/fs.rs:680-837`), which validates a third time (`:690`).
- **Is it treated as untrusted? Yes, in practice.** No comment says
  "untrusted", but:
  - it is validated three times, with about 40 `ensure!` checks that would
    be pointless for a blob the VMM itself wrote;
  - the tests edit a saved blob by hand and expect restore to refuse it:
    `policy_tests.rs:743-760` (a writable flag on a read-only handle),
    `symlink_tests.rs:562-593` (a forged symbolic-link handle),
    `fs_tests.rs:265-282` (a `..` alias), and
    `aggregate_fs_tests.rs:930-948` (aliases changed to `secret.json`, `""`,
    and a path under the file).
- **Consequence.** The *policy checks* hold against any blob. The *identity
  checks* compare the host against numbers taken from the blob
  (`validate_identity`, `microvm/state.rs:149-164`), so against a forger they
  prove nothing: anyone can `stat` the file and copy its numbers. "The same
  host objects" therefore holds only for an authentic blob, which is an
  assumption about where snapshots are stored, outside virtio-fs.

## 1. What save records, and what restore does

**Save** (`microvm/fs.rs:490-677`) changes nothing on the device:

1. The profile and the layout must match (`:495-503`; the layout check is
   `aggregate().is_some() == profile.is_aggregate()`).
2. The inode table, under its read lock (`:505-571`). For each node:
   - 1 to 256 aliases, with totals of at most 16 Ki aliases and 1 MiB
     (`:516-527`);
   - each alias re-checked on the host (`validate_reopenable_alias`, `:531`):
     not hidden, and no ancestor a symbolic link;
   - all aliases stat to the same (dev, ino, kind) (`:533-538`);
   - it records `SavedInode{node_id, volume_id, relative_aliases,
     lookup_count, guest_inode_id, object_identity}` (`:550-557`), and then
     `next_node_id`.
3. The file table (`:574-626`). Each handle's inode must be a table node
   ("untracked inode", `:581-583`). It records the file's `fstat` (kind and
   identity), `open_flags`, and the cached listing (`directory_entries`,
   `built`), with totals of at most 16 Ki entries and 4 MiB (`:600-612`).
4. The session's negotiation must equal the file system's copy (`:629-633`).
5. The policy, as share-relative strings (`saved_policy_paths`,
   `state.rs:137-147`), and the aggregate's children with their root
   identities. A root identity is the bytes of (dev, ino), not a path
   (`profile.rs:677-702`).

No host path is recorded: `root_path` is consumed in `new_microvm` and never
stored (`fs.rs:250-251`; `saved_state.rs:6-8`).

**Restore, checks on the blob alone** (`validate_microvm_state`,
`state.rs:330-592`):

- *Header* (`:334-430`): schema, not dormant, aggregate layout, attachment
  id, root identity, access mode, owner mode; **the policy equals the
  profile's** (`:365-385`; for an aggregate, the whole children list); the
  fixed ABI (queues, no DAX, timeouts 0, request size); at most 4096 inodes
  and 4096 handles, `next_*_id != 0`; the negotiation
  (`SessionState::validate`, `fuse/src/session/saved_state.rs:77-115`, plus
  the exact microVM `want` flags).
- *Inodes* (`:432-521`): ids nonzero and unique; a known volume
  (`volume_policy`, `:315-328`); `lookup_count != 0`; 1 to 256 aliases within
  the global bounds, each at most 4096 bytes and decoding to plain components
  with no NUL, `\` or `:` (`validate_relative_path`, `:702-734`); a root's
  aliases are all `""` and no other node has `""`; no (volume, alias) twice;
  one node 1 in a single share, none in an aggregate (its synthetic root);
  `next_node_id > max id`.
- *Handles* (`:523-589`): ids nonzero and unique, on a known node; kind equals
  the identity's, and is not a symbolic link; `reopen_flags` (`:594-631`: no
  `O_TMPFILE`, no unknown flag, not `O_NOACCESS`; strips
  `O_CREAT|O_EXCL|O_TRUNC`); **no writable handle on a read-only volume**
  (`:543-548`); **if the policy restricts writes, a writable handle only on a
  node all of whose aliases are writable** (`:438-509`, `:549-554`); the
  directory snapshot bounded, with valid names and cookies
  (`file.rs:166-200`), only on a directory; `next_handle_id > max id`.

**Restore, reopen on the host and rebuild** (`microvm/fs.rs`):

1. The volumes are the *current* ones (`:696-717`). For a single share,
   saved node 1 must have the current root's identity (`:700-715`).
2. The inode table (`:719-783`). For each saved node:
   - each alias is checked again (plain, root or not, no duplicate), reopened
     with `validate_reopenable_alias` (`:752`, `state.rs:179-209`: not
     hidden, via `ensure_path_allowed`, `microvm/inode.rs:47-61`, and
     `lstat` one component at a time, so no ancestor is a link), and must
     have the saved identity (`:754`);
   - `VirtioFsInode::from_saved` (`microvm/inode.rs:120-145`) rebuilds it:
     path = **the first alias**; `lookup_count` from the blob;
     `ensure_object_allowed` on every alias (pinned identities,
     traverse-only must be a directory, the single-file share's file,
     `:69-86`); new `path` and `aliases` locks (`with_attr`);
   - `guest_inode_nr` must equal the saved value (`:765-768`), and no two
     nodes may share a dedup key (`:770-773`);
   - finally `next_handle = next_node_id` (`:783`).
3. The file table: `HandleMap::starting_at(next_handle_id)` (`:785`). For
   each saved handle:
   - its node must be in the new table (`:792`);
   - `inode.open(reopen flags minus NOFOLLOW/LARGEFILE/CLOEXEC/NOCTTY)`
     (`:800`), which re-checks confinement and refuses a link
     (`inode.rs:388-393`). **This runs as OpenVMM** (F1 below);
   - `fstat` must match the saved identity and kind (`:804-813`);
   - `restore_directory_snapshot` (`file.rs:67-83`) validates the entries
     again, and allows a built snapshot only on a directory.
4. Commit, with nothing after it that can fail (`:824-835`):
   `session.restore_state`, then both tables replaced under their write
   locks, then `negotiation`. The old inodes, root included, and the old open
   files are dropped. If any earlier step fails, the device is unchanged.

A dormant blob restores only the session (`device.rs:234-241`); an active
blob on a dormant slot is refused (`:242-246`).

## 2. The security core: what a restored state must satisfy

Every FUSE contract requires only `self.owns(g)` and `g.wf()` (and the keeper
hands out exactly these, `fs_ghost_t.rs:80-92`). So the existing spec stays
sound after a restore if the following hold:

| Needed after restore | Where it comes from |
|---|---|
| `owns`: `g`'s table halves have the ids pinned by the **existing** locks' rules (`lib.spec.rs:331-345`) | Restore writes into the same `inner.inodes` and `inner.files` locks (`fs.rs:829-834`), so the proof updates both halves (§3). |
| `owns`: `g.microvm`, `g@.aggregate`, `g.shares` unchanged | `inner.mode`, the profile and the registry are untouched. |
| `InodeTableRule` core (`lib.spec.rs:262-266`): the half's inodes are the table's, and `aggregate` is the table's flag | **Restore relies on `profile.is_aggregate() == (mode is Aggregate)`** (`InodeMap::new(profile.is_aggregate())`, `fs.rs:719`), **but does not check it**; save does (`fs.rs:499-503`). It holds by construction (`fs.rs:281-285, 332-335`), so it belongs in `VirtioFs::wf()`. |
| `FileTableRule` core (`lib.spec.rs:288-297`): `writable == opens_for_writing(open_flags)` | The flags stored are the ones the file was really reopened with (`inode.rs:392`). |
| The path and aliases rules: each new inode's locks hold its entries of `paths` and `aliases`; the root's rule is "path empty, names {empty}" | For a single share, restore builds a **new** node-1 inode with `from_saved`, so `from_saved` needs a ghost argument `is_root` to pick the root's rule (no runtime change). Its alias set is `{""}`: `state.rs:489-505`. |
| `inv` (`lib.spec.rs:380-387`): every non-root node has a path and names, and the path is among the names | Aliases are non-empty (`state.rs:457-461`), and path = first alias (`microvm/inode.rs:126`). |
| `wf`'s key facts (`lib.spec.rs:305-319`) | Proof only: fresh keys from `g.next_key`, as in `lookup`. |
| Each handle's recorded node holds the file's inode (a `FileTableRule` TODO) | `restored_inodes.get(saved.node_id)` (`fs.rs:792`). |

**The policy facts need not survive a restore, because every request
re-checks them:**

- names visible, and no link on the way: `get_inode` ->
  `validate_confined_object` (open item 6);
- a write through a handle: `write` calls `check_writable(file.inode())`
  again (`lib.rs:595-596`), and the contract asks for `file_can_write` at
  that moment (`fuse/src/lib.rs:565`);
- the listing rule: each reply judges the cached entries again (`file.rs`
  microvm, `:57-61`, code change 23). A forged blob picks the cached names
  and kinds, but the spec already trusts the record's kind (open item 4),
  and the share's attach refuses a traverse-only path that is not a
  directory (`fs.rs:215-220`).

So restore's policy checks are defense in depth for the spec, not needed for
its soundness. They still deserve their own clause (D2), which they do
establish:

- every restored name is visible: `validate_reopenable_alias` and
  `ensure_object_allowed`;
- no writable handle on a read-only share: `state.rs:543-548`;
- every writable handle's node has all its names writable: `:549-554`. When
  `!restricts_writes`, `can_write(p)` is `visibility(p) == Visible`
  (`policy.rs:249-267`), so a visible alias suffices.

One proof obligation: that check uses the **profile's** policy
(`state.rs:315-328`), while the spec uses the **volume's**. They differ for a
single-file child (`fs.rs:185-201`), but agree on its only reachable path:
a lemma, not a bug.

**Findings** (found by reading the code; none was run).

- **F1. Restore reopens every handle as OpenVMM, not as its caller**
  (property 3; owner mode `Caller`, Linux).
  - A normal `open` runs as the caller's host uid
    (`microvm/owner.rs:112-131`). The blob records no caller, and
    `fs.rs:800` has no `run_as`, so the host checks OpenVMM's rights.
  - Scenario: `out/` is writable by policy, and `out/peer.db` belongs to
    host uid 2000 with mode 0600, so guest uid 1001 gets `EACCES` on open.
    An edited blob adds a `SavedHandle` on its node with `O_RDWR`. The
    policy checks pass (every name is writable), and OpenVMM (root, or the
    share's owner) opens the file. The guest's later `write` runs as 1001,
    but on an fd that is already open for writing, so it succeeds.
  - With an authentic blob the effect is only that OpenVMM's rights stand in
    for the opener's (for example, after a host `chmod` between save and
    restore). Its weight depends on D4. Question for Pedro: reopen as the
    export owner, or record the opener's uid and gid?
- **F2. A normal guest can make save fail.** Under CONVENTIONS "Errors",
  each case needs a stated reason or a fix.
  - (a) *An unlinked file that is still looked up or open.* `unlink` removes
    the name from the node's aliases (`lib.rs:1007` ->
    `inode.rs:582-594`), and the node stays in the table until `FORGET`.
    Save then bails: "inode has no bounded reopenable aliases"
    (`fs.rs:516-519`). So the common pattern `open("out/tmp");
    unlink("out/tmp")` (temporary files, SQLite journals) blocks every
    snapshot until the guest closes the file and forgets the node. A rename
    onto an existing name does the same to the node it replaces
    (`limits.rs:104`, and the code it mirrors). This may be by design (a
    nameless file cannot be reopened by path), but then save's contract
    must list it, and NVX should know.
  - (b) *Listings: no runtime limit for the save-time totals.* At runtime,
    each handle holds at most 4096 entries and 64 KiB (`file.rs:124,142`,
    microvm), over up to 4096 handles. Save refuses more than 16 Ki entries
    or 4 MiB in total (`fs.rs:600-612`; constants at
    `saved_state.rs:29-30`). Five handles that list a 3300-entry directory
    are enough. grep finds no other use of these two constants.
  - (c) A malicious guest `FORGET`s a node and keeps its handle: "open
    handle refers to an untracked inode" (`fs.rs:581-583`; see `BUGS.md` 1).
- Minor (limits, forged blob only): `lookup_count` comes from the blob
  unbounded, and `fetch_add(1)` wraps (`inode.rs:303,317`).
- Minor (round trip): after a restore, a node's main path is its **least**
  alias (save lists aliases in `BTreeSet` order, `fs.rs:515`, and
  `from_saved` takes the first). `paths[n]` may change across save/restore,
  `aliases[n]` does not. Harmless, but restore's ensures must say so.

## 3. The ghost state across a restore

The keeper holds `g` for the old tables. Restore rebuilds the tables, but in
the **same locks**, whose rules pin the old `FracGhost` ids for good
(`InodeTableRule.id` and `FileTableRule.id`, fixed when each lock was
created). A fresh `g`, as `new_microvm` makes one, can therefore never be
owned. Restore must **change the old `g`**, as `DESTROY` does
(`lib.rs:821-830`; `after_destroy`, `lib.spec.rs:536-546`):

- update both table halves under the write locks;
- start fresh `paths` and `aliases` `Auth`s, or insert into the current ones
  with fresh keys;
- give each new non-root inode's locks their `PointsTo`.

**Option A (recommended): restore borrows `g` from the keeper.**

```rust
// microvm/device.rs, restore_device (ghost code only; no runtime change)
let tracked g = take_fs_ghost(&device.fs);          // A1: no queue started
fs.restore_microvm_state(profile, state, &device.fs, Tracked(&mut g));
give_back_fs_ghost(&device.fs, g);

// microvm/fs.rs
fn restore_microvm_state(&self, profile, state: SavedState, session) -> (r: anyhow::Result<()>)
    with Tracked(g): Tracked<&mut VirtioFsGhost>
    requires self.wf(), session.owns_ghost(old(g)) /* = self.owns */, old(g).wf()
    ensures  self.owns(final(g)), final(g).wf(),
             r is Err ==> final(g)@ == old(g)@,
             r is Ok  ==> restored_ok(final(g)@, old(g)@, state@)   // D1
```

What it costs:

- **A2's text**: "only `process_virtiofs_request` **and `restore_device`**
  take and give back".
- **A1 gets a second case.** Restore runs only when no queue has started
  (`transport/task/restore.rs:26-29`), and the device task handles one
  command at a time, so no queue task runs meanwhile. `Session::restore_state`
  itself demands that no dispatch be active
  (`fuse/src/session/saved_state.rs:125-126`). The admission gate is
  save's argument (`device.rs:196-204`, `virtio.rs:430-436`), not
  restore's.
- **`session.owns_ghost(g) <==> self.owns(g)`.** `owns_ghost` is
  `(*self.fs).owns(g)` through `dyn Fuse` (`fuse/src/session.spec.rs:17-19`),
  so `VirtioFsDevice::wf()` must say that `device.fs` wraps a clone of
  `stateful_fs` (the same `inner` `Arc`, `device.rs:52-63`). That needs
  ensures on `Session::new` and on `VirtioFs::clone`.
- No new axiom.

**Option B: a new trusted axiom `replace_fs_ghost(session, g_new)`.**
**Infeasible without a code change**: a fresh `g`'s ids are not the ones the
existing table locks pin, so `owns` fails. It would work only if restore
built a new `VirtioFsInner`, with new locks, and replaced both `stateful_fs`
and the session's `fs`: a runtime change in Pedro's code. Rejected.

**Option C: restore as `DESTROY` followed by N `lookup`s and M `open`s, in
ghost terms only.** This is a proof technique for A, not an alternative: it
reuses the lemmas of `after_destroy`, `after_reach` and `after_open`, and
defines `after_restore(state@)` as that fold, which suits CONVENTIONS ("the
shape of its intent").

The dormant slot needs no ghost change: its `owns` is the empty view
(`lib.spec.rs:350-362`), and a dormant restore touches only the session.

## 4. What needs a host model, and "save records no host paths"

- **"Restore accepts only the same host objects" needs the host model** (open
  item 3: path -> (object, kind) per volume). On Ok, it would say that for
  every node n and alias p, the object at p is the one n's saved identity
  names, and that each handle's file is that object. The code's checks:
  `validate_identity` (`fs.rs:713, 754, 808`) and `guest_inode_nr`
  (`:765`). Without the model, (dev, ino) is an opaque number from `lstat`,
  and what it means is "the same as the blob says", which is worth something
  only for an authentic blob (§0). The same goes for the pinned identities in
  `from_saved`, for the "no link as ancestor" walk, and for the root's
  identity.
- **"Save records no host paths" needs no host model**, in two parts:
  - (i) every path in the saved view is a share path (`is_share_path`,
    `lxutil/src/lib.spec.rs`): the aliases (`fs.rs:528-530`,
    `encode_relative_path`), the policy strings, and the directory entries'
    names (one plain name, or `.`/`..`);
  - (ii) the saved view is a **function** of `g@`, the profile, the
    negotiation and opaque identities:
    `s@ == saved_view(g@, profile, negotiation, ids)`. That is the shape of
    the earlier snapshot spec's `saved_view`
    (`system-proof/snapshot-spec-review-20261023:snapshot-spec-docs/DESIGN.md`,
    "`SaveRestore`'s contract on views"). `root_path` is in none of these
    (`VirtioFs` has no field for it), so it cannot leak.

  What it leaves open: the (dev, ino) identities, the root's included
  (`aggregate_children[i].root_identity`), are host facts, though not paths.
  Whether recording them is acceptable is for Pedro or NVX to say.

## 5. Decisions for the lead

**D1. How exact is restore's Ok clause?** (a) Full:
`final(g)@ == old(g)@.after_restore(state@)`, with a `SavedStateView`.
(b) The core only: `owns`, `wf`, flags and shares unchanged, and the domains
of `inodes` and `files` are the saved ids. (c) (b) plus D2's policy facts.
*Recommend (c) now and (a) later*; (a) is functional correctness.

**D2. State restore's policy checks as a contract, even though requests
re-check?** (a) Yes: a `restored_ok` clause saying every name is visible, no
writable handle sits on a read-only share, and every writable handle's node
has `node_can_write`. (b) No. *Recommend (a)*: it is cheap, the tests pin it
(`policy_tests.rs:743`), and it holds against a forged blob.

**D3. Where does restore get `g`?** (a) Take and give-back in
`restore_device`, with A2 reworded. (b) A `replace_fs_ghost` axiom
(infeasible). (c) A code change so that restore builds a new `VirtioFsInner`.
*Recommend (a).*

**D4. Is the blob trusted?** (a) No: every restore claim must hold for any
blob, and the identity claims become "as the blob says". (b) Yes, as a stated
assumption: "a blob this device's save produced". *Recommend (a) for the
policy core and (b) for "same host objects"*; ask Pedro, since F1's weight
depends on it.

**D5. F2: save's failure reasons.** (a) Accept all three as listed reasons.
(b) Report (a) and (b) to Pedro: for (a), skip or refuse a nameless node; for
(b), enforce the 16 Ki / 4 MiB totals when a snapshot is built. *Recommend (b)
for F2(b)*, a plain mismatch, and asking Pedro about F2(a).

**D6. F1.** Record it in `BUGS.md` now as a property-3 question for Pedro,
or wait for property 3? *Recommend now.*

**Code changes needed: none for the spec.** Ghost and proof additions only:

- a ghost `is_root` on `from_saved`;
- ghost code in `restore_device` (its body must become verified, or an
  `external_body` wrapper must state it);
- `VirtioFs::wf` gains `mode is Aggregate <==> profile.is_aggregate()`;
- `VirtioFsDevice::wf` gains "`fs` wraps `stateful_fs`".

Fixing F1 and F2 is Pedro's call; the spec does not need it.

**Risks:**

- about 350 lines of `anyhow`, closure and `HashMap` code, where Verus
  support is thin: expect `external_body` helpers with small trusted
  contracts;
- the root-rule split in `from_saved`;
- the keys of inodes dropped by a restore that fails part-way (invisible, as
  in `lookup`);
- proving the `dyn Session` "owns iff" fact.

**Size, in steps:**

| Step | What | Size |
|---|---|---|
| S1 | `SavedStateView` and `saved_view` (`microvm/saved_state.spec.rs`) | ~120 lines |
| S2 | Restore's contract (D1 c, D2 a), the ghost handover in `restore_device`, A1/A2 text in `fs_ghost_t.rs` | ~80 lines, 1 day |
| S3 | Save's contract: `g` unchanged, (i) share paths, (ii) `saved_view`, failure reasons (D5) | ~60 lines |
| S4 | Proof (agent): `from_saved`/`with_attr` ghost, table halves, the fold lemma (§3 C), validate's lemmas | ~600-900 lines, the bulk |
| S5 | With the host model (deferred): same objects, pinned identities, no-link ancestors | later |

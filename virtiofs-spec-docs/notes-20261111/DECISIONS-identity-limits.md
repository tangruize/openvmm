# Identity and limits: the decisions made (2026-11-11, while Shan was away)

Shan chose identity's D1 (b). The rest are my choices, for review. Each says what
I did, what I did not take, and why. The options are those of
`DESIGN-identity.md` and `DESIGN-limits.md`, section 4.

## Identity (property 3): `microvm/owner.spec.rs`

- **D1. Both owner modes (Shan's choice).** `CallerIdentity::runs_as` says as
  whom a request runs: `Vmm` mode (the default), as OpenVMM; `Caller` mode, a
  request that touches host files as the mapped caller; the seven others
  (`INIT`, `DESTROY`, `FORGET`, `BATCH_FORGET`, `INTERRUPT`, `RELEASE`,
  `RELEASEDIR`) as OpenVMM.
- **D2. Checked: the mapping only; trusted, in words: the switching.** Done:
  `maps`, `wf` (never host root), and contracts on `for_export_root_owner`,
  `host_identity`, `accesses_host_files`. The trusted part (T1-T4:
  `with_fs_identity` changes only the thread's credentials, holds the identity
  while the operation runs, restores it, runs it synchronously; and every host
  call of a request is made inside `session.dispatch`) is in words at the top of
  the file. Not taken: a thread-credential token on every `lxutil` call (the
  note's option A): it would check little beyond the single wrap point until the
  host model also says what an identity may do. Not done either: the contract on
  `with_fs_identity` and `dispatch_with`, because `run_as` takes its operation as
  `&mut dyn FnMut()`, which Verus cannot specify. The file says how it becomes a
  contract once `run_as` takes a generic closure, the change the keeper's proof
  already needs (left to the proof agent, as before).
- **D3. Exempt requests and restore run as OpenVMM: accepted.** In words under
  "not claimed". No `BUGS.md` entry: Pedro documents restore's reopening as
  OpenVMM, and the exempt requests only close descriptors.
- **D4. "Caller" means the header's ids, mapped.** The guest's kernel writes
  them; the security content is "never host root, never OpenVMM's capabilities
  or groups". Not taken: also assuming no other host user's files are in the
  share (a deployment rule the code does not enforce).
- **D5. Where the trusted words are: in `owner.spec.rs`**, beside the property,
  not in a new `lxutil/src/unix/credentials_t.rs`. The note recommended the
  `_t.rs` file, but with no trusted contract to put there yet, a file of words
  only would be read apart from what it is about. When the contract comes, it
  belongs in `credentials_t.rs`.
- Small additions it needed (Verus-only): `RequestView` gains `opcode`, `uid`,
  `gid`; `fuse` re-exports `RequestView`; seven opcode constants and
  `CallerIdentity` are visible to Verus.

## Limits (property 4)

- **D1. The invariant and the error rules (recommended (b)).** Done:
  `within_limits` in `inv` (microVM only): at most 4096 nodes and 4096 handles,
  256 names per node, 16 Ki names in all, 1 MiB of name bytes (Linux). Error
  rules: `no_room_for_node` (mkdir, mknod, symlink: ENOSPC for a full table or
  name total, E2BIG for bytes), `no_room_for_name` (link), `no_room_for_handle`
  (open, open_dir but the synthetic root, create: ENOSPC). `mkdir` got its own
  rule set, `mkdir_errors`, so `unlink` and `rmdir`, which share `entry_errors`,
  do not get the rule. Not taken: (c), node rules for `lookup` and `create`
  (whether they add a node is the host's to say).
- **Not done: `rename`'s byte rule.** The code simulates the rename to count the
  bytes after it; stating that exactly is easy to get wrong, and leaving a rule
  out only makes the spec weaker, never false. `inv` still bounds the bytes
  after a successful rename.
- **D2. No lookup counts (recommended (a)).** The limits do not need them.
  Open item 2 is re-scoped to `forget`'s correctness, liveness and snapshots.
- **D3. Gap A fixed in the code (recommended (a)).** A lookup whose object's key
  had a node of another file type skipped the node cap. Now it takes the new
  node's check, as the insert does. `BUGS.md` bug 9, code change 32, patch 0011,
  with a test (fails without the fix). Not taken: a trusted assumption instead
  (b), or leaving the node cap out of the invariant (c).
- **D4. Gap B, the listing totals only at save: a question for Pedro**
  (recommended (a)). In `BUGS.md`'s questions; the caches stay out of the spec.
  Not taken now: checking the totals when a cache is built (b), a code change
  for Pedro to decide.
- **D5. The byte cap on Linux only (recommended (a)),** as `path_too_long`
  (`name_bytes_fit` is `true` on Windows, where the code counts UTF-16 bytes).
- **D6. The caps in `inv` (recommended (a)),** so no success clause changed:
  every contract's `final(g).wf()` already promises them. Literals with comments,
  as `path_too_long` does.
- **D7. The request size bound: left for later,** with the virtio queue's spec
  (it drops the request, it is not a FUSE reply).

## Checks

Verus: no errors (`lx`, `lxutil`, `fuse`, `virtiofs`). Tests: all pass (171).
The patch series 0001-0011 applies in order to Pedro's `db42fde04`; with 0011,
the tests, `fmt` and clippy are clean there.

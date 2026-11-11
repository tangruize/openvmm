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
- **D2. Checked: the mapping, and that the request runs as `runs_as`.**
  (Revised after Shan's review, 2026-11-11: the first version left the
  switching in words.) `Session::dispatch` requires the thread's credential
  token (`lx::cred::ThreadCred`) to be `runs_as`, and `dispatch_with`'s proof
  shows it. Trusted: the token's meaning, as contracts on `lxutil`'s
  `switch_fs_identity` and `FsIdentityGuard::restore`, and two keeper axioms
  (the token goes with `g`). This needed a code change for verification only,
  not for Pedro (code change 33): `run_as`'s `&mut dyn FnMut()` closure became
  the trait `SwitchIdentity`, and `lxutil` exposes `with_fs_identity`'s two
  halves. A generic closure was tried first: Verus's attribute style cannot,
  on stable Rust, put a contract on a closure expression or build a tracked
  value in it; patching Verus for that was judged harder than the change. Not
  taken: the token on every `lxutil` call (option A), which adds little until
  the host model says what an identity may do. In words still: every host call
  of a request is made inside `Session::dispatch`.
- **D3. Exempt requests and restore run as OpenVMM: accepted.** In words under
  "not claimed". No `BUGS.md` entry: Pedro documents restore's reopening as
  OpenVMM, and the exempt requests only close descriptors.
- **D4. "Caller" means the header's ids, mapped.** The guest's kernel writes
  them; the security content is "never host root, never OpenVMM's capabilities
  or groups". Not taken: also assuming no other host user's files are in the
  share (a deployment rule the code does not enforce).
- **D5. Where the trusted part is:** the token in `lx/src/cred_t.rs` (`lx`,
  because `fuse` names it and depends only on `lx`), the contracts on
  `lxutil/src/unix/credentials.rs`'s two functions, and the words in
  `owner.spec.rs`.
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

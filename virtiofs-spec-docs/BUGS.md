# virtio-fs: bugs found while writing the spec

Bugs in Pedro Henrique Penna's virtio-fs (`feature/microvm/dev` at
`db42fde04`) found while writing the spec, each with what goes wrong, how a
guest triggers it, and the fix. Fixed on this branch; each fix is also a
`CODE-CHANGES.md` entry, and a patch in `patches/`, for Pedro.

The patches in `patches/` (0001-0010) are for Pedro's code itself: they
apply in order to `feature/microvm/dev` at `db42fde04`, and after each
one, `virtiofs` builds, its tests pass (each patch adds one), and
`cargo fmt` and `clippy` are clean (checked 2026-11-11). They hold the
code and its tests only, none of the spec's Verus annotations.

## 1. A handle keeps working after the guest gives its node back

*Found 2026-11-10, while writing the contract of `read_dir_plus`. Fixed:
`patches/0001-virtiofs-a-handle-only-with-its-node.patch`.*

**The cause.** In FUSE, giving back a node (`FORGET`) and closing a handle
(`RELEASE`, `RELEASEDIR`) are separate messages. `FORGET` removes the node
from the inode table when its count reaches 0, but an open handle's
`VirtioFsFile` keeps its own reference to the inode (`inode:
Arc<VirtioFsInode>`), so the handle still works. From then on nothing keeps
that inode's path and names up to date: a `rename` updates only the inodes
in the table (`rename_alias_prefix`, `remove_alias_prefix`). And the
handle-based operations looked up a handle by its number only
(`get_file(fh)`), without the request's node id, which every FUSE request
carries. Linux's FUSE driver always releases a handle before it forgets
the node, so a normal guest never gets there; a guest whose kernel is
under an attacker's control (the microVM's threat model) can send
`FORGET` and keep the handle.

**What goes wrong.**

- *Integrity.* `/w` is writable, with a denied path `/w/e/x` inside it.
  The guest opens `/w/d/x` for writing (handle 5, node 9), sends
  `FORGET(node 9)` and keeps handle 5, then renames `/w/d` to `/w/e`
  (allowed: both paths are writable). The file is now at `/w/e/x`, a
  denied path, but node 9's names still say `/w/d/x`. A `write` through
  handle 5 checks `check_writable(file.inode())`, those stale names, which
  are writable: the write goes through. With node 9 in the table, the
  rename would have updated its names to `/w/e/x`, and the write would be
  refused. The same holds for reads through a handle (confidentiality).
- *Correctness.* The guest opens directory `/a` (handle 9, node 7), sends
  `FORGET(node 7)`, renames `/a` to `/b`, and makes a new directory `/a`.
  `read_dir_plus(handle 9)` lists the entries of the directory that handle
  9 opened, now `/b` (`x`, `y`), but looks each one up at node 7's stale
  path, `/a/x` and `/a/y` (`lookup_helper(&self.inode, name)`): the reply
  gives the guest, under the name `x`, the node of another object (or
  none).

**The fix.** One check, in `get_file`, which all nine handle-based
operations use (`get_attr` and `get_statx` with a handle, `set_attr` with
a handle, `read`, `write`, `read_dir`, `read_dir_plus`, `fsync`,
`fsync_dir`): it takes the request's node id, and refuses unless the guest
still holds that node (`get_inode`, `EINVAL` otherwise, as for any request
about a node it does not hold) and the handle is that node's (`EBADF`
otherwise). `release` and `release_dir` do not use `get_file`, so a handle
can still be closed after its node is given back. A normal guest sees no
difference: Linux's driver sends the handle's own node id, and releases
before it forgets. Test:
`microvm/policy_tests.rs`, `a_handle_is_used_only_with_its_node_while_the_guest_holds_it`.
All `virtiofs` tests passed (162, one ignored).

**The patch** (production code; the full patch, with the test, is in
`patches/`):

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index 71780f585..b6525a8a0 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -182,7 +182,7 @@ fn get_attr(&self, request: &Request, flags: u32, fh: u64) -> lx::Result<fuse_at
         // Windows and works if the file was deleted. The synthetic root's directory handle has no
         // backing file, so fall through to the node-based branch for it.
         let attr = if flags & FUSE_GETATTR_FH != 0 && !self.is_synthetic_root_handle(node_id, fh) {
-            let file = self.get_file(fh)?;
+            let file = self.get_file(node_id, fh)?;
             file.get_attr()?
         } else if self.is_synthetic_root(node_id) {
             self.synthetic_root_attr()
@@ -209,7 +209,7 @@ fn get_statx(
         let statx = if getattr_flags & FUSE_GETATTR_FH != 0
             && !self.is_synthetic_root_handle(node_id, fh)
         {
-            let file = self.get_file(fh)?;
+            let file = self.get_file(node_id, fh)?;
             file.get_statx()?
         } else if self.is_synthetic_root(node_id) {
             self.synthetic_root_statx(mask)
@@ -231,7 +231,7 @@ fn set_attr(&self, request: &Request, arg: &fuse_setattr_in) -> lx::Result<fuse_
         // If a file handle is specified, set the attributes on the open file. This is faster on
         // Windows and works if the file was deleted.
         let attr = if arg.valid & FATTR_FH != 0 {
-            let file = self.get_file(arg.fh)?;
+            let file = self.get_file(node_id, arg.fh)?;
             // Block truncation and other modifications on readonly filesystems
             if arg.valid & !(FATTR_FH | FATTR_LOCKOWNER) != 0 {
                 self.check_writable(file.inode())?;
@@ -446,8 +446,8 @@ fn read_link(&self, request: &Request) -> lx::Result<lx::LxString> {
         inode.read_link()
     }
 
-    fn read(&self, _request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
-        let file = self.get_file(arg.fh)?;
+    fn read(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
+        let file = self.get_file(request.node_id(), arg.fh)?;
         let mut buffer = guest_buffer(arg.size)?;
         let size = file.read(&mut buffer, arg.offset)?;
         buffer.truncate(size);
@@ -458,7 +458,7 @@ fn write(&self, request: &Request, arg: &fuse_write_in, data: &[u8]) -> lx::Resu
         if data.len() > MAX_GUEST_BUFFER_SIZE {
             return Err(lx::Error::E2BIG);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         self.check_writable(file.inode())?;
         file.write(data, arg.offset, request.uid())
     }
@@ -483,7 +483,7 @@ fn read_dir(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>>
         if self.is_synthetic_root_handle(request.node_id(), arg.fh) {
             return self.read_synthetic_root_dir(arg.offset, arg.size, false);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         file.read_dir(self, arg.offset, arg.size, false)
     }
 
@@ -491,7 +491,7 @@ fn read_dir_plus(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec
         if self.is_synthetic_root_handle(request.node_id(), arg.fh) {
             return self.read_synthetic_root_dir(arg.offset, arg.size, true);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         file.read_dir(self, arg.offset, arg.size, true)
     }
 
@@ -554,14 +554,14 @@ fn statfs(&self, request: &Request) -> lx::Result<fuse_kstatfs> {
         inode.stat_fs()
     }
 
-    fn fsync(&self, _request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
-        let file = self.get_file(fh)?;
+    fn fsync(&self, request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
+        let file = self.get_file(request.node_id(), fh)?;
         let data_only = flags & FUSE_FSYNC_FDATASYNC != 0;
         file.fsync(data_only)
     }
 
-    fn fsync_dir(&self, _request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
-        let file = self.get_file(fh)?;
+    fn fsync_dir(&self, request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
+        let file = self.get_file(request.node_id(), fh)?;
         let data_only = flags & FUSE_FSYNC_FDATASYNC != 0;
         file.fsync_dir(data_only)
     }
@@ -820,14 +820,30 @@ fn reserve_inode(&self) -> lx::Result<InodeReservation<'_>> {
     }
 
     /// Retrieve the file object with the specified file handle.
-    fn get_file(&self, fh: u64) -> lx::Result<Arc<VirtioFsFile>> {
-        let files = self.inner.files.read();
-        let file = files.get(fh).ok_or_else(|| {
-            tracelimit::warn_ratelimited!(fh, "Request for unknown file");
-            lx::Error::EBADF
-        })?;
-
-        Ok(Arc::clone(file))
+    /// Looks up the open file `fh` of a request about node `node_id`.
+    ///
+    /// The handle must belong to the request's node, and the guest must still
+    /// hold that node. After `FORGET`, nothing keeps the node's path and names
+    /// up to date (a rename updates only the inodes in the table), and they
+    /// decide what a request through the handle may do: which paths are
+    /// writable (`check_writable`), and where `read_dir_plus` looks up the
+    /// entries it lists. A guest that gave the node back but kept the handle
+    /// could otherwise write to a file that a rename has since moved to a
+    /// path it may not write, or get nodes of the wrong objects.
+    fn get_file(&self, node_id: u64, fh: u64) -> lx::Result<Arc<VirtioFsFile>> {
+        let file = {
+            let files = self.inner.files.read();
+            Arc::clone(files.get(fh).ok_or_else(|| {
+                tracelimit::warn_ratelimited!(fh, "Request for unknown file");
+                lx::Error::EBADF
+            })?)
+        };
+        let inode = self.get_inode(node_id)?;
+        if !std::ptr::eq(&*inode, file.inode()) {
+            tracelimit::warn_ratelimited!(node_id, fh, "Request for a file of another node");
+            return Err(lx::Error::EBADF);
+        }
+        Ok(file)
     }
 
     /// Reserve a file handle before opening or creating a host file.
```

## 2. A rename of a name onto itself drops names from the nodes

*Found 2026-11-10, in a review of the contracts against the code. Fixed:
`patches/0002-virtiofs-rename-onto-itself-keeps-names.patch` (applies after
0001).*

**What goes wrong.** `rename(dir, x, dir, x)` (the same name in the same
directory) changes nothing on the host. But after the host call, the code
updates the names as for any rename: `remove_alias_prefix(new_path)`, then
`rename_alias_prefix(old_path, new_path)` (`lib.rs`, `rename`). With the
two paths equal, the first drops every name at or below `x` from every node
of the share, and the second finds nothing left to move back. The nodes
keep their main paths, but their records of their names no longer match the
host, and the code's later decisions use those records (which names a
change must find writable, where a main path moves when one is removed).
Linux's FUSE driver handles a rename onto the same name itself, so only a
guest whose kernel an attacker controls can send it.

**The fix.** A rename of a name onto itself still goes to the host, which
answers it as Linux does (`Ok`; `EEXIST` with `RENAME_NOREPLACE`; `ENOENT`
if there is no such name), and then skips the name updates, since nothing
moved. (Corrected 2026-11-11: the first version of this fix returned `Ok`
at once, without asking the host, so the guest got `Ok` where Linux gives
`EEXIST` or `ENOENT`; found by an independent review.) Test:
`microvm/policy_tests.rs`, `renaming_a_name_onto_itself_keeps_its_names`
(fails without the fix, and with the first version).

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index b6525a8a0..811df5797 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -534,8 +534,19 @@ fn rename(
         self.check_writable_entry(&inode, &old_path)?;
         self.check_writable_entry(&new_inode, &new_path)?;
         let exchange = flags & RENAME_EXCHANGE != 0;
-        self.preflight_rename_aliases(inode.volume_id(), &old_path, &new_path, exchange)?;
+        // Renaming a name onto itself changes nothing on the host, which still
+        // answers it as Linux does (`EEXIST` with `RENAME_NOREPLACE`, `ENOENT`
+        // if there is no such name); the name updates below would drop every
+        // name at or below it from every node (`remove_alias_prefix` of the
+        // new path), so they are skipped.
+        let onto_itself = old_path == new_path;
+        if !onto_itself {
+            self.preflight_rename_aliases(inode.volume_id(), &old_path, &new_path, exchange)?;
+        }
         inode.rename(name, &new_inode, new_name, flags)?;
+        if onto_itself {
+            return Ok(());
+        }
         let mut inodes = self.inner.inodes.write();
         if exchange {
             inodes.exchange_alias_prefixes(inode.volume_id(), &old_path, &new_path);
```

## 3. A create of an existing file skips `open`'s rule for that file

*Found 2026-11-10, in a review of the contracts against the code. Fixed:
`patches/0003-virtiofs-create-on-existing-file-follows-open.patch` (applies
after 0002).*

**What goes wrong.** A `create` without `O_EXCL` opens the file at the name
if one is there, with the guest's flags: `O_TRUNC` truncates it, and
`O_WRONLY` or `O_RDWR` opens it for writing. `create` checks only that the
parent directory and the name used are writable (`check_writable`,
`check_writable_entry`). `open` of the same file also requires every name of
it to be writable (`check_open_readonly`, through `readonly()`), since a
change to a file shows through all its names. So a file whose other name is
not writable cannot be truncated by `open`, but can by `create` at its
writable name. Under the host assumption (no host hard link across the
policy's boundary), a file gets such a name only by a guest `link` while
both names were writable, and then a `rename` that moves the other name
under a denied path inside a writable one.

**The fix.** Before the host opens the file, a `create` that may write or
truncate an existing file looks up whether the guest holds that file's node
(`InodeMap::find`, by the same identity key as the inode table's dedup), and
applies `open`'s rule to it: EROFS unless every name of it is writable.
With `O_EXCL`, an existing file makes `create` fail anyway. In the microVM,
nothing changes the file between this check and the open, since requests
run one at a time. Test: `microvm/policy_tests.rs`,
`a_create_that_truncates_an_existing_file_needs_all_its_names_writable`
(fails without the fix; the test makes the second name with a host hard
link, which is simpler to set up than the link and rename above).

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index 811df5797..c3a1bc238 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -300,6 +300,7 @@ fn create(
         self.check_writable(&inode)?;
         let path = inode.child_path(name)?;
         self.check_writable_entry(&inode, &path)?;
+        self.check_create_existing(&inode, &path, options)?;
         self.preflight_create_inode(&inode, name, &path)?;
         self.preflight_file_insert()?;
         let inode_reservation = self.reserve_inode()?;
@@ -676,6 +677,39 @@ fn check_writable_entry(&self, parent: &VirtioFsInode, path: &Path) -> lx::Resul
         }
     }
 
+    /// Check that a `create` that may open an existing file at `path` for
+    /// writing, or truncate it, follows `open`'s rule for that file: if the
+    /// guest holds the file's node, every name of it must be writable (as
+    /// `check_open_readonly` requires of `open`), and EROFS if not. With
+    /// `O_EXCL`, an existing file makes `create` fail anyway.
+    fn check_create_existing(
+        &self,
+        parent: &VirtioFsInode,
+        path: &Path,
+        options: i32,
+    ) -> lx::Result<()> {
+        let changes = matches!(options & lx::O_ACCESS_MASK, lx::O_WRONLY | lx::O_RDWR)
+            || options & lx::O_TRUNC != 0;
+        if options & lx::O_EXCL != 0 || !changes {
+            return Ok(());
+        }
+        let Ok(stat) = parent.volume.lstat(path) else {
+            // Nothing there: the create makes a new file.
+            return Ok(());
+        };
+        let key = if parent.volume.supports_stable_file_id() {
+            DedupKey::Ino(parent.volume_id(), stat.inode_nr)
+        } else {
+            DedupKey::Path(parent.volume_id(), path.to_path_buf())
+        };
+        if let Some(existing) = self.inner.inodes.read().find(&key) {
+            if existing.readonly() {
+                return Err(lx::Error::EROFS);
+            }
+        }
+        Ok(())
+    }
+
     /// Check whether the open flags are permitted on a read-only filesystem.
     fn check_open_readonly(&self, inode: &VirtioFsInode, options: i32) -> lx::Result<()> {
         if !inode.readonly() {
@@ -1117,6 +1151,13 @@ pub fn release_reservation(&mut self, reservation: &HandleReservation) {
         self.inodes_by_node_id.release_reservation(reservation);
     }
 
+    /// The inode that the table holds for `key`, if any.
+    pub fn find(&self, key: &DedupKey) -> Option<Arc<VirtioFsInode>> {
+        self.inodes_by_key
+            .get(key)
+            .map(|(inode, _)| Arc::clone(inode))
+    }
+
     /// Insert an inode into the map, returning its node ID.
     pub fn insert(&mut self, inode: VirtioFsInode) -> lx::Result<(Arc<VirtioFsInode>, u64)> {
         // Reuse an existing node id for the same host file; see `DedupKey`
```

## 4. A link to a file whose names are all gone links another file

*Found 2026-11-11, in an independent review of the contracts against the
code. Fixed: `patches/0006-virtiofs-link-refuses-an-object-with-no-names.patch`
(applies after 0005).*

**What goes wrong.** Each node keeps its names (aliases) and a main path,
the name the code uses on the host. When the last name of a file is
removed (`unlink`, or a `rename` over it), the node keeps its main path,
which may then name another object: for example, the guest creates a new
file at the same name. A `link` to the old node then calls
`link_stat(target.get_path(), new_name)`, so the host links the **new**
file, and the code adds the new name to the **old** node. The old node now
has a name of another object, and its reply carries the new file's
attributes. It also has a name that is not its main path, which breaks the
spec's invariant (a node with names has its main path among them). A Linux
guest kernel normally refuses such a link itself (its count of the file's
links is 0), so only a guest whose kernel an attacker controls can send it.

**The fix.** As Linux's `linkat` refuses a new name for a file whose links
are all gone, `link` refuses a target node with no names left, with
`ENOENT`, before the host is asked. Test: `microvm/policy_tests.rs`,
`a_link_to_an_object_with_no_names_left_is_refused` (fails without the
fix: the host links the new file as `b`).

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index c12a04cea..b3875e75e 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -411,6 +411,12 @@ fn link(&self, request: &Request, name: &lx::LxStr, target: u64) -> lx::Result<f
         }
         let inode = self.get_inode(request.node_id())?;
         let target_inode = self.get_inode(target)?;
+        // Like Linux's linkat, refuse a new name for an object whose names are
+        // all gone: its stored path may now name another object, which the
+        // host would link instead.
+        if target_inode.aliases().is_empty() {
+            return Err(lx::Error::ENOENT);
+        }
         // Like Linux's linkat, which reports a read-only destination before a
         // link across filesystems, check where the link would go first.
         self.check_writable(&inode)?;
```

## 5. A rename of a directory moves a hidden path below it into view

*Found 2026-11-11, in an independent review of the contracts against the
code. Fixed: `patches/0007-virtiofs-microvm-rename-moves-no-hidden-path.patch`
(applies after 0006).*

**What goes wrong.** In a read-write microVM share whose policy denies
`logs/secret`, the directory `logs` is visible and writable. `rename` checks
only the two names it is given, so the guest may rename `logs` to `moved`.
The host moves `secret` with it, to `moved/secret`, which no rule hides.
Then:

- a listing of `moved` shows `secret` (listings judge names by the policy,
  and do not check the pinned identities);
- `lookup("moved/secret")` is still refused (the pinned identity of the
  denied object, checked when a node is made);
- `unlink(moved, "secret")` succeeds, and the host deletes the hidden file
  (`unlink` checks only that `moved/secret` is writable).

So the guest cannot read the hidden file, but it can learn its name and
remove it. `RENAME_EXCHANGE` of `logs` with a writable directory does the
same. The guide says the guest's modifications of hidden parts fail, and
that "every mutation checks the names that it changes": a rename of `logs`
changes the name of `logs/secret` as well. Found by reading the code;
confirmed by a test against the original code.

**The fix.** `rename` refuses, with `EROFS`, a rename of a path with a
denied path strictly below it (with `RENAME_EXCHANGE`, of either path),
before the host is asked. This needs only the policy. Moving something
onto a path with a denied path below it is left alone: the target can be
replaced only if it is empty, and anything that then lands below it is the
guest's own. Test: `microvm/policy_tests.rs`, `a_rename_moves_no_hidden_path`
(fails without the fix). The guide's list of modifications that fail with
`EROFS` could add "renaming a directory that contains a denied path".

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index b3875e75e..b8d7ffcfb 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -542,7 +542,13 @@ fn rename(
         let new_path = new_inode.child_path(new_name)?;
         self.check_writable_entry(&inode, &old_path)?;
         self.check_writable_entry(&new_inode, &new_path)?;
+        // A rename moves every path below the renamed one too (both ways for
+        // an exchange), and the guest may not change a hidden path.
         let exchange = flags & RENAME_EXCHANGE != 0;
+        self.check_moves_nothing_hidden(&inode, &old_path)?;
+        if exchange {
+            self.check_moves_nothing_hidden(&new_inode, &new_path)?;
+        }
         // Renaming a name onto itself changes nothing on the host, which still
         // answers it as Linux does (`EEXIST` with `RENAME_NOREPLACE`, `ENOENT`
         // if there is no such name); the name updates below would drop every
@@ -685,6 +691,18 @@ fn check_writable_entry(&self, parent: &VirtioFsInode, path: &Path) -> lx::Resul
         }
     }
 
+    /// Check that a rename of the entry at `path` in the directory `parent`
+    /// moves no path that the volume's access policy hides (one strictly
+    /// below `path`), and return EROFS if it would: the guest may not change
+    /// what it cannot see.
+    fn check_moves_nothing_hidden(&self, parent: &VirtioFsInode, path: &Path) -> lx::Result<()> {
+        if parent.volume.hides_below(path) {
+            Err(lx::Error::EROFS)
+        } else {
+            Ok(())
+        }
+    }
+
     /// Check that a `create` that may open an existing file at `path` for
     /// writing, or truncate it, follows `open`'s rule for that file: if the
     /// guest holds the file's node, every name of it must be writable (as
diff --git a/vm/devices/virtio/virtiofs/src/microvm/inode.rs b/vm/devices/virtio/virtiofs/src/microvm/inode.rs
index b7a4c216e..4971b2010 100644
--- a/vm/devices/virtio/virtiofs/src/microvm/inode.rs
+++ b/vm/devices/virtio/virtiofs/src/microvm/inode.rs
@@ -90,6 +90,12 @@ pub(crate) fn path_writable(&self, path: &Path) -> bool {
         !self.readonly && self.policy.is_writable(path)
     }
 
+    /// Returns whether the access policy hides a path strictly below `path`,
+    /// which a rename of `path` would move.
+    pub(crate) fn hides_below(&self, path: &Path) -> bool {
+        self.policy.hides_below(path)
+    }
+
     /// Returns whether the access policy makes some visible path read-only.
     pub(crate) fn restricts_writes(&self) -> bool {
         self.policy.restricts_writes()
diff --git a/vm/devices/virtio/virtiofs/src/microvm/policy.rs b/vm/devices/virtio/virtiofs/src/microvm/policy.rs
index c4233358a..ad9b0e58c 100644
--- a/vm/devices/virtio/virtiofs/src/microvm/policy.rs
+++ b/vm/devices/virtio/virtiofs/src/microvm/policy.rs
@@ -163,6 +163,14 @@ pub(crate) fn is_writable(&self, path: &Path) -> bool {
                     .any(|writable| path.starts_with(writable)))
     }
 
+    /// Returns whether a denied path lies strictly below `path`, so that a
+    /// rename of `path` would move a hidden path with it.
+    pub(crate) fn hides_below(&self, path: &Path) -> bool {
+        self.denied
+            .iter()
+            .any(|denied| denied.as_path() != path && denied.starts_with(path))
+    }
+
     /// Returns whether some path that the guest can reach is not writable in a
     /// read-write share, so that each modification must check its paths.
     pub(crate) fn restricts_writes(&self) -> bool {
```

## 6. On Windows, a write through a read-only handle can succeed

*Found 2026-11-11, from an independent review of the contracts against the
code (by reading the code; not tested, since it needs a Windows host).
Fixed: `patches/0008-virtiofs-write-needs-a-handle-opened-for-writing.patch`
(applies after 0007).*

**What goes wrong.** `write` checks the handle and the access policy, but
not how the handle was opened: it leaves that to the host, where `pwrite`
on a file opened read-only fails. On Linux it does (`EBADF`). On Windows,
lxutil gives a handle opened `O_RDONLY | O_TRUNC` write access, since
Windows needs it to truncate (`open_flags_to_access_mask`,
`lxutil/src/windows/util.rs`), so a later `write` through that read-only
handle succeeds. Not a hole in the access policy: `open` with `O_TRUNC`
already needs the file to be writable by the policy, and `write` checks
the policy again. But a write through a read-only handle must fail.

**The fix.** As Linux's `write(2)` does, `write` refuses a handle whose
access mode is not `O_WRONLY` or `O_RDWR`, with `EBADF`, before the host
is asked. Test: `microvm/policy_tests.rs`,
`a_write_needs_a_handle_opened_for_writing` (on Linux it passes without
the fix too, since the host refuses with the same code).

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index b8d7ffcfb..ea1fab516 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -467,6 +467,15 @@ fn write(&self, request: &Request, arg: &fuse_write_in, data: &[u8]) -> lx::Resu
             return Err(lx::Error::E2BIG);
         }
         let file = self.get_file(request.node_id(), arg.fh)?;
+        // Like Linux's write(2), refuse a handle not opened for writing,
+        // rather than leave it to the host: on Windows, a handle opened with
+        // `O_RDONLY | O_TRUNC` has write access (truncating needs it there).
+        if !matches!(
+            file.open_flags() as i32 & lx::O_ACCESS_MASK,
+            lx::O_WRONLY | lx::O_RDWR
+        ) {
+            return Err(lx::Error::EBADF);
+        }
         self.check_writable(file.inode())?;
         file.write(data, arg.offset, request.uid())
     }
```

## 7. The file table can hand out the aggregate root's own handle

*Found 2026-11-11, from an independent review of the contracts against the
code. Low severity: it needs 2^64 - 2 opens, or a restored counter at its
last value. Fixed:
`patches/0009-virtiofs-the-file-table-never-hands-out-the-root-handle.patch`
(applies after 0008).*

**What goes wrong.** In an aggregate, the handle `u64::MAX` is the
synthetic root's own (`SYNTHETIC_ROOT_FH`): `read_dir`, `read_dir_plus`
and `release_dir` serve the root's registry for it, not the file table.
Its comment says it "can't collide with a real handle, since `HandleMap`
allocates from 1 upward". But `HandleMap` hands out every value from its
start up to `u64::MAX` itself, and stops only when the counter wraps to 0
(`can_insert`; the test `handle_map_stops_after_max_handle` hands out
`u64::MAX`). So an `open` can return `u64::MAX`, and then one handle means
both a file and the root. A restore accepts any saved counter above every
saved handle, `u64::MAX` included, so a saved state can bring the counter
there at once.

**The fix.** `HandleMap` never hands out `u64::MAX`: a table whose counter
reaches it is full (`ENOSPC`, as when the counter wraps). Restore needs no
change: it requires the counter to be above every saved handle, so no
saved handle is `u64::MAX`. The comment is corrected. Test:
`handle_map_stops_after_max_handle` (fails without the fix); the tests of a
full table now use `u64::MAX - 1` as the last handle.

```diff
diff --git a/vm/devices/virtio/virtiofs/src/aggregate.rs b/vm/devices/virtio/virtiofs/src/aggregate.rs
index 5b9694132..bbf05e91f 100644
--- a/vm/devices/virtio/virtiofs/src/aggregate.rs
+++ b/vm/devices/virtio/virtiofs/src/aggregate.rs
@@ -27,7 +27,8 @@
 /// Reserved file handle `open_dir` returns for the synthetic aggregate root;
 /// `read_dir`/`read_dir_plus`/`release_dir` recognize it and serve the root
 /// registry instead of the handle map. `u64::MAX` can't collide with a real
-/// handle, since `HandleMap` allocates from 1 upward.
+/// handle: `HandleMap` allocates from 1 upward and never hands out
+/// `u64::MAX` (`HandleMap::can_insert`).
 pub(crate) const SYNTHETIC_ROOT_FH: u64 = u64::MAX;
 
 /// A single host folder exposed as a named child of the synthetic aggregate root.
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index ea1fab516..b58ec56b8 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -1111,8 +1111,14 @@ fn is_current(&self, reservation: &HandleReservation) -> bool {
         Arc::ptr_eq(&self.generation, &reservation.generation)
     }
 
+    /// Whether the next handle may be handed out. `u64::MAX` never is: it is
+    /// the aggregate root's own handle (`aggregate::SYNTHETIC_ROOT_FH`), so a
+    /// table that reaches it is full. (A restored table starts above every
+    /// saved handle, so it never holds `u64::MAX` either.)
     pub fn can_insert(&self) -> bool {
-        self.next_handle != 0 && !self.values.contains_key(&self.next_handle)
+        self.next_handle != 0
+            && self.next_handle != u64::MAX
+            && !self.values.contains_key(&self.next_handle)
     }
 
     /// Retrieves a value from the map.
```

## 8. A microVM listing that fails partway keeps lookup references

*Found 2026-11-11, from an independent review of the code; confirmed by a
test (guest requests only, one after another). Fixed:
`patches/0010-virtiofs-microvm-a-failed-listing-keeps-no-lookup-reference.patch`
(applies after 0009).*

**What goes wrong.** Each entry of a `read_dir_plus` reply is looked up,
and hands the guest a lookup reference, which the guest gives back with
`forget`, but only for the entries it receives. In the microVM
(`read_dir_microvm`, `microvm/file.rs`), an error partway through a reply
returns the error and discards the reply, with the entries already looked
up: their references are never given back, and their nodes stay in the
table. The error can be the lookup itself (`ENOENT` when the guest removed
the entry after the listing's cache was built), the inode number not
matching the cache (`EIO`), or the check of an entry's path at that moment
(`listable_now`, from code change 23). In the test: a directory with `a`,
`b`, `c` and `existing`; a first reply of `.` and `..` builds the cache;
the guest unlinks the last entry; the next reply fails with `ENOENT`, and
`a`'s lookup count is one higher. Repeated, this fills the inode table
(`MAX_INODES`), and the guest's own lookups fail with `ENOSPC`: a leak of
the guest's own resources, not of anything it may not see or change. The
listing outside the microVM (`file.rs`) already avoids it.

**The fix.** As outside the microVM: after an error, the reply returns the
entries written so far, and reports the error only if there are none; the
guest's next read starts at the failing entry. An entry whose lookup
succeeded but that the guest does not receive (its inode number no longer
matches, or it does not fit) gives its reference back at once
(`VirtioFs::forget_lookups`, the body of `forget`, now shared). Test:
`microvm/policy_tests.rs`,
`a_listing_that_fails_keeps_no_lookup_reference_it_does_not_hand_out`
(fails without the fix).

```diff
diff --git a/vm/devices/virtio/virtiofs/src/lib.rs b/vm/devices/virtio/virtiofs/src/lib.rs
index b58ec56b8..dec409c9a 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -259,18 +259,7 @@ fn lookup(&self, request: &Request, name: &lx::LxStr) -> lx::Result<fuse_entry_o
     }
 
     fn forget(&self, node_id: u64, lookup_count: u64) {
-        if node_id == FUSE_ROOT_ID {
-            return;
-        }
-        // This must be done under lock so an inode can't be resurrected between the lookup count
-        // reaching zero and removing it from the list.
-        let mut inodes = self.inner.inodes.write();
-        if let Some(inode) = inodes.get(node_id) {
-            if inode.forget(node_id, lookup_count) == 0 {
-                tracing::trace!(node_id, "Removing inode");
-                inodes.remove(node_id);
-            }
-        }
+        self.forget_lookups(node_id, lookup_count);
     }
 
     fn open(&self, request: &Request, flags: u32) -> lx::Result<fuse_open_out> {
@@ -823,6 +812,23 @@ pub fn new_aggregate() -> Self {
         }
     }
 
+    /// Gives back `lookup_count` lookup references of node `node_id`
+    /// (`forget`), and removes the node when none is left.
+    pub(crate) fn forget_lookups(&self, node_id: u64, lookup_count: u64) {
+        if node_id == FUSE_ROOT_ID {
+            return;
+        }
+        // This must be done under lock so an inode can't be resurrected between the lookup count
+        // reaching zero and removing it from the list.
+        let mut inodes = self.inner.inodes.write();
+        if let Some(inode) = inodes.get(node_id) {
+            if inode.forget(node_id, lookup_count) == 0 {
+                tracing::trace!(node_id, "Removing inode");
+                inodes.remove(node_id);
+            }
+        }
+    }
+
     fn lookup_helper(&self, inode: &VirtioFsInode, name: &lx::LxStr) -> lx::Result<fuse_entry_out> {
         let (new_inode, attr) = inode.lookup_child(name)?;
         self.preflight_inode_insert(&new_inode)?;
diff --git a/vm/devices/virtio/virtiofs/src/microvm/file.rs b/vm/devices/virtio/virtiofs/src/microvm/file.rs
index bddfefa9a..e77920ebd 100644
--- a/vm/devices/virtio/virtiofs/src/microvm/file.rs
+++ b/vm/devices/virtio/virtiofs/src/microvm/file.rs
@@ -52,14 +52,26 @@ pub(crate) fn read_dir_microvm(
 
         // Whether the reply stopped because the next entry did not fit.
         let mut full = false;
+        // Each readdirplus entry hands the guest a lookup reference, which the
+        // guest can release only if it receives the entry. So after an error,
+        // return the entries written so far, and report the error only if
+        // there are none; the guest's next read starts at the failing entry.
+        let mut stopped = None;
         for entry in &entries[start..] {
             let name = lx::LxStr::from_bytes(&entry.name);
             // The cache was filtered against the directory's path when it was
             // built; the guest may have renamed the directory since, so each
             // entry is judged again against its path now, and one that may
             // no longer be listed is left out (its cookie is skipped).
-            if entry.name != b"." && entry.name != b".." && !self.listable_now(name, entry.kind)? {
-                continue;
+            if entry.name != b"." && entry.name != b".." {
+                match self.listable_now(name, entry.kind) {
+                    Ok(true) => {}
+                    Ok(false) => continue,
+                    Err(error) => {
+                        stopped = Some(error);
+                        break;
+                    }
+                }
             }
             if plus {
                 if entry.name == b"." || entry.name == b".." {
@@ -78,12 +90,22 @@ pub(crate) fn read_dir_microvm(
                         full = true;
                         break;
                     }
-                    let fuse_entry = fs.lookup_helper(&self.inode, name)?;
-                    if fuse_entry.attr.ino != entry.guest_inode_id {
-                        return Err(lx::Error::EIO);
-                    }
-                    if !buffer.dir_entry_plus(name, entry.next_cookie, fuse_entry) {
-                        return Err(lx::Error::EIO);
+                    let fuse_entry = match fs.lookup_helper(&self.inode, name) {
+                        Ok(fuse_entry) => fuse_entry,
+                        Err(error) => {
+                            stopped = Some(error);
+                            break;
+                        }
+                    };
+                    // An entry the guest will not receive must not keep the
+                    // lookup reference just taken.
+                    let node_id = fuse_entry.nodeid;
+                    if fuse_entry.attr.ino != entry.guest_inode_id
+                        || !buffer.dir_entry_plus(name, entry.next_cookie, fuse_entry)
+                    {
+                        fs.forget_lookups(node_id, 1);
+                        stopped = Some(lx::Error::EIO);
+                        break;
                     }
                 }
             } else if !buffer.dir_entry(name, entry.guest_inode_id, entry.next_cookie, entry.kind) {
@@ -92,6 +114,12 @@ pub(crate) fn read_dir_microvm(
             }
         }
 
+        if let Some(error) = stopped {
+            if buffer.is_empty() {
+                return Err(error);
+            }
+            return Ok(buffer);
+        }
         // Not even one entry fits in the guest's buffer. (A reply left empty
         // because every remaining entry was left out ends the listing.)
         if full && buffer.is_empty() {
```

## Questions for Pedro (not bugs, yet)

- **Device nodes from the guest** (answered by Pedro, 2026-11-10, about
  `mknod`: "the profile [should] accept only regular files, FIFOs and
  sockets in mknod, as defense in depth"; done, code change 24).
  `mknod` accepted any file type the guest asked for, character and block
  devices included; with `CAP_MKNOD`, and a share not on a `nodev`
  filesystem, a guest with one writable path could make a node for a host
  disk. Linux also lets any user make a character device numbered 0:0
  without `CAP_MKNOD` (a "whiteout", which overlayfs uses to mark a file
  deleted; on a share that is an overlayfs upper layer, it would hide a
  lower file from the merged view), in two ways: `mknod` of a character
  device 0:0, and `rename` with `RENAME_WHITEOUT` (Linux 5.8 and later),
  which leaves one at the old name. The microVM now refuses both, before
  asking the host: `mknod` of every type but a regular file (or 0, which
  is one), a FIFO or a socket, with `EPERM` (what Linux reports for a
  device node made without `CAP_MKNOD`); and `rename` with any flag but
  `RENAME_NOREPLACE` and `RENAME_EXCHANGE`, with `EINVAL` (what Linux
  reports for a flag it does not accept). (Corrected 2026-11-11: this item
  first said the `mknod` check closed the whiteout case; an independent
  review found the `rename` flag, and a test confirmed it.) Test
  `the_guest_makes_no_device_node` (fails without either check: the
  character device 0:0 is made); patch
  `patches/0005-virtiofs-microvm-makes-no-device-node.patch`.
- **Host hard links between a writable and a read-only path**, which the
  check of a file's names does not see (`CODE-OVERVIEW.md`, "Changes to a
  file itself, and its other names"). Pedro's guide states the rule ("Do
  not place such aliases inside an export"); the spec takes it as a host
  assumption (2026-11-11: the axiom `attached_disk`,
  `lxutil/src/host_t.rs`), so the question is only whether he wants the
  code to check it when a share is attached.
- **Mount points inside a microVM share.** `openat2` with
  `RESOLVE_BENEATH` keeps every path beneath the share's root, but does not
  stop it from crossing a mount point inside the share (`LxVolume`'s doc:
  "Mount crossings ... are not restricted"): with a bind mount of, say, the
  host's `/var` at `<share>/data`, the guest reads `/var` through
  `data/...`. The spec's confinement claim assumes that a microVM share
  contains no mount point (only the host's setup can put one there). Is
  that the intent? If so, adding `RESOLVE_NO_XDEV` to `openat2` (for
  `confine_paths`, or the microVM's volumes) would let the kernel enforce
  it, and the assumption would go; if a share may contain mounts on
  purpose, the claim is "beneath the root in the host's namespace" only.
- **Windows: names in another case, and 8.3 short names, reach hidden
  objects** (found 2026-11-11 by reading the code; not tested, since it
  needs a Windows host). lxutil opens every path with
  `OBJ_CASE_INSENSITIVE` (`lxutil/src/windows/util.rs`,
  `open_relative_file`), so in a directory that is not case-sensitive,
  `SECRET` opens `secret`; on a volume with 8.3 names, so does `SECRE~1`.
  The policy compares names exactly (`ensure_path_allowed`,
  `microvm/inode.rs`). In a read-write Windows microVM share that denies
  `secret`, `lookup("SECRET")` is still refused (the pinned identity of
  the denied object), but `unlink(root, "SECRET")` passes every check,
  since `SECRET` is a visible, writable path, and NTFS deletes the hidden
  file; `rename` of `SECRET` moves it, and `create` of `SECRET` with
  `O_EXCL` tells whether it exists. The guide's warnings about host
  aliases do not cover it. The spec assumes, for now, that different
  paths are different objects on Windows. Is this a bug to fix in the
  code, or a rule for how a Windows share is set up (every directory
  case-sensitive, 8.3 names off)? A fix in the code is not simple:
  without `OBJ_CASE_INSENSITIVE`, Windows still matches names without
  case by default (unless the system's `obcaseinsensitive` setting is
  off); comparing names without case in the policy would need NTFS's own
  case table, and would still miss short names; checking the identity of
  the object that a change touches against the pinned identities would
  cover both.
- **A directory listing is filtered once, when it starts (a note, not a
  bug).** In the microVM, a directory handle's listing is cached at its
  first read (`VirtioFsFile::directory_snapshot`,
  `build_microvm_directory_snapshot`, `microvm/file.rs`): the code reads
  the whole host directory and keeps the entries the policy lets the guest
  list (`entry_listable`), judged against the directory's path at that
  moment; later reads, and a restore, serve the rest of the cache as it
  is. If the guest renames the directory between two reads, a later reply
  can hold a name whose path is hidden by then. Example: `/pub/w/secret`
  is hidden; the guest opens `/pub/d`, reads `a` with a small buffer (the
  cache holds `a` and `secret`), renames `/pub/d` to `/pub/w`, and reads
  `secret`. This leaks nothing: the name was listable when the cache was
  built, and what is now at `/pub/w/secret` is what the guest moved there
  (a rename cannot replace a directory that has entries). It is also
  allowed by POSIX (a listing may reflect the directory as of when it
  started), and the cache keeps the listing's offsets stable across reads
  and save/restore. The one surprise: two handles on the same directory
  can list different entries (an older one still shows `secret`). We made
  every reply judge its entries against the directory's path at that
  moment (one `entry_listable` call per entry handed out, the cache and its
  offsets unchanged), because it makes the spec much simpler and the
  guarantee stronger; it is code change 23 (`CODE-CHANGES.md`), with a
  test, and the patch is
  `patches/0004-virtiofs-a-listing-judges-its-entries-at-each-reply.patch`.
  Tell us if you would rather keep the original behavior.

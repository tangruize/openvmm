# virtio-fs: bugs found while writing the spec

Bugs in Pedro Henrique Penna's virtio-fs (`feature/microvm/dev` at
`db42fde04`) found while writing the spec, each with what goes wrong, how a
guest triggers it, and the fix. Fixed on this branch; each fix is also a
`CODE-CHANGES.md` entry, and a patch in `patches/`, for Pedro.

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
index cf163f1d7..20ce80e5b 100644
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -197,7 +197,7 @@ fn get_attr(&self, request: &Request, flags: u32, fh: u64) -> lx::Result<fuse_at
         // Windows and works if the file was deleted. The synthetic root's directory handle has no
         // backing file, so fall through to the node-based branch for it.
         let attr = if flags & FUSE_GETATTR_FH != 0 && !self.is_synthetic_root_handle(node_id, fh) {
-            let file = self.get_file(fh)?;
+            let file = self.get_file(node_id, fh)?;
             file.get_attr()?
         } else if self.is_synthetic_root(node_id) {
             self.synthetic_root_attr()
@@ -226,7 +226,7 @@ fn get_statx(
         let statx = if getattr_flags & FUSE_GETATTR_FH != 0
             && !self.is_synthetic_root_handle(node_id, fh)
         {
-            let file = self.get_file(fh)?;
+            let file = self.get_file(node_id, fh)?;
             file.get_statx()?
         } else if self.is_synthetic_root(node_id) {
             self.synthetic_root_statx(mask)
@@ -250,7 +250,7 @@ fn set_attr(&self, request: &Request, arg: &fuse_setattr_in) -> lx::Result<fuse_
         // If a file handle is specified, set the attributes on the open file. This is faster on
         // Windows and works if the file was deleted.
         let attr = if arg.valid & FATTR_FH != 0 {
-            let file = self.get_file(arg.fh)?;
+            let file = self.get_file(node_id, arg.fh)?;
             // Block truncation and other modifications on readonly filesystems
             if arg.valid & !(FATTR_FH | FATTR_LOCKOWNER) != 0 {
                 self.check_writable(file.inode())?;
@@ -485,8 +485,8 @@ fn read_link(&self, request: &Request) -> lx::Result<lx::LxString> {
 
     #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
     #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut spec::VirtioFsGhost>))]
-    fn read(&self, _request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
-        let file = self.get_file(arg.fh)?;
+    fn read(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>> {
+        let file = self.get_file(request.node_id(), arg.fh)?;
         let mut buffer = guest_buffer(arg.size)?;
         let size = file.read(&mut buffer, arg.offset)?;
         buffer.truncate(size);
@@ -499,7 +499,7 @@ fn write(&self, request: &Request, arg: &fuse_write_in, data: &[u8]) -> lx::Resu
         if data.len() > MAX_GUEST_BUFFER_SIZE {
             return Err(lx::Error::E2BIG);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         self.check_writable(file.inode())?;
         file.write(data, arg.offset, request.uid())
     }
@@ -530,7 +530,7 @@ fn read_dir(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec<u8>>
         if self.is_synthetic_root_handle(request.node_id(), arg.fh) {
             return self.read_synthetic_root_dir(arg.offset, arg.size, false);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         file.read_dir(self, arg.offset, arg.size, false)
     }
 
@@ -540,7 +540,7 @@ fn read_dir_plus(&self, request: &Request, arg: &fuse_read_in) -> lx::Result<Vec
         if self.is_synthetic_root_handle(request.node_id(), arg.fh) {
             return self.read_synthetic_root_dir(arg.offset, arg.size, true);
         }
-        let file = self.get_file(arg.fh)?;
+        let file = self.get_file(request.node_id(), arg.fh)?;
         file.read_dir(self, arg.offset, arg.size, true)
     }
 
@@ -615,16 +615,16 @@ fn statfs(&self, request: &Request) -> lx::Result<fuse_kstatfs> {
 
     #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
     #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut spec::VirtioFsGhost>))]
-    fn fsync(&self, _request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
-        let file = self.get_file(fh)?;
+    fn fsync(&self, request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
+        let file = self.get_file(request.node_id(), fh)?;
         let data_only = flags & FUSE_FSYNC_FDATASYNC != 0;
         file.fsync(data_only)
     }
 
     #[cfg_attr(verus_keep_ghost, verus_verify(external_body))]
     #[cfg_attr(verus_keep_ghost, verus_spec(with Tracked(g): Tracked<&mut spec::VirtioFsGhost>))]
-    fn fsync_dir(&self, _request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
-        let file = self.get_file(fh)?;
+    fn fsync_dir(&self, request: &Request, fh: u64, flags: u32) -> lx::Result<()> {
+        let file = self.get_file(request.node_id(), fh)?;
         let data_only = flags & FUSE_FSYNC_FDATASYNC != 0;
         file.fsync_dir(data_only)
     }
@@ -897,14 +897,30 @@ fn reserve_inode(&self) -> lx::Result<InodeReservation<'_>> {
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

**The fix.** After the permission checks (so that a rename the guest may not
do is still refused), a rename of a name onto itself returns at once. Test:
`microvm/policy_tests.rs`, `renaming_a_name_onto_itself_keeps_its_names`
(fails without the fix).

```diff
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -590,6 +590,12 @@
         let new_path = new_inode.child_path(new_name)?;
         self.check_writable_entry(&inode, &old_path)?;
         self.check_writable_entry(&new_inode, &new_path)?;
+        // Renaming a name onto itself changes nothing on the host; the name
+        // updates below would otherwise drop every name at or below it from
+        // every node (`remove_alias_prefix` of the new path).
+        if old_path == new_path {
+            return Ok(());
+        }
         let exchange = flags & RENAME_EXCHANGE != 0;
         self.preflight_rename_aliases(inode.volume_id(), &old_path, &new_path, exchange)?;
         inode.rename(name, &new_inode, new_name, flags)?;
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
--- a/vm/devices/virtio/virtiofs/src/lib.rs
+++ b/vm/devices/virtio/virtiofs/src/lib.rs
@@ -327,6 +327,7 @@
         self.check_writable(&inode)?;
         let path = inode.child_path(name)?;
         self.check_writable_entry(&inode, &path)?;
+        self.check_create_existing(&inode, &path, options)?;
         self.preflight_create_inode(&inode, name, &path)?;
         self.preflight_file_insert()?;
         let inode_reservation = self.reserve_inode()?;
@@ -748,6 +749,39 @@
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
@@ -1189,6 +1223,11 @@
         self.inodes_by_node_id.release_reservation(reservation);
     }
 
+    /// The inode that the table holds for `key`, if any.
+    pub fn find(&self, key: &DedupKey) -> Option<Arc<VirtioFsInode>> {
+        self.inodes_by_key.get(key).map(|(inode, _)| Arc::clone(inode))
+    }
+
     /// Insert an inode into the map, returning its node ID.
     pub fn insert(&mut self, inode: VirtioFsInode) -> lx::Result<(Arc<VirtioFsInode>, u64)> {
         // Reuse an existing node id for the same host file; see `DedupKey`
```

## Questions for Pedro (not bugs, yet)

- **Device nodes from `mknod`** (answered by Pedro, 2026-11-10: "the
  profile [should] accept only regular files, FIFOs and sockets in mknod,
  as defense in depth"; done, code change 24). `mknod` accepted any file
  type the guest asked for, character and block devices included; with
  `CAP_MKNOD`, and a share not on a `nodev` filesystem, a guest with one
  writable path could make a node for a host disk. The microVM now refuses
  every type but a regular file (or 0, which is one), a FIFO or a socket,
  with `EPERM` (what Linux reports for a device node made without
  `CAP_MKNOD`), before asking the host. Found while testing it: Linux lets
  any user make a character device numbered 0:0 without `CAP_MKNOD` (a
  "whiteout", which overlayfs uses to mark a file deleted), so the guest
  could already make those; on a share that is an overlayfs upper layer,
  that would hide a lower file from the merged view. The check closes it.
  Test `mknod_makes_no_device_node` (fails without the check: the
  character device 0:0 is made); patch
  `patches/0005-virtiofs-microvm-mknod-makes-no-device-node.patch`.
- **Host hard links between a writable and a read-only path**, which the
  check of a file's names does not see (`CODE-OVERVIEW.md`, "Changes to a
  file itself, and its other names").
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

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
All `virtiofs` tests pass (162, one ignored).

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

## Questions for Pedro (not bugs, yet)

- **Device nodes from `mknod`.** `mknod` accepts any file type the guest
  asks for, character and block devices included; if OpenVMM ever runs
  with `CAP_MKNOD` and the share is not on a `nodev` filesystem, a guest
  with one writable path could make a node for a host disk and read it
  through the share. Is it intended to rely on OpenVMM never having
  `CAP_MKNOD`?
- **Host hard links between a writable and a read-only path**, which the
  check of a file's names does not see (`CODE-OVERVIEW.md`, "Changes to a
  file itself, and its other names").

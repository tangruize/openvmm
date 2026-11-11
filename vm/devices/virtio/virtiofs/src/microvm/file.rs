// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! microVM directory enumeration and persisted directory snapshots.

use super::saved_state::MAX_DIRECTORY_ENTRIES_PER_HANDLE;
use super::saved_state::MAX_DIRECTORY_ENTRY_NAME_BYTES;
use super::saved_state::MAX_DIRECTORY_SNAPSHOT_BYTES;
use super::saved_state::SavedDirectoryEntry;
use crate::VirtioFs;
use crate::file::VirtioFsFile;
use fuse::DirEntryWriter;
use fuse::protocol::fuse_entry_out;

impl VirtioFsFile {
    pub(crate) fn read_dir_microvm(
        &self,
        fs: &VirtioFs,
        offset: u64,
        size: u32,
        plus: bool,
    ) -> lx::Result<Vec<u8>> {
        if size as usize > crate::MAX_GUEST_BUFFER_SIZE {
            return Err(lx::Error::E2BIG);
        }

        let entries = {
            let mut snapshot = self.directory_snapshot.write();
            if !snapshot.built {
                if offset != 0 {
                    return Err(lx::Error::EINVAL);
                }
                snapshot.entries = self.build_microvm_directory_snapshot()?;
                snapshot.built = true;
            }
            snapshot.entries.clone()
        };

        let start = if offset == 0 {
            0
        } else {
            entries
                .iter()
                .position(|entry| entry.next_cookie == offset)
                .map(|index| index + 1)
                .ok_or(lx::Error::EINVAL)?
        };
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(size as usize)
            .map_err(|_| lx::Error::ENOMEM)?;

        // Whether the reply stopped because the next entry did not fit.
        let mut full = false;
        // Each readdirplus entry hands the guest a lookup reference, which the
        // guest can release only if it receives the entry. So after an error,
        // return the entries written so far, and report the error only if
        // there are none; the guest's next read starts at the failing entry.
        let mut stopped = None;
        for entry in &entries[start..] {
            let name = lx::LxStr::from_bytes(&entry.name);
            // The cache was filtered against the directory's path when it was
            // built; the guest may have renamed the directory since, so each
            // entry is judged again against its path now, and one that may
            // no longer be listed is left out (its cookie is skipped).
            if entry.name != b"." && entry.name != b".." {
                match self.listable_now(name, entry.kind) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => {
                        stopped = Some(error);
                        break;
                    }
                }
            }
            if plus {
                if entry.name == b"." || entry.name == b".." {
                    if !buffer.dir_entry_plus(
                        name,
                        entry.next_cookie,
                        fuse_entry_out::new_dot(entry.guest_inode_id, entry.kind << 12),
                    ) {
                        full = true;
                        break;
                    }
                } else {
                    // This check must happen before lookup_helper, because a
                    // readdirplus lookup creates a guest lookup reference.
                    if !buffer.check_dir_entry_plus(name) {
                        full = true;
                        break;
                    }
                    let fuse_entry = match fs.lookup_helper(&self.inode, name) {
                        Ok(fuse_entry) => fuse_entry,
                        Err(error) => {
                            stopped = Some(error);
                            break;
                        }
                    };
                    // An entry the guest will not receive must not keep the
                    // lookup reference just taken.
                    let node_id = fuse_entry.nodeid;
                    if fuse_entry.attr.ino != entry.guest_inode_id
                        || !buffer.dir_entry_plus(name, entry.next_cookie, fuse_entry)
                    {
                        fs.forget_lookups(node_id, 1);
                        stopped = Some(lx::Error::EIO);
                        break;
                    }
                }
            } else if !buffer.dir_entry(name, entry.guest_inode_id, entry.next_cookie, entry.kind) {
                full = true;
                break;
            }
        }

        if let Some(error) = stopped {
            if buffer.is_empty() {
                return Err(error);
            }
            return Ok(buffer);
        }
        // Not even one entry fits in the guest's buffer. (A reply left empty
        // because every remaining entry was left out ends the listing.)
        if full && buffer.is_empty() {
            return Err(lx::Error::EINVAL);
        }
        Ok(buffer)
    }

    /// Whether the guest may list the entry `name`, of type `kind`, of this
    /// directory under the directory's path now, by the rule that built the
    /// cache (`entry_listable`, and a hidden path refused by `child_path`).
    fn listable_now(&self, name: &lx::LxStr, kind: u32) -> lx::Result<bool> {
        match self.inode.child_path(name) {
            Ok(path) => Ok(self.inode.volume.entry_listable(&path, kind as u8)),
            Err(error) if error == lx::Error::EACCES => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn build_microvm_directory_snapshot(&self) -> lx::Result<Vec<SavedDirectoryEntry>> {
        let stat = self.object_stat()?;
        if stat.mode & lx::S_IFMT != lx::S_IFDIR {
            return Err(lx::Error::ENOTDIR);
        }

        let self_inode_nr = self.inode.guest_inode_nr();
        let mut total_bytes = 0usize;
        let mut entries = Vec::new();
        self.file.write().read_dir(0, |entry| {
            if entries.len() == MAX_DIRECTORY_ENTRIES_PER_HANDLE {
                return Err(lx::Error::E2BIG);
            }
            let name = entry.name.as_bytes();
            if name.is_empty() || name.len() > MAX_DIRECTORY_ENTRY_NAME_BYTES {
                return Err(lx::Error::EINVAL);
            }
            if name != b"." && name != b".." {
                match self.inode.child_path(&entry.name) {
                    Ok(path) if self.inode.volume.entry_listable(&path, entry.file_type) => {}
                    Ok(_) => return Ok(true),
                    Err(error) if error == lx::Error::EACCES => return Ok(true),
                    Err(error) => return Err(error),
                }
            }
            total_bytes = total_bytes
                .checked_add(name.len())
                .ok_or(lx::Error::E2BIG)?;
            if total_bytes > MAX_DIRECTORY_SNAPSHOT_BYTES {
                return Err(lx::Error::E2BIG);
            }
            let next_cookie = u64::try_from(entries.len())
                .ok()
                .and_then(|index| index.checked_add(1))
                .ok_or(lx::Error::E2BIG)?;
            entries.try_reserve(1).map_err(|_| lx::Error::ENOMEM)?;
            entries.push(SavedDirectoryEntry {
                name: name.to_vec(),
                next_cookie,
                guest_inode_id: if name == b"." || name == b".." {
                    self_inode_nr
                } else {
                    self.inode.guest_ino(entry.inode_nr)
                },
                kind: entry.file_type as u32,
            });
            Ok(true)
        })?;
        Self::validate_directory_entries(&entries)?;
        Ok(entries)
    }

    pub(crate) fn validate_directory_entries(entries: &[SavedDirectoryEntry]) -> lx::Result<()> {
        if entries.len() > MAX_DIRECTORY_ENTRIES_PER_HANDLE {
            return Err(lx::Error::E2BIG);
        }
        let mut total_bytes = 0usize;
        let mut previous_cookie = 0;
        for (index, entry) in entries.iter().enumerate() {
            let dot_entry = entry.name == b"." || entry.name == b"..";
            if entry.name.is_empty()
                || entry.name.len() > MAX_DIRECTORY_ENTRY_NAME_BYTES
                || entry.kind > 15
                || (!dot_entry
                    && (entry.name.contains(&b'/')
                        || entry.name.contains(&b'\0')
                        || entry.name.contains(&b'\\')
                        || entry.name.contains(&b':')))
            {
                return Err(lx::Error::EINVAL);
            }
            if (index == 0 && entry.next_cookie == 0)
                || (index != 0 && entry.next_cookie <= previous_cookie)
            {
                return Err(lx::Error::EINVAL);
            }
            previous_cookie = entry.next_cookie;
            total_bytes = total_bytes
                .checked_add(entry.name.len())
                .ok_or(lx::Error::E2BIG)?;
            if total_bytes > MAX_DIRECTORY_SNAPSHOT_BYTES {
                return Err(lx::Error::E2BIG);
            }
        }
        Ok(())
    }
}

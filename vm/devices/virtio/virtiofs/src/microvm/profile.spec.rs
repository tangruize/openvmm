// Spec of a microVM profile's root identity, for human review: which host
// object it names, as `validate_opened_object` (`profile.rs`) checks it
// against the root's stat when a share is attached or restored.

use super::MicroVmAggregateChild;
use super::MicroVmVirtioFsProfile;
use lxutil::host::HostObject;
use vstd::bytes::spec_u32_from_le_bytes;
use vstd::bytes::spec_u64_from_le_bytes;
use vstd::bytes::spec_u64_to_le_bytes;
use vstd::prelude::*;

verus! {

impl MicroVmVirtioFsProfile {
    /// A root identity names host object `o`.
    ///
    /// On Linux: the bytes `openvmm-microvm-fs-unix-v1\0`, then `o`'s device
    /// number and inode number, each in 8 little-endian bytes: the whole
    /// identity is compared.
    #[cfg(unix)]
    pub(crate) open spec fn identity_names(identity: Seq<u8>, o: HostObject) -> bool {
        identity == b"openvmm-microvm-fs-unix-v1\0"@ + spec_u64_to_le_bytes(o.dev) + spec_u64_to_le_bytes(o.ino)
    }

    /// On Windows: the bytes `openvmm-microvm-fs-windows-v1\0`, then a length
    /// `n` (4 little-endian bytes), the volume's name (`n` UTF-16 units), and the
    /// file id (8 little-endian bytes), which is `o`'s inode number. Only the file
    /// id is compared with the object; the volume's name is checked by path
    /// (`validate_object_path`).
    #[cfg(windows)]
    pub(crate) open spec fn identity_names(identity: Seq<u8>, o: HostObject) -> bool {
        let prefix = b"openvmm-microvm-fs-windows-v1\0"@;
        let rest = identity.subrange(prefix.len() as int, identity.len() as int);
        let n = spec_u32_from_le_bytes(rest.subrange(0, 4)) as int;
        &&& prefix.is_prefix_of(identity)
        &&& rest.len() == 4 + 2 * n + 8
        &&& spec_u64_from_le_bytes(rest.subrange(4 + 2 * n, rest.len() as int)) == o.ino
    }

    /// The profile's root identity names host object `o` (a single share's
    /// root; an aggregate has none).
    pub(crate) closed spec fn identifies_root(&self, o: HostObject) -> bool {
        Self::identity_names(self.root_identity@, o)
    }

    /// The profile's `i`-th aggregate child's root identity names host
    /// object `o`.
    pub(crate) closed spec fn child_identifies_root(&self, i: int, o: HostObject) -> bool {
        &&& 0 <= i < self.children@.len()
        &&& self.children@[i].identifies_root(o)
    }
}

impl MicroVmAggregateChild {
    /// The child's root identity names host object `o`.
    pub(crate) closed spec fn identifies_root(&self, o: HostObject) -> bool {
        MicroVmVirtioFsProfile::identity_names(self.root_identity@, o)
    }
}

} // verus!

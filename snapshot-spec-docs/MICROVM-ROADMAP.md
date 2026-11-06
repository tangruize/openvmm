# The microVM work: its big parts

What our colleague Pedro Henrique Penna's microVM fork of OpenVMM adds, as
candidates for verification beyond the snapshot save and restore we specify
now (2026-11-06).

Source: NVX's `doc/openvmm-upstream-roadmap.md` (microsoft/nvx, branch
`dev`), which maps the fork's 212 commits (`feature/microvm`) to planned
upstream PRs (PR01-PR27, then 81 later commits by position). The roadmap
names no authors; the attribution is from our repo, where Pedro authored 310
commits (2026-09-15 to 10-13) in the same areas. Line counts are lines he
added in our repo, which is behind the roadmap's pin (position 212), so his
later commits may be missing.

The snapshot save and restore itself is PR11, PR12, PR17, PR18 and PR20.
Besides it:

1. **virtio-fs for microVM shares.** Share requests performed as the
   guest's caller; a per-subtree access policy; a filesystem deny policy
   (PR22); safe symlinks on writable shares (PR27); several host directories,
   or single host files, in one share; bounded guest allocations; and FUSE
   namespaces, handles and directory cookies kept across snapshots. About
   10,000 lines in `vm/devices/virtio/virtiofs`.
2. **The NVX time ABI and CPU profiles.** Guest time captured and restored
   across snapshots, with backends for KVM, MSHV and WHP (positions
   132-161); pinned CPU profiles for Intel and AMD hosts; the CPUID a guest
   sees. About 20,000 lines in `vmm_core/cpu_profile` (much of it likely
   profile data), plus `virt`, `virt_kvm`, `virt_mshv` and `virt_whp`. This
   is the guest time our snapshot spec leaves out of scope.
3. **Networking and egress policy.** An egress policy by protocol and
   port, IPv6, guest listeners published to private networks, and new
   policies installed at restore (PR21; positions 169, 175, 179-182, 199).
   About 10,000 lines in `net_consomme` and `net_backend_resources`.
4. **virtio devices.** The fixed virtio-MMIO ABI and shared interrupt
   status (PR16), generic virtio persistence (PR13), `virtio_console`
   (about 6,800 lines), and device DMA restricted to the guest's RAM ranges
   (positions 183-184).
5. **The microVM machine and boot.** The microVM machine profile and its
   minimal chipset (PR15), and direct boot without firmware (PR14).
6. **Control and management.** An authenticated control console (PR23),
   managed workloads and outcome reporting (PR24), the management plane
   (PR25), and the snapshot manifests and command line. About 28,000 lines
   in `openvmm_entry` and `openvmm_helpers`.
7. **Storage for sandboxes.** Sandbox block roles and snapshot tiers
   (PR19), and `disk_zerofill` and `disk_blob` (positions 176-178).
8. **Smaller fixes.** Hypervisor backend fixes (PR05), device hardening
   (PR06), host primitives (PR02-PR04), and a fallible VM lifecycle (PR09).

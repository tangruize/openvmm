# Testing plan: the hypervisor facts for MSHV and WHP

A plan for testing, on real MSHV and WHP hosts, what a VP's save and
restore assume about the hypervisor (2026-11-06).

## Why

The snapshot proof shows that saving and restoring a VP works, but only
under facts about the hypervisor that are trusted, not proved: what the
hypervisor does when OpenVMM reads or writes a VP's registers, APIC, XSAVE
area and so on. The proof never checks these facts. If one is wrong, the
proof still passes, and proves something false about the real system. Only
reading the hypervisor's source and testing it can find such a mistake.

For KVM this is done: the proof agent wrote KVM's facts
(`vmm_core/virt_kvm/src/arch/x86_64/kvm_t.rs` in
`noah-verification/openvmm`, branch `integration/snapshot-spec`) and
tested them (`vmm_core/virt_kvm/src/arch/x86_64/vp_kvm_tests.rs` there).
Its tests found that several facts we had assumed were false for KVM,
e.g. that a VP reads back exactly what was written (KVM sets the
segments' accessed bit and cuts `SYSENTER_EIP`/`ESP` to the host's
address width), and that reading a VP changes nothing (on a host with
MPX, reading the MP state can change the XSAVE header).

The microVM also runs on MSHV and WHP, on Intel and AMD hosts, and their
facts have not been written or tested. MSHV's driver is open source (Linux's
`mshv` driver), but the Hyper-V hypervisor behind it is not; WHP is closed
source, with only Microsoft's documentation. So for these two, testing is
the main way to learn what they really do.

## What to test

OpenVMM reads and writes a VP's state through each backend's state access
(`vmm_core/virt_mshv/src/x86_64`, `vmm_core/virt_whp/src/vp_state.rs`),
over `HvCallGetVpRegisters` / `HvCallSetVpRegisters` and the VP state
ioctls on MSHV, and `WHvGetVirtualProcessorRegisters` /
`WHvSetVirtualProcessorRegisters` and the VP state calls on WHP. Test
through these, the same calls the snapshot code makes.

The spec's claims (`vmm_core/virt/src/state.rs`): a getter returns the
element's current value and changes nothing; a setter sets the element,
and only it; `commit` changes nothing; on a getter's error nothing
changes. For each element of a VP's saved state, check:

1. **Read-back.** Set a value, then read it: is it the value set, or a
   normalized form of it (bits set or cleared, a value cut to a width,
   an address made canonical)? Write down the normalization exactly.
2. **Refusals.** Which values does the set refuse with an error, rather
   than normalize? After a refused set, is the VP unchanged?
3. **A read changes nothing.** Read every element twice, and read each
   element between reads of all the others: does any read change
   anything?
4. **A set changes only its element.** Set one element, then read all
   the others: which change? (KVM couples some: `cr8` with the APIC's
   TPR, the APIC base with the LVT timer, the registers with the
   activity.)
5. **A stopped VP stays as it was.** After the VP stops, does anything
   change it on its own? Send it a fixed, level-triggered, NMI and INIT
   interrupt (through the IOAPIC too), let time pass, then read every
   element. On KVM only the IRR, the TMR and the APIC timer's count may
   change.
6. **Restore order.** Restore a whole saved state in OpenVMM's order
   (registers before the APIC, ...): does the result depend on the order?
7. **Host features.** Which features change any answer above: Intel or
   AMD, the address width (LA57), the XSAVE format (compacted or
   standard), MPX, CET? Record what the backend reports for each.
8. **MSHV only: restore with fewer VPs.** A restore that names a VP
   count brings only that many VPs online on MSHV: check that the others
   are not created, and that those restored hold their saved state.

The elements (`VpSavedStateView`, `vmm_core/virt/src/x86/vp.rs`):
registers (general registers, segments, `cr0`, `cr3`, `cr4`, `cr8`,
`efer`, GDT, IDT), activity (MP state, NMI pending and masked, interrupt
shadow, pending event and interruption), XSAVE, APIC, `xcr0`, `xss`,
MTRRs, PAT, the virtual MSRs (`SYSENTER_CS`, `_EIP`, `_ESP`, `STAR`,
`LSTAR`, `KernelGsBase`, ...), debug registers, TSC, CET, CET shadow
stack, `TSC_AUX`, TSC deadline, the synthetic MSRs, message page, event
flags page, message queues and timers, and the nested state. Check which
elements each backend has at all.

## How

- **Harness.** Create a partition with one or two VPs through OpenVMM's
  own backend (`virt_mshv`, `virt_whp`), stop the VPs, and open a state
  access handle, as the snapshot code does. Items 5 and 8 need a tiny
  guest; the others need none.
- **Inputs.** Random values for each element, biased towards edge cases:
  reserved bits set, high bits set, non-canonical addresses, the largest
  and smallest values, values from another host (wider registers).
- **Oracle.** Start from the spec's claim (read-back equals the value
  set; nothing else changes). Each disagreement is minimized and
  classified: a normalization, a refusal, a coupling between elements, or
  a hypervisor bug.
- **Template.** The KVM tests do the same for KVM: e.g.
  `kvm_reads_back_each_element_as_normalized`,
  `after_stop_only_the_irr_tmr_and_timer_count_change`,
  `a_device_nmi_to_a_stopped_vp_changes_only_nmi_pending` and
  `restore_of_a_vp_state_naming_an_absent_element_is_refused`
  (`vp_kvm_tests.rs`).
- **Hosts.** MSHV and WHP, each on an Intel and an AMD host. Record each
  host's CPU model, OS build and hypervisor version.

## What to deliver

For each backend, and each element:

- its normalization (a precise rule), its refused values, and the other
  elements a set of it changes;
- what may change on a stopped VP;
- the host features that change any of these;
- for each fact, the test that shows it, the hosts it ran on, and, where
  one exists, the documentation that states it (the Hyper-V TLFS,
  Microsoft's WHP documentation, Linux's `mshv` driver).

These become the backends' trusted contracts, the counterparts of
`kvm_t.rs`, each fact citing its test. Start with the elements whose KVM
facts needed a normalization or a coupling (registers, activity, XSAVE,
APIC, the virtual MSRs, `TSC_AUX`), since the other hypervisors most
likely differ there too.

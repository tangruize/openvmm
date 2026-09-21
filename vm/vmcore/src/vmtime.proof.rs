// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl View for VmTime {
    type V = u64;

    closed spec fn view(&self) -> u64 {
        self.0
    }
}

proof fn lemma_wrapping_add_100ns(time: u64, nanos: u128)
    ensures
        vstd::wrapping::u128_specs::wrapping_add(time as u128, nanos / 100) as u64
            == ((time as nat + nanos as nat / 100)
                % 0x1_0000_0000_0000_0000) as u64,
{
    assert(time as int + nanos / 100 <= u128::MAX);
    let sum = (time as nat + nanos as nat / 100) as u128;
    assert(sum as u64 == (sum % 0x1_0000_0000_0000_0000u128) as u64)
        by (bit_vector);
}

} // verus!

// Copyright © Aptos Foundation

/// Regression test for fixed_point64::round verification.
/// Extracted from the framework to ensure opaque call chains
/// (round → floor/ceil) verify correctly in isolation.
module 0x1::fixed_point64 {
    struct FixedPoint64 has copy, drop, store { value: u128 }

    public fun floor(self: FixedPoint64): u128 {
        self.value >> 64
    }
    spec floor {
        pragma opaque;
        aborts_if false;
        ensures result == spec_floor(self);
    }
    spec fun spec_floor(self: FixedPoint64): u128 {
        self.value >> 64
    }

    public fun ceil(self: FixedPoint64): u128 {
        let floored_num = self.floor() << 64;
        if (self.value == floored_num) {
            return floored_num >> 64
        };
        let val = ((floored_num as u256) + (1 << 64));
        (val >> 64 as u128)
    }
    spec ceil {
        pragma opaque;
        aborts_if false;
        ensures result == spec_ceil(self);
    }
    spec fun spec_ceil(self: FixedPoint64): u128 {
        if (self.value % (1 << 64) == 0) { self.value >> 64 }
        else { (self.value >> 64) + 1 }
    }

    public fun round(self: FixedPoint64): u128 {
        let floored_num = self.floor() << 64;
        let boundary = floored_num + ((1 << 64) / 2);
        if (self.value < boundary) {
            floored_num >> 64
        } else {
            self.ceil()
        }
    }
    spec round {
        pragma opaque;
        aborts_if false;
        ensures result == spec_round(self);
    }
    spec fun spec_round(self: FixedPoint64): u128 {
        if (self.value % (1 << 64) < (1 << 64) / 2) { self.value >> 64 }
        else { (self.value >> 64) + 1 }
    }
}

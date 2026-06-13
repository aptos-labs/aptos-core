// RUN: publish
module 0x42::enum_computed_write {
    enum Boxed has drop {
        One { value: u64 },
        Two { value: u64, other: u64 },
    }

    // `value` is at the same offset in both variants.
    fun set_bumped(b: &mut Boxed, delta: u64) {
        b.value = delta + 1;
    }

    fun run(v: u64, delta: u64): u64 {
        let b = Boxed::Two { value: v, other: 0 };
        set_bumped(&mut b, delta);
        b.value
    }
}

// RUN: execute 0x42::enum_computed_write::run --args 5, 99
// CHECK: results: 100

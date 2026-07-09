// RUN: publish --print(micro-ops)
module 0x42::enum_variant_field_uniform {
    enum Boxed has drop {
        One { value: u64 },
        Two { value: u64, other: u64 },
    }

    // `value` sits at the same data-region offset in every variant, so the
    // by-reference read takes the uniform fast path.
    fun read_value(v: u64): u64 {
        let b = Boxed::One { value: v };
        let r = &b.value;
        *r
    }

    fun write_computed(b: &mut Boxed, delta: u64) {
        b.value = delta + 1;
    }

    fun write_simple(b: &mut Boxed, delta: u64) {
        b.value = delta;
    }

    fun run_computed(v: u64, delta: u64): u64 {
        let b = Boxed::Two { value: v, other: 0 };
        write_computed(&mut b, delta);
        b.value
    }

    fun run_simple(v: u64, delta: u64): u64 {
        let b = Boxed::One { value: v };
        write_simple(&mut b, delta);
        b.value
    }
}

// RUN: execute 0x42::enum_variant_field_uniform::read_value --args 41
// CHECK: results: 41

// RUN: execute 0x42::enum_variant_field_uniform::run_computed --args 5, 99
// CHECK: results: 100

// RUN: execute 0x42::enum_variant_field_uniform::run_simple --args 5, 42
// CHECK: results: 42

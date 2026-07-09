// RUN: publish
module 0x42::generic_phantom {
    struct Tagged<phantom T> has copy, drop {
        x: u64,
    }

    fun make<T>(x: u64): Tagged<T> {
        Tagged { x }
    }

    fun read<T>(t: &Tagged<T>): u64 {
        t.x
    }

    fun run(v: u64): u64 {
        let a = make<bool>(v);
        let b = make<u64>(v + 1);
        let c = make<Tagged<bool>>(v + 2);
        read(&a) + read(&b) + read(&c)
    }
}

// RUN: execute 0x42::generic_phantom::run --args 10
// CHECK: results: 33

// RUN: publish --print(stackless,micro-ops)
module 0x77::chained_field_fusion_generic {
    struct B<T> has drop, copy {
        z: T,
    }

    struct A<T> has drop, copy {
        first: T,
        y: B<T>,
    }

    struct S<T> has drop, copy {
        x: A<T>,
    }

    fun read_chain<T: drop + copy>(s: &S<T>): T {
        s.x.y.z
    }

    fun mk<T: drop + copy>(first: T, z: T): S<T> {
        S { x: A { first, y: B { z } } }
    }

    fun run_u64(z: u64): u64 {
        let s = mk<u64>(0, z);
        read_chain(&s)
    }

    fun run_u8(z: u8): u8 {
        let s = mk<u8>(0, z);
        read_chain(&s)
    }
}

// RUN: execute 0x77::chained_field_fusion_generic::run_u64 --args 123
// CHECK: results: 123
// RUN: execute 0x77::chained_field_fusion_generic::run_u8 --args 250
// CHECK: results: 250

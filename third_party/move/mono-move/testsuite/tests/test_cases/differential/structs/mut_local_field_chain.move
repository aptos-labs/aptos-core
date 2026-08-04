// RUN: publish --print(stackless,micro-ops)
module 0x88::mut_local_field_chain {
    struct B has drop, copy {
        pad: u64,
        z: u64,
    }

    struct A has drop, copy {
        y: B,
        w: u64,
    }

    struct S has drop, copy {
        x: A,
    }

    fun build(z: u64): S {
        S { x: A { y: B { pad: 0, z }, w: 0 } }
    }

    fun bump(z: u64): u64 {
        let s = build(z);
        let r = &mut s.x.y.z;
        *r = *r + 1;
        // Reads `s` again: must see the value written through `r`.
        s.x.y.z
    }
}

// RUN: execute 0x88::mut_local_field_chain::bump --args 41
// CHECK: results: 42

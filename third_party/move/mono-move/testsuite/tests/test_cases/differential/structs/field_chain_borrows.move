// RUN: publish --print(stackless,micro-ops)
module 0x88::field_chain_borrows {
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

    fun read_ref(r: &u64): u64 {
        *r
    }

    fun deep_mut(s: &mut S): &mut u64 {
        &mut s.x.y.z
    }

    fun read_via_call(z: u64): u64 {
        let s = build(z);
        read_ref(&s.x.y.z)
    }

    fun read_via_local_ref(z: u64): u64 {
        let s = build(z);
        let r = &s.x.y.z;
        *r
    }

    fun write_via_local(z: u64): u64 {
        let s = build(0);
        s.x.y.z = z + 1;
        s.x.y.z
    }

    fun bump_via_deep_mut(z: u64): u64 {
        let s = build(z);
        let r = deep_mut(&mut s);
        *r = *r + 1;
        s.x.y.z
    }
}

// RUN: execute 0x88::field_chain_borrows::read_via_local_ref --args 5
// CHECK: results: 5
// RUN: execute 0x88::field_chain_borrows::read_via_call --args 6
// CHECK: results: 6
// RUN: execute 0x88::field_chain_borrows::write_via_local --args 9
// CHECK: results: 10
// RUN: execute 0x88::field_chain_borrows::bump_via_deep_mut --args 7
// CHECK: results: 8

// RUN: publish
module 0x42::copy_semantics {
    use std::vector;

    enum E has copy, drop { V { f: u64 } }
    struct S has copy, drop { e: E }
    struct Two has copy, drop { a: E, b: E }
    enum Inner has copy, drop { Val { n: u64 } }
    enum Outer has copy, drop { Wrap { inner: Inner } }
    enum WithVec has copy, drop { V { items: vector<u64> } }

    fun get_f(e: E): u64 { match (e) { V { f } => f } }

    // Copy a local: mutate the copy, original unchanged.
    fun enum_copy(): u64 {
        let a = E::V { f: 1 };
        let b = a;
        b.f = 99;
        a.f * 1000 + b.f
    }

    // Copy a struct holding an enum.
    fun struct_with_enum_copy(): u64 {
        let s1 = S { e: E::V { f: 1 } };
        let s2 = s1;
        s2.e.f = 99;
        s1.e.f * 1000 + s2.e.f
    }

    // Read an enum field out of a struct by value.
    fun readfield_copy(): u64 {
        let s = S { e: E::V { f: 1 } };
        let x = s.e;
        x.f = 99;
        s.e.f * 1000 + x.f
    }

    // Deref-read an enum value out of a reference.
    fun readref_copy(): u64 {
        let e = E::V { f: 1 };
        let r = &e;
        let x = *r;
        x.f = 99;
        e.f * 1000 + x.f
    }

    // Multi-pointer aggregate: a struct with two enum fields, copied as a unit.
    fun two_field_struct_copy(): u64 {
        let t1 = Two { a: E::V { f: 1 }, b: E::V { f: 2 } };
        let t2 = t1;
        t2.a.f = 99;
        t2.b.f = 88;
        t1.a.f + t1.b.f + t2.a.f + t2.b.f
    }

    // Copy into a call argument.
    fun xfer_dst_copy(): u64 {
        let a = E::V { f: 7 };
        let via_copy = get_f(a);
        via_copy * 1000 + a.f
    }

    fun copy_no_mutate(): u64 {
        let a = E::V { f: 5 };
        let b = a;
        a.f + b.f
    }

    fun move_enum(): u64 {
        let a = E::V { f: 5 };
        let b = a;
        b.f
    }

    fun nested_enum_copy(): u64 {
        let a = Outer::Wrap { inner: Inner::Val { n: 1 } };
        let b = a;
        b.inner.n = 99;
        a.inner.n * 1000 + b.inner.n
    }

    fun enum_with_vector_copy(): u64 {
        let items = vector::empty<u64>();
        vector::push_back(&mut items, 10);
        vector::push_back(&mut items, 20);
        let a = WithVec::V { items };
        let b = a;
        *vector::borrow_mut(&mut b.items, 0) = 99;
        *vector::borrow(&a.items, 0) * 1000 + *vector::borrow(&b.items, 0)
    }
}

// RUN: execute 0x42::copy_semantics::enum_copy
// CHECK: results: 1099

// RUN: execute 0x42::copy_semantics::struct_with_enum_copy
// CHECK: results: 1099

// RUN: execute 0x42::copy_semantics::readfield_copy
// CHECK: results: 1099

// RUN: execute 0x42::copy_semantics::readref_copy
// CHECK: results: 1099

// RUN: execute 0x42::copy_semantics::two_field_struct_copy
// CHECK: results: 190

// RUN: execute 0x42::copy_semantics::xfer_dst_copy
// CHECK: results: 7007

// RUN: execute 0x42::copy_semantics::copy_no_mutate
// CHECK: results: 10

// RUN: execute 0x42::copy_semantics::move_enum
// CHECK: results: 5

// RUN: execute 0x42::copy_semantics::nested_enum_copy
// CHECK: results: 1099

// RUN: execute 0x42::copy_semantics::enum_with_vector_copy
// CHECK: results: 10099

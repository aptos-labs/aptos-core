// RUN: publish
module 0x42::generic_copy_gc {
    use std::vector;

    struct Box<T> has copy, drop {
        value: T,
    }

    fun copy_across_gc(seed: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, seed);
        vector::push_back(&mut v, seed + 1);
        let b = Box { value: v };
        let b2 = b;
        0x0::test_utils::force_gc();
        // Mutating the copy must not affect the original.
        vector::push_back(&mut b2.value, 1000);
        let Box { value: original } = b;
        let Box { value: copied } = b2;
        vector::length(&original) * 100 + vector::length(&copied)
    }

    fun generic_copy<T: copy + drop>(b: Box<T>): (Box<T>, Box<T>) {
        let b2 = b;
        0x0::test_utils::force_gc();
        (b, b2)
    }

    fun run_generic_copy(seed: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, seed);
        let (a, b) = generic_copy(Box { value: v });
        let Box { value: va } = a;
        let Box { value: vb } = b;
        *vector::borrow(&va, 0) + *vector::borrow(&vb, 0)
    }
}

// RUN: execute 0x42::generic_copy_gc::copy_across_gc --args 7
// CHECK: results: 203
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::generic_copy_gc::run_generic_copy --args 11
// CHECK: results: 22
// CHECK-GC-COUNT: 1

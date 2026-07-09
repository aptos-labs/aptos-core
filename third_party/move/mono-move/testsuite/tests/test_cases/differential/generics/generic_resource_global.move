// RUN: publish
module 0x42::generic_resource_global {
    struct S<T> has key {
        value: T,
    }

    struct D<T> has key, drop {
        value: T,
    }

    fun publish_check_take(s: signer, a: address, v: u64): u64 {
        move_to(&s, S<u64> { value: v });
        let present = if (exists<S<u64>>(a)) { 1 } else { 0 };
        let refs_equal = if (borrow_global<S<u64>>(a) == borrow_global<S<u64>>(a)) {
            1
        } else {
            0
        };
        let S { value } = move_from<S<u64>>(a);
        let absent = if (!exists<S<u64>>(a)) { 1 } else { 0 };
        value + present * 1000 + refs_equal * 100 + absent * 10
    }

    fun two_instantiations(s: signer, a: address, v: u64): u64 {
        move_to(&s, S<u64> { value: v });
        move_to(&s, S<bool> { value: true });
        let both = if (exists<S<u64>>(a) && exists<S<bool>>(a)) { 1 } else { 0 };
        let S { value } = move_from<S<u64>>(a);
        let S { value: flag } = move_from<S<bool>>(a);
        let only_u64_gone = if (!exists<S<u64>>(a) && !exists<S<bool>>(a)) { 1 } else { 0 };
        value + both * 100 + (if (flag) { 10 } else { 0 }) + only_u64_gone
    }

    fun mutate_whole(s: signer, a: address, v: u64): u64 {
        move_to(&s, D<u64> { value: v });
        *borrow_global_mut<D<u64>>(a) = D<u64> { value: v + 5 };
        let D { value } = move_from<D<u64>>(a);
        value
    }

    // Aborts: double-publish of the same instantiation.
    fun double_publish(s: signer, v: u64) {
        move_to(&s, S<u64> { value: v });
        move_to(&s, S<u64> { value: v + 1 });
    }

    // Aborts: moving out an instantiation that was never published.
    fun take_missing(a: address): u64 {
        let S { value } = move_from<S<bool>>(a);
        if (value) { 1 } else { 0 }
    }
}

// RUN: execute 0x42::generic_resource_global::publish_check_take --args 0x42, 0x42, 9
// CHECK: results: 1119

// RUN: execute 0x42::generic_resource_global::two_instantiations --args 0x7, 0x7, 50
// CHECK: results: 161

// RUN: execute 0x42::generic_resource_global::mutate_whole --args 0x9, 0x9, 10
// CHECK: results: 15

// RUN: execute 0x42::generic_resource_global::double_publish --args 0x11, 3
// CHECK-V1-SUBSTR: RESOURCE_ALREADY_EXISTS
// CHECK-V2-SUBSTR: already exists

// RUN: execute 0x42::generic_resource_global::take_missing --args 0x12
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: MoveFrom: resource does not exist

// Copying a struct with an enum field and a vector field. In `copy_once`, with a dead enum
// from `junk` on the heap, sizes 144 and 175 let the first field's copy fit and make the
// second collect; at 176 nothing collects. The copy's fields are then overwritten and both
// owners read, so a copy sharing either field with the original would change the result.

// RUN: publish
module 0x42::copy_batch_gc {
    enum E has copy, drop { V { a: u64, b: u64 } }

    struct Two has copy, drop { e: E, v: vector<u64> }

    fun fold_in(acc: u64, two: Two): u64 {
        let Two { e: payload, v: values } = two;
        match (payload) {
            E::V { a: first, b: second } => (acc ^ first) + (second << 1) + *std::vector::borrow(&values, 1),
        }
    }

    fun junk(value: u64): u64 {
        let garbage = E::V { a: value, b: value };
        match (garbage) { E::V { a: first, b: _ } => first }
    }

    fun scribble(two: &mut Two, value: u64) {
        *std::vector::borrow_mut(&mut two.v, 1) = value;
        match (&mut two.e) { E::V { a: first, b: _ } => *first = value + 1 };
    }

    public fun copy_once(): u64 {
        let live = Two { e: E::V { a: 7, b: 11 }, v: vector[1, 2, 3] };
        junk(0);
        let dup = live;
        scribble(&mut dup, 50);
        fold_in(0, dup) * 1000 + fold_in(0, live)
    }

    public fun churn(count: u64): u64 {
        let live = Two { e: E::V { a: 7, b: 11 }, v: vector[1, 2, 3] };
        let digest: u64 = 0;
        let index = 0;
        while (index < count) {
            let dup = live;
            scribble(&mut dup, index);
            digest = fold_in(digest, dup);
            junk(index);
            index = index + 1;
        };
        fold_in(digest, live)
    }
}

// RUN: execute 0x42::copy_batch_gc::copy_once --heap-size 144
// CHECK: results: 123031
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::copy_batch_gc::copy_once --heap-size 175
// CHECK: results: 123031
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::copy_batch_gc::copy_once --heap-size 176
// CHECK: results: 123031
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::copy_batch_gc::churn --args 20 --heap-size 176
// CHECK: results: 691
// CHECK-GC-COUNT: 19

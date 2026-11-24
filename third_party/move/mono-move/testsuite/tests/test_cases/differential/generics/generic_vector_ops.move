// Vector operations on `vector<T>` inside generic functions, instantiated with element types
// of different sizes: `u8`, `u128`, a struct with mixed-width fields and `vector<u64>`.

// RUN: publish
module 0x1::generic_vector_ops {
    use std::vector;

    struct Padded has copy, drop { a: u8, b: u64, c: u128 }

    // Returns values[0] and values[1] after the shuffle (values[1] holds the value written
    // through borrow_mut), the popped element, and len * 10 + drained count.
    fun churn<T: copy + drop>(elem0: T, elem1: T, elem2: T): (T, T, T, u64) {
        let values = vector[elem0, elem1];
        vector::push_back(&mut values, elem2);
        vector::swap(&mut values, 0, 2);
        let popped = vector::pop_back(&mut values);
        *vector::borrow_mut(&mut values, 1) = popped;
        let first = *vector::borrow(&values, 0);
        let second = *vector::borrow(&values, 1);
        let len = vector::length(&values);
        let drained = 0;
        while (!vector::is_empty(&values)) {
            vector::pop_back(&mut values);
            drained = drained + 1;
        };
        vector::destroy_empty(values);
        (first, second, popped, len * 10 + drained)
    }

    // Rotates left by one through swaps at computed indices.
    fun rotate<T: copy + drop>(values: &mut vector<T>) {
        let len = vector::length(values);
        let index = 1;
        while (index < len) {
            vector::swap(values, index - 1, index);
            index = index + 1;
        }
    }

    public fun run_u8(): (u8, u8, u8, u64) {
        churn(1u8, 2u8, 3u8)
    }

    public fun run_u128(): (u128, u128, u128, u64) {
        churn(340282366920938463463374607431768211455u128, 3u128, 5u128)
    }

    public fun run_padded(): (u8, u64, u128, u8, u64, u128, u8, u64, u128, u64) {
        let elem0 = Padded { a: 1, b: 2, c: 1267650600228229401496703205379 };
        let elem1 = Padded { a: 4, b: 5, c: 1267650600228229401496703205382 };
        let elem2 = Padded { a: 7, b: 8, c: 1267650600228229401496703205385 };
        let (first, second, popped, len) = churn(elem0, elem1, elem2);
        (first.a, first.b, first.c, second.a, second.b, second.c, popped.a, popped.b, popped.c, len)
    }

    // Also checks that the returned vectors are independent of each other and of `elem0`:
    // each gets a write at a different position.
    public fun run_vec(base: u64): (vector<u64>, vector<u64>, vector<u64>, vector<u64>, u64) {
        let elem0 = vector[base, base + 1];
        let elem1 = vector[base * 10];
        let elem2 = vector[base * 100, 5];
        let (first, second, popped, len) = churn(elem0, elem1, elem2);
        *vector::borrow_mut(&mut popped, 0) = 1;
        *vector::borrow_mut(&mut second, 1) = 2;
        *vector::borrow_mut(&mut first, 0) = 3;
        (first, second, popped, elem0, len)
    }

    public fun run_rotate(): (vector<u64>, vector<u64>, vector<u64>) {
        let values = vector[vector[1u64], vector[2, 2], vector[3, 3, 3]];
        rotate(&mut values);
        (*vector::borrow(&values, 0), *vector::borrow(&values, 1), *vector::borrow(&values, 2))
    }
}

// RUN: execute 0x1::generic_vector_ops::run_u8
// CHECK: results: 3, 1, 1, 22

// RUN: execute 0x1::generic_vector_ops::run_u128
// CHECK: results: 5, 340282366920938463463374607431768211455, 340282366920938463463374607431768211455, 22

// RUN: execute 0x1::generic_vector_ops::run_padded
// CHECK: results: 7, 8, 1267650600228229401496703205385, 1, 2, 1267650600228229401496703205379, 1, 2, 1267650600228229401496703205379, 22

// RUN: execute 0x1::generic_vector_ops::run_vec --args 7
// CHECK: results: [3, 5], [7, 2], [1, 8], [7, 8], 22

// RUN: execute 0x1::generic_vector_ops::run_vec --args 7 --heap-size 288
// CHECK: results: [3, 5], [7, 2], [1, 8], [7, 8], 22
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::generic_vector_ops::run_rotate
// CHECK: results: [2, 2], [3, 3, 3], [1]

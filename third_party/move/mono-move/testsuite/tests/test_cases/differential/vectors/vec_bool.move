// Differential coverage for `vector<bool>` (1-byte elements). Mirrors the
// narrow-integer vector tests but for the boolean element type, exercising
// push_back / pop_back / length / borrow / borrow_mut.

// RUN: publish
module 0x99::vec_bool {
    use std::vector;

    // Push three values, read each one back by index.
    public fun read_all(a: bool, b: bool, c: bool): (bool, bool, bool) {
        let v = vector::empty<bool>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // Overwrite the middle element through `borrow_mut`; the neighbours must
    // be untouched.
    public fun mutate_mid(a: bool, b: bool, c: bool, new_b: bool): (bool, bool, bool) {
        let v = vector::empty<bool>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // push/pop round-trip plus length, popping in reverse push order.
    public fun push_pop_len(a: bool, b: bool): (bool, bool, u64) {
        let v = vector::empty<bool>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        let second = vector::pop_back(&mut v);
        let first = vector::pop_back(&mut v);
        (first, second, vector::length(&v))
    }
}

// --- Element stride ---
// RUN: execute 0x99::vec_bool::read_all --args true, false, true
// CHECK: results: true, false, true
// RUN: execute 0x99::vec_bool::read_all --args false, true, false
// CHECK: results: false, true, false

// --- Mutation through a reference ---
// RUN: execute 0x99::vec_bool::mutate_mid --args true, true, true, false
// CHECK: results: true, false, true
// RUN: execute 0x99::vec_bool::mutate_mid --args false, false, false, true
// CHECK: results: false, true, false

// --- push / pop / length ---
// RUN: execute 0x99::vec_bool::push_pop_len --args true, false
// CHECK: results: true, false, 0
// RUN: execute 0x99::vec_bool::push_pop_len --args false, true
// CHECK: results: false, true, 0

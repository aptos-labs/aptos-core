// RUN: publish
module 0x99::vec_narrow_int {
    use std::vector;

    // --- u8 (1-byte) ---
    public fun u8_read_all(a: u8, b: u8, c: u8): (u8, u8, u8) {
        let v = vector::empty<u8>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun u8_mutate_mid(a: u8, b: u8, c: u8, new_b: u8): (u8, u8, u8) {
        let v = vector::empty<u8>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // push/pop round-trip plus length, popping in reverse push order.
    public fun u8_push_pop_len(a: u8, b: u8): (u8, u8, u64) {
        let v = vector::empty<u8>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        let second = vector::pop_back(&mut v);
        let first = vector::pop_back(&mut v);
        (first, second, vector::length(&v))
    }

    // --- u16 (2-byte) ---
    public fun u16_read_all(a: u16, b: u16, c: u16): (u16, u16, u16) {
        let v = vector::empty<u16>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun u16_mutate_mid(a: u16, b: u16, c: u16, new_b: u16): (u16, u16, u16) {
        let v = vector::empty<u16>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // --- u32 (4-byte) ---
    public fun u32_read_all(a: u32, b: u32, c: u32): (u32, u32, u32) {
        let v = vector::empty<u32>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun u32_mutate_mid(a: u32, b: u32, c: u32, new_b: u32): (u32, u32, u32) {
        let v = vector::empty<u32>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun u32_push_pop_len(a: u32, b: u32): (u32, u32, u64) {
        let v = vector::empty<u32>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        let second = vector::pop_back(&mut v);
        let first = vector::pop_back(&mut v);
        (first, second, vector::length(&v))
    }

    // --- i8 (1-byte signed) ---
    public fun i8_read_all(a: i8, b: i8, c: i8): (i8, i8, i8) {
        let v = vector::empty<i8>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun i8_mutate_mid(a: i8, b: i8, c: i8, new_b: i8): (i8, i8, i8) {
        let v = vector::empty<i8>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // --- i16 (2-byte signed) ---
    public fun i16_read_all(a: i16, b: i16, c: i16): (i16, i16, i16) {
        let v = vector::empty<i16>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    // --- i32 (4-byte signed) ---
    public fun i32_read_all(a: i32, b: i32, c: i32): (i32, i32, i32) {
        let v = vector::empty<i32>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }

    public fun i32_mutate_mid(a: i32, b: i32, c: i32, new_b: i32): (i32, i32, i32) {
        let v = vector::empty<i32>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        vector::push_back(&mut v, c);
        *vector::borrow_mut(&mut v, 1) = new_b;
        (*vector::borrow(&v, 0), *vector::borrow(&v, 1), *vector::borrow(&v, 2))
    }
}

// ===== u8 =====
// RUN: execute 0x99::vec_narrow_int::u8_read_all --args 10, 20, 30
// CHECK: results: 10, 20, 30
// RUN: execute 0x99::vec_narrow_int::u8_read_all --args 255, 0, 128
// CHECK: results: 255, 0, 128
// RUN: execute 0x99::vec_narrow_int::u8_mutate_mid --args 11, 22, 33, 99
// CHECK: results: 11, 99, 33
// RUN: execute 0x99::vec_narrow_int::u8_mutate_mid --args 1, 2, 3, 255
// CHECK: results: 1, 255, 3
// RUN: execute 0x99::vec_narrow_int::u8_push_pop_len --args 7, 200
// CHECK: results: 7, 200, 0
// RUN: execute 0x99::vec_narrow_int::u8_push_pop_len --args 255, 255
// CHECK: results: 255, 255, 0

// ===== u16 =====
// RUN: execute 0x99::vec_narrow_int::u16_read_all --args 1000, 40000, 65535
// CHECK: results: 1000, 40000, 65535
// Values above the u8 range must not be truncated.
// RUN: execute 0x99::vec_narrow_int::u16_read_all --args 65535, 0, 256
// CHECK: results: 65535, 0, 256
// RUN: execute 0x99::vec_narrow_int::u16_mutate_mid --args 100, 200, 300, 65535
// CHECK: results: 100, 65535, 300

// ===== u32 =====
// RUN: execute 0x99::vec_narrow_int::u32_read_all --args 100000, 3000000000, 4294967295
// CHECK: results: 100000, 3000000000, 4294967295
// RUN: execute 0x99::vec_narrow_int::u32_read_all --args 4294967295, 0, 65536
// CHECK: results: 4294967295, 0, 65536
// RUN: execute 0x99::vec_narrow_int::u32_mutate_mid --args 1, 2, 3, 4294967295
// CHECK: results: 1, 4294967295, 3
// RUN: execute 0x99::vec_narrow_int::u32_push_pop_len --args 7, 3000000000
// CHECK: results: 7, 3000000000, 0

// ===== i8 =====
// RUN: execute 0x99::vec_narrow_int::i8_read_all --args -128, 0, 127
// CHECK: results: -128, 0, 127
// RUN: execute 0x99::vec_narrow_int::i8_read_all --args -1, -2, -3
// CHECK: results: -1, -2, -3
// RUN: execute 0x99::vec_narrow_int::i8_mutate_mid --args 5, 6, 7, -128
// CHECK: results: 5, -128, 7

// ===== i16 =====
// RUN: execute 0x99::vec_narrow_int::i16_read_all --args -32768, 0, 32767
// CHECK: results: -32768, 0, 32767
// RUN: execute 0x99::vec_narrow_int::i16_read_all --args -1, -200, -30000
// CHECK: results: -1, -200, -30000

// ===== i32 =====
// RUN: execute 0x99::vec_narrow_int::i32_read_all --args -2147483648, 0, 2147483647
// CHECK: results: -2147483648, 0, 2147483647
// RUN: execute 0x99::vec_narrow_int::i32_mutate_mid --args 10, 20, 30, -2147483648
// CHECK: results: 10, -2147483648, 30

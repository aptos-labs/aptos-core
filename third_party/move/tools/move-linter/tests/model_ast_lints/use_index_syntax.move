// Tests for the use_index_syntax lint.
// Detects vector::borrow/borrow_mut calls that can use index syntax v[i].

module 0xc0ffee::m {
    use std::vector;

    struct MyStruct has copy, drop {
        value: u64,
    }

    // === Should warn: verbose vector::borrow calls ===

    // Warn: vector::borrow can be &v[i]
    public fun test_borrow_warn(v: &vector<u64>, i: u64): &u64 {
        vector::borrow(v, i)
    }

    // Warn: vector::borrow_mut can be &mut v[i]
    public fun test_borrow_mut_warn(v: &mut vector<u64>, i: u64): &mut u64 {
        vector::borrow_mut(v, i)
    }

    // Warn: *vector::borrow can be v[i]
    public fun test_deref_borrow_warn(v: &vector<u64>, i: u64): u64 {
        *vector::borrow(v, i)
    }

    // Warn: *vector::borrow_mut = x can be v[i] = x
    public fun test_deref_borrow_mut_assign_warn(v: &mut vector<u64>, i: u64, x: u64) {
        *vector::borrow_mut(v, i) = x;
    }

    // Warn: vector::borrow(...).field can be v[i].field
    public fun test_borrow_field_warn(v: &vector<MyStruct>, i: u64): u64 {
        vector::borrow(v, i).value
    }

    // Warn: vector::borrow_mut(...).field can be v[i].field
    public fun test_borrow_mut_field_read_warn(v: &mut vector<MyStruct>, i: u64): u64 {
        vector::borrow_mut(v, i).value
    }

    // Warn: vector::borrow_mut(...).field = x can be v[i].field = x
    public fun test_borrow_mut_field_assign_warn(v: &mut vector<MyStruct>, i: u64, x: u64) {
        vector::borrow_mut(v, i).value = x;
    }

    // Warn: nested vector::borrow
    public fun test_nested_borrow_warn(v: &vector<vector<u64>>, i: u64, j: u64): &u64 {
        vector::borrow(vector::borrow(v, i), j)
    }

    // Warn: vector::borrow in condition
    public fun test_borrow_in_condition_warn(v: &vector<u64>, i: u64): bool {
        *vector::borrow(v, i) > 0
    }

    // === Should NOT warn: already using index syntax ===

    // No warn: index syntax
    public fun test_index_no_warn(v: &vector<u64>, i: u64): u64 {
        v[i]
    }

    // No warn: mutable index syntax
    public fun test_index_mut_no_warn(v: &mut vector<u64>, i: u64, x: u64) {
        v[i] = x;
    }

    // No warn: index with field access
    public fun test_index_field_no_warn(v: &vector<MyStruct>, i: u64): u64 {
        v[i].value
    }

    // === Should NOT warn: not vector::borrow ===

    // No warn: other vector functions
    public fun test_other_vector_fns_no_warn(v: &mut vector<u64>, x: u64) {
        vector::push_back(v, x);
    }

    // No warn: vector::length is not borrow
    public fun test_vector_length_no_warn(v: &vector<u64>): u64 {
        vector::length(v)
    }

    // No warn: vector::empty is not borrow
    public fun test_vector_empty_no_warn(): vector<u64> {
        vector::empty()
    }

    // === Lint skip ===

    // No warn: lint skip
    #[lint::skip(use_index_syntax)]
    public fun test_skip_no_warn(v: &vector<u64>, i: u64): &u64 {
        vector::borrow(v, i)
    }
}

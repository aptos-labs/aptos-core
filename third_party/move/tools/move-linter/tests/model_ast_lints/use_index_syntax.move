// Tests for the use_index_syntax lint.
// Detects vector::borrow/borrow_mut calls that can use index syntax v[i],
// and borrow_global/borrow_global_mut calls that can use index syntax T[addr].

module 0xc0ffee::m {
    use std::vector;

    struct MyStruct has copy, drop {
        value: u64,
    }

    struct MyResource has key, copy, drop {
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

    // === Should NOT warn: already using vector index syntax ===

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

    // === Should warn: verbose borrow_global calls ===

    // Warn: borrow_global can be &MyResource[addr]
    public fun test_borrow_global_warn(addr: address): u64 acquires MyResource {
        let _r = borrow_global<MyResource>(addr);
        _r.value
    }

    // Warn: borrow_global_mut can be &mut MyResource[addr]
    public fun test_borrow_global_mut_warn(addr: address): u64 acquires MyResource {
        let _r = borrow_global_mut<MyResource>(addr);
        _r.value
    }

    // Warn: *borrow_global can be MyResource[addr]
    public fun test_deref_borrow_global_warn(addr: address): MyResource acquires MyResource {
        *borrow_global<MyResource>(addr)
    }

    // Warn: *borrow_global_mut = x can be MyResource[addr] = x
    public fun test_deref_borrow_global_mut_assign_warn(addr: address, x: MyResource) acquires MyResource {
        *borrow_global_mut<MyResource>(addr) = x;
    }

    // Warn: borrow_global(...).field can be MyResource[addr].field
    public fun test_borrow_global_field_warn(addr: address): u64 acquires MyResource {
        borrow_global<MyResource>(addr).value
    }

    // Warn: borrow_global_mut(...).field read can be MyResource[addr].field
    public fun test_borrow_global_mut_field_read_warn(addr: address): u64 acquires MyResource {
        borrow_global_mut<MyResource>(addr).value
    }

    // Warn: borrow_global_mut(...).field = x can be MyResource[addr].field = x
    public fun test_borrow_global_mut_field_assign_warn(addr: address, x: u64) acquires MyResource {
        borrow_global_mut<MyResource>(addr).value = x;
    }

    // Warn: borrow_global passed directly as function argument
    public fun test_borrow_global_as_arg_warn(addr: address): u64 acquires MyResource {
        helper_takes_immref(borrow_global<MyResource>(addr))
    }

    // === Should NOT warn: already using global storage index syntax ===

    // No warn: immutable index syntax
    public fun test_global_index_no_warn(addr: address): u64 acquires MyResource {
        let _r = &MyResource[addr];
        _r.value
    }

    // No warn: mutable index syntax
    public fun test_global_index_mut_no_warn(addr: address): u64 acquires MyResource {
        let _r = &mut MyResource[addr];
        _r.value
    }

    // No warn: deref index syntax
    public fun test_global_deref_index_no_warn(addr: address): MyResource acquires MyResource {
        MyResource[addr]
    }

    // No warn: index with field access
    public fun test_global_index_field_no_warn(addr: address): u64 acquires MyResource {
        MyResource[addr].value
    }

    // No warn: mutable index syntax used where immutable ref expected (implicit freeze)
    fun helper_takes_immref(r: &MyResource): u64 { r.value }

    #[lint::skip(needless_mutable_reference)]
    public fun test_global_index_mut_freeze_no_warn(addr: address): u64 acquires MyResource {
        helper_takes_immref(&mut MyResource[addr])
    }

    // No warn: mutable index syntax assigned to immutable ref binding
    #[lint::skip(needless_mutable_reference)]
    public fun test_global_index_mut_to_immref_no_warn(addr: address): u64 acquires MyResource {
        let _r: &MyResource = &mut MyResource[addr];
        _r.value
    }

    // === Should NOT warn: not borrow_global ===

    // No warn: exists is not borrow_global
    public fun test_exists_no_warn(addr: address): bool {
        exists<MyResource>(addr)
    }

    // No warn: move_from is not borrow_global
    public fun test_move_from_no_warn(addr: address): MyResource acquires MyResource {
        move_from<MyResource>(addr)
    }

    // === Lint skip ===

    // No warn: lint skip for vector
    #[lint::skip(use_index_syntax)]
    public fun test_skip_no_warn(v: &vector<u64>, i: u64): &u64 {
        vector::borrow(v, i)
    }

    // No warn: lint skip for global
    #[lint::skip(use_index_syntax)]
    public fun test_skip_global_no_warn(addr: address): u64 acquires MyResource {
        let _r = borrow_global<MyResource>(addr);
        _r.value
    }
}

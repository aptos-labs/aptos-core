// Test spec inference for mutations via references
module 0x42::mutations {

    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    struct Pair has copy, drop {
        first: Point,
        second: Point,
    }

    // ==================== Reference Parameters ====================

    // Simple write through reference parameter
    fun write_to_ref(r: &mut u64, val: u64) {
        *r = val;
    }

    // Increment reference parameter
    fun inc_ref(r: &mut u64) {
        *r = *r + 1;
    }

    // Write to struct field via reference parameter
    fun write_to_point_x(p: &mut Point, val: u64) {
        p.x = val;
    }

    // Increment struct field via reference parameter
    fun inc_point_x(p: &mut Point) {
        p.x = p.x + 1;
    }

    // Write to both fields via reference parameter
    fun write_to_point_both(p: &mut Point, x: u64, y: u64) {
        p.x = x;
        p.y = y;
    }

    // Nested field write via reference parameter
    fun write_to_nested(pair: &mut Pair, val: u64) {
        pair.first.x = val;
    }

    // ==================== Local Reference Creation ====================

    // Create reference to local and write
    fun create_ref_and_write(): u64 {
        let x = 0;
        let r = &mut x;
        *r = 42;
        x
    }

    // Double increment through local reference (should give result == 2)
    fun local_double_increment(): u64 {
        let x = 0;
        let r = &mut x;
        *r = *r + 1;
        *r = *r + 1;
        x
    }

    // Create reference to struct field and write
    fun create_field_ref_and_write(p: Point): Point {
        let result = p;
        let r = &mut result.x;
        *r = 10;
        result
    }

    // Chain of field references
    fun chain_field_refs(pair: Pair): Pair {
        let result = pair;
        let r_pair = &mut result;
        let r_point = &mut r_pair.first;
        let r_field = &mut r_point.x;
        *r_field = 99;
        result
    }

    // ==================== Multiple Updates ====================

    // Multiple writes through same reference
    fun multiple_writes(r: &mut u64) {
        *r = 1;
        *r = 2;
        *r = 3;
    }

    // Increment through reference
    fun increment_ref(r: &mut u64) {
        *r = *r + 1;
    }

    // Multiple increments
    fun double_increment(r: &mut u64) {
        *r = *r + 1;
        *r = *r + 1;
    }

    // Multiple increments on struct field
    fun double_increment_field(p: &mut Point) {
        p.x = p.x + 1;
        p.x = p.x + 1;
    }

    // Update multiple fields sequentially
    fun update_fields_seq(p: &mut Point) {
        let rx = &mut p.x;
        *rx = 1;
        let ry = &mut p.y;
        *ry = 2;
    }

    // ==================== Conditional Reference Creation ====================

    // Conditional: reference to different locals
    fun cond_ref_to_locals(cond: bool, val: u64): (u64, u64) {
        let a = 0;
        let b = 0;
        let r = if (cond) { &mut a } else { &mut b };
        *r = val;
        (a, b)
    }

    // Conditional: reference to different fields
    fun cond_ref_to_fields(cond: bool, p: Point, val: u64): Point {
        let result = p;
        let r = if (cond) { &mut result.x } else { &mut result.y };
        *r = val;
        result
    }

    // Conditional: reference to nested vs non-nested
    fun cond_ref_nested(cond: bool, pair: Pair, val: u64): Pair {
        let result = pair;
        let r = if (cond) { &mut result.first.x } else { &mut result.second.x };
        *r = val;
        result
    }

    // ==================== Reference from Parameter vs Local ====ou

    // Use parameter reference or create local reference based on condition
    fun cond_param_or_local(cond: bool, p_ref: &mut Point, val: u64): Point {
        let local = Point { x: 0, y: 0 };
        let r = if (cond) { p_ref } else { &mut local };
        r.x = val;
        local
    }

    // ==================== Unwritten &mut Parameters ====================

    // Noop function - &mut param is not modified at all
    fun noop_ref(_r: &mut u64) {
        // Does nothing - r should have ensures r == old(r)
    }

    // Conditional write - only one path modifies the param
    fun cond_write_ref(c: bool, r: &mut u64) {
        if (c) {
            *r = 1;
        }
        // When c is true: r == 1
        // When c is false: r == old(r)
    }

    // Conditional write to struct field
    fun cond_write_field(c: bool, p: &mut Point) {
        if (c) {
            p.x = 42;
        }
        // When c is true: p.x == 42
        // When c is false: p == old(p)
    }

    // ==================== Complex Borrow Graphs ====================

    // Multiple references into same struct
    fun multi_ref_same_struct(p: &mut Point, val_x: u64, val_y: u64) {
        let rx = &mut p.x;
        *rx = val_x;
        let ry = &mut p.y;
        *ry = val_y;
    }

    // Reference passed to another function
    fun pass_ref_to_fn(p: &mut Point, val: u64) {
        let rx = &mut p.x;
        write_to_ref(rx, val);
    }

    // Create reference, pass to function, continue using struct
    fun create_pass_continue(p: Point, val: u64): Point {
        let result = p;
        let r = &mut result.x;
        write_to_ref(r, val);
        result
    }

    // ==================== Return Value + Mutation ====================

    // Replace value through reference, returning old value
    fun replace_ref(r: &mut u64, new_val: u64): u64 {
        let old_val = *r;
        *r = new_val;
        old_val
    }

    // Caller that uses replace_ref
    fun call_replace(r: &mut u64): u64 {
        replace_ref(r, 99)
    }

    // ==================== Swap ====================

    // Swap two values through references
    fun swap_refs(a: &mut u64, b: &mut u64) {
        let tmp = *a;
        *a = *b;
        *b = tmp;
    }

    // ==================== Variant Field References ====================

    enum Shape has copy, drop {
        Circle { radius: u64 },
        Rect { w: u64, h: u64 },
    }

    // Write to a variant field — BorrowVariantField + WriteRef + WriteBack
    fun set_circle_radius(s: &mut Shape, new_r: u64) {
        s.radius = new_r;
    }

    // Read and write variant field
    fun inc_circle_radius(s: &mut Shape): u64 {
        let old_r = s.radius;
        s.radius = old_r + 1;
        old_r
    }

    // ==================== Multi-Variant Field Access ====================

    enum Token has copy, drop {
        Fungible { value: u64 },
        SemiFungible { value: u64, id: u64 },
        NonFungible { id: u64 },
    }

    // Shared field across Fungible and SemiFungible — should abort if NonFungible
    fun set_token_value(t: &mut Token, v: u64) {
        t.value = v;
    }

    // ==================== Global Mutations ====================

    struct Counter has key {
        value: u64,
    }

    // Increment global resource field
    fun increment_global(addr: address) acquires Counter {
        let c = &mut Counter[addr];
        c.value = c.value + 1;
    }

    // Set global resource field to a specific value
    fun set_global_value(addr: address, v: u64) acquires Counter {
        Counter[addr].value = v;
    }

    // Address variable reassigned after mutable resource indexing but before write_back.
    // The backward WP correctly resolves the original address because BorrowGlobal
    // is processed last and introduces the address into the spec at that point.
    fun addr_reassigned_after_borrow(addr: address, new_addr: address, v: u64): address acquires Counter {
        let c = &mut Counter[addr];
        addr = new_addr;
        c.value = v;
        addr
    }

    // Two sequential borrows of same global, different temps — should chain
    fun double_global_write(addr: address, v1: u64, v2: u64) acquires Counter {
        let c1 = &mut Counter[addr];
        c1.value = v1;
        let c2 = &mut Counter[addr];
        c2.value = v2;
    }

    // Two sequential borrows of same global, each incrementing — should chain
    fun double_increment_global(addr: address) acquires Counter {
        let c1 = &mut Counter[addr];
        c1.value = c1.value + 1;
        let c2 = &mut Counter[addr];
        c2.value = c2.value + 1;
    }

    // Two borrows of same type but DIFFERENT addresses — should NOT chain
    fun different_addr_global(a1: address, a2: address, v1: u64, v2: u64) acquires Counter {
        Counter[a1].value = v1;
        Counter[a2].value = v2;
    }

    // ==================== Conditional Address Aliasing ====================

    // Borrow global at address chosen by condition
    // Should infer path-conditional ensures on both a1 and a2
    fun cond_addr_global_write(cond: bool, a1: address, a2: address, v: u64) acquires Counter {
        let addr = if (cond) { a1 } else { a2 };
        let c = &mut Counter[addr];
        c.value = v;
    }

    // Conditional address with increment
    fun cond_addr_global_inc(cond: bool, a1: address, a2: address) acquires Counter {
        let addr = if (cond) { a1 } else { a2 };
        let c = &mut Counter[addr];
        c.value = c.value + 1;
    }

    // Two sequential borrows at conditionally chosen addresses
    fun cond_addr_sequential(cond: bool, a1: address, a2: address, v1: u64, v2: u64) acquires Counter {
        let addr1 = if (cond) { a1 } else { a2 };
        let c1 = &mut Counter[addr1];
        c1.value = v1;
        let addr2 = if (cond) { a2 } else { a1 };
        let c2 = &mut Counter[addr2];
        c2.value = v2;
    }

    // Two sequential increments at conditionally chosen addresses
    fun cond_addr_increment(cond: bool, a1: address, a2: address, v1: u64, v2: u64) acquires Counter {
        let addr1 = if (cond) { a1 } else { a2 };
        let c1 = &mut Counter[addr1];
        c1.value = c1.value + v1;
        let addr2 = if (cond) { a2 } else { a1 };
        let c2 = &mut Counter[addr2];
        c2.value = c2.value + v2;
    }

    // ==================== N>=3 Chained Frames ====================

    // N=3: three different addresses writing to same resource type
    fun triple_addr_global(a1: address, a2: address, a3: address,
                           v1: u64, v2: u64, v3: u64) acquires Counter {
        Counter[a1].value = v1;
        Counter[a2].value = v2;
        Counter[a3].value = v3;
    }

    // ==================== Mixed Mutation + Function Call ====================

    // Mutation in program order before function call
    fun mutation_then_call(a1: address, a2: address, v: u64) acquires Counter {
        Counter[a1].value = v;
        increment_global(a2);
    }

    // Function call in program order before mutation
    fun call_then_mutation(a1: address, a2: address, v: u64) acquires Counter {
        increment_global(a2);
        Counter[a1].value = v;
    }

    // MoveFrom + borrow_global_mut on different address
    fun remove_and_modify(a1: address, a2: address, v: u64): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(a1);
        Counter[a2].value = v;
        value
    }
}

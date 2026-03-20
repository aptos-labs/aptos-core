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
    spec write_to_ref(r: &mut u64, val: u64) {
        ensures [inferred] r == val;
    }


    // Increment reference parameter
    fun inc_ref(r: &mut u64) {
        *r = *r + 1;
    }
    spec inc_ref(r: &mut u64) {
        ensures [inferred] r == old(r) + 1;
        aborts_if [inferred] r > MAX_U64 - 1;
    }


    // Write to struct field via reference parameter
    fun write_to_point_x(p: &mut Point, val: u64) {
        p.x = val;
    }
    spec write_to_point_x(p: &mut Point, val: u64) {
        ensures [inferred] p == update_field(old(p), x, val);
    }


    // Increment struct field via reference parameter
    fun inc_point_x(p: &mut Point) {
        p.x = p.x + 1;
    }
    spec inc_point_x(p: &mut Point) {
        ensures [inferred] p == update_field(old(p), x, old(p).x + 1);
        aborts_if [inferred] p.x > MAX_U64 - 1;
    }


    // Write to both fields via reference parameter
    fun write_to_point_both(p: &mut Point, x: u64, y: u64) {
        p.x = x;
        p.y = y;
    }
    spec write_to_point_both(p: &mut Point, x: u64, y: u64) {
        ensures [inferred] p == update_field(update_field(old(p), x, x), y, y);
    }


    // Nested field write via reference parameter
    fun write_to_nested(pair: &mut Pair, val: u64) {
        pair.first.x = val;
    }
    spec write_to_nested(pair: &mut Pair, val: u64) {
        ensures [inferred] pair == update_field(old(pair), first, update_field(old(pair).first, x, val));
    }


    // ==================== Local Reference Creation ====================

    // Create reference to local and write
    fun create_ref_and_write(): u64 {
        let x = 0;
        let r = &mut x;
        *r = 42;
        x
    }
    spec create_ref_and_write(): u64 {
        ensures [inferred] result == 42;
    }


    // Double increment through local reference (should give result == 2)
    fun local_double_increment(): u64 {
        let x = 0;
        let r = &mut x;
        *r = *r + 1;
        *r = *r + 1;
        x
    }
    spec local_double_increment(): u64 {
        ensures [inferred] result == 2;
    }


    // Create reference to struct field and write
    fun create_field_ref_and_write(p: Point): Point {
        let result = p;
        let r = &mut result.x;
        *r = 10;
        result
    }
    spec create_field_ref_and_write(p: Point): Point {
        ensures [inferred] result == update_field(p, x, 10);
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
    spec chain_field_refs(pair: Pair): Pair {
        ensures [inferred] result == update_field(pair, first, update_field(pair.first, x, 99));
    }


    // ==================== Multiple Updates ====================

    // Multiple writes through same reference
    fun multiple_writes(r: &mut u64) {
        *r = 1;
        *r = 2;
        *r = 3;
    }
    spec multiple_writes(r: &mut u64) {
        ensures [inferred] r == 3;
    }


    // Increment through reference
    fun increment_ref(r: &mut u64) {
        *r = *r + 1;
    }
    spec increment_ref(r: &mut u64) {
        ensures [inferred] r == old(r) + 1;
        aborts_if [inferred] r > MAX_U64 - 1;
    }


    // Multiple increments
    fun double_increment(r: &mut u64) {
        *r = *r + 1;
        *r = *r + 1;
    }
    spec double_increment(r: &mut u64) {
        ensures [inferred] r == old(r) + 2;
        aborts_if [inferred] r > MAX_U64 - 2;
    }


    // Multiple increments on struct field
    fun double_increment_field(p: &mut Point) {
        p.x = p.x + 1;
        p.x = p.x + 1;
    }
    spec double_increment_field(p: &mut Point) {
        ensures [inferred] p == update_field(old(p), x, old(p).x + 2);
        aborts_if [inferred] p.x > MAX_U64 - 2;
    }


    // Update multiple fields sequentially
    fun update_fields_seq(p: &mut Point) {
        let rx = &mut p.x;
        *rx = 1;
        let ry = &mut p.y;
        *ry = 2;
    }
    spec update_fields_seq(p: &mut Point) {
        ensures [inferred] p == update_field(update_field(old(p), x, 1), y, 2);
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
    spec cond_ref_to_locals(cond: bool, val: u64): (u64, u64) {
        ensures [inferred] !cond ==> result_2 == val;
        ensures [inferred] cond ==> result_2 == 0;
        ensures [inferred] cond ==> result_1 == val;
        ensures [inferred] !cond ==> result_1 == 0;
    }


    // Conditional: reference to different fields
    fun cond_ref_to_fields(cond: bool, p: Point, val: u64): Point {
        let result = p;
        let r = if (cond) { &mut result.x } else { &mut result.y };
        *r = val;
        result
    }
    spec cond_ref_to_fields(cond: bool, p: Point, val: u64): Point {
        ensures [inferred] cond ==> result == update_field(p, x, val);
        ensures [inferred] !cond ==> result == update_field(p, y, val);
    }


    // Conditional: reference to nested vs non-nested
    fun cond_ref_nested(cond: bool, pair: Pair, val: u64): Pair {
        let result = pair;
        let r = if (cond) { &mut result.first.x } else { &mut result.second.x };
        *r = val;
        result
    }
    spec cond_ref_nested(cond: bool, pair: Pair, val: u64): Pair {
        ensures [inferred] cond ==> result == update_field(pair, first, update_field(pair.first, x, val));
        ensures [inferred] !cond ==> result == update_field(pair, second, update_field(pair.second, x, val));
    }


    // ==================== Reference from Parameter vs Local ====ou

    // Use parameter reference or create local reference based on condition
    fun cond_param_or_local(cond: bool, p_ref: &mut Point, val: u64): Point {
        let local = Point { x: 0, y: 0 };
        let r = if (cond) { p_ref } else { &mut local };
        r.x = val;
        local
    }
    spec cond_param_or_local(cond: bool, p_ref: &mut Point, val: u64): Point {
        ensures [inferred] cond ==> result == Point{x: 0, y: 0};
        ensures [inferred] !cond ==> p_ref == old(p_ref);
        ensures [inferred] cond ==> p_ref == update_field(old(p_ref), x, val);
        ensures [inferred] !cond ==> result == Point{x: val, y: 0};
    }


    // ==================== Unwritten &mut Parameters ====================

    // Noop function - &mut param is not modified at all
    fun noop_ref(_r: &mut u64) {
        // Does nothing - r should have ensures r == old(r)
    }
    spec noop_ref(_r: &mut u64) {
        ensures [inferred] _r == old(_r);
    }


    // Conditional write - only one path modifies the param
    fun cond_write_ref(c: bool, r: &mut u64) {
        if (c) {
            *r = 1;
        }
        // When c is true: r == 1
        // When c is false: r == old(r)
    }
    spec cond_write_ref(c: bool, r: &mut u64) {
        ensures [inferred] c ==> r == 1;
        ensures [inferred] !c ==> r == old(r);
    }


    // Conditional write to struct field
    fun cond_write_field(c: bool, p: &mut Point) {
        if (c) {
            p.x = 42;
        }
        // When c is true: p.x == 42
        // When c is false: p == old(p)
    }
    spec cond_write_field(c: bool, p: &mut Point) {
        ensures [inferred] c ==> p == update_field(old(p), x, 42);
        ensures [inferred] !c ==> p == old(p);
    }


    // ==================== Complex Borrow Graphs ====================

    // Multiple references into same struct
    fun multi_ref_same_struct(p: &mut Point, val_x: u64, val_y: u64) {
        let rx = &mut p.x;
        *rx = val_x;
        let ry = &mut p.y;
        *ry = val_y;
    }
    spec multi_ref_same_struct(p: &mut Point, val_x: u64, val_y: u64) {
        ensures [inferred] p == update_field(update_field(old(p), x, val_x), y, val_y);
    }


    // Reference passed to another function
    fun pass_ref_to_fn(p: &mut Point, val: u64) {
        let rx = &mut p.x;
        write_to_ref(rx, val);
    }
    spec pass_ref_to_fn(p: &mut Point, val: u64) {
        ensures [inferred] p == update_field(old(p), x, result_of<write_to_ref>(old(p).x, val));
        aborts_if [inferred] aborts_of<write_to_ref>(p.x, val);
    }


    // Create reference, pass to function, continue using struct
    fun create_pass_continue(p: Point, val: u64): Point {
        let result = p;
        let r = &mut result.x;
        write_to_ref(r, val);
        result
    }
    spec create_pass_continue(p: Point, val: u64): Point {
        ensures [inferred] result == update_field(p, x, result_of<write_to_ref>(p.x, val));
        aborts_if [inferred] aborts_of<write_to_ref>(p.x, val);
    }


    // ==================== Return Value + Mutation ====================

    // Replace value through reference, returning old value
    fun replace_ref(r: &mut u64, new_val: u64): u64 {
        let old_val = *r;
        *r = new_val;
        old_val
    }
    spec replace_ref(r: &mut u64, new_val: u64): u64 {
        ensures [inferred] result == old(r);
        ensures [inferred] r == new_val;
    }


    // Caller that uses replace_ref
    fun call_replace(r: &mut u64): u64 {
        replace_ref(r, 99)
    }
    spec call_replace(r: &mut u64): u64 {
        ensures [inferred] result == {
            let (_t0,_t1) = result_of<replace_ref>(r, 99);
            _t0
        };
        ensures [inferred] r == {
            let (_t0,_t1) = result_of<replace_ref>(r, 99);
            _t1
        };
        aborts_if [inferred] aborts_of<replace_ref>(r, 99);
    }


    // ==================== Swap ====================

    // Swap two values through references
    fun swap_refs(a: &mut u64, b: &mut u64) {
        let tmp = *a;
        *a = *b;
        *b = tmp;
    }
    spec swap_refs(a: &mut u64, b: &mut u64) {
        ensures [inferred] b == old(a);
        ensures [inferred] a == old(b);
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
    spec set_circle_radius(s: &mut Shape, new_r: u64) {
        ensures [inferred] s == update_field(old(s), radius, new_r);
        aborts_if [inferred] !(s is Circle);
    }


    // Read and write variant field
    fun inc_circle_radius(s: &mut Shape): u64 {
        let old_r = s.radius;
        s.radius = old_r + 1;
        old_r
    }
    spec inc_circle_radius(s: &mut Shape): u64 {
        ensures [inferred] result == old(s).radius;
        ensures [inferred] s == update_field(old(s), radius, old(s).radius + 1);
        aborts_if [inferred] !(s is Circle);
        aborts_if [inferred] s.radius > MAX_U64 - 1;
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
    spec set_token_value(t: &mut Token, v: u64) {
        ensures [inferred] t == update_field(old(t), value, v);
        aborts_if [inferred] !(t is Fungible | SemiFungible);
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
    spec increment_global(addr: address) {
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 1);
        aborts_if [inferred] global<Counter>(addr).value > MAX_U64 - 1;
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // Set global resource field to a specific value
    fun set_global_value(addr: address, v: u64) acquires Counter {
        Counter[addr].value = v;
    }
    spec set_global_value(addr: address, v: u64) {
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, v);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
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
    spec addr_reassigned_after_borrow(addr: address, new_addr: address, v: u64): address {
        ensures [inferred] result == new_addr;
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, v);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // Two sequential borrows of same global, different temps — should chain
    fun double_global_write(addr: address, v1: u64, v2: u64) acquires Counter {
        let c1 = &mut Counter[addr];
        c1.value = v1;
        let c2 = &mut Counter[addr];
        c2.value = v2;
    }
    spec double_global_write(addr: address, v1: u64, v2: u64) {
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, v2);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // Two sequential borrows of same global, each incrementing — should chain
    fun double_increment_global(addr: address) acquires Counter {
        let c1 = &mut Counter[addr];
        c1.value = c1.value + 1;
        let c2 = &mut Counter[addr];
        c2.value = c2.value + 1;
    }
    spec double_increment_global(addr: address) {
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, old(global<Counter>(addr)).value + 2);
        aborts_if [inferred] global<Counter>(addr).value > MAX_U64 - 2;
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // Two borrows of same type but DIFFERENT addresses — should NOT chain
    fun different_addr_global(a1: address, a2: address, v1: u64, v2: u64) acquires Counter {
        Counter[a1].value = v1;
        Counter[a2].value = v2;
    }
    spec different_addr_global(a1: address, a2: address, v1: u64, v2: u64) {
        ensures [inferred] global<Counter>(a2) == update_field(at_14@global<Counter>(a2), value, v2);
        ensures [inferred = sathard] forall x: address: x != a1 ==> at_14@global<Counter>(x) == old(global<Counter>(x));
        ensures [inferred] at_14@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
        aborts_if [inferred] !exists<Counter>(a2);
        aborts_if [inferred] !at_14@exists<Counter>(a1);
        modifies [inferred] global<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
    }


    // ==================== Conditional Address Aliasing ====================

    // Borrow global at address chosen by condition
    // Should infer path-conditional ensures on both a1 and a2
    fun cond_addr_global_write(cond: bool, a1: address, a2: address, v: u64) acquires Counter {
        let addr = if (cond) { a1 } else { a2 };
        let c = &mut Counter[addr];
        c.value = v;
    }
    spec cond_addr_global_write(cond: bool, a1: address, a2: address, v: u64) {
        ensures [inferred] cond ==> global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v);
        ensures [inferred] !cond ==> global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, v);
        aborts_if [inferred] cond && !exists<Counter>(a1);
        aborts_if [inferred] !cond && !exists<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
        modifies [inferred] global<Counter>(a2);
    }


    // Conditional address with increment
    fun cond_addr_global_inc(cond: bool, a1: address, a2: address) acquires Counter {
        let addr = if (cond) { a1 } else { a2 };
        let c = &mut Counter[addr];
        c.value = c.value + 1;
    }
    spec cond_addr_global_inc(cond: bool, a1: address, a2: address) {
        ensures [inferred] cond ==> global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, old(global<Counter>(a1)).value + 1);
        ensures [inferred] !cond ==> global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, old(global<Counter>(a2)).value + 1);
        aborts_if [inferred] cond && global<Counter>(a1).value > MAX_U64 - 1;
        aborts_if [inferred] cond && !exists<Counter>(a1);
        aborts_if [inferred] !cond && global<Counter>(a2).value > MAX_U64 - 1;
        aborts_if [inferred] !cond && !exists<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
        modifies [inferred] global<Counter>(a2);
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
    spec cond_addr_sequential(cond: bool, a1: address, a2: address, v1: u64, v2: u64) {
        ensures [inferred] cond ==> global<Counter>(a2) == update_field(at_21@global<Counter>(a2), value, v2);
        ensures [inferred] !cond ==> global<Counter>(a1) == update_field(at_21@global<Counter>(a1), value, v2);
        ensures [inferred] cond ==> (forall x: address: x != a1 ==> at_21@global<Counter>(x) == old(global<Counter>(x)));
        ensures [inferred] cond ==> at_21@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
        ensures [inferred] !cond ==> (forall x: address: x != a2 ==> at_21@global<Counter>(x) == old(global<Counter>(x)));
        ensures [inferred] !cond ==> at_21@global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, v1);
        aborts_if [inferred] cond && !exists<Counter>(a2);
        aborts_if [inferred] !cond && !exists<Counter>(a1);
        aborts_if [inferred] cond && !at_21@exists<Counter>(a1);
        aborts_if [inferred] !cond && !at_21@exists<Counter>(a2);
        modifies [inferred] global<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
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
    spec cond_addr_increment(cond: bool, a1: address, a2: address, v1: u64, v2: u64) {
        ensures [inferred] cond ==> global<Counter>(a2) == update_field(at_23@global<Counter>(a2), value, at_23@global<Counter>(a2).value + v2);
        ensures [inferred] !cond ==> global<Counter>(a1) == update_field(at_23@global<Counter>(a1), value, at_23@global<Counter>(a1).value + v2);
        ensures [inferred] cond ==> (forall x: address: x != a1 ==> at_23@global<Counter>(x) == old(global<Counter>(x)));
        ensures [inferred] cond ==> at_23@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, old(global<Counter>(a1)).value + v1);
        ensures [inferred] !cond ==> (forall x: address: x != a2 ==> at_23@global<Counter>(x) == old(global<Counter>(x)));
        ensures [inferred] !cond ==> at_23@global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, old(global<Counter>(a2)).value + v1);
        aborts_if [inferred] cond && at_23@global<Counter>(a2).value + v2 > MAX_U64;
        aborts_if [inferred] cond && !exists<Counter>(a2);
        aborts_if [inferred] !cond && at_23@global<Counter>(a1).value + v2 > MAX_U64;
        aborts_if [inferred] !cond && !exists<Counter>(a1);
        aborts_if [inferred] cond && global<Counter>(a1).value + v1 > MAX_U64;
        aborts_if [inferred] cond && !at_23@exists<Counter>(a1);
        aborts_if [inferred] !cond && global<Counter>(a2).value + v1 > MAX_U64;
        aborts_if [inferred] !cond && !at_23@exists<Counter>(a2);
        modifies [inferred] global<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
    }


    // ==================== N>=3 Chained Frames ====================

    // N=3: three different addresses writing to same resource type
    fun triple_addr_global(a1: address, a2: address, a3: address,
                           v1: u64, v2: u64, v3: u64) acquires Counter {
        Counter[a1].value = v1;
        Counter[a2].value = v2;
        Counter[a3].value = v3;
    }
    spec triple_addr_global(a1: address, a2: address, a3: address, v1: u64, v2: u64, v3: u64) {
        ensures [inferred] global<Counter>(a3) == update_field(at_24@global<Counter>(a3), value, v3);
        ensures [inferred = sathard] forall x: address: x != a2 ==> at_24@global<Counter>(x) == at_18@global<Counter>(x);
        ensures [inferred] at_24@global<Counter>(a2) == update_field(at_18@global<Counter>(a2), value, v2);
        ensures [inferred = sathard] forall x: address: x != a1 ==> at_18@global<Counter>(x) == old(global<Counter>(x));
        ensures [inferred] at_18@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
        aborts_if [inferred] !exists<Counter>(a3);
        aborts_if [inferred] !at_24@exists<Counter>(a2);
        aborts_if [inferred] !at_18@exists<Counter>(a1);
        modifies [inferred] global<Counter>(a3);
        modifies [inferred] global<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
    }


    // ==================== Mixed Mutation + Function Call ====================

    // Mutation in program order before function call
    fun mutation_then_call(a1: address, a2: address, v: u64) acquires Counter {
        Counter[a1].value = v;
        increment_global(a2);
    }
    spec mutation_then_call(a1: address, a2: address, v: u64) {
        ensures [inferred] ensures_of<increment_global>(a2);
        ensures [inferred] at_13@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v);
        aborts_if [inferred] aborts_of<increment_global>(a2);
        aborts_if [inferred] !at_13@exists<Counter>(a1);
        modifies [inferred] global<Counter>(a1);
    }


    // Function call in program order before mutation
    fun call_then_mutation(a1: address, a2: address, v: u64) acquires Counter {
        increment_global(a2);
        Counter[a1].value = v;
    }
    spec call_then_mutation(a1: address, a2: address, v: u64) {
        ensures [inferred] global<Counter>(a1) == update_field(post_call_7@global<Counter>(a1), value, v);
        ensures [inferred] ensures_of<increment_global>(a2)@post_call_7;
        aborts_if [inferred] !exists<Counter>(a1);
        aborts_if [inferred] aborts_of<increment_global>(a2);
        modifies [inferred] global<Counter>(a1);
    }


    // MoveFrom + borrow_global_mut on different address
    fun remove_and_modify(a1: address, a2: address, v: u64): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(a1);
        Counter[a2].value = v;
        value
    }
    spec remove_and_modify(a1: address, a2: address, v: u64): u64 {
        ensures [inferred] result == global<Counter>(a1).value;
        ensures [inferred] global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, v);
        ensures [inferred] !exists<Counter>(a1);
        aborts_if [inferred] !exists<Counter>(a2);
        aborts_if [inferred] !exists<Counter>(a1);
        modifies [inferred] global<Counter>(a2);
        modifies [inferred] global<Counter>(a1);
    }

}
/*
Verification: exiting with compilation errors
error: state label `at_14` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:497:64
    │
497 │         ensures [inferred] global<Counter>(a2) == update_field(at_14@global<Counter>(a2), value, v2);
    │                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_14` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:498:69
    │
498 │         ensures [inferred = sathard] forall x: address: x != a1 ==> at_14@global<Counter>(x) == old(global<Counter>(x));
    │                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_14` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:499:28
    │
499 │         ensures [inferred] at_14@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
    │                            ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_14` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:501:31
    │
501 │         aborts_if [inferred] !at_14@exists<Counter>(a1);
    │                               ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:554:73
    │
554 │         ensures [inferred] cond ==> global<Counter>(a2) == update_field(at_21@global<Counter>(a2), value, v2);
    │                                                                         ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:555:74
    │
555 │         ensures [inferred] !cond ==> global<Counter>(a1) == update_field(at_21@global<Counter>(a1), value, v2);
    │                                                                          ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:556:69
    │
556 │         ensures [inferred] cond ==> (forall x: address: x != a1 ==> at_21@global<Counter>(x) == old(global<Counter>(x)));
    │                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:557:37
    │
557 │         ensures [inferred] cond ==> at_21@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
    │                                     ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:558:70
    │
558 │         ensures [inferred] !cond ==> (forall x: address: x != a2 ==> at_21@global<Counter>(x) == old(global<Counter>(x)));
    │                                                                      ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:559:38
    │
559 │         ensures [inferred] !cond ==> at_21@global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, v1);
    │                                      ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:562:39
    │
562 │         aborts_if [inferred] cond && !at_21@exists<Counter>(a1);
    │                                       ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_21` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:563:40
    │
563 │         aborts_if [inferred] !cond && !at_21@exists<Counter>(a2);
    │                                        ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:579:73
    │
579 │         ensures [inferred] cond ==> global<Counter>(a2) == update_field(at_23@global<Counter>(a2), value, at_23@global<Counter>(a2).value + v2);
    │                                                                         ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:579:107
    │
579 │         ensures [inferred] cond ==> global<Counter>(a2) == update_field(at_23@global<Counter>(a2), value, at_23@global<Counter>(a2).value + v2);
    │                                                                                                           ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:580:74
    │
580 │         ensures [inferred] !cond ==> global<Counter>(a1) == update_field(at_23@global<Counter>(a1), value, at_23@global<Counter>(a1).value + v2);
    │                                                                          ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:580:108
    │
580 │         ensures [inferred] !cond ==> global<Counter>(a1) == update_field(at_23@global<Counter>(a1), value, at_23@global<Counter>(a1).value + v2);
    │                                                                                                            ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:581:69
    │
581 │         ensures [inferred] cond ==> (forall x: address: x != a1 ==> at_23@global<Counter>(x) == old(global<Counter>(x)));
    │                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:582:37
    │
582 │         ensures [inferred] cond ==> at_23@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, old(global<Counter>(a1)).value + v1);
    │                                     ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:583:70
    │
583 │         ensures [inferred] !cond ==> (forall x: address: x != a2 ==> at_23@global<Counter>(x) == old(global<Counter>(x)));
    │                                                                      ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:584:38
    │
584 │         ensures [inferred] !cond ==> at_23@global<Counter>(a2) == update_field(old(global<Counter>(a2)), value, old(global<Counter>(a2)).value + v1);
    │                                      ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:585:38
    │
585 │         aborts_if [inferred] cond && at_23@global<Counter>(a2).value + v2 > MAX_U64;
    │                                      ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:587:39
    │
587 │         aborts_if [inferred] !cond && at_23@global<Counter>(a1).value + v2 > MAX_U64;
    │                                       ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:590:39
    │
590 │         aborts_if [inferred] cond && !at_23@exists<Counter>(a1);
    │                                       ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_23` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:592:40
    │
592 │         aborts_if [inferred] !cond && !at_23@exists<Counter>(a2);
    │                                        ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_24` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:608:64
    │
608 │         ensures [inferred] global<Counter>(a3) == update_field(at_24@global<Counter>(a3), value, v3);
    │                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_24` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:609:69
    │
609 │         ensures [inferred = sathard] forall x: address: x != a2 ==> at_24@global<Counter>(x) == at_18@global<Counter>(x);
    │                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_18` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:609:97
    │
609 │         ensures [inferred = sathard] forall x: address: x != a2 ==> at_24@global<Counter>(x) == at_18@global<Counter>(x);
    │                                                                                                 ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_24` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:610:28
    │
610 │         ensures [inferred] at_24@global<Counter>(a2) == update_field(at_18@global<Counter>(a2), value, v2);
    │                            ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_18` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:610:70
    │
610 │         ensures [inferred] at_24@global<Counter>(a2) == update_field(at_18@global<Counter>(a2), value, v2);
    │                                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_18` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:611:69
    │
611 │         ensures [inferred = sathard] forall x: address: x != a1 ==> at_18@global<Counter>(x) == old(global<Counter>(x));
    │                                                                     ^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_18` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:612:28
    │
612 │         ensures [inferred] at_18@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v1);
    │                            ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_24` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:614:31
    │
614 │         aborts_if [inferred] !at_24@exists<Counter>(a2);
    │                               ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_18` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:615:31
    │
615 │         aborts_if [inferred] !at_18@exists<Counter>(a1);
    │                               ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_13` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:631:28
    │
631 │         ensures [inferred] at_13@global<Counter>(a1) == update_field(old(global<Counter>(a1)), value, v);
    │                            ^^^^^^^^^^^^^^^^^^^^^^^^^

error: state label `at_13` is not defined; labels in memory accesses must reference a post-state label defined by a behavior predicate in the same spec
    ┌─ tests/inference/mutations.enriched.move:633:31
    │
633 │         aborts_if [inferred] !at_13@exists<Counter>(a1);
    │                               ^^^^^^^^^^^^^^^^^^^^^^^^^
*/

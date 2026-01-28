module 0x42::references {

    struct S has drop {
        x: u64,
        y: u64,
    }

    // Test simple mutable borrow and write-back
    fun simple_mut_ref(): u64 {
        let s = S { x: 1, y: 2 };
        let r = &mut s.x;
        *r = 10;
        s.x
    }

    // Test multiple borrows from same struct
    fun multiple_field_borrows(): u64 {
        let s = S { x: 1, y: 2 };
        let rx = &s.x;
        let ry = &s.y;
        *rx + *ry
    }

    // Test borrow in conditional
    fun conditional_borrow(cond: bool): u64 {
        let s = S { x: 1, y: 2 };
        let result;
        if (cond) {
            let r = &s.x;
            result = *r;
        } else {
            let r = &s.y;
            result = *r;
        };
        result
    }

    // Test reassigning a reference
    fun reassign_ref(cond: bool): u64 {
        let s = S { x: 1, y: 2 };
        let r = &s.x;
        if (cond) {
            r = &s.y;
        };
        *r
    }

    // Test nested struct borrow
    struct Outer has drop {
        inner: S,
    }

    fun nested_borrow(): u64 {
        let o = Outer { inner: S { x: 10, y: 20 } };
        let r = &o.inner.x;
        *r
    }

    // ========== Function call tests ==========

    // Helper: modifies value through mutable reference
    fun modify_ref(r: &mut u64) {
        *r = *r + 1;
    }

    // Helper: reads through immutable reference
    fun read_ref(r: &u64): u64 {
        *r
    }

    // Helper: modifies struct field
    fun modify_struct(s: &mut S) {
        s.x = s.x + 10;
    }

    // Test: call with mutable ref should conservatively mark struct as modified
    fun call_with_mut_ref(): u64 {
        let s = S { x: 1, y: 2 };
        let r = &mut s.x;
        modify_ref(r);  // After this call, s should have new reaching def
        s.x
    }

    // Test: call with immutable ref should NOT mark struct as modified
    fun call_with_immut_ref(): u64 {
        let s = S { x: 1, y: 2 };
        let r = &s.x;
        let val = read_ref(r);  // s should NOT get new reaching def
        val + s.y
    }

    // Test: call that modifies struct directly
    fun call_modify_struct(): u64 {
        let s = S { x: 1, y: 2 };
        modify_struct(&mut s);  // s should have new reaching def
        s.x
    }

    // Test: multiple calls in sequence
    fun multiple_calls(): u64 {
        let s = S { x: 1, y: 2 };
        let r = &mut s.x;
        modify_ref(r);      // s gets new def
        let r2 = &mut s.y;
        modify_ref(r2);     // s gets another new def
        s.x + s.y
    }

    // Test: call in conditional
    fun call_in_conditional(cond: bool): u64 {
        let s = S { x: 1, y: 2 };
        if (cond) {
            modify_struct(&mut s);  // s gets new def in this branch only
        };
        s.x  // s should have defs from both init and the call
    }
}

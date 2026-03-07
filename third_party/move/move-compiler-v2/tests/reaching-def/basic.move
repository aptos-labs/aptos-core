module 0x42::basic {

    // Test basic assignment reaching definitions
    fun simple_assign(a: u64, b: u64): u64 {
        let x = a + b;
        let y = x * 2;
        y
    }

    // Test with reassignment - x has two definitions
    fun reassign(cond: bool): u64 {
        let x = 1;
        if (cond) {
            x = 2;
        };
        x  // x should have reaching defs from both assignments
    }

    // Test move semantics
    struct R has drop { value: u64 }

    fun move_test(): u64 {
        let r = R { value: 42 };
        let r2 = r;  // r is moved here
        r2.value
    }

    // Test multiple return values - exercises iterating over multiple dests in Call
    fun return_pair(a: u64, b: u64): (u64, u64) {
        (a + 1, b + 1)
    }

    fun tuple_destructure(): u64 {
        let (x, y) = return_pair(1, 2);
        x + y  // both x and y have reaching defs from the call
    }

    // Test conditional with multiple return values
    fun conditional_tuple(cond: bool): u64 {
        let (a, b);
        if (cond) {
            (a, b) = return_pair(1, 2);
        } else {
            (a, b) = return_pair(10, 20);
        };
        a + b  // a and b each have defs from both branches
    }
}

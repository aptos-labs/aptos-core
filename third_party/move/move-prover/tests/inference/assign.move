// Test basic spec inference for assignments and simple returns
module 0x42::assign {

    // Identity function - should infer: ensures result == x
    fun identity(x: u64): u64 {
        x
    }

    // Function with existing spec - should NOT be modified
    fun with_spec(x: u64): u64 {
        x + 1
    }
    spec with_spec {
        ensures result == x + 1;
    }

    // Multiple parameters, returns first - should infer: ensures result == x
    fun first(x: u64, _y: u64): u64 {
        x
    }

    // Returns second parameter - should infer: ensures result == y
    fun second(_x: u64, y: u64): u64 {
        y
    }

    // Single assignment chain - should infer: ensures result == x
    fun single_assign(x: u64): u64 {
        let y = x;
        y
    }

    // Multiple assignment chain - should infer: ensures result == x
    fun chain_assign(x: u64): u64 {
        let y = x;
        let z = y;
        z
    }

    // Long chain - should infer: ensures result == x
    fun long_chain(x: u64): u64 {
        let a = x;
        let b = a;
        let c = b;
        let d = c;
        d
    }
}

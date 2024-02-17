module NamedAddr::Detector {
    // Function ends with an explicit return statement
    public fun with_explicit_return(x: u64): u64 {
        let y = x + 1;
        return y
    }

    // Function ends with an expression other than a return statement
    public fun without_return(x: u64): u64 {
        x + 1
    }

    // Function with no return statement (implicitly returning `()`)
    public fun no_return() {
        let _ = 42;
    }

    // Function with a return statement, but not at the end
    public fun return_not_at_end(x: u64): u64 {
        if (x > 10) {
            return x;
        };
        x + 2
    }

    // Function with nested blocks
    public fun nested_blocks(x: u64): u64 {
        if (x < 5) {
            if (x == 4) {
                return 3;
            };
            2
        } else {
            1
        }
    }
}

module 0xc0ffee::m {
    // Basic if-else without semicolon
    public fun test1(a: bool) {
        let x;
        if (a) {
            x = 1;
        } else {
            x = 2;
        } // missing semicolon, but should compile
        assert!(x > 0, 1);
    }

    // Nested if-else without semicolons
    public fun test2(a: bool, b: bool) {
        let x;
        if (a) {
            if (b) {
                x = 1;
            } else {
                x = 2;
            }
            x = x + 1;
        } else {
            x = 3;
        } // no semicolon after outer if-else
        assert!(x > 0, 1);
    }

    // If with no else, ending with block - no semicolon needed
    public fun test3(a: bool) {
        let x = 0;
        if (a) {
            x = 1;
        } // no semicolon, no else branch
        assert!(x >= 0, 1);
    }

    // While loop without semicolon
    public fun test4() {
        let x = 0;
        while (x < 5) {
            x = x + 1;
        } // no semicolon after while
        assert!(x == 5, 1);
    }

    // Loop with break, without semicolon
    public fun test5() {
        let x = 0;
        loop {
            x = x + 1;
            if (x >= 5) break;
        } // no semicolon after loop
        assert!(x == 5, 1);
    }

    // Multiple consecutive block expressions without semicolons
    public fun test6(a: bool, b: bool) {
        let x = 0;
        if (a) {
            x = 1;
        } // no semicolon
        if (b) {
            x = x + 1;
        } // no semicolon
        assert!(x >= 0, 1);
    }

    // Mixed: some with semicolons, some without
    public fun test7(a: bool) {
        let x = 0;
        if (a) {
            x = 1;
        }; // optional semicolon provided
        if (!a) {
            x = 2;
        } // no semicolon
        assert!(x > 0, 1);
    }

    // Block expression without semicolon
    public fun test8(a: bool) {
        let x;
        {
            if (a) {
                x = 1;
            } else {
                x = 2;
            }
        } // no semicolon after block
        assert!(x > 0, 1);
    }

    // Nested blocks without semicolons
    public fun test9(a: bool) {
        let x;
        {
            {
                if (a) {
                    x = 1;
                } else {
                    x = 2;
                }
            }
        } // no semicolons after nested blocks
        assert!(x > 0, 1);
    }

    // If-else chain without semicolons
    public fun test10(a: u64) {
        let x;
        if (a == 0) {
            x = 0;
        } else if (a == 1) {
            x = 1;
        } else if (a == 2) {
            x = 2;
        } else {
            x = 3;
        } // no semicolon after if-else chain
        assert!(x <= 3, 1);
    }

    // While loop nested in if-else without semicolons
    public fun test11(a: bool) {
        let x = 0;
        if (a) {
            while (x < 3) {
                x = x + 1;
            } // no semicolon after while
        } else {
            x = 10;
        } // no semicolon after if-else
        assert!(x > 0, 1);
    }

    // Loop nested in if-else without semicolons
    public fun test12(a: bool) {
        let x = 0;
        if (a) {
            loop {
                x = x + 1;
                if (x >= 3) break;
            } // no semicolon after loop
        } else {
            x = 10;
        } // no semicolon after if-else
        assert!(x > 0, 1);
    }

    // Complex nesting without semicolons
    public fun test13(a: bool, b: bool) {
        let x = 0;
        if (a) {
            if (b) {
                while (x < 2) {
                    x = x + 1;
                } // no semicolon
            } else {
                loop {
                    x = x + 1;
                    if (x >= 2) break;
                } // no semicolon
            } // no semicolon
        } else {
            x = 10;
        } // no semicolon
        assert!(x > 0, 1);
    }

    // Block ending with if-else, followed by statement
    public fun test14(a: bool) {
        let x;
        {
            if (a) {
                x = 1;
            } else {
                x = 2;
            }
        } // no semicolon after block containing if-else
        let y = x + 1;
        assert!(y > 1, 1);
    }

    // If without else, followed by while, no semicolons
    public fun test15(a: bool) {
        let x = 0;
        if (a) {
            x = 1;
        } // no semicolon
        while (x < 5) {
            x = x + 1;
        } // no semicolon
        assert!(x >= 1, 1);
    }

    public fun test16(a: bool): u64 {
        let x = 0;
        if (a) x = 2;
        x
    }

    public fun test17(a: bool): u64 {
        {
            if (a) 2 else 3
        }
        4
    }

    public fun test18(a: bool): u64 {
        let x = 0;
        if (a) {
            return x
        } else {
            x = 3;
        } x
    }

    public fun test19(n: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            sum = sum + i;
        }
        sum
  }
}

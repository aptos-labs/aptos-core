module 0x42::control_flow {

    // Test nested if-else: x should have defs from all 4 branches at the end
    fun nested_if(a: bool, b: bool): u64 {
        let x;
        if (a) {
            if (b) {
                x = 1;
            } else {
                x = 2;
            }
        } else {
            if (b) {
                x = 3;
            } else {
                x = 4;
            }
        };
        x
    }

    // Test if-else chain: result has defs from all branches
    fun if_else_chain(n: u64): u64 {
        let result;
        if (n == 0) {
            result = 10;
        } else if (n == 1) {
            result = 20;
        } else if (n == 2) {
            result = 30;
        } else {
            result = 40;
        };
        result
    }

    // Test partial assignment in branches
    fun partial_assign(cond: bool): u64 {
        let x = 0;
        let y = 0;
        if (cond) {
            x = 1;
            y = 1;
        } else {
            // only y is assigned, x keeps initial value
            y = 2;
        };
        x + y
    }

    // Test multiple variables with different branch coverage
    fun multi_var_branches(a: bool, b: bool): u64 {
        let x = 0;
        let y = 0;
        let z = 0;
        if (a) {
            x = 1;
            if (b) {
                y = 1;
            };
        } else {
            z = 1;
        };
        x + y + z
    }
}

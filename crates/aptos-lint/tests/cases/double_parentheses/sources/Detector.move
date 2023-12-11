module NamedAddr::Detector {
    public fun unnecessary_parentheses_examples() {
        let a = 10;
        let b = 20;

        // Simple expression with unnecessary double parentheses
        let c = (a + b);
        // Conditional expression with unnecessary double parentheses


        // While loop with unnecessary double parentheses
        while (((a < b))) {
            a = a + 1;
        };

        // Function call with unnecessary double parentheses
        let d = add(((a)), ((b)));

        // Correct usage without double parentheses
        let e = (a + b);
        let f = add(a, b);
    }

    public fun add(x: u64, y: u64): u64 {
        x + y
    }
}
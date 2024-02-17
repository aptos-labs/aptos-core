module NamedAddr::Detector {
    public fun test_empty_if() {
        let x = 5;
        // Empty 'then' branch
        // if (x > 10) {
        // } else {
        //     x = x + 1;
        // }

        // Empty 'else' branch
        if (x < 10) {
            x = x - 1;
        } 

        // // Non-empty branches
        // if (x == 5) {
        //     x = x * 2;
        // } else {
        //     x = x / 2;
        // }

        // // Nested 'if' inside a loop with an empty branch that should be ignored by the lint
        // let y = 0;
        // while (y < 10) {
        //     if (y == 5) {
        //     } else {
        //         y = y + 1;
        //     }
        // }

        // // Nested 'if' without a loop should not be ignored
        // if (x > 0) {
        //     if (x < 5) {
        //     } else {
        //         x = x - 1;
        //     }
        // }
    }
}

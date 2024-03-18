module NamedAddr::Detector {
    const ERROR_NUM: u64 = 2;
    // public fun func1(x: bool): bool {
    //     if (x) {
    //         false
    //     } else {
    //         true
    //     }   
    // }

    // public fun example_false_true(c: bool): bool {
    //     if (c) { false } else { true }
    // }

    // public fun example_no_lint(c: bool): bool {
    //     if (c) { true } else { c }
    // }

    // public fun nested_example(c1: bool, c2: bool): bool {
    //     if (c1) {
    //         if (c2) { false } else { true }
    //     } else {
    //         true
    //     }
    // }

    public fun negation_example(c: bool): bool {
        if (!c) { true } else { false }
    }

}

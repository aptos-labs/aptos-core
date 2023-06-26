// test.move
module 0xbeef::test_entry_cases {
    // this will run
    public entry fun benchmark() {
    }

    // this is not marked as entry so it will not run
    public fun benchmark1() {
    }
}

// test multiple modules in a package
module 0xbeef::test_with_params {
    // this will run
    public entry fun benchmark_test1() {
    }

    // this will give a warning, no params allowed
    public entry fun benchmark_test2(_x: u64) {
    }
}

module 0xbeef::test_diff_names {
    // this will run, but convention is snake case
    public entry fun benchmark1() {
    }
}

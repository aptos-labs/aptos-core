module addr::warnings_as_errors {
    public fun test_function(x: u64): u64 {
        let y = 3; // Unused var y yields warning.
        x
    }
}

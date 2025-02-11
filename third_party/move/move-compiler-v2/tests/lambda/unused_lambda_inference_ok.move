module 0x8675309::M {
    public fun f() {
        // Even though the lambda is not used and abilities have nothing to be inferred from,
        // SomeFunctionValue constraint should assign a default
        let _x: |u64|u64 has drop = |i| i + 1;
    }
}

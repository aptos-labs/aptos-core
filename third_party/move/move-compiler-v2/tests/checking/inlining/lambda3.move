module 0x8675309::M {
    public fun lambda_not_allowed() {
        let _x = |i| i + 1; // expected lambda not allowed
    }
}

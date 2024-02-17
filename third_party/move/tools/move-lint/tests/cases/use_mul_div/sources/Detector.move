module NamedAddr::Detector {
    use aptos_std::math128;
    use aptos_std::math64;
    public fun inefficient_mul_div(x: u64, y: u64, z: u64): u64 {
        (x * y) / z
    }

    // Example with efficient arithmetic that should not be flagged by the lint rule
    public fun efficient_mul_div(x: u64, y: u64, z: u64): u64 {
        math64::mul_div(x, y, z)
    }

    // Control example with different arithmetic that should not be flagged
    public fun different_arithmetic(x: u64, y: u64, z: u64): u64 {
        x + y - z
    }
    
    // Example with u128 type
    public fun inefficient_mul_div_u128(x: u128, y: u128, z: u128): u128 {
        (x * y) / z
    }

    // Example with efficient arithmetic and u128 type
    public fun efficient_mul_div_u128(x: u128, y: u128, z: u128): u128 {
        math128::mul_div(x, y, z)
    }
}

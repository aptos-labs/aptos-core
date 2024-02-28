module NamedAddr::Detector {
    public fun safe_multiplication() {
        let a: u64 = 10;
        let b: u64 = 20;
        let _ = a * b; // Should not trigger a warning
    }

    public fun potential_overflow_multiplication() {
        let a: u64 = 1_000_000_000_000_000;
        let b: u64 = 2_000_000_000_000_000;
        let _ = a * b; // Should trigger a warning
    }
}

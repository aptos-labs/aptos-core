// Test that enums used in variant testing are tracked.
module 0x42::m {
    enum Status has drop {
        Active,
        Inactive { reason: u64 },
    }

    public fun is_active(s: &Status): bool {
        s is Status::Active
    }

    public fun test(): bool {
        let s = Status::Active;
        is_active(&s)
    }
}

module 0x42::Test {
    enum E {
        V1 { x: u64 },
        V2 { y: u64 }
    }

    spec E {
        // This invariant should only apply to V1, not V2
        invariant self.x > 0;
        // This invariant should only apply to V2, not V1
        invariant self.y < 100;
    }

    public fun test_v1(): E {
        // This should pass - V1 has x > 0
        E::V1 { x: 5 }
    }

    public fun test_v2(): E {
        // This should pass - V2 has y < 100
        E::V2 { y: 50 }
    }

    public fun test_v1_fail(): E {
        // This should fail - V1 has x <= 0
        E::V1 { x: 0 }
    }

    public fun test_v2_fail(): E {
        // This should fail - V2 has y >= 100
        E::V2 { y: 100 }
    }
}

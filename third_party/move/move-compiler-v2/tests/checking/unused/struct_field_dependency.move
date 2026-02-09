module 0x42::test {
    // PrivateInner is used only as a field type in PublicOuter
    struct PrivateInner {
        x: u64
    }

    // PublicOuter is used by a function
    public struct PublicOuter {
        inner: PrivateInner
    }

    // This function uses PublicOuter, so PrivateInner should NOT be flagged as unused
    public fun use_outer(outer: PublicOuter): u64 {
        outer.inner.x
    }

    // UnusedStruct is truly unused
    struct UnusedStruct {
        y: u64
    }
}

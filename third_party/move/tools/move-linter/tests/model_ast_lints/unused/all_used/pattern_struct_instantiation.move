// Test that struct usage is tracked both for the pattern struct itself
// and for struct types used in type instantiations.
module 0x42::m {
    struct Outer<T> has drop { inner: T }
    struct Inner has drop { value: u64 }

    // Both Outer and Inner should be marked as used:
    // - Outer: from the pattern itself
    // - Inner: from the type instantiation in the pattern
    public fun test(): u64 {
        let x = Outer<Inner> { inner: Inner { value: 42 } };
        let Outer<Inner> { inner } = x;
        inner.value
    }
}

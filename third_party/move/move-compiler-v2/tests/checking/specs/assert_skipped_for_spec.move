module 0x42::M {

    fun bar(x: u64): u64 {
        assert!(x > 0, 1);
        x - 1
    }

    spec fun foo(): u64 {
        // We should be able to call `bar` here because the assert is skipped when a Move function
        // is called from a spec function.
        bar(2)
    }
}

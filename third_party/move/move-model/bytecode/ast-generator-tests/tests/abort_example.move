module 0x815::m {
    public fun f(length: u64): u64 {
        assert!(length > 0, 1);
        assert!(length < 100, 2);
        let counter = 0;
        while (counter < length) counter += 1;
        counter
    }
}

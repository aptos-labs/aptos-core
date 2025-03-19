module fixed_point64::fixed_point64 {
    public fun one() : u64 {
        1
    }

    #[test]
    fun test_one() {
        assert!(one() == 1, 1);
    }
}

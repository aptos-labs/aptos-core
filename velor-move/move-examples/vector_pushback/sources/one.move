module 0xdeadbeef::one {
    public fun mod(v: vector<u64>): vector<u64> {
        v.push_back(1);
        v
    }

    #[test]
    fun test() {
        let v = vector[];
        v = mod(v);
        assert!(v[0] == 1);
    }
}

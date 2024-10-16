module 0x42::Test {
    fun t() {
        // test invalid vector instatiation
        let v = vector<&u64>[];
        let v = vector<&mut u64>[];
        let v = vector<()>[];
        let v = vector<(u64, bool)>[];
    }
}

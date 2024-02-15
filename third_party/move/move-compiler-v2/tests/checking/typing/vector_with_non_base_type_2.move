module 0x42::Test {
    fun t() {
        // test invalid vector instatiation
        let v = vector<&u64>[];
    }

    fun t_1() {
        // test invalid vector instatiation
        let v = vector<&mut u64>[];
    }

    fun t_2() {
        // test invalid vector instatiation
        let v = vector<()>[];
    }

    fun t_3() {
        // test invalid vector instatiation
        let v = vector<(u64, bool)>[];
    }

}

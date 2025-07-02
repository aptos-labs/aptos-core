//# publish
module 0xc0ffee::m {
    public fun test(): u64 {
        let v: vector<vector<vector<vector<vector<u64>>>>> = vector[];
        std::vector::length(&v)
    }
}

//# run 0xc0ffee::m::test

//# publish
module 0xc0ffee::n {
    public fun test(): u64 {
        let v: vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<vector<u64>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> = vector[];
        std::vector::length(&v)
    }
}

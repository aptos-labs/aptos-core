module 0xc0ffee::m {
    public fun test1(p: u8): u8 {
        match (p) {
            0 => 1,
            1 => 2,
            _ => 3
        }
    }

    public fun test2(p: vector<u8>): u8 {
        match (p) {
            b"" => 0,
            x"deadbeef" => 1,
            _ => 2
        }
    }

    public fun test3(p: bool, q: bool, r: bool): u8 {
        match ((p, q)) {
            (true, true) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) if r => 3,
            _ => 5
        }
    }
}

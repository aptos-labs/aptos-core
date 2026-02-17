//# publish
module 0xc0ffee::m {
    struct Point has drop {
        x: u64,
        y: u64,
    }

    public fun test_struct_match(): u64 {
        let p = Point { x: 3, y: 7 };
        match (p) {
            Point { x, y } => x + y,
        }
    }
}

//# run 0xc0ffee::m::test_struct_match

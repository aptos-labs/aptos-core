//# publish
module 0xc0ffee::m {
    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    public fun test(): u64 {
        let x = 1;
        add({x = x - 1; x + 8}, {x = x + 3; x - 3}) + add({x = x * 2; x * 2}, {x = x + 1; x})
    }
}

//# run 0xc0ffee::m::test

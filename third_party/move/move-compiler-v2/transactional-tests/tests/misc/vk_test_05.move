//# publish --print-bytecode
module 0xc0ffee::m {
    public fun test1(a: u64, b: u64): u64 {
        let a = a + 1;
        let b = b + 1;
        a + b
    }

    public fun test2(a: u64, b: u64, c: u64, d: u64): u64 {
        let a = 1 + a;
        let b = 1 + b;
        let c = 1 + c;
        let d = 1 + d;
        let _ = a + b;
        c + d
    }

    public fun test4(p: u64, q: u64): u64 {
        let x = p + 1;
        let y = q + 1;
        x + y
    }
    
}

//# publish --print-bytecode
module 0xc0ffee::n {
    public fun test1(): u64 {
        let x = 1;
        let y = x;
        x = 2;
        let z = x;
        z + y
    }

    public fun test2(): u64 {
        let x = 1;
        let y = x;
        x = 2;
        let z = x;
        y + z
    }

    public fun test3(p: u64, q: u64): u64 {
        let x = p + 1;
        let y = q + 1;
        x + y
    }
}

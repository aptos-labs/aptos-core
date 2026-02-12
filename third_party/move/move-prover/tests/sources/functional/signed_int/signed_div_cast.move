module 0x42::TestSignedDivCast {

    spec module {
        pragma verify = true;
    }

    public fun foo(a: i8, b: u8): i64 {
        if (a >= 0) {
            ((a as u64) / (b as u64)) as i64
        } else {
            - ((((- a) as u64) / (b as u64)) as i64)
        }
    }
    spec foo(a: i8, b: u8): i64 {
        aborts_if b == 0;
        aborts_if a == MIN_I8;
        ensures result == a / b;
    }

}

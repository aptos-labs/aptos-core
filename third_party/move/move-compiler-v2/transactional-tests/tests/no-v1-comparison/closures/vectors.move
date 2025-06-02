//# publish
module 0x42::test {
    use 0x1::vector;

    fun make(): vector< |u64|u64 has drop > {
        vector[|x| x+1, |x| x+2, |x| x+3]
    }

    public fun eval(x: u64): u64 {
        let v = make();
        let r = 0;
        vector::for_each(v, |f| r = r + f(x));
        r
    }
}

//# run 0x42::test::eval --args 2

module 0xc0ffee::m {
    fun blah(_x: &u64) {}

    public fun test(x: &u64) {
        blah(x);
        blah(x);
        blah(x);
        blah(x);
        blah(x);
    }

}

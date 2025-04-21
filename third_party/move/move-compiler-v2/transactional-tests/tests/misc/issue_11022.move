// Example taken from https://github.com/aptos-labs/aptos-core/issues/11022
//# publish
module 0xc0ffee::dummy1 {
    const CC: u64 = 1;

    public inline fun expose(): u64 {
        CC
    }
}

//# publish
module 0xc0ffee::dummy2 {
    public fun main(): u64 {
        0xc0ffee::dummy1::expose()
    }
}

module 0xc0ffee::dummy1 {
    const CC: u64 = 1;

    public inline fun expose(): u64 {
        CC
    }
}

module 0xc0ffee::dummy2 {
    public fun main(): u64 {
        0xc0ffee::dummy1::expose()
    }
}

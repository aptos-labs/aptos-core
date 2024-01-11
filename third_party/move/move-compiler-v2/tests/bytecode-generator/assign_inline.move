module 0x42::assign {

    const CC: u64 = 1;

    public inline fun expose(x: u64): (u64, u64) {
        (CC, x)
    }

    public fun main(): (u64, u64) {
        let (a, b) = expose(3);
        (a, b)
    }
}

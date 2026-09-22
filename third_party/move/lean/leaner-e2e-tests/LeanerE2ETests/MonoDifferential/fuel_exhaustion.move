// RUN: publish
module 0x45::spinning {
    public fun spin(n: u64): u64 {
        if (n == 0) {
            0
        } else {
            spin(n - 1) + 1
        }
    }
}
// LEANER: fuel 100
// RUN: execute 0x45::spinning::spin --args 10000

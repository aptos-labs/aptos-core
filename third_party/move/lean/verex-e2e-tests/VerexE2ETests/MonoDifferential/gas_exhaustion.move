// RUN: publish
module 0x44::burning {
    public fun burn(n: u64): u64 {
        if (n == 0) {
            0
        } else {
            burn(n - 1) + 1
        }
    }
}
// VEREX: gas 100
// VEREX: fuel 1000000
// RUN: execute 0x44::burning::burn --args 10000

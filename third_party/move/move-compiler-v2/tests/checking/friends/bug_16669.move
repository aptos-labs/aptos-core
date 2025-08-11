module 0xc0ffee::m {
    friend inline fun foo(): u64 {
        12
    }

    friend fun foo_1(): u64 {
        12
    }

    public(friend) inline fun foo_2(): u64 {
        12
    }

    friend native fun foo_3(): u64;

    friend entry fun foo_4(): u64 {
        12
    }
}

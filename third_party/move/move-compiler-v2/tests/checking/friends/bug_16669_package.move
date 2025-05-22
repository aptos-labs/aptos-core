module 0xc0ffee::m {
    package inline fun foo(): u64 {
        12
    }

    package fun foo_1(): u64 {
        12
    }

    public(package) inline fun foo_2(): u64 {
        12
    }
}

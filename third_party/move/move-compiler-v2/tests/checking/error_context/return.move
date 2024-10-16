module 0x42::m {

    fun return_1(): u64 {
        true
    }

    fun return_2(c: bool): u64 {
        if (c) return false;
        1
    }

    fun return_3() {
        1
    }

    fun return_4(c: bool) {
        if (c) return true;
    }

    fun return_5(): bool {
    }

    fun return_6(r: &u64): &mut u64 {
        r
    }
}

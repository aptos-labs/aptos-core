module 0x42::m {

    fun an_1() {
        (true : u64);
    }

    fun an_2() {
        ((1, true) : u64);
    }

    fun an_3() {
        ((@0x1, true) : address);
    }
}

module 0x42::m {

    struct S has copy, drop;

    fun f(_s: S) {
        S
    }
}

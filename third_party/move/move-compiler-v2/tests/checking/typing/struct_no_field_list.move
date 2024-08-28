module 0x42::m {

    struct S has copy, drop;

    fun f(_s: S): S {
        // dropping paramter, constructing new value
        S
    }

    fun d() {
        let S = S;
        let S{} = S;
        let S() = S;
    }
}

module 0x8675309::M {
    // struct Coin {}
    struct R<T: key>  { r: T }

    fun t1() {
        let R { r: _ } = R {r: 0};
    }
}

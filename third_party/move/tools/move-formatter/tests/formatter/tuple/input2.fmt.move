address 0x42 {
module example {
    // all 3 of these functions are equivalent

    // when no return type is provided, it is assumed to be `()`
    fun returs_unit_1() {}

    // there is an implicit () value in empty expression blocks
    fun returs_unit_2(): () {}

    // explicit version of `returs_unit_1` and `returs_unit_2`
    fun returs_unit_3(): () {
        ()
    }

    fun returns_3_values(): (u64, bool, address) {
        // comment
        (0, /*comment*/ false /*comment*/ , @0x42) // comment
    }

    fun returns_4_values(x: &u64): (&u64, u8, u128, vector<u8>) {
        (x /*comment*/ , 0 /*comment*/ , /*comment*/ 1 /*comment*/ , /*comment*/ b"foobar" /*comment*/ )
    }
}
}
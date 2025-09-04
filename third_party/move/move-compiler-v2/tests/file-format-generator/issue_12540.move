// Code example taken from https://github.com/velor-chain/velor-core/issues/12540
module 0xc0ffee::m {
    fun point_add_internal(a: &u64, b: &u64, _in_place: bool): u64 {
        *a + *b
    }

    public fun point_add_assign(a: &mut u64, b: &u64): &mut u64 {
        point_add_internal(a, b, true);
        a
    }
}

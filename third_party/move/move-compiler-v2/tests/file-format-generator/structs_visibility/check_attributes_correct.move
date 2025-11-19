module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64), // same type at the same offset
        V2(u64),
    }

    #[pack]
    public fun pack_correct(x: u64): Wrapper {
        Wrapper::V1(x)
    }

    #[unpack]
    public fun unpack_correct(wrapper: Wrapper): u64 {
        let Wrapper::V1(x) = wrapper;
        x
    }

    #[test_variant]
    public fun test_variant_correct(wrapper: &Wrapper): bool {
        wrapper is Wrapper::V1
    }

    #[borrow=0]
    public fun borrow_correct(wrapper: &Wrapper): &u64 {
        &wrapper.0
    }

    #[borrow_mut=0]
    public fun borrow_mut_correct(wrapper: &mut Wrapper): &mut u64 {
        &mut wrapper.0
    }

}

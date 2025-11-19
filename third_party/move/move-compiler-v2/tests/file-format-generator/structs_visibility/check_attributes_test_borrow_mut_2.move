module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64, u64), // same type at the same offset
        V2(u64, u64),
    }

    #[borrow=1]
    public fun borrow_incorrect(wrapper: &mut Wrapper): &mut u64 {
        &mut wrapper.0
    }

}

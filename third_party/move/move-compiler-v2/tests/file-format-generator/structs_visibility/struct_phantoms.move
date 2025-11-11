module 0x42::phantoms {

    public struct S<phantom T> has drop {
        addr: address,
    }

    struct T {}


}

module 0x42::phantoms2 {
    use 0x42::phantoms::S;
    use 0x42::phantoms::T;

    fun test_phantoms() {
       let _s = S<T>{ addr: @0x12 };
        // _s is dropped
    }
}

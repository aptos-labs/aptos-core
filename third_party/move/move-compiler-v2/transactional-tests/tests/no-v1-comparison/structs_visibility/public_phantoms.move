//# publish
module 0x42::phantoms {


    /// A struct with a phantom parameter. Even if the parameter is not dropable, the struct should still be.
    public struct S<phantom T> has drop {
        addr: address,
    }

    struct T {} // no abilities


}

//# publish
module 0x42::phantoms2 {
    use 0x42::phantoms::S;
    use 0x42::phantoms::T;

    fun test_phantoms() {
       let _s = S<T>{ addr: @0x12 };
        // _s is dropped
    }
}


//# run 0x42::phantoms2::test_phantoms

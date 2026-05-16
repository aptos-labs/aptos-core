module 0x2::ModifiesOfFunCall {
    struct Data has key { value: u64 }

    public fun get_addr(a: address): address { a }

    fun apply(f: |address| has store, a: address) {
        f(a)
    }
    spec apply {
        pragma opaque;
        modifies_of<f>(a: address) Data[get_addr(a)];
        aborts_if aborts_of<f>(a);
        ensures ensures_of<f>(a);
    }
}

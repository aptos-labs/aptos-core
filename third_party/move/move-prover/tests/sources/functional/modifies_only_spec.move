module 0x2::ModifiesOnlySpec {
    struct Data has key { x: u64 }

    public fun get_addr(a: address): address { a }

    fun mutate(addr: address) acquires Data {
        let d = borrow_global_mut<Data>(addr);
        d.x = d.x + 1;
    }
    spec mutate {
        modifies global<Data>(get_addr(addr));
    }
}

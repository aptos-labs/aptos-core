module 0x2::ModifiesFunCall {
    struct Data has key { x: u64 }

    public fun get_addr(a: address): address { a }

    public fun mutate(addr: address) acquires Data {
        let d = borrow_global_mut<Data>(addr);
        d.x = d.x + 1;
    }
    spec mutate {
        let a = get_addr(addr);
        aborts_if !exists<Data>(a);
        modifies global<Data>(get_addr(addr));
        ensures global<Data>(addr).x == old(global<Data>(addr).x) + 1;
    }
}

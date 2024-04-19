module 0x42::m {
    struct A has key {
        addr: address,
    }

    public fun foo(input: address): address acquires A {
        let a = move_from(input);
        let A { addr } = a;
        addr
    }
}

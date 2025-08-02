module 0x42::m {
    public fun g(x: &mut u64, _y: &address): &mut u64 {
        x
    }

    public fun f(x: &u64, _y: &address): &u64 {
        x
    }

    public fun f1(
        addr: address,
    ){
        let h = 5;
        let au = g(&mut 3, &addr);
        let du = f(&h, &addr);
        *du > 0;
        *au > 1;
    }
}

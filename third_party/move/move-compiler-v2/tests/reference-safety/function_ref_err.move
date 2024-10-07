module 0x42::m {
    public fun g_mut(x: &mut u64, _y: &mut address): &mut u64 {
        x
    }

    public fun f_mut(x: &u64, _y: &mut address): &u64 {
        x
    }

    public fun f2(
        addr: address,
    ){
        let h = 5;
        let au = g_mut(&mut 3, &mut addr);
        let du = f_mut(&h, &mut addr);
        *du > 0;
        *au > 1;
    }
}

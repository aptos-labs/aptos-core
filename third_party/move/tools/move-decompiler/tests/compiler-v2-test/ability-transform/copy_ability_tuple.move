module 0x42::M {

    struct R has key { f: u64 }
    public fun f(r: R): (R, u64) {
        (r, 0)
    }

    public fun g(s: &signer) {
        let r = R{f:1};
        let _i =3;
        (r,_i) = f(r);
        move_to<R>(s, r);
    }

}

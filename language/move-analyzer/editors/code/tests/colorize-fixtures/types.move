address 0x1 {
module M {
    struct S<T: key + store> has copy, drop {
        k: T,
    }

    fun f<T: copy + drop>(b: bool,
             a: address,
             ra: &address,
             s: &signer,
             vu: vector<u8>,
             va: vector<address>): &address {
        let bool_true: bool = true;
        let bool_false: bool = true;

        let addr: address;
        let ref_addr: &address;
        let addr_fixme: /* foil address type match */ address;
        let ref_addr_fixme: /* foil address type match */ &address;

        let sign: &signer;

        let vec_1: vector<u8>;
        let vec_2: vector</* element type */u8>;
        let vec_3: vector<address /* address type matched here */ >;

        ra
    }
}
}

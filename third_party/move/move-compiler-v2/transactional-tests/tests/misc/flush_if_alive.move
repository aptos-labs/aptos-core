//# publish
module 0x42::m {

    struct Cap1 has copy, store {
        x: address
    }

    struct Cap2 has copy, store {
        x: address
    }

    struct Cap3 has copy, store {
        x: address
    }

    struct Store has key {
        c: Cap3
    }

    fun destroy(c: Cap2) {
        let Cap2{x : _x} = c;
    }

    fun stuff(): (Cap1, Cap2, Cap3) {
        abort 0
    }

    fun init(s: &signer): (Cap1, Cap3) {
        let (cap1, cap2, cap3) = stuff();
        // cap3 is on top of the stack after call to `stuff`. However, it is also returned below,
        // so we must flush the stack and save it to a local.
        move_to(s, Store{c: cap3});
        destroy(cap2);
        (cap1, cap3)
    }
}

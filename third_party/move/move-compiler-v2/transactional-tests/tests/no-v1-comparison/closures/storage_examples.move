//# publish
module 0xc0ffee::m {
    struct Wrapper(|&signer|u64) has copy, drop, store, key;

    #[persistent]
    fun foo(_s: &signer): u64 {
        7
    }

    public fun init(s: &signer) {
        let w = Wrapper(foo);
        move_to(s, w)
    }

    public fun fetch_and_run(s: &signer): u64{
        let w = borrow_global<Wrapper>(std::signer::address_of(s));
        (w.0)(s)
    }
}

//# run 0xc0ffee::m::init --signers 0xc0ffee

//# run 0xc0ffee::m::fetch_and_run --signers 0xc0ffee

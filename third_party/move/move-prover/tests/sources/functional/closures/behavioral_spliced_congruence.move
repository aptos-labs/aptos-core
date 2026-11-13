// The derived contract of a lambda that forwards to an opaque callee splices the
// callee's `ensures result == f(args)` with the lambda's arguments. Those arguments
// are projections of the lambda's inputs, related to the values the callee was
// called with only by Move equality (`$IsEqual`): an `Option<String>` field compares
// by contents, not by representation. The spliced spec function therefore needs the
// same congruence axiom over `$IsEqual` as material inlined from a behavioral
// predicate; without it the lambda's `result == result_of<f>(..)` cannot be matched
// and the obligation is left to a search that does not finish.
module 0x42::behavioral_spliced_congruence {
    use std::option::Option;
    use std::string::String;

    struct Info has copy, drop, store {
        account: address,
        client_id: Option<String>,
        price: u64
    }

    struct Callbacks has copy, drop {
        settle: |Info, Info, u64| u64 has copy + drop
    }

    public fun into_inner(self: Info): (address, Option<String>, u64) {
        (self.account, self.client_id, self.price)
    }
    spec into_inner {
        aborts_if false;
        ensures result_1 == self.account;
        ensures result_2 == self.client_id;
        ensures result_3 == self.price;
    }

    spec fun spec_settle(
        a: address, ca: Option<String>, pa: u64,
        b: address, cb: Option<String>, pb: u64,
        size: u64
    ): u64;

    fun settle(
        a: address, ca: Option<String>, pa: u64,
        b: address, cb: Option<String>, pb: u64,
        size: u64
    ): u64 {
        assert!(a != b, 1);
        let _ = ca;
        let _ = cb;
        pa + pb + size
    }
    spec settle {
        pragma opaque;
        aborts_if a == b;
        aborts_if pa + pb + size > MAX_U64;
        ensures [abstract] result == spec_settle(a, ca, pa, b, cb, pb, size);
    }

    /// The lambda destructures both records and forwards every field.
    fun callbacks(): Callbacks {
        Callbacks {
            settle: |taker, maker, size| {
                let (a, ca, pa) = into_inner(taker);
                let (b, cb, pb) = into_inner(maker);
                settle(a, ca, pa, b, cb, pb, size)
            }
        }
    }

    fun run(cb: &Callbacks, taker: Info, maker: Info, size: u64): u64 {
        (cb.settle)(taker, maker, size)
    }
    spec run {
        aborts_if aborts_of<cb.settle>(taker, maker, size);
        ensures result == result_of<cb.settle>(taker, maker, size);
    }

    fun run_concrete(taker: Info, maker: Info, size: u64): u64 {
        let cb = callbacks();
        run(&cb, taker, maker, size)
    }
    spec run_concrete {
        aborts_if taker.account == maker.account;
        aborts_if taker.price + maker.price + size > MAX_U64;
        ensures result == spec_settle(
            taker.account, taker.client_id, taker.price,
            maker.account, maker.client_id, maker.price, size
        );
    }

    /// Negative: the wrong size.
    fun run_concrete_wrong(taker: Info, maker: Info, size: u64): u64 {
        let cb = callbacks();
        run(&cb, taker, maker, size)
    }
    spec run_concrete_wrong {
        pragma aborts_if_is_partial;
        ensures result == spec_settle(
            taker.account, taker.client_id, taker.price,
            maker.account, maker.client_id, maker.price, size + 1
        );
    }
}

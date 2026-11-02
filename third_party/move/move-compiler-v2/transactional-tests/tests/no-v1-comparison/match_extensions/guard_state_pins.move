// These guards attempt to mutate matched storage through aliases that the
// syntactic guard check does not track. Reference safety must reject each case
// to preserve the match coverage checker's assumption that the value is stable.

//# publish
module 0xc0ffee::alias {
    enum W has copy, drop { W1(u64), W2(u64) }
    // A copy of the matched reference taken before the match.
    public fun test(w: &mut W): u64 {
        let w2 = w;
        match (w) {
            W::W1(_) if ({ *w2 = W::W2(10); false }) => 0,
            W::W2(_) => 20,
            _ => 99,
        }
    }
}

//# publish
module 0xc0ffee::derived {
    enum W has copy, drop { W1(u64), W2(u64) }
    fun payload(w: &mut W): &mut u64 { match (w) { W::W1(v) => v, W::W2(v) => v } }
    // The call returns a reference into `w`. Reference safety must prevent the
    // guard from writing through `w` while the match holds that reference.
    public fun test(w: &mut W): u64 {
        match (payload(w)) {
            1 if ({ *w = W::W2(2); false }) => 0,
            2 => 20,
            _ => 99,
        }
    }
}

//# publish
module 0xc0ffee::global {
    enum W has copy, drop, store { W1(u64), W2(u64) }
    struct R has key { w: W }
    // Reference safety must reject a second mutable borrow of the matched resource.
    public fun test(a: address): u64 acquires R {
        match (&mut borrow_global_mut<R>(a).w) {
            W::W1(_) if ({ borrow_global_mut<R>(a).w = W::W2(1); false }) => 0,
            W::W2(_) => 20,
            _ => 99,
        }
    }
}

//# publish
module 0xc0ffee::field {
    enum W has copy, drop { W1(u64), W2(u64) }
    struct S has drop { f: W, g: u64 }
    // Reference safety must reject writes to `s.f` while the match borrows it.
    // The guard check does not restrict other fields of `s`.
    public fun test(s: &mut S): u64 {
        match (&mut s.f) {
            W::W1(_) if ({ s.f = W::W2(1); false }) => 0,
            W::W2(_) => 20,
            _ => 99,
        }
    }
}

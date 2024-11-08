module 0x42::m {

    struct S has key, drop { t: T }

    struct T has store, drop {
        w: W
    }

    struct W has store, drop {
        x: u64
    }

    fun merge(self: &mut W, s: W) {
        self.x += s.x;
    }

    fun merge_non_receiver(self1: &mut W, s: W) {
        self1.x += s.x;
    }

    fun foo_non_receiver(account: address, w: W) acquires S {
        S[account].t.w.merge_non_receiver(w)
    }

    fun foo_(account: address, w: W) acquires S {
        (&S[account].t.w).merge(w)
    }

    fun boo_(v: vector<S>, w: W) {
        (&v[0].t.w).merge(w)
    }

}

module 0x42::m {

    struct S has key, drop { t: T }

    struct T has key, store, drop {
        w: W
    }

    struct W has key, store, drop {
        x: u64
    }

    fun merge(self: &mut W, s: W) {
        self.x += s.x;
    }

    fun foo(account: address, w: W) acquires S {
        S[account].t.w.merge(w)
    }

    fun foo_2(account: address, w: W) acquires W {
        W[account].merge(w)
    }

    fun foo_3(account: address, w: W) acquires W {
        borrow_global_mut<W>(account).merge(w)
    }

    fun boo(v: vector<S>, w: W) {
        v[0].t.w.merge(w)
    }

    fun foo_(account: address, w: W) acquires S {
        (&mut S[account].t.w).merge(w)
    }

    fun boo_(v: vector<S>, w: W) {
        (&mut v[0].t.w).merge(w)
    }

    fun greater(self: &W, s: W): bool {
        self.x > s.x
    }

    fun foo_greater(account: address, w: W): bool acquires S {
        S[account].t.w.greater(w)
    }

    fun foo_greater_2(account: address, w: W): bool acquires W {
        W[account].greater(w)
    }

    fun boo_greater(v: vector<S>, w: W): bool {
        v[0].t.w.greater(w)
    }

    fun boo_greater_2(v: vector<W>, w: W): bool {
        v[0].greater(w)
    }

    fun foo_greater_(account: address, w: W): bool acquires S {
        (&mut S[account].t.w).greater(w)
    }

    fun boo_greater_(v: vector<S>, w: W): bool {
        (&mut v[0].t.w).greater(w)
    }

    fun boo_greater_2_(v: vector<W>, w: W): bool {
        (&v[0]).greater(w)
    }

    struct Wrapper<T: copy> has drop, key, store, copy {
        inner: T
    }

    fun unwrap<T: copy>(self: &Wrapper<T>): T {
        self.inner
    }

    fun dispatch<T: store + copy>(account: address): T acquires Wrapper {
        Wrapper<T>[account].unwrap()
    }

}

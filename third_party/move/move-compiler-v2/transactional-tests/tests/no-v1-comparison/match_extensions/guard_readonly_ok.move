//# publish
module 0xc0ffee::m {
    // Guards may read the matched value through its reference or pattern bindings,
    // call helpers with immutable references, shadow names, capture copyable values,
    // and mutate unrelated state. These tests also check that arm bodies retain
    // mutable access to the matched value and pattern bindings.
    enum W has copy, drop { W1(u64), W2(u64) }
    struct S has drop { f: W, g: u64 }
    struct Coin has drop { v: u64 }
    enum E has drop { V(Coin), N }

    fun is_big(w: &W): bool { match (w) { W::W1(v) => *v > 10, W::W2(v) => *v > 100 } }
    fun big(v: &u64): bool { *v > 10 }
    fun next(c: &mut u64): u64 { let v = *c; *c = v + 1; v }
    fun peek(self: &W): bool { self is W::W1 }
    fun peek_coin(c: &Coin): bool { c.v > 3 }
    fun clamp(amount: &mut u64, limit: u64) { if (*amount > limit) *amount = limit; }

    // Enum path: reads through the matched reference and through bindings.
    fun enum_reads_impl(w: &mut W): u64 {
        match (w) {
            W::W1(_) if (is_big(w)) => 1,
            W::W1(p) if (big(p) && is_big(w)) => 2,
            W::W1(_) if (w.peek() && w.0 == 3) => 3,
            W::W1(p) if (*p == 4) => { *p = 40; *p },
            W::W1(_) => { *w = W::W2(5); 5 },
            W::W2(v) => *v,
        }
    }
    public fun enum_reads(v: u64): u64 { let w = W::W1(v); enum_reads_impl(&mut w) }

    // Primitive path: the matched reference is read through pattern bindings.
    fun prim_reads_impl(x: &mut u64): u64 {
        match (x) {
            r if (*r == 1) => 1,
            r if (big(r)) => 2,
            r if (*r == 2) => { *r = 20; *r },
            5 => { *x = 50; *x },
            _ => 0,
        }
    }
    public fun prim_reads(v: u64): u64 { let x = v; prim_reads_impl(&mut x) }

    // Mixed path: the enum element is read through a pattern binding.
    fun mixed_reads_impl(w: &mut W, k: u64): u64 {
        match ((w, k)) {
            (v, 1) if (is_big(v)) => 1,
            (W::W1(p), y) if (*p == y) => { *p = 0; 2 },
            (W::W2(_), _) => 3,
            _ => 4,
        }
    }
    public fun mixed_reads(v: u64, k: u64): u64 { let w = W::W1(v); mixed_reads_impl(&mut w, k) }

    // Guards may read through immediately frozen mutable reborrows and match
    // on frozen references.
    fun frozen_reborrow_impl(w: &mut W): u64 {
        match (w) {
            W::W1(_) if (is_big(&mut *w)) => 1,
            W::W1(_) if (match (freeze(w)) { W::W1(p) => *p == 1, _ => false }) => 2,
            _ => 0,
        }
    }
    public fun frozen_reborrow(v: u64): u64 { let w = W::W1(v); frozen_reborrow_impl(&mut w) }

    // Calls through function values may read through immutable-reference parameters.
    inline fun select_by(w: &mut W, pred: |&W| bool): u64 {
        match (w) {
            W::W1(_) if (pred(w)) => 1,
            _ => 0,
        }
    }
    public fun invoke_readonly(v: u64): u64 { let w = W::W1(v); select_by(&mut w, |x| is_big(x)) }

    // Borrowing a block result borrows a temporary copy; capturing a copyable
    // binding in a closure copies it.
    public fun temporaries_and_captures(requested: u64, limit: u64): u64 {
        match (requested) {
            amount if ({ clamp(&mut { amount }, limit); amount > 100 }) => amount,
            amount if ({ let g = || amount > 3; g() }) => amount + 1,
            _ => 0,
        }
    }

    // `&*w` borrows a copy of `*w`, so the guard may mutate the original through `w`.
    // With `p: &u64`, `&mut *p` also borrows a copy, leaving the matched value intact.
    fun bump(v: &mut u64): bool { *v = *v + 1; *v > 10 }
    fun copy_reborrows_impl(w: &mut W): u64 {
        let r = match (&*w) {
            W::W1(_) if ({ let d = w; *d = W::W2(7); false }) => 0,
            W::W1(p) if (bump(&mut *p)) => 1,
            W::W1(p) => *p,
            W::W2(_) => 2,
        };
        r * 100 + (match (w) { W::W1(v) => *v, W::W2(v) => *v })
    }
    public fun copy_reborrows(v: u64): u64 { let w = W::W1(v); copy_reborrows_impl(&mut w) }

    // Borrowing a non-copy payload immutably is a read.
    public fun borrow_payload(v: u64): u64 {
        match (E::V(Coin { v })) {
            E::V(c) if (peek_coin(&c)) => c.v,
            _ => 0,
        }
    }

    // The guard's `let` shadows the binding; the body sees the original.
    public fun shadowing(requested: u64, limit: u64): u64 {
        match (requested) {
            amount if ({ let amount = limit; amount <= limit }) => amount,
            _ => 0,
        }
    }

    // The match uses the value returned by `next`; guards may mutate `c`
    // without changing that value.
    public fun unrelated_effects(): u64 {
        let c = 0;
        let r = match (next(&mut c)) {
            0 if ({ c = c + 10; false }) => 1,
            0 if (next(&mut c) == 11) => 2,
            _ => 3,
        };
        r * 100 + c
    }

    // Matching one field leaves the other fields of the struct writable.
    fun disjoint_field_impl(s: &mut S): u64 {
        match (&mut s.f) {
            W::W1(_) if ({ s.g = s.g + 1; false }) => 0,
            W::W1(v) => { *v = 2; s.g },
            _ => 9,
        }
    }
    public fun disjoint_field(): u64 {
        let s = S { f: W::W1(1), g: 0 };
        disjoint_field_impl(&mut s)
    }
}

//# run 0xc0ffee::m::enum_reads --args 11u64

//# run 0xc0ffee::m::enum_reads --args 3u64

//# run 0xc0ffee::m::enum_reads --args 4u64

//# run 0xc0ffee::m::enum_reads --args 7u64

//# run 0xc0ffee::m::prim_reads --args 1u64

//# run 0xc0ffee::m::prim_reads --args 2u64

//# run 0xc0ffee::m::prim_reads --args 5u64

//# run 0xc0ffee::m::prim_reads --args 9u64

//# run 0xc0ffee::m::mixed_reads --args 11u64 1u64

//# run 0xc0ffee::m::mixed_reads --args 3u64 3u64

//# run 0xc0ffee::m::mixed_reads --args 3u64 1u64

//# run 0xc0ffee::m::frozen_reborrow --args 11u64

//# run 0xc0ffee::m::frozen_reborrow --args 1u64

//# run 0xc0ffee::m::frozen_reborrow --args 2u64

//# run 0xc0ffee::m::invoke_readonly --args 11u64

//# run 0xc0ffee::m::invoke_readonly --args 3u64

//# run 0xc0ffee::m::temporaries_and_captures --args 500u64 100u64

//# run 0xc0ffee::m::temporaries_and_captures --args 5u64 100u64

//# run 0xc0ffee::m::temporaries_and_captures --args 2u64 100u64

//# run 0xc0ffee::m::copy_reborrows --args 11u64

//# run 0xc0ffee::m::copy_reborrows --args 3u64

//# run 0xc0ffee::m::borrow_payload --args 5u64

//# run 0xc0ffee::m::borrow_payload --args 1u64

//# run 0xc0ffee::m::shadowing --args 900u64 100u64

//# run 0xc0ffee::m::unrelated_effects

//# run 0xc0ffee::m::disjoint_field

// A function value stored in a struct or enum field without a `reads_of`/`modifies_of`
// declaration may read and write any memory, including memory outside the verification
// root, except the resources of the module running it (re-entering a module locks them). Its
// behavioral predicates are functions of the whole pre-state of the invocation, so
// an entry-state clause covers only an invocation with nothing run before it.
//
// Each function states its required verdict. A false claim verifying here is unsound.
module 0x42::cb {
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    enum CbE has drop, copy { V1 { f: |u64| u64 has copy + drop, g: |u64| u64 has copy + drop } }
    struct Two has drop, copy { f: |u64| u64 has copy + drop, g: |u64| u64 has copy + drop }
    struct Own has key { v: u64 }

    // One callback, entry-state clause, nothing before it. Verifies.
    public fun one(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec one { aborts_if aborts_of<c.f>(x); ensures result == result_of<c.f>(x); }

    // Same over an enum field. Verifies.
    public fun one_e(c: &CbE, x: u64): u64 { (c.f)(x) }
    spec one_e { aborts_if aborts_of<c.f>(x); ensures result == result_of<c.f>(x); }

    // A read of own memory before the callback, no write. Verifies.
    public fun one_after_read(c: &Cb, x: u64): u64 acquires Own {
        let b = if (exists<Own>(@0x42)) Own[@0x42].v else 0;
        (c.f)(x) + (b - b)
    }
    spec one_after_read { aborts_if aborts_of<c.f>(x); }

    // f then g, entry-state clause for g. Rejected: f may write what g reads.
    public fun two(c: &Two, x: u64): u64 { let r = (c.f)(x); let _s = (c.g)(x); r }
    spec two { aborts_if aborts_of<c.f>(x); aborts_if aborts_of<c.g>(x); }

    // The opaque form a client consumes. Rejected.
    public fun two_opaque(self: &Two, x: u64): u64 { let r = (self.f)(x); let _s = (self.g)(x); r }
    spec two_opaque { pragma opaque; aborts_if aborts_of<self.f>(x); aborts_if aborts_of<self.g>(x); }

    // Same over an enum. Rejected.
    public fun two_e(c: &CbE, x: u64): u64 { let r = (c.f)(x); let _s = (c.g)(x); r }
    spec two_e { aborts_if aborts_of<c.f>(x); aborts_if aborts_of<c.g>(x); }

    // Only the first callback's abort stated, the rest left partial. Verifies.
    public fun two_partial(c: &Two, x: u64): u64 { let r = (c.f)(x); let _s = (c.g)(x); r }
    spec two_partial { pragma aborts_if_is_partial; aborts_if aborts_of<c.f>(x); }

    // The same callback twice. Rejected.
    public fun twice(c: &Cb, x: u64): (u64, u64) { ((c.f)(x), (c.f)(x)) }
    spec twice { ensures result_1 == result_2; }

    // The same callback twice, abort of the second stated at entry. Rejected.
    public fun twice_abort(c: &Cb, x: u64): u64 { let _a = (c.f)(x); (c.f)(x) }
    spec twice_abort { aborts_if aborts_of<c.f>(x); }

    // Vacuity control. Rejected.
    public fun twice_false(c: &Cb, x: u64): (u64, u64) { ((c.f)(x), (c.f)(x)) }
    spec twice_false { ensures false; }

    // Own memory unchanged across the callback. Verifies: the callback can reach `Own` only by
    // re-entering this module, and the VM rejects that access.
    public fun keeps_own(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keeps_own { ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }

    // Opaque form, consumed by `client::use_keeps_own`. Verifies.
    public fun keeps_own_opaque(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec keeps_own_opaque { pragma opaque; ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }

    public fun set_own(v: u64) acquires Own { if (exists<Own>(@0x42)) Own[@0x42].v = v; }
    spec set_own {
        pragma opaque;
        modifies global<Own>(@0x42);
        aborts_if false;
        ensures exists<Own>(@0x42) ==> global<Own>(@0x42).v == v;
        ensures exists<Own>(@0x42) == old(exists<Own>(@0x42));
    }

    public fun make_one(f: |u64| u64 has copy + drop): Cb { Cb { f } }
    spec make_one { aborts_if false; }

    // Opaque single-callback wrapper. Verifies.
    public fun inner(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec inner { pragma opaque; aborts_if aborts_of<c.f>(x); ensures result == result_of<c.f>(x); }

    // Two opaque calls running the callback; the clause covers the first only. Rejected.
    public fun outer(c: &Cb, x: u64): u64 { let _a = inner(c, x); inner(c, x) }
    spec outer { aborts_if aborts_of<c.f>(x); }

    // A write to own memory before the callback, clause at entry. Rejected: the callback's
    // predicates are functions of the whole pre-state, own memory included, although the
    // callback cannot read it.
    public fun write_then_call(c: &Cb, x: u64): u64 acquires Own {
        if (exists<Own>(@0x42)) Own[@0x42].v = 0;
        (c.f)(x)
    }
    spec write_then_call { aborts_if aborts_of<c.f>(x); }
}

module 0x42::client {
    use 0x42::cb;

    fun pure_id(x: u64): u64 { x }
    spec pure_id { aborts_if false; ensures result == x; }

    // The callback writes the library's memory by re-entering it, so the call aborts. Verifies
    // against the library's summary, which claims the memory unchanged.
    fun use_keeps_own(x: u64): u64 {
        let c = cb::make_one(|y| { cb::set_own(y); y });
        cb::keeps_own_opaque(&c, x)
    }
    spec use_keeps_own {
        requires exists<cb::Own>(@0x42) && global<cb::Own>(@0x42).v != x;
        ensures global<cb::Own>(@0x42) == old(global<cb::Own>(@0x42));
    }

    // Pure callback through the transparent single-callback function. Verifies.
    fun use_one_pure(x: u64): u64 {
        let c = cb::make_one(|y| pure_id(y));
        cb::one(&c, x)
    }
    spec use_one_pure { aborts_if false; ensures result == x; }

    // Pure callback through the opaque single-callback wrapper. Verifies.
    fun use_inner_pure(x: u64): u64 {
        let c = cb::make_one(|y| pure_id(y));
        cb::inner(&c, x)
    }
    spec use_inner_pure { aborts_if false; ensures result == x; }

    // Verifies: whatever callback `cb::inner` runs can write `Mine` only by re-entering this
    // module.
    struct Mine has key { v: u64 }
    fun use_inner_keeps_mine(x: u64): u64 {
        let c = cb::make_one(|y| pure_id(y));
        cb::inner(&c, x)
    }
    spec use_inner_keeps_mine { ensures global<Mine>(@0x42) == old(global<Mine>(@0x42)); }
}

// Callbacks run inside opaque functions: a declared `*` frame and no frame.
module 0x42::wl {
    struct Own has key { v: u64 }
    struct CbW has drop, copy { f: |u64| u64 has copy + drop }
    spec CbW { modifies_of<f> *; }
    struct CbU has drop, copy { f: |u64| u64 has copy + drop }
    public fun make_w(f: |u64| u64 has copy + drop): CbW { CbW { f } }
    public fun make_u(f: |u64| u64 has copy + drop): CbU { CbU { f } }
    public fun set_own(v: u64) acquires Own { if (exists<Own>(@0x42)) Own[@0x42].v = v; }
    spec set_own {
        pragma opaque;
        modifies global<Own>(@0x42);
        aborts_if false;
        ensures exists<Own>(@0x42) ==> global<Own>(@0x42).v == v;
        ensures exists<Own>(@0x42) == old(exists<Own>(@0x42));
    }
    public fun run_w(c: &CbW, x: u64): u64 { (c.f)(x) }
    spec run_w { pragma opaque; }
    public fun run_u(c: &CbU, x: u64): u64 { (c.f)(x) }
    spec run_u { pragma opaque; }
}

module 0x42::wc {
    use 0x42::wl;
    // The callback writes `Own` inside the opaque call. Rejected. (True at runtime: `wl` is on
    // the stack, so the write aborts; the callee's own module is not claimed. The false
    // form, a module off the stack, is `coinish::check_other` in unframed_field_reentrancy.)
    fun use_w(x: u64): u64 { let c = wl::make_w(|y| { wl::set_own(y); y }); wl::run_w(&c, x) }
    spec use_w {
        requires exists<wl::Own>(@0x42) && global<wl::Own>(@0x42).v != x;
        ensures global<wl::Own>(@0x42) == old(global<wl::Own>(@0x42));
    }
    // Same through an unframed field. Rejected.
    fun use_u(x: u64): u64 { let c = wl::make_u(|y| { wl::set_own(y); y }); wl::run_u(&c, x) }
    spec use_u {
        requires exists<wl::Own>(@0x42) && global<wl::Own>(@0x42).v != x;
        ensures global<wl::Own>(@0x42) == old(global<wl::Own>(@0x42));
    }
}

module 0x42::gl {
    struct Own has key { v: u64 }
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    public fun make(f: |u64| u64 has copy + drop): Cb { Cb { f } }
    public fun set_own(v: u64) acquires Own { if (exists<Own>(@0x42)) Own[@0x42].v = v; }
    spec set_own {
        pragma opaque;
        modifies global<Own>(@0x42);
        aborts_if false;
        ensures exists<Own>(@0x42) ==> global<Own>(@0x42).v == v;
        ensures exists<Own>(@0x42) == old(exists<Own>(@0x42));
    }
    public fun apply_w(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec apply_w { pragma opaque; modifies_of<f> *; }

    // An unframed field invoked in a loop. Rejected.
    public fun loop_call(c: &Cb, n: u64) {
        let i = 0;
        while (i < n) { (c.f)(i); i = i + 1; };
    }
    spec loop_call { pragma opaque; ensures global<Own>(@0x42) == old(global<Own>(@0x42)); }
}

module 0x42::gc {
    use 0x42::gl;
    // A `*` parameter frame of an opaque callee. Rejected (true at runtime, as for `use_w`).
    fun use_apply_w(x: u64): u64 { gl::apply_w(|y| { gl::set_own(y); y }, x) }
    spec use_apply_w {
        requires exists<gl::Own>(@0x42) && global<gl::Own>(@0x42).v != x;
        ensures global<gl::Own>(@0x42) == old(global<gl::Own>(@0x42));
    }
}

// Behavioral predicates over a field inside a spec function.
module 0x42::sf {
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    spec fun fails_at(c: Cb, x: u64): bool { aborts_of<c.f>(x) }

    // The entry-state clause through a spec function, second invocation. Rejected.
    public fun twice_sf(c: &Cb, x: u64): u64 { let _a = (c.f)(x); (c.f)(x) }
    spec twice_sf { aborts_if fails_at(c, x); }

    // Single invocation. Verifies.
    public fun one_sf(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec one_sf { aborts_if fails_at(c, x); }

    // Vacuity control. Rejected.
    public fun twice_sf_false(c: &Cb, x: u64): u64 { let _a = (c.f)(x); (c.f)(x) }
    spec twice_sf_false { ensures false; }
}

module 0x42::more {
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    struct Own has key { v: u64 }

    // Recursion; the second invocation's abort stated at entry. Rejected.
    public fun rec(c: &Cb, n: u64) {
        if (n == 0) return;
        (c.f)(n);
        rec(c, n - 1)
    }
    spec rec {
        pragma opaque;
        requires n <= 2;
        aborts_if n == 2 && (aborts_of<c.f>(2) || aborts_of<c.f>(1));
        aborts_if n == 1 && aborts_of<c.f>(1);
    }

    // A loop invoking the callback; every iteration's abort stated at entry. Rejected.
    public fun loop_abort(c: &Cb, n: u64) {
        let i = 0;
        while (i < n) {
            (c.f)(0);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant i > 0 ==> !aborts_of<c.f>(0);
        };
    }
    spec loop_abort {
        requires n > 0;
        aborts_if aborts_of<c.f>(0);
    }

    // The operand is a spec `let` alias of the field. Rejected.
    public fun twice_let(c: &Cb, x: u64): u64 { let _a = (c.f)(x); (c.f)(x) }
    spec twice_let { let h = c.f; aborts_if aborts_of<h>(x); }

    // Vacuity control. Rejected.
    public fun twice_let_false(c: &Cb, x: u64): u64 { let _a = (c.f)(x); (c.f)(x) }
    spec twice_let_false { ensures false; }

    // Generic struct field, second invocation after a write. Rejected.
    struct G<T: copy + drop> has drop, copy { f: |T| u64 has copy + drop }
    public fun gen_two<T: copy + drop>(c: &G<T>, t: T): u64 acquires Own {
        let _a = (c.f)(t);
        if (exists<Own>(@0x42)) Own[@0x42].v = 0;
        (c.f)(t)
    }
    spec gen_two { aborts_if aborts_of<c.f>(t); }
}

module 0x42::more2 {
    use std::vector;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }
    struct M has drop, copy { h: |&mut u64| has copy + drop }

    // The field value moved into a local first. Rejected.
    public fun via_local(c: &Cb, x: u64): (u64, u64) { let g = c.f; (g(x), g(x)) }
    spec via_local { ensures result_1 == result_2; }

    // Vacuity control. Rejected.
    public fun via_local_false(c: &Cb, x: u64): (u64, u64) { let g = c.f; (g(x), g(x)) }
    spec via_local_false { ensures false; }

    // Function values read from a vector. Rejected.
    public fun via_vector(v: &vector<|u64| u64 has copy + drop>, x: u64): (u64, u64) {
        let g = *vector::borrow(v, 0);
        (g(x), g(x))
    }
    spec via_vector { requires len(v) > 0; ensures result_1 == result_2; }

    // The `&mut` output of a field callback, twice. Rejected.
    public fun mut_twice(c: &M, x: u64): (u64, u64) {
        let a = x;
        (c.h)(&mut a);
        let b = x;
        (c.h)(&mut b);
        (a, b)
    }
    spec mut_twice { ensures result_1 == result_2; }
}

// A `*` parameter frame covers state outside the verification root as well. The module
// has no memory, so only that state can make the second invocation abort.
module 0x42::wp {
    public fun one_w(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec one_w { pragma opaque; modifies_of<f> *; aborts_if aborts_of<f>(x); }

    // Rejected.
    public fun two_w(f: |u64| u64 has copy + drop, x: u64): u64 { let _a = f(x); f(x) }
    spec two_w { pragma opaque; modifies_of<f> *; aborts_if aborts_of<f>(x); }
}

// A generic library with no memory of its own: the state its callbacks share lives
// entirely outside its verification root.
module 0x43::cbs {
    enum Cbs<M: store + copy + drop> has drop {
        V1 { f: |u64, M| u64 has drop + copy, g: |M| u64 has drop + copy }
    }

    public fun pack<M: store + copy + drop>(f: |u64, M| u64 has drop + copy, g: |M| u64 has drop + copy): Cbs<M> {
        Cbs::V1 { f, g }
    }
    spec pack { aborts_if false; }

    // g runs after f, which may write what g reads. Rejected.
    public fun two<M: store + copy + drop>(self: &Cbs<M>, x: u64, m: M): u64 {
        let r = (self.f)(x, m);
        let _s = (self.g)(m);
        r
    }
    spec two {
        pragma opaque;
        aborts_if aborts_of<self.f>(x, m);
        aborts_if aborts_of<self.g>(m);
    }
}

module 0x44::use_lib {
    use 0x43::cbs;
    struct Gate has key { open: bool }

    fun closer(x: u64, _m: u64): u64 acquires Gate { Gate[@0x44].open = false; x }
    spec closer {
        pragma opaque;
        modifies Gate[@0x44];
        aborts_if !exists<Gate>(@0x44);
        ensures !Gate[@0x44].open;
        ensures result == x;
    }
    fun checker(_m: u64): u64 acquires Gate { if (Gate[@0x44].open) 1 else abort 9 }
    spec checker {
        pragma opaque;
        aborts_if !exists<Gate>(@0x44);
        aborts_if !Gate[@0x44].open;
        ensures result == 1;
    }

    // With an open gate, execution always aborts: closer closes it, checker aborts.
    // Verifies against the summary of `cbs::two`, which must fail to prove.
    public fun witness(x: u64): u64 {
        let c = cbs::pack(|y, m| closer(y, m), |m| checker(m));
        c.two(x, 0u64)
    }
    spec witness {
        requires exists<Gate>(@0x44) && Gate[@0x44].open && x < 1000;
        aborts_if false;
    }
}

// A loop invoking a closure whose target runs an unframed field.
module 0x42::lc {
    use 0x42::wl;
    // Function types used nowhere else in this file, so no other closure is a candidate.
    struct Cb has drop, copy { f: |u32| u32 has copy + drop }
    fun helper(c: Cb, x: u32): u32 { (c.f)(x) }
    fun pure_helper(x: u16): u16 { x }

    // Rejected: the field may write another module's memory in any iteration.
    public fun loop_closure(c: Cb, n: u32) {
        let g: |u32| u32 has copy + drop = |x| helper(c, x);
        let i = 0;
        while (i < n) { g(i); i = i + 1; };
    }
    spec loop_closure { ensures global<wl::Own>(@0x42) == old(global<wl::Own>(@0x42)); }

    // The same loop over a closure with known effects. Verifies.
    public fun loop_closure_pure(n: u16) {
        let g: |u16| u16 has copy + drop = |x| pure_helper(x);
        let i = 0;
        while (i < n) { g(i); i = i + 1; };
    }
    spec loop_closure_pure { ensures global<wl::Own>(@0x42) == old(global<wl::Own>(@0x42)); }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean

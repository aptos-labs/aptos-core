// Tests that global memory modified inside a loop body is havocked at the
// loop header: the loop-exit path must not retain pre-loop memory. Without
// the havoc, the false frame claims below verify vacuously.
module 0x42::loop_memory_havoc {
    struct R has key { x: u64 }

    // Loop mutates global memory through an in-body borrow; the false
    // post-loop frame claim must fail.
    fun bump_loop_frame_fails(addr: address, n: u64) acquires R {
        let i = 0;
        while (i < n) {
            let r = borrow_global_mut<R>(addr);
            r.x = r.x + 1;
            i = i + 1;
        };
    }
    spec bump_loop_frame_fails {
        requires exists<R>(addr);
        ensures global<R>(addr).x == old(global<R>(addr).x);
    }

    // Control: the same false claim without a loop.
    fun bump_once_fails(addr: address) acquires R {
        let r = borrow_global_mut<R>(addr);
        r.x = r.x + 1;
    }
    spec bump_once_fails {
        requires exists<R>(addr);
        ensures global<R>(addr).x == old(global<R>(addr).x);
    }

    // Loop invariants recover full precision over the havocked memory: the
    // exact final value and the complete abort condition are provable.
    // Here the initial value is snapshotted into a local; see
    // `bump_loop_with_old_invariant` below for the direct `old(..)` form.
    fun bump_loop_with_invariant(addr: address, n: u64) acquires R {
        let x0 = borrow_global<R>(addr).x;
        let i = 0;
        while ({
            spec {
                invariant i <= n;
                invariant exists<R>(addr);
                invariant global<R>(addr).x == x0 + i;
            };
            i < n
        }) {
            let r = borrow_global_mut<R>(addr);
            r.x = r.x + 1;
            i = i + 1;
        };
    }
    spec bump_loop_with_invariant {
        aborts_if !exists<R>(addr);
        aborts_if global<R>(addr).x + n > MAX_U64;
        ensures global<R>(addr).x == old(global<R>(addr).x) + n;
    }

    // Ghost variables are backed by global memory and follow the same rules.
    spec module {
        global gcount: num;
    }

    // Ghost updated inline in the loop body: the false frame claim must fail.
    fun ghost_inline_update_fails(n: u64) {
        let i = 0;
        while (i < n) {
            spec {
                update gcount = gcount + 1;
            };
            i = i + 1;
        };
    }
    spec ghost_inline_update_fails {
        ensures gcount == old(gcount);
    }

    // Ghost updated by a called function inside the loop: must fail too.
    fun bump_ghost() {
        spec {
            update gcount = gcount + 1;
        };
    }
    spec bump_ghost {
        aborts_if false;
        ensures gcount == old(gcount) + 1;
    }

    fun ghost_callee_update_fails(n: u64) {
        let i = 0;
        while (i < n) {
            bump_ghost();
            i = i + 1;
        };
    }
    spec ghost_callee_update_fails {
        ensures gcount == old(gcount);
    }

    // Ghost update inside a loop body with a substantial preamble of
    // straight-line code before the loop. Exercises the Prop(Assume)-based
    // ghost-update detection: `spec.on_impl` is keyed by file-format offsets
    // that don't align with the stackless code offsets used at loop-analysis
    // time, so the earlier offset-keyed scan silently missed updates whose
    // effective offsets landed outside the loop body. The Prop(Assume) scan
    // is in the correct offset space.
    fun ghost_update_after_preamble_fails(n: u64) {
        let a = 1;
        let b = 2;
        let c = a + b;
        let d = c * 2;
        let e = d + 1;
        let f = e * 3;
        let g = f + 4;
        let h = g * 5;
        let _ = h;
        let i = 0;
        while (i < n) {
            spec {
                update gcount = gcount + 1;
            };
            i = i + 1;
        };
    }
    spec ghost_update_after_preamble_fails {
        ensures gcount == old(gcount);
    }

    // Same for ghost memory: an invariant proves the exact final ghost value.
    fun ghost_update_with_invariant(n: u64) {
        spec {
            update gcount = 0;
        };
        let i = 0;
        while ({
            spec {
                invariant i <= n;
                invariant gcount == i;
            };
            i < n
        }) {
            spec {
                update gcount = gcount + 1;
            };
            i = i + 1;
        };
    }
    spec ghost_update_with_invariant {
        ensures gcount == n;
    }

    // Ghost *fields* follow the same rules as ghost memory: an update inside
    // a loop havocs the base local's ghost state at the loop header, so a
    // false frame claim over the ghost field must not verify.
    struct GS has copy, drop { x: u64 }
    spec GS {
        ghost gf: u64;
    }
    fun ghost_field_loop_fails(n: u64): GS {
        let s = GS { x: 0 };
        spec { update s.gf = 0; };
        let i = 0;
        while (i < n) {
            spec { update s.gf = 1; };
            i = i + 1;
        };
        s
    }
    spec ghost_field_loop_fails {
        ensures result.gf == 0;
    }

    // Positive: an invariant re-establishes the exact final ghost-field value.
    fun ghost_field_loop_with_invariant(n: u64): GS {
        let s = GS { x: 0 };
        spec { update s.gf = 0; };
        let i = 0;
        while ({
            spec {
                invariant i <= n;
                invariant s.gf == i;
            };
            i < n
        }) {
            spec { update s.gf = s.gf + 1; };
            i = i + 1;
        };
        s
    }
    spec ghost_field_loop_with_invariant {
        ensures result.gf == n;
    }

    // A ghost variable not updated by the loop is framed across it.
    fun ghost_untouched_framed(addr: address, n: u64) acquires R {
        spec {
            update gcount = 7;
        };
        let i = 0;
        while (i < n) {
            let r = borrow_global_mut<R>(addr);
            r.x = r.x + 1;
            i = i + 1;
        };
    }
    spec ghost_untouched_framed {
        requires exists<R>(addr);
        ensures gcount == 7;
    }

    // A function value declared via `modifies_of` to modify R, invoked in the
    // loop body: the false frame claim must fail.
    fun fv_modifies_update_fails(f: |address| has drop + copy, addr: address, n: u64) {
        let i = 0;
        while (i < n) {
            f(addr);
            i = i + 1;
        };
    }
    spec fv_modifies_update_fails {
        modifies_of<f>(a: address) R[a];
        requires exists<R>(addr);
        ensures global<R>(addr).x == old(global<R>(addr).x);
    }

    // A function value stored in a struct field carries its `modifies_of`
    // declaration from the struct spec; invoking it in a loop body must havoc
    // the declared memory.
    struct Counter has key { x: u64 }
    struct Config has key { active: bool }

    #[persistent]
    fun bump_counter(s: &signer) {
        let addr = std::signer::address_of(s);
        if (exists<Counter>(addr) && Counter[addr].x < 18446744073709551615) {
            Counter[addr].x = Counter[addr].x + 1;
        }
    }
    spec bump_counter {
        pragma opaque;
        modifies Counter[std::signer::address_of(s)];
        aborts_if false;
    }

    struct StoredModifier has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec StoredModifier {
        modifies_of<f>(s: signer) Counter[std::signer::address_of(s)];
        invariant forall s: signer: !aborts_of<f>(s);
    }

    // The false frame claim over the field's declared modifies target must fail.
    fun fv_field_modifies_update_fails(addr: address, s: &signer, n: u64) acquires StoredModifier {
        let m = &StoredModifier[addr];
        let i = 0;
        while (i < n) {
            (m.f)(s);
            i = i + 1;
        };
    }
    spec fv_field_modifies_update_fails {
        requires exists<StoredModifier>(addr);
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }

    // Memory outside the field's declaration stays framed across the loop.
    fun fv_field_untouched_framed(addr: address, s: &signer, n: u64) acquires StoredModifier {
        let m = &StoredModifier[addr];
        let i = 0;
        while (i < n) {
            (m.f)(s);
            i = i + 1;
        };
    }
    spec fv_field_untouched_framed {
        requires exists<StoredModifier>(addr);
        requires exists<Config>(std::signer::address_of(s));
        ensures Config[std::signer::address_of(s)].active
            == old(Config[std::signer::address_of(s)].active);
    }

    // A closure built in the same function as the loop: the function under
    // processing is held out of the targets holder, so its own bytecode must
    // be scanned for closure origins.
    fun fv_local_closure_fails(s: &signer, n: u64) {
        let f: |&signer| has copy + drop = bump_counter;
        let i = 0;
        while (i < n) {
            f(s);
            i = i + 1;
        };
    }
    spec fv_local_closure_fails {
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }

    // A closure built in a helper and returned reaches the loop without any
    // parameter or field declaration; its closed-over function's modified
    // memory must still be havocked.
    fun make_bumper(): |&signer| has copy + drop {
        bump_counter
    }

    fun fv_returned_closure_fails(s: &signer, n: u64) {
        let f = make_bumper();
        let i = 0;
        while (i < n) {
            f(s);
            i = i + 1;
        };
    }
    spec fv_returned_closure_fails {
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }

    // A curried closure of a different type from the captured callback:
    // the closure scan looks up the closed-over function's usage summary,
    // which must include the forwarded `modifies_of<f>` frame via
    // `invoke_frame` transitively.
    #[persistent]
    fun tick_at(a: address) acquires Counter {
        if (exists<Counter>(a) && Counter[a].x < 18446744073709551615) {
            Counter[a].x = Counter[a].x + 1;
        }
    }
    spec tick_at {
        pragma opaque;
        modifies Counter[a];
        aborts_if false;
    }
    fun curry_helper(f: |address| has drop + copy, a: address) { f(a); }
    spec curry_helper {
        pragma aborts_if_is_partial;
        modifies_of<f>(x: address) Counter[x];
    }
    #[persistent]
    fun curry_wrap(f: |address| has drop + copy, a: address, _n: u64) {
        curry_helper(f, a);
    }
    fun make_curry(f: |address| has drop + copy, a: address): |u64| has drop + copy {
        |n| curry_wrap(f, a, n)
    }
    fun fv_curry_fails(a: address, n: u64) {
        let g = make_curry(tick_at, a);
        let i = 0;
        while (i < n) {
            g(i);
            i = i + 1;
        };
    }
    spec fv_curry_fails {
        requires exists<Counter>(a);
        ensures Counter[a].x == old(Counter[a].x);
    }

    // A forwarding helper with a `modifies_of<f> *` wildcard: its own
    // accessed footprint is empty, so the summary carries the wildcard as a
    // propagated flag rather than a resolved memory set. The caller's loop
    // must resolve the flag against its own accessed footprint (which
    // includes memory mentioned in its specs).
    fun wild_forwarder(f: |&signer| has drop + copy, s: &signer) { f(s); }
    spec wild_forwarder {
        pragma aborts_if_is_partial;
        modifies_of<f> *;
    }
    fun fv_wild_forward_fails(f: |&signer| has drop + copy, s: &signer, n: u64) {
        let i = 0;
        while (i < n) {
            wild_forwarder(f, s);
            i = i + 1;
        };
    }
    spec fv_wild_forward_fails {
        pragma aborts_if_is_partial;
        modifies_of<f> *;
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }

    // A forwarding helper whose only effect is invoking its declared
    // parameter: the helper's `modifies_of<f>` frame must propagate through
    // its `modified` summary so that a caller's loop invoking the helper
    // still havocs the declared memory. Verified transitively (depth 2).
    fun fwd_inner(f: |&signer| has drop + copy, s: &signer) { f(s); }
    spec fwd_inner {
        modifies_of<f>(x: signer) Counter[std::signer::address_of(x)];
    }
    fun fwd_outer(f: |&signer| has drop + copy, s: &signer) { fwd_inner(f, s); }

    fun fv_forward_depth2_fails(f: |&signer| has drop + copy, s: &signer, n: u64) {
        let i = 0;
        while (i < n) {
            fwd_outer(f, s);
            i = i + 1;
        };
    }
    spec fv_forward_depth2_fails {
        modifies_of<f>(x: signer) Counter[std::signer::address_of(x)];
        requires exists<Counter>(std::signer::address_of(s));
        ensures Counter[std::signer::address_of(s)].x
            == old(Counter[std::signer::address_of(s)].x);
    }

    // Recursive call: the function under processing is held out of the
    // targets holder, so its own fixpointed usage summary must be consulted
    // for self-referential effects.
    fun rec_bump(a: address, n: u64, depth: u64) acquires R {
        if (depth == 0) {
            let r = &mut R[a];
            r.x = if (r.x < 100) { r.x + 1 } else { r.x };
            return
        };
        let i = 0;
        while (i < n) {
            rec_bump(a, n, depth - 1);
            i = i + 1;
        };
    }
    spec rec_bump {
        requires exists<R>(a);
        ensures R[a].x == old(R[a].x);
    }

    // Generic direct writeback: the resource instantiation `R<T>` must be
    // havocked in the caller's own type context.
    struct G<phantom T> has key { v: u64 }

    fun gen_bump<T>(a: address, n: u64) acquires G {
        let i = 0;
        while (i < n) {
            let g = &mut G<T>[a];
            g.v = g.v + 1;
            i = i + 1;
        };
    }
    spec gen_bump {
        requires exists<G<T>>(a);
        ensures G<T>[a].v == old(G<T>[a].v);
    }

    // A function value without access declarations is pure and cannot touch
    // global state: the frame claim verifies without havoc.
    fun fv_pure_framed(f: |u64| u64 has drop + copy, addr: address, n: u64): u64 acquires R {
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = f(acc);
            i = i + 1;
        };
        let _ = acc;
        borrow_global<R>(addr).x
    }
    spec fv_pure_framed {
        requires exists<R>(addr);
        ensures global<R>(addr).x == old(global<R>(addr).x);
    }

    // Memory not touched by the loop is framed: no havoc for it, so pre-loop
    // facts survive without an invariant.
    struct S has key { y: u64 }

    fun untouched_memory_framed(addr: address, n: u64): u64 acquires R, S {
        borrow_global_mut<S>(addr).y = 7;
        let i = 0;
        while (i < n) {
            let r = borrow_global_mut<R>(addr);
            r.x = r.x + 1;
            i = i + 1;
        };
        borrow_global<S>(addr).y
    }
    spec untouched_memory_framed {
        requires exists<R>(addr);
        requires exists<S>(addr);
        ensures result == 7;
    }

    // `old(..)` over global state can be used directly in a loop invariant;
    // it resolves to the memory at function entry, so no snapshot local is
    // needed (compare `bump_loop_with_invariant` above).
    fun bump_loop_with_old_invariant(addr: address, n: u64) {
        let i = 0;
        while (i < n) {
            let r = &mut R[addr];
            r.x = r.x + 1;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant exists<R>(addr);
            invariant R[addr].x == old(R[addr].x) + i;
        };
    }
    spec bump_loop_with_old_invariant {
        requires exists<R>(addr);
        aborts_if global<R>(addr).x + n > MAX_U64;
        ensures global<R>(addr).x == old(global<R>(addr).x) + n;
    }
}

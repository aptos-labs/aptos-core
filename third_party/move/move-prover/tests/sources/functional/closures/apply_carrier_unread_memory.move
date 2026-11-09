// Results of a call through a function value are bound to the target's `result_of` carrier
// only when the target's specification determines them: the carrier ranges over the
// specification's memory, not over what the body reads.
module 0x42::apply_carrier_unread_memory {
    struct S has key { v: u64 }
    struct R has key { w: u64 }

    // Reads R, which its specification never names.
    fun g(x: u64): u64 acquires S, R {
        if (S[@0x42].v > 0 && exists<R>(@0x42)) (x % 2) + (R[@0x42].w % 2) else 0
    }
    spec g {
        pragma opaque;
        aborts_if !exists<S>(@0x42);
        ensures global<S>(@0x42).v == 0 ==> result == 0;
    }

    // Names every resource it reads, without determining its result.
    fun g_named(x: u64): u64 acquires S, R {
        if (S[@0x42].v > 0 && exists<R>(@0x42)) (x % 2) + (R[@0x42].w % 2) else 0
    }
    spec g_named {
        pragma opaque;
        aborts_if !exists<S>(@0x42);
        ensures global<S>(@0x42).v == 0 ==> result == 0;
        ensures exists<R>(@0x42) ==> result <= 1 + global<R>(@0x42).w;
    }

    // The `&mut` output reads R; only the value result is determined.
    fun set_from_r(c: &mut u64): u64 acquires R {
        if (exists<R>(@0x42)) *c = R[@0x42].w % 2;
        0
    }
    spec set_from_r { pragma opaque; aborts_if false; ensures result == 0; }

    fun flip() acquires R {
        if (exists<R>(@0x42)) {
            let r = &mut R[@0x42];
            r.w = if (r.w % 2 == 0) 1 else 0
        }
    }
    spec flip { pragma opaque; modifies global<R>(@0x42); aborts_if false; }

    // With S.v = 1 and R.w = 0 the results are (0, 1).
    fun two_calls_across_write(): (u64, u64) acquires S, R {
        let h: |u64| u64 has copy + drop = |y: u64| g(y);
        let a = h(0);
        flip();
        let b = h(0);
        (a, b)
    }
    spec two_calls_across_write {
        requires exists<S>(@0x42) && global<S>(@0x42).v == 1;
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    fun two_calls_direct(): (u64, u64) acquires S, R {
        let a = g(0);
        flip();
        let b = g(0);
        (a, b)
    }
    spec two_calls_direct {
        requires exists<S>(@0x42) && global<S>(@0x42).v == 1;
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: same as through the function value
    }

    // Naming all read memory does not determine the result.
    fun two_calls_named(x: u64): (u64, u64) acquires S, R {
        let h: |u64| u64 has copy + drop = |y: u64| g_named(y);
        (h(x), h(x))
    }
    spec two_calls_named { ensures result_1 == result_2; } // error: the result is not determined

    // With R.w = 0 the outputs are (0, 1).
    fun mut_outputs_across_write(): (u64, u64) acquires R {
        let h: |&mut u64| u64 has copy + drop = |c: &mut u64| set_from_r(c);
        let c1 = 7;
        let c2 = 7;
        h(&mut c1);
        flip();
        h(&mut c2);
        (c1, c2)
    }
    spec mut_outputs_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the `&mut` output reads the changed R
    }

    fun read_r(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w else 0 }
    spec read_r {
        pragma opaque;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w else 0);
    }

    // Higher-order targets: their effects omit what the callback they run reads.
    fun run_unsat(f: || u64 has copy + drop): u64 { f() }
    spec run_unsat { pragma opaque; requires false; aborts_if false; }

    fun run(f: || u64 has copy + drop): u64 { f() }
    spec run { pragma opaque, verify = false; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun callback_across_write_unsat(): (u64, u64) acquires R {
        let h: |(|| u64 has copy + drop)| u64 has copy + drop = run_unsat;
        let a = h(read_r);
        flip();
        let b = h(read_r);
        (a, b)
    }
    spec callback_across_write_unsat {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    fun callback_across_write(): (u64, u64) acquires R {
        let h: |(|| u64 has copy + drop)| u64 has copy + drop = run;
        let a = h(read_r);
        flip();
        let b = h(read_r);
        (a, b)
    }
    spec callback_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    // Reads R, which its specification never names, but the specification determines the
    // result.
    fun g_determined(x: u64): u64 acquires R {
        if (exists<R>(@0x42) && R[@0x42].w > 7) x else x
    }
    spec g_determined { pragma opaque; aborts_if false; ensures result == x; }

    // Binding is kept for a determined result.
    fun determined_across_write(): (u64, u64) acquires R {
        let h: |u64| u64 has copy + drop = |y: u64| g_determined(y);
        let a = h(3);
        flip();
        let b = h(3);
        (a, b)
    }
    spec determined_across_write { ensures result_1 == result_2; }

    // Reads R, which its specification never names, under a `requires`.
    fun g_requires(x: u64): u64 acquires S, R {
        if (S[@0x42].v > 0 && exists<R>(@0x42)) (x % 2) + (R[@0x42].w % 2) else 0
    }
    spec g_requires {
        pragma opaque;
        requires exists<S>(@0x42);
        aborts_if false;
        ensures global<S>(@0x42).v == 0 ==> result == 0;
    }

    // With S.v = 1 and R.w = 0 the results are (0, 1).
    fun requires_across_write(): (u64, u64) acquires S, R {
        let h: |u64| u64 has copy + drop = |y: u64| g_requires(y);
        let a = h(0);
        flip();
        let b = h(0);
        (a, b)
    }
    spec requires_across_write {
        requires exists<S>(@0x42) && global<S>(@0x42).v == 1;
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    // Invokes a local function value, so its effects are not known.
    fun read_r_locally(): u64 acquires R {
        let f: || u64 has copy + drop = || read_r();
        f()
    }
    spec read_r_locally { pragma opaque, verify = false; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun unbounded_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = || read_r_locally();
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec unbounded_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    // A native's contract need not name the state it reads.
    native fun tick(): u64;
    spec tick { pragma opaque; aborts_if false; }

    fun tick_twice(): u64 { tick() }
    spec tick_twice { pragma opaque; aborts_if false; }

    native fun same(x: u64): u64;
    spec same { pragma opaque; aborts_if false; ensures result == x; }

    fun native_twice(): (u64, u64) {
        let h: || u64 has copy + drop = tick;
        (h(), h())
    }
    spec native_twice { ensures result_1 == result_2; } // error: each call may differ

    fun native_wrapper_twice(): (u64, u64) {
        let h: || u64 has copy + drop = || tick_twice();
        (h(), h())
    }
    spec native_wrapper_twice { ensures result_1 == result_2; } // error: each call may differ

    // Binding is kept for a native whose specification determines the result.
    fun determined_native_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = same;
        (h(x), h(x))
    }
    spec determined_native_twice { ensures result_1 == result_2; }

    struct Q has key { u: u64 }

    fun flip_q() acquires Q {
        if (exists<Q>(@0x42)) {
            let q = &mut Q[@0x42];
            q.u = if (q.u % 2 == 0) 1 else 0
        }
    }
    spec flip_q { pragma opaque; modifies global<Q>(@0x42); aborts_if false; }

    // Partial specifications naming the memory they read, over a native.
    fun r_plus_same(x: u64): u64 acquires R {
        if (exists<R>(@0x42)) same(x % 8) + (R[@0x42].w % 2) else same(x % 8)
    }
    spec r_plus_same {
        pragma opaque;
        aborts_if false;
        ensures exists<R>(@0x42) ==> result <= 9;
    }

    fun r_plus_tick(x: u64): u64 acquires R {
        if (exists<R>(@0x42)) (tick() % 8) + (R[@0x42].w % 2) + x * 0 else 0
    }
    spec r_plus_tick {
        pragma opaque, verify = false;
        aborts_if false;
        ensures exists<R>(@0x42) ==> result <= 9;
    }

    fun signer_r(s: &signer): address acquires R {
        if (exists<R>(@0x42) && R[@0x42].w > 7) std::signer::address_of(s)
        else std::signer::address_of(s)
    }
    spec signer_r {
        pragma opaque, verify = false;
        aborts_if false;
        ensures exists<R>(@0x42) ==> result == std::signer::address_of(s);
    }

    // Partial specifications over natives.
    fun pure_native_across_write(x: u64): (u64, u64) acquires R, Q {
        let h: |u64| u64 has copy + drop = |y: u64| r_plus_same(y);
        let a = h(x);
        flip_q();
        let b = h(x);
        (a, b)
    }
    spec pure_native_across_write { ensures result_1 == result_2; } // error: the result is not determined

    fun native_across_write(x: u64): (u64, u64) acquires R, Q {
        let h: |u64| u64 has copy + drop = |y: u64| r_plus_tick(y);
        let a = h(x);
        flip_q();
        let b = h(x);
        (a, b)
    }
    spec native_across_write {
        ensures result_1 == result_2; // error: each call may differ
    }

    // A native's specification memory is not in its caller's usage summary.
    native fun peek(): u64;
    spec peek {
        pragma opaque;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w else 0);
    }

    fun peek_twice(): u64 { peek() }
    spec peek_twice { pragma opaque, verify = false; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun native_memory_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = || peek_twice();
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec native_memory_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    fun signer_across_write(s: &signer): (address, address) acquires R, Q {
        let h: |&signer| address has copy + drop = |t: &signer| signer_r(t);
        let a = h(s);
        flip_q();
        let b = h(s);
        (a, b)
    }
    spec signer_across_write { ensures result_1 == result_2; } // error: the result is not determined

    // A callback's `reads_of` frame is part of what a target handing it over accesses.
    fun read_r_odd(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 }
    spec read_r_odd {
        pragma opaque;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w % 2 else 0);
    }

    fun apply_reads(f: || u64 has copy + drop): u64 { f() }
    spec apply_reads { pragma opaque; reads_of<f> R; aborts_if aborts_of<f>(); }

    fun apply_reads_transparent(f: || u64 has copy + drop): u64 { f() }
    spec apply_reads_transparent { reads_of<f> R; }

    fun apply_reads_any(f: || u64 has copy + drop): u64 { f() }
    spec apply_reads_any { pragma opaque; reads_of<f> *; aborts_if aborts_of<f>(); }

    fun apply_modifies(f: || u64 has copy + drop): u64 { f() }
    spec apply_modifies {
        pragma opaque;
        modifies_of<f>() global<R>(@0x42);
        aborts_if aborts_of<f>();
    }

    fun hand_over(): u64 { apply_reads(read_r_odd) }
    spec hand_over { pragma opaque; aborts_if false; }

    fun hand_over_unsat(): u64 { apply_reads(read_r_odd) }
    spec hand_over_unsat { pragma opaque; requires false; aborts_if false; }

    fun hand_over_transparent(): u64 { apply_reads_transparent(read_r_odd) }
    spec hand_over_transparent { pragma opaque; aborts_if false; }

    fun hand_over_any(): u64 { apply_reads_any(read_r_odd) }
    spec hand_over_any { pragma opaque; aborts_if false; }

    fun hand_over_modifies(): u64 { apply_modifies(read_r_odd) }
    spec hand_over_modifies { pragma opaque; aborts_if false; }

    fun hand_over_named(): u64 { apply_reads(read_r_odd) }
    spec hand_over_named {
        pragma opaque, verify = false;
        aborts_if false;
        ensures exists<R>(@0x42) ==> result <= global<R>(@0x42).w;
    }

    // With R.w = 0 the results are (0, 1).
    fun reads_frame_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = hand_over;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec reads_frame_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    fun reads_frame_unsat_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = hand_over_unsat;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec reads_frame_unsat_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    fun transparent_reads_frame_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = hand_over_transparent;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec transparent_reads_frame_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    fun modifies_frame_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = hand_over_modifies;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec modifies_frame_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the callback reads the changed R
    }

    fun reads_frame_twice(): (u64, u64) {
        let h: || u64 has copy + drop = hand_over;
        (h(), h())
    }
    spec reads_frame_twice {
        ensures result_1 == result_2; // error: the carrier names no memory
    }

    fun reads_any_twice(): (u64, u64) {
        let h: || u64 has copy + drop = hand_over_any;
        (h(), h())
    }
    spec reads_any_twice {
        ensures result_1 == result_2; // error: what the callback reads is not known
    }

    // The carrier names the memory of the `reads_of` frame, but does not determine the result.
    fun reads_frame_named_across_write(): (u64, u64) acquires Q {
        let h: || u64 has copy + drop = hand_over_named;
        let a = h();
        flip_q();
        let b = h();
        (a, b)
    }
    spec reads_frame_named_across_write {
        ensures result_1 == result_2; // error: the result is not determined
    }

    // An intrinsic whose specification reads memory counts as a native.
    native fun peek_intrinsic(): u64;
    spec peek_intrinsic {
        pragma opaque, intrinsic = true;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w % 2 else 0);
    }

    fun peek_intrinsic_twice(): u64 { peek_intrinsic() }
    spec peek_intrinsic_twice { pragma opaque, verify = false; aborts_if false; }

    fun peek_move_intrinsic(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 }
    spec peek_move_intrinsic {
        pragma opaque, intrinsic = true;
        aborts_if false;
        ensures result == (if (exists<R>(@0x42)) global<R>(@0x42).w % 2 else 0);
    }

    fun peek_move_intrinsic_twice(): u64 { peek_move_intrinsic() }
    spec peek_move_intrinsic_twice { pragma opaque, verify = false; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun intrinsic_memory_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = peek_intrinsic_twice;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec intrinsic_memory_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    fun move_intrinsic_memory_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = peek_move_intrinsic_twice;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec move_intrinsic_memory_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    // An intrinsic the prover does not implement stands for a body nothing summarizes.
    fun peek_bodied_intrinsic(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 }
    spec peek_bodied_intrinsic { pragma opaque, intrinsic = true; aborts_if false; }

    fun peek_bodied_intrinsic_twice(): u64 { peek_bodied_intrinsic() }
    spec peek_bodied_intrinsic_twice { pragma opaque, verify = false; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun bodied_intrinsic_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = peek_bodied_intrinsic_twice;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec bodied_intrinsic_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    // A memory-free intrinsic under a partial specification.
    fun has_key(t: &extensions::table::Table<u64, u64>, k: u64): bool {
        extensions::table::contains(t, k)
    }
    spec has_key { pragma opaque; aborts_if false; }

    fun table_across_write(t: &extensions::table::Table<u64, u64>, k: u64): (bool, bool)
        acquires Q
    {
        let h: |&extensions::table::Table<u64, u64>, u64| bool has copy + drop = has_key;
        let a = h(t, k);
        flip_q();
        let b = h(t, k);
        (a, b)
    }
    spec table_across_write { ensures result_1 == result_2; } // error: the result is not determined

    // Natives of standard-library modules that are functions of their arguments, under
    // partial specifications.
    fun pushed(x: u64): u64 {
        let v = vector[x];
        std::vector::push_back(&mut v, 1);
        std::vector::length(&v) + (*std::vector::borrow(&v, 0) % 2)
    }
    spec pushed { pragma opaque; aborts_if false; ensures result <= 3; }

    fun serialized(x: u64): u64 { std::vector::length(&std::bcs::to_bytes(&x)) }
    spec serialized { pragma opaque, verify = false; aborts_if false; }

    fun utf8_one(x: u8): bool {
        std::string::length(&std::string::utf8(vector[x % 64 + 48])) == 1
    }
    spec utf8_one { pragma opaque, verify = false; aborts_if false; }

    fun hashed(x: u64): u64 { std::vector::length(&std::hash::sha3_256(std::bcs::to_bytes(&x))) }
    spec hashed { pragma opaque, verify = false; aborts_if false; }

    fun vector_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = pushed;
        (h(x), h(x))
    }
    spec vector_twice { ensures result_1 == result_2; } // error: the result is not determined

    fun bcs_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = serialized;
        (h(x), h(x))
    }
    spec bcs_twice { ensures result_1 == result_2; } // error: the result is not determined

    fun string_twice(x: u8): (bool, bool) {
        let h: |u8| bool has copy + drop = utf8_one;
        (h(x), h(x))
    }
    spec string_twice { ensures result_1 == result_2; } // error: the result is not determined

    fun hash_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = hashed;
        (h(x), h(x))
    }
    spec hash_twice { ensures result_1 == result_2; } // error: the result is not determined

    // Natives over a `&mut` parameter or over state no argument holds.
    native fun bump(c: &mut u64): u64;
    spec bump { pragma opaque; aborts_if false; ensures result == 0; }

    fun bumped(x: u64): u64 {
        let c = x;
        bump(&mut c);
        c
    }
    spec bumped { pragma opaque, verify = false; aborts_if false; }

    fun mut_native_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = bumped;
        (h(x), h(x))
    }
    spec mut_native_twice { ensures result_1 == result_2; } // error: each call may differ

    fun context_id(): u64 { 0x1::transaction_context::unique_id() }
    spec context_id { pragma opaque, verify = false; aborts_if false; }

    fun context_twice(): (u64, u64) {
        let h: || u64 has copy + drop = context_id;
        (h(), h())
    }
    spec context_twice { ensures result_1 == result_2; } // error: each call may differ

    fun local_digest(x: u64): u64 {
        std::vector::length(&0x42::hash::digest(std::bcs::to_bytes(&x)))
    }
    spec local_digest { pragma opaque, verify = false; aborts_if false; }

    fun local_digest_twice(x: u64): (u64, u64) {
        let h: |u64| u64 has copy + drop = local_digest;
        (h(x), h(x))
    }
    spec local_digest_twice { ensures result_1 == result_2; } // error: each call may differ

    // A native the prelude models exactly is a function of its arguments.
    fun apply_bytes(f: |vector<u8>| vector<u8> has copy + drop, b: vector<u8>): vector<u8> {
        f(b)
    }
    spec apply_bytes {
        pragma opaque;
        aborts_if aborts_of<f>(b);
        ensures result == result_of<f>(b);
    }

    fun prelude_native_through_hof(b: vector<u8>): (vector<u8>, vector<u8>) {
        (apply_bytes(std::hash::sha2_256, b), std::hash::sha2_256(b))
    }
    spec prelude_native_through_hof { ensures result_1 == result_2; }

    // Binding is kept for a determined target.
    fun g_byte(x: u8): u8 acquires R { if (exists<R>(@0x42) && R[@0x42].w > 7) x else x }
    spec g_byte { pragma opaque; aborts_if false; ensures result == x; }

    fun run_byte(f: |u8| u8 has copy + drop, x: u8): u8 { f(x) }
    spec run_byte {
        pragma opaque;
        reads_of<f> R;
        aborts_if aborts_of<f>(x);
        ensures result == result_of<f>(x);
    }

    fun result_of_determined_across_write(): (u8, u8) acquires R {
        let a = run_byte(g_byte, 3);
        flip();
        let b = run_byte(g_byte, 3);
        (a, b)
    }
    spec result_of_determined_across_write { ensures result_1 == result_2; }

    // A result determined over post-state memory.
    fun r_value(): u64 acquires R { R[@0x42].w }
    spec r_value { pragma opaque; aborts_if !exists<R>(@0x42); ensures result == global<R>(@0x42).w; }

    fun post_state_determined_across_write(): (u64, u64) acquires Q {
        let h: || u64 has copy + drop = r_value;
        let a = h();
        flip_q();
        let b = h();
        (a, b)
    }
    spec post_state_determined_across_write { ensures result_1 == result_2; }

    // Reads R under a `requires` on S, without aborting when it fails.
    fun r_odd_requires(): u64 acquires R { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 }
    spec r_odd_requires {
        pragma opaque;
        requires exists<S>(@0x42);
        aborts_if false;
        ensures result <= 1;
    }

    // With S absent and R.w = 0 the results are (0, 1).
    fun requires_failed_across_write(): (u64, u64) acquires R {
        let h: || u64 has copy + drop = r_odd_requires;
        let a = h();
        flip();
        let b = h();
        (a, b)
    }
    spec requires_failed_across_write {
        requires !exists<S>(@0x42);
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    // Mutually recursive targets; the member processed first had an incomplete usage summary.
    fun rec_wrap(n: u64): u64 { rec_read(n) }
    spec rec_wrap { pragma opaque; aborts_if false; }

    fun rec_read(n: u64): u64 acquires R {
        if (n == 0) { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 } else rec_wrap(n - 1)
    }
    spec rec_read { pragma opaque; aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun rec_wrap_across_write(): (u64, u64) acquires R {
        let h: |u64| u64 has copy + drop = rec_wrap;
        let a = h(1);
        flip();
        let b = h(1);
        (a, b)
    }
    spec rec_wrap_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    fun rec_read_across_write(): (u64, u64) acquires R {
        let h: |u64| u64 has copy + drop = rec_read;
        let a = h(2);
        flip();
        let b = h(2);
        (a, b)
    }
    spec rec_read_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }
}

// The recursive targets declared in the other order.
module 0x42::apply_carrier_recursion {
    struct R has key { w: u64 }

    fun rec_read(n: u64): u64 acquires R {
        if (n == 0) { if (exists<R>(@0x42)) R[@0x42].w % 2 else 0 } else rec_wrap(n - 1)
    }
    spec rec_read { pragma opaque; aborts_if false; }

    fun rec_wrap(n: u64): u64 { rec_read(n) }
    spec rec_wrap { pragma opaque; aborts_if false; }

    fun flip() acquires R {
        if (exists<R>(@0x42)) {
            let r = &mut R[@0x42];
            r.w = if (r.w % 2 == 0) 1 else 0
        }
    }
    spec flip { pragma opaque; modifies global<R>(@0x42); aborts_if false; }

    // With R.w = 0 the results are (0, 1).
    fun rec_wrap_across_write(): (u64, u64) acquires R {
        let h: |u64| u64 has copy + drop = rec_wrap;
        let a = h(1);
        flip();
        let b = h(1);
        (a, b)
    }
    spec rec_wrap_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }

    fun rec_read_across_write(): (u64, u64) acquires R {
        let h: |u64| u64 has copy + drop = rec_read;
        let a = h(2);
        flip();
        let b = h(2);
        (a, b)
    }
    spec rec_read_across_write {
        requires exists<R>(@0x42) && global<R>(@0x42).w == 0;
        ensures result_1 == result_2; // error: the second call reads the changed R
    }
}


// Natives over state no argument holds.
module 0x1::transaction_context {
    native public fun unique_id(): u64;
    spec unique_id { pragma opaque; aborts_if false; }
}

// A module named like an argument-determined one, outside the standard library.
module 0x42::hash {
    native public fun digest(data: vector<u8>): vector<u8>;
    spec digest { pragma opaque; aborts_if false; }
}

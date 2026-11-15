// Code run by a function value nothing bounds holds no signer for a fixed system address, so it
// cannot pass a check like `system_addresses::assert_aptos_framework`. A resource every write of
// which is at a system address, and which code it can run reaches only through such a check on
// a signer it supplied, is unchanged across that code. Anything weaker leaves it havoced.
//
// Each function states its required verdict. A false claim verifying here is unsound.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
    public fun assert_framework_reserved(addr: address) { assert!(addr == @0x1 || addr == @0x7, 4) }
    public fun is_aptos_framework_address(addr: address): bool { addr == @0x1 }
}

module 0x1::create_signer {
    friend 0x1::sa_minted;
    friend 0x1::sa_gov;
    friend 0x1::sa_minted_pub;
    friend 0x1::sa_past_gate;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

// (1) Written only through an asserted framework signer.
module 0x1::sa_gated {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(fw)].v = v;
    }
    public fun set_by_addr(fw: &signer, v: u64) acquires R {
        let a = signer::address_of(fw);
        system_addresses::assert_framework_reserved(a);
        R[a].v = v;
    }
    public fun value(): u64 acquires R { R[@0x1].v }
}

// (2) Also written at a user's own address.
module 0x1::sa_peruser {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun mine(user: &signer) { move_to(user, R { v: 0 }) }
    public fun bump_mine(user: &signer) acquires R { R[signer::address_of(user)].v += 1; }
}

// (3) Written through a signer nothing checks.
module 0x1::sa_unasserted {
    use std::signer;
    struct R has key { v: u64 }
    public fun set(s: &signer, v: u64) acquires R { R[signer::address_of(s)].v = v; }
}

// (4) Written through a framework signer minted where no callback can reach.
module 0x1::sa_minted {
    use 0x1::create_signer;
    struct R has key { v: u64 }
    fun genesis_like() { let fw = create_signer::create_signer(@0x1); move_to(&fw, R { v: 0 }) }
}

// (5) Written at the constant address in a friend function reached only through a check.
module 0x1::sa_const_friend {
    use 0x1::system_addresses;
    friend 0x1::sa_const_caller;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public(friend) fun tick() acquires R { R[@0x1].v += 1; }
}
module 0x1::sa_const_caller {
    use 0x1::system_addresses;
    use 0x1::sa_const_friend;
    public fun tick_gated(fw: &signer) { system_addresses::assert_aptos_framework(fw); sa_const_friend::tick() }
}

// (5b) Written at the constant address by a public function anyone may call.
module 0x1::sa_const_public {
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun add(n: u64) acquires R { R[@0x1].v += n; }
}

// (6) Declares a native.
module 0x1::sa_native {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    native fun peek(): u64;
    public fun set(fw: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(fw)].v = v;
    }
}

// The check is on one signer, the write through another.
module 0x1::sa_other_signer {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun set(fw: &signer, other: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(other)].v = v;
    }
}

// A minted framework signer passed past the writer's check.
module 0x1::sa_checked {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun set(fw: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(fw)].v = v;
    }
}
module 0x1::sa_past_gate {
    use 0x1::create_signer;
    use 0x1::sa_checked;
    public fun reset() { let fw = create_signer::create_signer(@0x1); sa_checked::set(&fw, 0) }
}

// A framework signer minted in a public function anyone may call.
module 0x1::sa_minted_pub {
    use std::signer;
    use 0x1::create_signer;
    struct R has key { v: u64 }
    public fun reset() acquires R {
        let fw = create_signer::create_signer(@0x1);
        R[signer::address_of(&fw)].v = 0;
    }
}

// Written at the constant address by an `entry` function that is not public: a transaction
// may call it, code run by a callback cannot.
module 0x1::sa_entry {
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    entry fun bump() acquires R { R[@0x1].v += 1; }
}

// Reached through a cycle of friend functions from an unchecked public function.
module 0x1::sa_cycle {
    struct R has key { v: u64 }
    public fun start(n: u64) acquires R { a(n) }
    fun a(n: u64) acquires R { if (n > 0) b(n - 1) }
    fun b(n: u64) acquires R { R[@0x1].v += 1; a(n) }
    spec a { pragma opaque; }
    spec b { pragma opaque; }
}

// Written at the constant address by a function a public closure runs.
module 0x1::sa_closure {
    struct R has key { v: u64 }
    fun w() acquires R { R[@0x1].v += 1; }
    public fun make(): || has drop { || w() }
}

// Checked inline: the address of the signer compared with a system address, or a predicate.
module 0x1::sa_inline {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { assert!(signer::address_of(fw) == @0x1, 1); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer, v: u64) acquires R {
        assert!(system_addresses::is_aptos_framework_address(signer::address_of(fw)), 1);
        R[signer::address_of(fw)].v = v;
    }
}

// Compared with an address that is not a system address.
module 0x1::sa_inline_user {
    use std::signer;
    struct R has key { v: u64 }
    public fun set(s: &signer, v: u64) acquires R {
        assert!(signer::address_of(s) == @0x60, 1);
        R[signer::address_of(s)].v = v;
    }
}

// Checked by a function of the program that checks its parameter on every path.
module 0x1::sa_checker {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    fun check(s: &signer) { system_addresses::assert_aptos_framework(s) }
    public fun init(fw: &signer) { check(fw); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer, v: u64) acquires R { check(fw); R[signer::address_of(fw)].v = v; }
}

// Checked on some paths only.
module 0x1::sa_maybe {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    fun maybe_check(s: &signer, b: bool) { if (b) system_addresses::assert_aptos_framework(s) }
    public fun set(s: &signer, b: bool, v: u64) acquires R { maybe_check(s, b); R[signer::address_of(s)].v = v; }
}

// A framework signer obtained from a function that checks the caller's signer first, or
// from one that does not.
module 0x1::sa_gov {
    use 0x1::create_signer;
    friend 0x1::sa_gov_unchecked;
    use 0x1::system_addresses;
    public fun get_checked(core: &signer): signer {
        system_addresses::assert_aptos_framework(core);
        create_signer::create_signer(@0x1)
    }
    public(friend) fun get_unchecked(): signer { create_signer::create_signer(@0x1) }
}
module 0x1::sa_gov_checked {
    use std::signer;
    use 0x1::system_addresses;
    use 0x1::sa_gov;
    struct R has key { v: u64 }
    public fun set(fw: &signer) acquires R { system_addresses::assert_aptos_framework(fw); R[signer::address_of(fw)].v += 1; }
    public fun force(core: &signer) acquires R { let fw = sa_gov::get_checked(core); set(&fw) }
}
module 0x1::sa_gov_unchecked {
    use std::signer;
    use 0x1::system_addresses;
    use 0x1::sa_gov;
    struct R has key { v: u64 }
    public fun set(fw: &signer) acquires R { system_addresses::assert_aptos_framework(fw); R[signer::address_of(fw)].v += 1; }
    public fun force() acquires R { let fw = sa_gov::get_unchecked(); set(&fw) }
}

// Checked by the existence of a capability only ever published at a system address, or of one
// anyone may publish.
module 0x1::sa_cap {
    use std::signer;
    use 0x1::system_addresses;
    struct Cap has key {}
    struct Open has key {}
    struct R has key { v: u64 }
    struct S has key { v: u64 }
    public fun init(fw: &signer) {
        system_addresses::assert_aptos_framework(fw);
        move_to(fw, Cap {});
        move_to(fw, R { v: 0 });
        move_to(fw, S { v: 0 });
    }
    public fun grant_open(user: &signer) { move_to(user, Open {}) }
    public fun set(s: &signer, v: u64) acquires R {
        assert!(exists<Cap>(signer::address_of(s)), 1);
        R[@0x1].v = v;
    }
    public fun set_open(s: &signer, v: u64) acquires S {
        assert!(exists<Open>(signer::address_of(s)), 1);
        S[@0x1].v = v;
    }
}

module 0x60::user {
    use 0x1::sa_gated;
    use 0x1::sa_peruser;
    use 0x1::sa_unasserted;
    use 0x1::sa_minted;
    use 0x1::sa_const_friend;
    use 0x1::sa_const_public;
    use 0x1::sa_native;
    use 0x1::sa_other_signer;
    use 0x1::sa_checked;
    use 0x1::sa_minted_pub;
    use 0x1::sa_entry;
    use 0x1::sa_cycle;
    use 0x1::sa_closure;
    use 0x1::sa_inline;
    use 0x1::sa_inline_user;
    use 0x1::sa_checker;
    use 0x1::sa_maybe;
    use 0x1::sa_gov_checked;
    use 0x1::sa_gov_unchecked;
    use 0x1::sa_cap;

    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // Runs a function value read from storage, opaque.
    struct Hook has key { f: |u64| u64 has copy + drop + store }
    fun run_hook(x: u64): u64 acquires Hook { let f = Hook[@0x60].f; f(x) }
    spec run_hook { pragma opaque; }

    // (1) Unchanged across an invocation. Verifies.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }

    // (1) Same, across an opaque call that runs a stored function value. Verifies.
    public fun gated_dispatch(x: u64): u64 acquires Hook { run_hook(x) }
    spec gated_dispatch {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }

    // (1) Same, at the head of a loop invoking the field. Verifies.
    public fun gated_loop(c: &Cb, n: u64): u64 {
        let v0 = sa_gated::value();
        let i = 0;
        let s = 0;
        while (i < n) {
            s = (c.f)(i);
            i = i + 1;
        } spec {
            invariant global<sa_gated::R>(@0x1).v == v0;
        };
        s
    }
    spec gated_loop {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }

    // Vacuity control. Rejected.
    public fun gated_false(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_false {
        requires exists<sa_gated::R>(@0x1);
        ensures false;
    }

    // The resource's own writer still writes it. Rejected.
    public fun gated_own_write(fw: &signer, c: &Cb, x: u64): u64 {
        sa_gated::set(fw, x);
        (c.f)(x)
    }
    spec gated_own_write {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }

    // (2) The callback may bump its own instance. Rejected.
    public fun peruser_any(c: &Cb, x: u64, a: address): u64 { (c.f)(x) }
    spec peruser_any {
        requires exists<sa_peruser::R>(a);
        ensures global<sa_peruser::R>(a) == old(global<sa_peruser::R>(a));
    }

    // (2) Havoced as a whole, so the framework instance is not kept either. Rejected.
    public fun peruser_fw(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec peruser_fw {
        requires exists<sa_peruser::R>(@0x1);
        ensures global<sa_peruser::R>(@0x1) == old(global<sa_peruser::R>(@0x1));
    }

    // (3) The callback may write through its own signer. Rejected.
    public fun unasserted_any(c: &Cb, x: u64, a: address): u64 { (c.f)(x) }
    spec unasserted_any {
        requires exists<sa_unasserted::R>(a);
        ensures global<sa_unasserted::R>(a) == old(global<sa_unasserted::R>(a));
    }

    // (3) Havoced. Rejected.
    public fun unasserted_fw(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec unasserted_fw {
        requires exists<sa_unasserted::R>(@0x1);
        ensures global<sa_unasserted::R>(@0x1) == old(global<sa_unasserted::R>(@0x1));
    }

    // (4) Minted framework signer in a function no callback reaches. Verifies.
    public fun minted_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec minted_invoke {
        requires exists<sa_minted::R>(@0x1);
        ensures global<sa_minted::R>(@0x1) == old(global<sa_minted::R>(@0x1));
    }

    // (5) Constant address, reached only through a check. Verifies.
    public fun const_friend_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec const_friend_invoke {
        requires exists<sa_const_friend::R>(@0x1);
        ensures global<sa_const_friend::R>(@0x1) == old(global<sa_const_friend::R>(@0x1));
    }

    // (5b) The callback may call `sa_const_public::add`. Rejected.
    public fun const_public_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec const_public_invoke {
        requires exists<sa_const_public::R>(@0x1);
        ensures global<sa_const_public::R>(@0x1) == old(global<sa_const_public::R>(@0x1));
    }

    // (6) A module with a native. Rejected.
    public fun native_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec native_invoke {
        requires exists<sa_native::R>(@0x1);
        ensures global<sa_native::R>(@0x1) == old(global<sa_native::R>(@0x1));
    }

    // The check protects another signer than the one written through. Rejected.
    public fun other_signer_any(c: &Cb, x: u64, a: address): u64 { (c.f)(x) }
    spec other_signer_any {
        requires exists<sa_other_signer::R>(a);
        ensures global<sa_other_signer::R>(a) == old(global<sa_other_signer::R>(a));
    }

    // The callback may call `sa_past_gate::reset`, which passes the check. Rejected.
    public fun past_gate_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec past_gate_invoke {
        requires exists<sa_checked::R>(@0x1);
        ensures global<sa_checked::R>(@0x1) == old(global<sa_checked::R>(@0x1));
    }

    // The callback may call `sa_minted_pub::reset`. Rejected.
    public fun minted_pub_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec minted_pub_invoke {
        requires exists<sa_minted_pub::R>(@0x1);
        ensures global<sa_minted_pub::R>(@0x1) == old(global<sa_minted_pub::R>(@0x1));
    }

    // Only a transaction may call `sa_entry::bump`. Verifies.
    public fun entry_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec entry_invoke {
        requires exists<sa_entry::R>(@0x1);
        ensures global<sa_entry::R>(@0x1) == old(global<sa_entry::R>(@0x1));
    }

    // The callback may call `sa_cycle::start`. Rejected.
    public fun cycle_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec cycle_invoke {
        requires exists<sa_cycle::R>(@0x1);
        ensures global<sa_cycle::R>(@0x1) == old(global<sa_cycle::R>(@0x1));
    }

    // The callback may run the closure `sa_closure::make` returns. Rejected.
    public fun closure_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec closure_invoke {
        requires exists<sa_closure::R>(@0x1);
        ensures global<sa_closure::R>(@0x1) == old(global<sa_closure::R>(@0x1));
    }

    // Checked inline. Verifies.
    public fun inline_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec inline_invoke {
        requires exists<sa_inline::R>(@0x1);
        ensures global<sa_inline::R>(@0x1) == old(global<sa_inline::R>(@0x1));
    }

    // Compared with a user address: the callback may hold that signer. Rejected.
    public fun inline_user_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec inline_user_invoke {
        requires exists<sa_inline_user::R>(@0x60);
        ensures global<sa_inline_user::R>(@0x60) == old(global<sa_inline_user::R>(@0x60));
    }

    // Checked by a checking function. Verifies.
    public fun checker_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec checker_invoke {
        requires exists<sa_checker::R>(@0x1);
        ensures global<sa_checker::R>(@0x1) == old(global<sa_checker::R>(@0x1));
    }

    // The callback may call `sa_maybe::set(own, false, ..)`. Rejected.
    public fun maybe_any(c: &Cb, x: u64, a: address): u64 { (c.f)(x) }
    spec maybe_any {
        requires exists<sa_maybe::R>(a);
        ensures global<sa_maybe::R>(a) == old(global<sa_maybe::R>(a));
    }

    // The framework signer is obtained only past a check on the caller's. Verifies.
    public fun gov_checked_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gov_checked_invoke {
        requires exists<sa_gov_checked::R>(@0x1);
        ensures global<sa_gov_checked::R>(@0x1) == old(global<sa_gov_checked::R>(@0x1));
    }

    // The callback may call `sa_gov_unchecked::force`. Rejected.
    public fun gov_unchecked_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gov_unchecked_invoke {
        requires exists<sa_gov_unchecked::R>(@0x1);
        ensures global<sa_gov_unchecked::R>(@0x1) == old(global<sa_gov_unchecked::R>(@0x1));
    }

    // Checked by the existence of a capability only at system addresses. Verifies.
    public fun cap_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec cap_invoke {
        requires exists<sa_cap::R>(@0x1);
        ensures global<sa_cap::R>(@0x1) == old(global<sa_cap::R>(@0x1));
    }

    // The callback may publish `Open` for itself, then call `sa_cap::set_open`. Rejected.
    public fun cap_open_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec cap_open_invoke {
        requires exists<sa_cap::S>(@0x1);
        ensures global<sa_cap::S>(@0x1) == old(global<sa_cap::S>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean

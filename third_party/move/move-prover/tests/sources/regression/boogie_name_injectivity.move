// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Generated Boogie names must be injective.
//
// `0x42::a::b_c` and `0x42::a_b::c` are two different Move functions, and
// `0x42::m::A_B` and `0x42::m_A::B` are two different resource types. While the
// address, module and entity parts of a Boogie name were joined with a bare `_`
// -- a character a Move identifier may contain -- each pair rendered to one name.
// The emission dedups were keyed on that rendered name, so the second entity was
// dropped and the surviving declaration served both: one procedure for two
// functions, one `$Memory` variable for two resource types.
//
// Both directions of the resulting unsoundness are pinned. `sum` is a true
// specification that was unprovable, and `confuse` is a false one that was
// proved. `no_collision_control` is the same false claim against a module whose
// name does not collide; it must fail both before and after, so that `confuse`
// failing is not merely everything failing.

module 0x42::a {
    public fun b_c(): u64 { 1 }
}

module 0x42::a_b {
    public fun c(): u64 { 2 }
}

module 0x42::distinct_procedures {
    use 0x42::a;
    use 0x42::a_b;

    /// Must verify: `1 + 2`. Unprovable while one procedure serves both
    /// functions, because it can only return one of the two values.
    public fun sum(): u64 {
        a::b_c() + a_b::c()
    }

    spec sum {
        ensures result == 3;
    }
}

module 0x42::m {
    struct A_B has key, drop { f: u64 }

    public fun has_it(a: address): bool {
        exists<A_B>(a)
    }
}

module 0x42::m_A {
    struct B has key, drop { f: u64 }

    public fun put(s: &signer) {
        move_to(s, B { f: 1 })
    }
}

// Control: same shape, but this module name does not collide with a struct of
// `0x42::m_A`.
module 0x42::mq {
    struct B has key, drop { f: u64 }

    public fun has_it(a: address): bool {
        exists<B>(a)
    }
}

module 0x42::distinct_memories {
    use std::signer;
    use 0x42::m;
    use 0x42::m_A;
    use 0x42::mq;

    /// Must fail: `put` publishes `0x42::m_A::B`, so nothing of type
    /// `0x42::m::A_B` exists at that address. Provable while both resource types
    /// share one datatype, one `$Memory` variable and one `_$memory_$id`.
    public fun confuse(s: &signer): bool {
        m_A::put(s);
        m::has_it(signer::address_of(s))
    }

    spec confuse {
        ensures result == true;
    }

    /// Must fail, before and after.
    public fun no_collision_control(s: &signer): bool {
        m_A::put(s);
        mq::has_it(signer::address_of(s))
    }

    spec no_collision_control {
        ensures result == true;
    }
}

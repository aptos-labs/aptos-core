// Copyright © Aptos Foundation
// Data invariants over behavioral predicates must be state-quantified: the
// free/ambient-state form is rejected, also when hidden behind a spec-fun
// wrapper (a state binder does not distribute into called spec functions).
module 0x42::data_inv_free_state {
    use std::signer;

    struct Counter has key { v: u64 }

    struct A has key, drop {
        f: |&signer| has copy + store + drop,
    }
    spec A {
        modifies_of<f>(s: signer) Counter[signer::address_of(s)];
        invariant forall s: signer: !aborts_of<f>(s); // error: free state
    }

    struct B has key, drop {
        g: |&signer| has copy + store + drop,
    }
    spec B {
        modifies_of<g>(s: signer) Counter[signer::address_of(s)];
        invariant forall s: signer: wraps(g, s); // error: free state behind a wrapper
    }

    spec fun wraps(h: |&signer| has copy + store + drop, s: signer): bool {
        !aborts_of<h>(s)
    }

    struct C has key, drop {
        h: |&signer| has copy + store + drop,
    }
    spec C {
        modifies_of<h>(s: signer) Counter[signer::address_of(s)];
        // An existential state binder needs only a witness state, so the
        // invariant stays state-dependent.
        invariant exists S in *: forall s: signer: (S |~ !aborts_of<h>(s)); // error: exists over states
    }
}

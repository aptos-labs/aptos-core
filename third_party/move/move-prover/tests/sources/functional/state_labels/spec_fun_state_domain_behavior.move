// A state-domain label inside a spec function must reach the memory arguments
// of a behavioral predicate. If the label is discarded, the quantified claim
// below collapses to the caller's current state and the false postcondition can
// be proved from its precondition.
module 0x42::spec_fun_state_domain_behavior {
    struct Counter has key { value: u64 }

    fun read(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec read {
        pragma opaque;
        aborts_if !exists<Counter>(addr);
        ensures result == Counter[addr].value;
    }

    spec fun readable_in_every_state(addr: address): bool {
        forall S in *: S |~ !aborts_of<read>(addr)
    }

    fun expose_false_claim(_addr: address) {}
    spec expose_false_claim {
        requires exists<Counter>(_addr);
        ensures readable_in_every_state(_addr); // error: Counter need not exist in every state
    }
}

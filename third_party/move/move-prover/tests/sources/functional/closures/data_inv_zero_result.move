// Copyright © Aptos Foundation
// A state-quantified `aborts_of` data invariant over a stored function value
// with no outputs (`|&signer|`) and a `modifies_of` memory footprint: the
// invariant discharges the stored call's abort check at a use site even after
// the declared memory has moved, and packing a function that can abort fails
// the invariant.
module 0x42::data_inv_zero_result {
    use std::signer;

    struct Counter has key { v: u64 }

    struct Mod has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec Mod {
        modifies_of<f>(s: signer) Counter[signer::address_of(s)];
        invariant forall S in *, s: signer: S |~ !aborts_of<f>(s);
    }

    #[persistent]
    fun safe_inc(s: &signer) {
        let addr = signer::address_of(s);
        if (exists<Counter>(addr) && Counter[addr].v < 18446744073709551615) {
            Counter[addr].v = Counter[addr].v + 1;
        }
    }
    spec safe_inc {
        pragma opaque;
        modifies Counter[signer::address_of(s)];
        aborts_if false;
    }

    #[persistent]
    fun unsafe_inc(s: &signer) {
        let addr = signer::address_of(s);
        Counter[addr].v = Counter[addr].v + 1;
    }
    spec unsafe_inc {
        pragma opaque;
        modifies Counter[signer::address_of(s)];
        aborts_if !exists<Counter>(signer::address_of(s));
        aborts_if Counter[signer::address_of(s)].v + 1 > MAX_U64;
    }

    public fun make(owner: &signer) {
        move_to(owner, Mod { f: safe_inc });
    }

    public fun make_bad(owner: &signer) {
        move_to(owner, Mod { f: unsafe_inc }); // error: data invariant does not hold
    }

    public fun use_mod(modifier_addr: address, s: &signer) acquires Mod {
        let m = &Mod[modifier_addr];
        (m.f)(s);
    }
    spec use_mod {
        requires exists<Mod>(modifier_addr);
        aborts_if !exists<Mod>(modifier_addr);
    }
}

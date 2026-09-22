module 0x42::deferred_modifies_check {
    struct R has key {
        value: u64,
    }

    public fun initialize(account: &signer) {
        move_to(account, R { value: 0 });
    }
    spec initialize(account: &signer) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies R[signer::address_of(account)];
    }


    public fun write_value(addr: address, value: u64) acquires R {
        borrow_global_mut<R>(addr).value = value;
    }
    spec write_value(addr: address, value: u64) {
        pragma opaque = true;
        modifies R[addr];
        ensures [inferred] update<R>(addr, update_field(old(R[addr]), value, value));
        aborts_if [inferred] !exists<R>(addr);
    }


    public fun update_from_caller(addr: address, value: u64) acquires R {
        write_value(addr, value);
    }

    spec update_from_caller {
        modifies global<R>(addr);
        pragma opaque = true;
        ensures [inferred] ensures_of<write_value>(addr, value);
        aborts_if [inferred] aborts_of<write_value>(addr, value);
    }
}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `deferred_modifies_check::initialize` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition would have introduced a new module dependency
  ┌─ tests/inference/deferred_modifies_check.move:6:5
  │
6 │ ╭     public fun initialize(account: &signer) {
7 │ │         move_to(account, R { value: 0 });
8 │ │     }
  │ ╰─────^

Verification: Succeeded.
*/

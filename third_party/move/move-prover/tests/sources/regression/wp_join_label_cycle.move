// A closure, forwarded through an inline wrapper to a function taking a
// function value, creates a value on one branch and writes to it after the
// join. No call in the closure touches global memory. Spec inference used to
// name a new memory state at each call, and at the join it merged the
// creating branch's state with the entry state under one label: a cyclic
// state definition, reported as a bug. The one remaining error is separate:
// the closure's inferred spec does not carry the option's length bound back to
// the call in `apply`.
module 0x42::wp_join_label_cycle {
    use std::option::{Self, Option};
    use std::vector;

    struct State has store, drop {
        ids: Option<vector<u64>>,
        n: u64
    }

    fun apply(st: &mut State, f: |&mut State| bool): bool {
        f(st)
    }

    inline fun modify(st: &mut State, f: |&mut State| bool): bool {
        apply(st, |v| f(v))
    }

    fun fresh(seed: &mut u64): vector<u64> {
        *seed = *seed + 1;
        vector[]
    }

    fun push(ids: &mut vector<u64>, id: u64) {
        vector::push_back(ids, id);
    }

    fun track(st: &mut State, id: u64): bool {
        modify(st, |s| {
            if (option::is_none(&s.ids)) {
                option::fill(&mut s.ids, fresh(&mut s.n));
            };
            push(option::borrow_mut(&mut s.ids), id);
            true
        })
    }
}

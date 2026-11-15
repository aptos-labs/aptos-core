// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A unit declaring a spec variable is verified through its model. A caller
// there sees an opaque callee as any caller does: its `[abstract]` clauses and
// not its `[concrete]` ones.
module 0x42::interface_view_model {
    spec module {
        global calls: u64;
    }

    spec fun chosen(x: u64): u64;

    fun pick(x: u64): u64 { x }
    spec pick {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == chosen(x);
        ensures [concrete] result == x;
    }

    fun abstract_seen(x: u64): u64 { pick(x) }
    spec abstract_seen {
        aborts_if false;
        ensures result == chosen(x);
    }

    fun concrete_unseen(x: u64): u64 { pick(x) }
    spec concrete_unseen {
        ensures result == x; // error: callers do not see a `[concrete]` clause
    }

    fun other_unseen(x: u64): u64 { pick(x) }
    spec other_unseen {
        ensures result == chosen(x + 1); // error: not what the callee states
    }
}

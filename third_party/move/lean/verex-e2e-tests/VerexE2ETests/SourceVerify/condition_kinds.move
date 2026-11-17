// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// One expression owes conditions of more than one kind: a mutable borrow of a
// resource owes the module's invariants where its write of memory ends, and
// the resource's data invariant where its mutation ends, both at its site.
// Each marker reads the condition of its own kind there, so a violation of
// either is still rejected and neither predicate stands for the other.
//
// `Config`'s invariant is part of its carrier, so the certifying cut owns it
// and its mutation end owes nothing; `Pricing` holds a function value, so its
// invariant is not carried and the mutation end owes it. The module's last
// invariant reads `Pricing`'s memory, so the write of that borrow owes a
// condition of its own at the same site.
module 0x42::condition_kinds {
    struct Flag has key {}
    struct Config has key { interval: u64 }
    spec Config { invariant interval > 0; }
    struct Pricing has key { rate: u64, price: |u64|u64 has copy + drop + store }
    spec Pricing { invariant rate > 0; }
    spec module {
        invariant exists<Flag>(@0x42) ==> exists<Config>(@0x42);
        invariant exists<Config>(@0x42) ==> global<Config>(@0x42).interval >= 2;
        invariant exists<Pricing>(@0x42) ==> exists<Config>(@0x42);
    }

    // Both the module's invariants and the data invariant hold where the loan
    // of `Config` ends.
    fun set_interval(interval: u64) acquires Config {
        let config = &mut Config[@0x42];
        config.interval = interval;
    }
    spec set_interval {
        requires interval >= 2;
        aborts_if !exists<Config>(@0x42);
        ensures Config[@0x42].interval == interval;
    }

    // The module's invariant is still checked where the write of a mutable
    // borrow ends, although the data invariant holds.
    fun drop_interval() acquires Config {
        let config = &mut Config[@0x42];
        config.interval = 1; // error: the module invariant fails after the write
    }
    spec drop_interval {
        pragma aborts_if_is_partial;
    }

    // The data invariant of a carried declaration is owed at the certifying
    // cut, where its mutation ends owes nothing.
    fun clear_interval() acquires Config {
        let config = &mut Config[@0x42];
        config.interval = 0; // error: the data invariant fails
    }
    spec clear_interval {
        pragma aborts_if_is_partial;
    }

    // The borrow of `Pricing` owes conditions of both kinds; both hold.
    fun set_rate(rate: u64) acquires Pricing {
        let pricing = &mut Pricing[@0x42];
        pricing.rate = rate;
    }
    spec set_rate {
        requires rate > 0;
        aborts_if !exists<Pricing>(@0x42);
    }

    // The data invariant owed where the mutation of `Pricing` ends, at the
    // site that owes the module's invariant too.
    fun clear_rate() acquires Pricing {
        let pricing = &mut Pricing[@0x42];
        pricing.rate = 0; // error: the data invariant fails where the mutation ends
    }
    spec clear_rate {
        pragma aborts_if_is_partial;
    }

    // A publication owes the module's invariants where its write ends, and no
    // data invariant.
    fun publish_flag(owner: &signer) {
        move_to(owner, Flag {}); // error: the module invariant fails after the write
    }
    spec publish_flag {
        pragma aborts_if_is_partial;
    }
}

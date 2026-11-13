// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Where a read takes the data invariant of the value it reads. Modular
// verification gives it of the memory the function is entered with; a read
// later in the body is of a memory a write or a call has made, and the
// invariant has to reach it from there. Each read is of a value the body
// did not write, so the invariant cannot come from the write's own
// obligation: another resource, or the written resource at another key.
module 0x42::data_invariant_reads {
    struct Tally has key { count: u64 }
    spec Tally {
        invariant count >= 10;
    }

    struct Other has key { n: u64 }
    spec Other {
        invariant n >= 10;
    }

    public fun read_at_entry(addr: address): u64 {
        Other[addr].n
    }
    spec read_at_entry {
        pragma aborts_if_is_partial;
        ensures result >= 10;
    }

    public fun read_after_write(addr: address): u64 {
        let t = &mut Tally[addr];
        t.count = t.count + 1;
        Other[addr].n
    }
    spec read_after_write {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
        ensures result >= 10;
    }

    public fun read_after_publish(account: &signer, addr: address): u64 {
        move_to(account, Tally { count: 10 });
        Other[addr].n
    }
    spec read_after_publish {
        pragma aborts_if_is_partial;
        modifies global<Tally>(std::signer::address_of(account));
        ensures result >= 10;
    }

    fun bump(addr: address) {
        let t = &mut Tally[addr];
        t.count = t.count + 1;
    }
    spec bump {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
    }

    public fun read_after_call(addr: address): u64 {
        bump(addr);
        Other[addr].n
    }
    spec read_after_call {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
        ensures result >= 10;
    }

    // A read of the resource that was written, at another key: the write's
    // own obligation says nothing of this value, and no join stands between
    // the write and the read.
    public fun read_other_key(addr: address, other: address): u64 {
        let t = &mut Tally[addr];
        t.count = t.count + 1;
        Tally[other].count
    }
    spec read_other_key {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
        ensures result >= 10;
    }

    // The invariant is all a read knows of the value: it does not bound it
    // any further. The negative control of the cases above.
    public fun read_after_write_false(addr: address): u64 {
        let t = &mut Tally[addr];
        t.count = t.count + 1;
        Other[addr].n
    }
    spec read_after_write_false {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
        ensures result >= 11; // error: the invariant gives only `n >= 10`
    }
}

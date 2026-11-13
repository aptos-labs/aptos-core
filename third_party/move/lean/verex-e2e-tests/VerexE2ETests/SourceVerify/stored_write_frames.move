// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::stored_write_frames {
    struct Counter<phantom T> has key { value: u64 }
    struct Config has key { value: u64 }
    struct Stored has copy, drop, store {
        action: |address, u64|u64 has copy+drop+store,
    }
    spec Stored {
        modifies_of<action>(owner: address, value: u64) Counter<bool>[owner];
    }

    #[persistent]
    fun write(owner: address, value: u64): u64 {
        Counter<bool>[owner].value = value;
        value
    }
    spec write {
        pragma opaque;
        modifies Counter<bool>[owner];
        aborts_if !exists<Counter<bool>>(owner);
        ensures result == value;
    }

    fun make(): Stored { Stored { action: write } }
    spec make { pragma opaque; aborts_if false; }

    fun call(stored: Stored, owner: address, value: u64): u64 {
        (stored.action)(owner, value)
    }
    spec call {
        pragma aborts_if_is_partial;
        modifies Counter<bool>[owner];
        ensures Config[owner] == old(Config[owner]);
    }

    fun via_opaque(owner: address, value: u64): u64 {
        call(make(), owner, value)
    }
    spec via_opaque {
        pragma aborts_if_is_partial;
        modifies Counter<bool>[owner];
        ensures Counter<u64>[owner] == old(Counter<u64>[owner]);
    }

    // A different generic resource instantiation is outside the frame.
    struct OtherStored has copy, drop, store {
        action: |address, u64|u64 has copy+drop+store,
    }
    spec OtherStored {
        modifies_of<action>(owner: address, value: u64) Counter<u64>[owner];
    }
    fun invalid(stored: Stored): OtherStored { OtherStored { action: stored.action } }

    struct AnyWriter has copy, drop, store {
        action: |address, u64|u64 has copy+drop+store,
    }
    spec AnyWriter { modifies_of<action> *; }
    fun make_any(): AnyWriter { AnyWriter { action: write } }
    fun call_any(stored: AnyWriter, owner: address, value: u64): u64 {
        (stored.action)(owner, value)
    }
    spec call_any { pragma aborts_if_is_partial; }

    // A wildcard frame does not guarantee that Config stays unchanged.
    fun wrong_frame(stored: AnyWriter, owner: address, value: u64): u64 {
        (stored.action)(owner, value)
    }
    spec wrong_frame {
        pragma aborts_if_is_partial;
        ensures Config[owner] == old(Config[owner]);
    }

    // An addressed frame permits changing this particular Counter.
    fun wrong_slot(stored: Stored, owner: address, value: u64): u64 {
        (stored.action)(owner, value)
    }
    spec wrong_slot {
        pragma aborts_if_is_partial;
        modifies Counter<bool>[owner];
        ensures Counter<bool>[owner] == old(Counter<bool>[owner]);
    }
}

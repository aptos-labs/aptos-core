// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::generic_enum_invariants {
    enum Tagged<T: copy + drop> has copy, drop {
        Empty,
        Filled { value: T, tag: u64 },
    }
    spec Tagged {
        invariant match (self) {
            Tagged::Empty => true,
            Tagged::Filled { value: _, tag } => tag > 0,
        };
    }

    fun make<T: copy + drop>(value: T, tag: u64): Tagged<T> {
        Tagged::Filled { value, tag }
    }
    spec make {
        pragma opaque;
        requires tag > 0;
        aborts_if false;
        ensures result is Tagged::Filled;
        ensures result.tag == tag;
        ensures result.value == value;
    }

    fun empty<T: copy + drop>(): Tagged<T> { Tagged::Empty }

    fun read<T: copy + drop>(value: Tagged<T>): u64 {
        match (value) {
            Tagged::Empty => 0,
            Tagged::Filled { value: _, tag } => tag,
        }
    }
    spec read {
        aborts_if false;
        ensures (value is Tagged::Filled) ==> result > 0;
    }

    fun caller(): Tagged<bool> { make(true, 7) }
    spec caller {
        aborts_if false;
        ensures result is Tagged::Filled;
        ensures result.tag == 7;
        ensures result.value;
    }

    // Deliberate invariant violation, including at an open type parameter.
    fun invalid<T: copy + drop>(value: T): Tagged<T> {
        Tagged::Filled { value, tag: 0 }
    }

    fun wrong_read<T: copy + drop>(value: Tagged<T>): u64 {
        match (value) {
            Tagged::Empty => 0,
            Tagged::Filled { value: _, tag } => tag,
        }
    }
    spec wrong_read {
        ensures result == 0;
    }
}

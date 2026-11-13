// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::function_wrappers {
    struct Identity<T: copy + drop>(|T|T) has copy, drop;

    fun identity(value: u64): u64 { value }
    spec identity { aborts_if false; ensures result == value; }

    fun make(): Identity<u64> { identity }

    fun invoke(wrapped: Identity<u64>, value: u64): u64 {
        wrapped(value)
    }
    spec invoke { pragma aborts_if_is_partial; }

    struct Zero(|u64|u64) has copy, drop;
    spec Zero {
        invariant forall value: u64: !aborts_of<self.0>(value);
        invariant forall value: u64, result: u64:
            ensures_of<self.0>(value, result) ==> result == 0;
    }

    fun zero(_value: u64): u64 { 0 }
    spec zero { aborts_if false; ensures result == 0; }

    fun make_zero(): Zero { zero }
    spec make_zero { pragma opaque; aborts_if false; }

    fun call_zero(wrapped: Zero, value: u64): u64 { wrapped(value) }
    spec call_zero { aborts_if false; ensures result == 0; }

    fun via_opaque(value: u64): u64 { call_zero(make_zero(), value) }
    spec via_opaque { aborts_if false; ensures result == 0; }

    struct Any(|u64|u64) has copy, drop;
    spec Any { modifies_of<self.0> *; }
    fun make_any(): Any { zero }

    // Implicit packing must check the wrapper's invariant too.
    fun invalid(): Zero { identity }
}

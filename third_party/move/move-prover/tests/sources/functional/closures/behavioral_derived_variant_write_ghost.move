// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A write to a variant field of an enum with a ghost field is not derived,
// so the write is not lost from the contract.
module 0x42::behavioral_derived_variant_write_ghost {
    enum S has copy, drop, store { V1 { v: u64 } }
    spec S { ghost g: num; }

    public fun apply_to(s: &mut S, f: |&mut S|) { f(s) }
    spec apply_to {
        pragma opaque;
        requires requires_of<f>(s);
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }

    public fun reset(x: &mut S) { x.v = 0 }

    fun caller_incorrect(s: &mut S) { apply_to(s, reset) }
    spec caller_incorrect { ensures s.v == old(s.v); }
}

// exclude_for: lean
// The Lean backend does not verify the higher-order function `apply_to` itself and does not
// interpret function-value calls across modules, with or without the variant writes.

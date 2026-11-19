// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4
// exclude_for: lean
// The Lean backend does not check the type of a split expression and accepts this program.

// A split on a receiver call is checked with the call's resolved type.
module 0x42::proof_receiver_call_split_err {
    struct S has copy, drop { f: u8 }

    fun get_f(self: &S): u8 { self.f }

    fun split_on_u8(s: &S): u8 { s.f }
    spec split_on_u8 {
        ensures result == s.f;
    } proof {
        split s.get_f();
    }
}

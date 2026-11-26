// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// A receiver-style call returning a function value with more abilities than the schema
// parameter asks for is accepted, as the function call form is.
module 0x42::schema_include_receiver_fun {
    struct S has copy, drop { f: u64 }

    fun adder(self: &S): |u64| u64 has copy + drop { |x| x + 1 }

    spec schema TakesFun {
        f: |u64| u64 has drop;
        ensures true;
    }

    fun receiver_form(s: &S): u64 { s.f }
    spec receiver_form {
        include TakesFun { f: s.adder() };
        ensures result == s.f;
    }

    fun call_form(s: &S): u64 { s.f }
    spec call_form {
        include TakesFun { f: adder(s) };
        ensures result == s.f;
    }

    fun receiver_form_incorrect(s: &S): u64 { s.f }
    spec receiver_form_incorrect {
        include TakesFun { f: s.adder() };
        ensures result == s.f + 1;
    }
}

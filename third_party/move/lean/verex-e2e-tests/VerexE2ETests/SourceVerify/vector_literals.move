// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Vector literals reach the denotation: in a body, a constant, and a contract.
module 0x42::vector_literals {
    const NAME: vector<u8> = b"abc";

    fun name(): vector<u8> { NAME }
    spec name {
        aborts_if false;
        ensures result == b"abc";
        ensures len(result) == 3;
    }

    fun bytes(): vector<u8> { b"abc" }
    spec bytes {
        aborts_if false;
        ensures result == b"abc";
        ensures result[1] == 98;
    }

    fun first(): u8 {
        let v = vector[7u8, 8u8];
        v[0]
    }
    spec first {
        aborts_if false;
        ensures result == 7;
    }

    fun nested(): vector<vector<u64>> {
        vector[vector[1u64], vector[]]
    }
    spec nested {
        aborts_if false;
        ensures len(result) == 2;
        ensures result[0][0] == 1;
    }

    fun bytes_wrong(): vector<u8> { b"abc" }
    spec bytes_wrong {
        aborts_if false;
        ensures result == b"abd"; // error: another literal
    }

    fun in_spec(x: vector<u8>): bool { x == b"xy" }
    spec in_spec {
        aborts_if false;
        ensures result == (x == b"xy");
    }
}

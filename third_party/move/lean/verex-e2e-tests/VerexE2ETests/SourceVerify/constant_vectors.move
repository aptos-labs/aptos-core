// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::constant_vectors {
    const BYTES: vector<u8> = vector[11, 22];
    const ADDRESSES: vector<address> = vector[@0x1, @0x2];
    const FLAGS: vector<bool> = vector[true, false];

    fun correct() {
        spec {
            assert BYTES[0] == 11 && BYTES[1] == 22;
            assert ADDRESSES[1] == @0x2 && FLAGS[0] && !FLAGS[1];
        };
    }

    fun read(): u8 { BYTES[1] }
    spec read { ensures result == 22; aborts_if false; }

    fun wrong_byte() { spec { assert BYTES[0] == 22; }; }
    fun wrong_address() { spec { assert ADDRESSES[0] == @0x2; }; }
}

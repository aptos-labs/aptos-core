// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::bool_guards {
    fun guarded(_flag: bool, x: u64): u64 {
        x
    }
    spec guarded {
        requires _flag ==> x == 1;
        requires !_flag ==> x == 2;
        ensures result >= 1;
    }

    fun two_flags(_a: bool, _b: bool, x: u64): u64 {
        x
    }
    spec two_flags {
        requires _a ==> x == 1;
        requires !_a && _b ==> x == 2;
        requires !_a && !_b ==> x == 3;
        ensures result >= 1 && result <= 3;
    }

    fun guarded_false(_flag: bool, x: u64): u64 {
        x
    }
    spec guarded_false {
        requires _flag ==> x == 1;
        requires !_flag ==> x == 2;
        ensures result >= 2; // error: `_flag` true gives 1
    }

    fun unguarded_false(_flag: bool, x: u64): u64 {
        x
    }
    spec unguarded_false {
        requires _flag ==> x == 1;
        ensures result == 1; // error: `_flag` false leaves x free
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::returned_reference_freeze {
    struct Pair<T> has copy, drop { left: T, right: T }

    fun split<T>(pair: &mut Pair<T>): (&mut T, &mut T) {
        (&mut pair.left, &mut pair.right)
    }

    fun mixed(pair: &mut Pair<u64>): (&u64, &mut u64) {
        split(pair)
    }

    fun shared(pair: &mut Pair<u64>): (&u64, &u64) {
        split(pair)
    }

    fun left(pair: &mut Pair<u64>): &mut u64 { &mut pair.left }
    fun shared_left(pair: &mut Pair<u64>): &u64 { left(pair) }

    fun specified_pair(pair: &mut Pair<u64>): (&mut u64, &mut u64) {
        split(pair)
    }
    spec specified_pair {
        aborts_if false;
        ensures result_1 == old(pair.left);
        ensures result_2 == old(pair.right);
    }

    fun check_mixed(): u64 {
        let pair = Pair { left: 4, right: 5 };
        let (left, right) = mixed(&mut pair);
        *right = 9;
        *left
    }
    spec check_mixed { aborts_if false; ensures result == 4; }

    fun check_mutation(): u64 {
        let pair = Pair { left: 4, right: 5 };
        let (left, right) = mixed(&mut pair);
        *right = 9;
        let before = *left;
        pair.right + before
    }
    spec check_mutation { aborts_if false; ensures result == 13; }

    fun check_shared(): u64 {
        let pair = Pair { left: 4, right: 5 };
        let (left, right) = shared(&mut pair);
        *left + *right
    }
    spec check_shared { aborts_if false; ensures result == 9; }

    fun check_single(): u64 {
        let pair = Pair { left: 4, right: 5 };
        *shared_left(&mut pair)
    }
    spec check_single { aborts_if false; ensures result == 4; }

    fun wrong(): u64 { check_mixed() }
    spec wrong { aborts_if false; ensures result == 9; }
}

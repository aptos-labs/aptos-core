// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A then-branch ending in an `if` without `else`: the following `else` is the
// outer `if`'s, by its column.
module 0x42::if_else_offside {
    struct Coin has drop { value: u64 }

    fun consume(coin: Coin): u64 {
        let Coin { value } = coin;
        value
    }

    fun nested(x: u64, flag: bool): u64 {
        let result;
        if (x > 0) {
            result = 1;
            if (flag) result = 2;
        } else {
            result = 3;
        };
        result
    }
    spec nested {
        aborts_if false;
        ensures x > 0 && flag ==> result == 2;
        ensures x > 0 && !flag ==> result == 1;
        ensures x == 0 ==> result == 3;
    }

    fun consumed_in_both(coin: Coin, flag: bool): u64 {
        let result = 0;
        if (flag) {
            result = result + consume(coin);
            if (result > 0) result = 1;
        } else {
            result = consume(coin) % 2;
        };
        result
    }
    spec consumed_in_both {
        aborts_if false;
        ensures flag ==> result <= 1;
        ensures !flag ==> result == coin.value % 2;
    }
}

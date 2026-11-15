// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A borrow of a boolean literal borrows a value, not a local.
module 0x42::borrow_literal {
    fun read(value: &bool): bool { *value }
    spec read {
        aborts_if false;
        ensures result == value;
    }

    fun of_false(): bool { read(&false) }
    spec of_false {
        aborts_if false;
        ensures !result;
    }

    fun of_true(): bool { read(&true) }
    spec of_true {
        aborts_if false;
        ensures result;
    }
}

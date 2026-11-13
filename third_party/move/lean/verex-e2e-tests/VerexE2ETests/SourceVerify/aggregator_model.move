// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An aggregator's verification model holds a value beside its declared fields:
/// `spec_aggregator_set_val` writes it, `spec_aggregator_get_val` reads it, and `==`
/// compares it, so a ghost field could not carry it.
module 0x1::aggregator {
    struct Aggregator has store {
        handle: address,
        key: address,
        limit: u128,
    }

    spec Aggregator {
        pragma intrinsic;
    }

    public native fun add(aggregator: &mut Aggregator, value: u128);
    spec add {
        pragma opaque;
        aborts_if spec_aggregator_get_val(aggregator) + value > spec_get_limit(aggregator);
        ensures spec_get_limit(aggregator) == spec_get_limit(old(aggregator));
        ensures aggregator == spec_aggregator_set_val(old(aggregator),
            spec_aggregator_get_val(old(aggregator)) + value);
    }

    public fun limit(aggregator: &Aggregator): u128 {
        aggregator.limit
    }
    spec limit {
        aborts_if false;
        ensures result == spec_get_limit(aggregator);
    }

    spec native fun spec_get_limit(a: Aggregator): u128;
    spec native fun spec_aggregator_set_val(a: Aggregator, v: u128): Aggregator;
    spec native fun spec_aggregator_get_val(a: Aggregator): u128;

    fun add_once(a: &mut Aggregator, value: u128) {
        add(a, value)
    }
    spec add_once {
        aborts_if spec_aggregator_get_val(a) + value > spec_get_limit(a);
        ensures spec_aggregator_get_val(a) == spec_aggregator_get_val(old(a)) + value;
        ensures spec_get_limit(a) == spec_get_limit(old(a));
    }

    fun read_back(a: Aggregator, v: u128): (Aggregator, u128) {
        (a, v)
    }
    spec read_back {
        aborts_if false;
        ensures spec_aggregator_get_val(spec_aggregator_set_val(result_1, v)) == v;
        ensures spec_get_limit(spec_aggregator_set_val(result_1, v)) == spec_get_limit(result_1);
    }

    fun compares_value(a: &mut Aggregator, value: u128) {
        add(a, value)
    }
    spec compares_value {
        aborts_if spec_aggregator_get_val(a) + value > spec_get_limit(a);
        ensures a == old(a); // error: `==` compares the value, which a positive `value` moved
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The invariant of a vector's elements, owed at a call or a construction, is
// stated by the invariant the caller assumes of the value it holds.
module 0x42::stated_vector_invariant {
    struct Point has copy, drop, store { x: u64, y: u64 }
    spec Point {
        invariant x <= 100;
        invariant y <= 100;
    }

    struct Curve has copy, drop, store { low: u64, points: vector<Point> }
    spec Curve {
        invariant low <= 100;
    }

    struct Config has copy, drop, store { read: Curve, write: Curve }

    fun measure(curve: &Curve): u64 { curve.low }
    spec measure {
        pragma opaque;
        aborts_if false;
        ensures result == curve.low;
    }

    fun measure_read(config: &Config): u64 { measure(&config.read) }
    spec measure_read {
        aborts_if false;
        ensures result == config.read.low;
    }

    fun assemble(read: Curve, write: Curve): Config { Config { read, write } }
    spec assemble {
        aborts_if false;
        ensures result == Config { read, write };
    }

    fun bad_curve(points: vector<Point>): Curve { Curve { low: 101, points } } // error: the constructed value's invariant
}

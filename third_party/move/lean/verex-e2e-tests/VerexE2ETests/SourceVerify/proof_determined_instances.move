// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Instances whose binders the goal and the stated facts determine.
module 0x42::proof_determined_instances {
    struct Point has copy, drop { x: u64, y: u64 }
    spec Point {
        invariant x <= 100;
        invariant y <= 200;
    }

    // A shifted position: `j - 1` against `k` gives `j := k + 1`.
    fun shifted(v: vector<u64>, k: u64): vector<u64> {
        v
    }
    spec shifted {
        requires forall j in 1..len(v): v[j - 1] < v[j];
        requires k + 1 < len(v);
        ensures v[k] < v[k + 1];
    }

    // A forward offset: `j + 1` against `k + 1` gives `j := k`.
    fun offset(v: vector<u64>, k: u64): vector<u64> {
        v
    }
    spec offset {
        requires forall j in 0..len(v) - 1: v[j] <= v[j + 1];
        requires k + 1 < len(v);
        ensures v[k] <= v[k + 1];
    }

    // A read's value carries what is known of an element, by the stated read.
    fun first_x(points: vector<Point>): u64 {
        points[0].x
    }
    spec first_x {
        requires len(points) > 0;
        ensures result <= 100;
    }

    fun at_y(points: vector<Point>, k: u64): u64 {
        points[k].y
    }
    spec at_y {
        requires k < len(points);
        ensures result <= 200;
    }
}

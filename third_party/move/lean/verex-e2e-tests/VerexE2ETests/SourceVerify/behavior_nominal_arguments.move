// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A lambda over a mutable struct or enum, read through an opaque
// higher-order function's `ensures_of` and `aborts_of`.
module 0x42::behavior_nominal_arguments {
    struct Pair has copy, drop { left: u64, right: u64 }

    enum Shape has copy, drop { Point { x: u64 }, Segment { x: u64, length: u64 } }

    fun apply_pair(f: |&mut Pair| has drop, p: &mut Pair) {
        f(p)
    }
    spec apply_pair {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(p);
        ensures ensures_of<f>(old(p), p);
    }

    fun apply_shape(f: |&mut Shape| has drop, s: &mut Shape) {
        f(s)
    }
    spec apply_shape {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(s);
        ensures ensures_of<f>(old(s), s);
    }

    fun swap(p: &mut Pair) {
        apply_pair(|q| {
            let t = q.left;
            q.left = q.right;
            q.right = t;
        }, p)
    }
    spec swap {
        aborts_if false;
        ensures p.left == old(p.right);
        ensures p.right == old(p.left);
    }

    fun stretch(s: &mut Shape) {
        apply_shape(|t| { t.length = t.length + 1; }, s)
    }
    spec stretch {
        requires s is Segment;
        requires s.length < 100;
        aborts_if false;
        ensures s.length == old(s.length) + 1;
        ensures s.x == old(s.x);
    }

    // A point has no length: the write aborts.
    fun stretch_any(s: &mut Shape) {
        apply_shape(|t| { t.length = t.length + 1; }, s)
    }
    spec stretch_any {
        requires (s is Point) || s.length < 100;
        aborts_if false; // error: a point aborts
    }

    fun swap_wrong(p: &mut Pair) {
        apply_pair(|q| {
            let t = q.left;
            q.left = q.right;
            q.right = t;
        }, p)
    }
    spec swap_wrong {
        ensures p.left == old(p.left); // error: the values are exchanged
    }
}

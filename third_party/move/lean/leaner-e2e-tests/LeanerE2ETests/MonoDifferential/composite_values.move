// Composite return values that need no stdlib function bodies: Move 2
// vector literals and address constants are built-in expression forms, so the
// Lean side stays a single-module unit while both engines still marshal and
// compare vectors, nested vectors, and addresses.

// RUN: publish
module 0x47::composites {
    public fun pair(a: u64, b: u64): vector<u64> {
        vector[a, b]
    }

    public fun nested(n: u64): vector<vector<u64>> {
        vector[vector[n], vector[n, n], vector[]]
    }

    public fun empty(): vector<u64> {
        vector[]
    }

    public fun owner(): address {
        @0x47
    }

    public fun signed_pair(a: i64, b: i64): vector<i64> {
        vector[a, b]
    }
}

// RUN: execute 0x47::composites::pair --args 7, 9
// RUN: execute 0x47::composites::nested --args 3
// RUN: execute 0x47::composites::empty
// RUN: execute 0x47::composites::owner
// RUN: execute 0x47::composites::signed_pair --args -5, 5

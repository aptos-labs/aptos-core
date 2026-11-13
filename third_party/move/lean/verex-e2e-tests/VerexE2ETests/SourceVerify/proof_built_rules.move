// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A leaf equation of a term and a built value rewrites the term: an element
/// the path read reads as the value.
module 0x42::proof_built_rules {
    fun at<T: copy + drop>(v: vector<T>, i: u64): T {
        v[i]
    }
    spec at {
        requires i < len(v);
        aborts_if false;
        ensures result == v[i];
    }
}

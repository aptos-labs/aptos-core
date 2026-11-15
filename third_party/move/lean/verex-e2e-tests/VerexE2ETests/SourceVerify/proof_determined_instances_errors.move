// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An instance whose remaining premise does not hold is no instance.
module 0x42::proof_determined_instances_errors {
    fun not_an_instance(v: vector<u64>, k: u64): vector<u64> {
        v
    }
    spec not_an_instance {
        requires forall j in 1..len(v): v[j - 1] < v[j];
        requires k < len(v);
        ensures v[k] < v[k + 1]; // error: the premise `k + 1 < len(v)` is not known
    }
}

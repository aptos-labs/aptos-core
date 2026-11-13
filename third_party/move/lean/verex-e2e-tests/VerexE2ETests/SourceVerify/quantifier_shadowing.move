// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::quantifier_shadowing {
    fun scoped(v: &vector<u64>): u64 {
        let answer = { let v = 7; v };
        spec { assert forall j in 0..len(v): v[j] == v[j]; };
        answer
    }
    spec scoped { aborts_if false; ensures result == 7; }

    fun nested(v: &vector<vector<u64>>) {
        let _unused = { let v = 7; v };
        spec {
            assert forall row in v:
                forall j in 0..len(row): row[j] == row[j];
        };
    }
    spec nested { aborts_if false; }

    fun wrong(v: &vector<u64>) {
        let _unused = { let v = 0; v };
        spec { assert forall j in 0..len(v): v[j] == 0; };
    }
    spec wrong { aborts_if false; }
}

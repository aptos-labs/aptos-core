-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.XIRCommon
import MoveModel.Frontend.Elab

/-! Move-source vectors imported through exchange XIR and executed by the
Lean MoveModel.IR interpreter. -/

namespace Tests.Frontend.Vectors

open MoveModel.IR
open MoveModel.Frontend.XIR

def imported : MProgram := moveM% "
module 0x42::vectors {
    use std::vector;

    fun exercise(first: u64): u64 {
        let v = vector[first, 20];
        vector::push_back(&mut v, 30);
        let last = vector::pop_back(&mut v);
        v[1] = 25;
        last + vector::length(&v) + *vector::borrow(&v, 0) + v[1]
    }

    fun constant_sum(): u64 {
        let v = vector[4, 5, 6];
        v[0] + v[1] + v[2]
    }

    fun make_pair(first: u64, second: u64): vector<u64> {
        vector[first, second]
    }

    fun pop_empty(value: u64): u64 {
        let v = vector[value];
        vector::pop_back(&mut v);
        vector::pop_back(&mut v)
    }

    fun borrow_out_of_bounds(value: u64): u64 {
        let v = vector[value];
        *vector::borrow(&v, 1)
    }
}
"

private def run := Tests.runXIR imported

#test run "exercise" [] [.u64 10] = Tests.okU64 67
#test run "constant_sum" [] [] = Tests.okU64 15
#test run "make_pair" [] [.u64 7, .u64 9]
  = Tests.okVals [.vector [.u64 7, .u64 9]]
#test run "pop_empty" [] [.u64 1] = Tests.aborted 0x20000
#test run "borrow_out_of_bounds" [] [.u64 1] = Tests.aborted 0x20000

end Tests.Frontend.Vectors

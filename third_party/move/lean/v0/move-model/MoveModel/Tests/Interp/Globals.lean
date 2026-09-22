-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.Tests.XIRCommon
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Global Memory

The resource instructions (`move_to`/`move_from`/`exists`) and global
borrows (`borrow_global`/`borrow_global_mut`), including their abort
behavior (resource collision, missing resource — runtime aborts carry
code 0).  A `&signer` argument is passed as a plain address at the
boundary.
-/

namespace Tests.Interp.Globals

open MoveModel.IR
open MoveModel.Frontend.XIR

def globalsM : MProgram := moveM% "
module 0x42::globals {
    struct Counter has key { n: u64 }
    struct Flag has key { on: bool }

    fun publish(s: &signer, n: u64) { move_to(s, Counter { n }) }
    fun is_published(a: address): bool { exists<Counter>(a) }
    fun read(a: address): u64 acquires Counter { borrow_global<Counter>(a).n }
    fun bump(a: address) acquires Counter {
        let c = borrow_global_mut<Counter>(a);
        c.n = c.n + 1;
    }
    fun take(a: address): u64 acquires Counter {
        let Counter { n } = move_from<Counter>(a);
        n
    }
    fun set_flag(s: &signer, on: bool) { move_to(s, Flag { on }) }
}
"

def globals : Program := globalsM.toProgram

-- Pin the resource ids used in the memory expectations below.
#guard globalsM.resourceId "Counter" == 0
#guard globalsM.resourceId "Flag" == 1

def run (f : String) (mem : IMem) (args : List Value) : Outcome :=
  Tests.runXIR globalsM f mem args

/-! ## Publish (`move_to` through a `&signer`), and collision abort -/

#test run "publish" [] [.address 5, .u64 7]
  = okRet [(0, 5, .struct [.u64 7])] []
#test run "publish" [(0, 5, .struct [.u64 1])] [.address 5, .u64 7]
  = abortedIn [(0, 5, .struct [.u64 1])] 0

/-! ## Existence test -/

#test run "is_published" [(0, 5, .struct [.u64 7])] [.address 5]
  = okRet [(0, 5, .struct [.u64 7])] [.bool true]
#test run "is_published" [] [.address 5] = okBool false

/-! ## Read through `borrow_global`, and missing-resource abort -/

#test run "read" [(0, 5, .struct [.u64 7])] [.address 5]
  = okRet [(0, 5, .struct [.u64 7])] [.u64 7]
#test run "read" [] [.address 5] = aborted 0

/-! ## Write through `borrow_global_mut` -/

#test run "bump" [(0, 5, .struct [.u64 7])] [.address 5]
  = okRet [(0, 5, .struct [.u64 8])] []
#test run "bump" [] [.address 5] = aborted 0

/-! ## `move_from`, and missing-resource abort -/

#test run "take" [(0, 5, .struct [.u64 7])] [.address 5]
  = okU64 7
#test run "take" [] [.address 5] = aborted 0

/-! ## A second resource type gets its own id -/

#test run "set_flag" [] [.address 9, .bool true]
  = okRet [(1, 9, .struct [.bool true])] []

/-! ## Resources of different types at the same address are independent -/

#test run "read" [(1, 5, .struct [.bool true]), (0, 5, .struct [.u64 7])]
    [.address 5]
  = okRet [(1, 5, .struct [.bool true]), (0, 5, .struct [.u64 7])] [.u64 7]

end Tests.Interp.Globals

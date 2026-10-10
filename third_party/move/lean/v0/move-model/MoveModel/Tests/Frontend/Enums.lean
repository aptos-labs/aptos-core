-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.XIRCommon
import MoveModel.Frontend.Elab

/-! Move-source enums, including payloads and reference matching, imported
through exchange XIR and executed by the Lean interpreter. -/

namespace Tests.Frontend.Enums

open MoveModel.IR
open MoveModel.Frontend.XIR

def imported : MProgram := moveM% "
module 0x42::enums {
    enum Choice has drop {
        None,
        One(u64),
        Pair { left: u64, right: u64 },
    }

    fun inspect(choice: Choice): u64 {
        match (choice) {
            Choice::None => 0,
            Choice::One(value) => value,
            Choice::Pair { left, right } => left + right,
        }
    }

    fun make(value: u64): Choice {
        Choice::One(value)
    }

    fun is_none(choice: &Choice): bool {
        match (choice) {
            Choice::None => true,
            _ => false,
        }
    }

    fun check_none(use_none: bool): bool {
        let choice = if (use_none) Choice::None else Choice::One(1);
        is_none(&choice)
    }

}
"

private def run := Tests.runXIR imported

#test run "inspect" [] [.variant 0 []] = Tests.okU64 0
#test run "inspect" [] [.variant 1 [.u64 9]] = Tests.okU64 9
#test run "inspect" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okU64 9
#test run "make" [] [.u64 7] = Tests.okVals [.variant 1 [.u64 7]]
#test run "check_none" [] [.bool true] = Tests.okBool true
#test run "check_none" [] [.bool false] = Tests.okBool false

end Tests.Frontend.Enums

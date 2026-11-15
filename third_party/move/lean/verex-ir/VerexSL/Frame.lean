-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Syntax

namespace VerexSL

/-- A mixed wildcard closes listed resource families except at their listed
keys, while leaving unlisted families open. This frontend-owned metadata uses
existing contract attributes without changing the interchange schema. -/
def hasLooseFrame (contract : VerexIR.FunctionContract) : Bool :=
  contract.pragmas.any fun
    | .assign "verex_loose_frame" (.constant (.bool true)) _ => true
    | _ => false

/-- Whether a contract states a frame. -/
def hasFrameClauses (contract : VerexIR.FunctionContract) : Bool :=
  contract.hasFrame || contract.modifiesAll || contract.readsAll ||
    !contract.modifies.isEmpty || !contract.reads.isEmpty

/-- Whether a contract leaves global memory open and states nothing else:
no condition, no pragma, and `modifies *`. It has nothing to verify. -/
def statesNothing (contract : VerexIR.FunctionContract) : Bool :=
  contract.conditions.isEmpty && contract.pragmas.isEmpty && contract.modifiesAll &&
    contract.modifies.isEmpty && contract.reads.isEmpty

/-- The property of the `aborts_if false` lowering appends to every Rust
contract, a name no source can spell. -/
def implicitNoPanicProperty : String := "no panic"

/-- Whether a condition is that clause. -/
def isImplicitNoPanic (condition : VerexIR.Condition) : Bool :=
  condition.properties.any fun
    | .assign name (.constant (.bool true)) _ => name == implicitNoPanicProperty
    | _ => false

end VerexSL

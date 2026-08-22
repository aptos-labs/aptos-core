-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Basic
import Move.Addresses
import Move.ConventionalAddresses
import Move.Action
import Move.Attributes
import Move.Syntax
import Move.Semantics.Outcome
import Move.Semantics.Spec
import Move.Semantics.Checked
import Move.Semantics.Reference
import Move.Semantics.Vector
import Move.Semantics.Global
import Move.Verify.Contract
import Move.Verify.WP
import Move.Verify.Borrow
import Move.Verify.SourceProgram
import Move.Verify.BorrowChecker
import Move.Verify.Compare
import Move.Verify.Syntax
import Move.Verify.Tactics
import Move.Compiler.LIR
import Move.Compiler.LCNF
import Move.Compiler.Normalize
import Move.Compiler.Elab
import Move.Compiler.Export

/-!
# Move source language

`Move` is the source-language and compiler namespace for smart contracts
authored in Lean and lowered to `MoveModel.IR`. The compiler tool remains
named Leaner. The language is defined in `Move/leaner-move.md`, introduced by
example in `Move/README.md`, and designed in `Move/design-plan.md` and
`Move/verification-design.md`.
-/

namespace Move

end Move

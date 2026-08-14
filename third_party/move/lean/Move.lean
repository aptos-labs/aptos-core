-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Syntax
import Move.Prover.Ivl.Semantics
import Move.Prover.Ivl.Wp
import Move.Prover.Ivl.WpSound
import Move.Prover.Ivl.LoopCut
import Move.IR.Util
import Move.IR.Value
import Move.IR.State
import Move.IR.ValueTyping
import Move.IR.Spec
import Move.IR.Contract
import Move.IR.Syntax
import Move.IR.Semantics
import Move.IR.Execution
import Move.IR.CodeTyping
import Move.IR.Checked
import Move.IR.Frame
import Move.IR.RefElim
import Move.IR.RefElimCorrect
import Move.IR.Interp
import Move.IR.InterpCorrect
import Move.Prover.Translate.Compile
import Move.Prover.Translate.Sim
import Move.Prover.Translate.Adequacy
import Move.Frontend.Repr
import Move.Frontend.Decode
import Move.Frontend.Elab
import Move.Examples.Account
import Move.Examples.BorrowAccount
import Move.Examples.CrossCall
import Move.Examples.CountDown

/-!
# Move — Lean formalization of Move.

See `README.md` for the architecture and theorem map.
-/

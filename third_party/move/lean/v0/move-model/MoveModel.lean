-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Syntax
import MoveModel.Prover.Ivl.Semantics
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Ivl.WpSound
import MoveModel.Prover.Ivl.LoopCut
import MoveModel.IR.Util
import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.ValueTyping
import MoveModel.IR.Spec
import MoveModel.IR.Contract
import MoveModel.IR.Syntax
import MoveModel.IR.Module
import MoveModel.IR.Mono.Transform
import MoveModel.IR.Mono.Correctness.CFG
import MoveModel.IR.Mono.Correctness.Instances
import MoveModel.IR.Mono.Correctness.Coverage
import MoveModel.IR.Semantics
import MoveModel.IR.Execution
import MoveModel.IR.CodeTyping
import MoveModel.IR.Checked
import MoveModel.IR.Frame
import MoveModel.IR.RefElim.Transform
import MoveModel.IR.RefElim.Correctness
import MoveModel.IR.Interp.Exec
import MoveModel.IR.Interp.Correctness
import MoveModel.Prover.Translate.Compile
import MoveModel.Prover.Translate.Sim
import MoveModel.Prover.Translate.Adequacy
import MoveModel.Prover.Translate.Mono
import MoveModel.Frontend.XIR
import MoveModel.Frontend.XIR.FromIR
import MoveModel.Frontend.XIR.Json
import MoveModel.Frontend.Decode
import MoveModel.Frontend.Elab

/-!
# MoveModel — semantic formalization of Move.

See `README.md` for the architecture and theorem map.
-/

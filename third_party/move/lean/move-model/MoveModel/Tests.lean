-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.Common
import MoveModel.Tests.Frontend.Enums
import MoveModel.Tests.Frontend.Generics
import MoveModel.Tests.Frontend.Vectors
import MoveModel.Tests.IR.Mono
import MoveModel.Tests.Interp.Arith
import MoveModel.Tests.Interp.ControlFlow
import MoveModel.Tests.Interp.CrossCallRefs
import MoveModel.Tests.Interp.Globals
import MoveModel.Tests.Interp.Mutations
import MoveModel.Tests.Interp.RefElimAgree
import MoveModel.Tests.Interp.References
import MoveModel.Tests.Interp.Structs
import MoveModel.Tests.Interp.Vectors
import MoveModel.Tests.Prover.Account
import MoveModel.Tests.Prover.Adequacy
import MoveModel.Tests.Prover.BorrowAccount
import MoveModel.Tests.Prover.CountDown
import MoveModel.Tests.Prover.CrossCall
import MoveModel.Tests.Prover.ElimSource
import MoveModel.Tests.Prover.MasmSource
import MoveModel.Tests.Prover.MoveSource
import MoveModel.Tests.XIR

/-! Aggregate root for the MoveModel regression suite: interpreter, IR,
frontend, and prover tests.  Lake needs a root module per test library, so a
new test file is added here too. -/

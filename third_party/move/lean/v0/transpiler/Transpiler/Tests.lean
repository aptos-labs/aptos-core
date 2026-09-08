-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Tests.Decode
import Transpiler.Tests.Frontend.Leaner
import Transpiler.Tests.Baseline
import Transpiler.Tests.Intrinsics.Sources
import Transpiler.Tests.LIR.Boundary
import Transpiler.Tests.Pipelines.MoveToLeanerLang
import Transpiler.Tests.Print.Axiom
import Transpiler.Tests.Programs.Account
import Transpiler.Tests.Programs.AptosFramework.Counter
import Transpiler.Tests.Programs.BasicCoin
import Transpiler.Tests.Programs.Constants
import Transpiler.Tests.Programs.Enums
import Transpiler.Tests.Programs.FunctionValues
import Transpiler.Tests.Programs.Generics
import Transpiler.Tests.Programs.Loops
import Transpiler.Tests.Programs.OrderedMap
import Transpiler.Tests.Programs.Vectors
import Transpiler.Tests.Programs.MoveStdlib.Std.Acl
import Transpiler.Tests.Programs.MoveStdlib.Std.Bcs
import Transpiler.Tests.Programs.MoveStdlib.Std.BitVector
import Transpiler.Tests.Programs.MoveStdlib.Std.Cmp
import Transpiler.Tests.Programs.MoveStdlib.Std.Error
import Transpiler.Tests.Programs.MoveStdlib.Std.Features
import Transpiler.Tests.Programs.MoveStdlib.Std.FixedPoint32
import Transpiler.Tests.Programs.MoveStdlib.Std.Hash
import Transpiler.Tests.Programs.MoveStdlib.Std.Mem
import Transpiler.Tests.Programs.MoveStdlib.Std.Option
import Transpiler.Tests.Programs.MoveStdlib.Std.Reflect
import Transpiler.Tests.Programs.MoveStdlib.Std.Result
import Transpiler.Tests.Programs.MoveStdlib.Std.Signer
import Transpiler.Tests.Programs.MoveStdlib.Std.String
import Transpiler.Tests.Verification.BitVectorShiftLeft
import Transpiler.Tests.Verification.OptionBorrowMut

/-! The transpiler regressions: the decoder over a CLI export, the printer
baselines (each `Programs/*.move` exported and transpiled, compared with the
generated file beside it), and the elaboration gate — the generated modules
are built against `Move` by importing them here. The suite needs the exchange
frontend: prefer `APTOS_MOVE_CLI` for the lightweight standalone Move CLI, or
use `APTOS_CLI`, the checkout-local debug binary, or `aptos` on `PATH`. -/

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Compiler.Export
import Move.Tests.Account

/-! Manual regeneration entry point for the Account XIR golden file. -/

open scoped Move.Compiler

/-! ## Tests -/

#export_leaner_xir Tests.MovePrograms.Account.compiled to
  "Move/Tests/Account.xir.json"

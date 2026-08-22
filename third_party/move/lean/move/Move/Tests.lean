-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Tests.Abilities
import Move.Tests.Account
import Move.Tests.Addresses
import Move.Tests.Arithmetic
import Move.Tests.Attributes
import Move.Tests.Callees
import Move.Tests.Calls
import Move.Tests.ControlForms
import Move.Tests.CorePrimitives
import Move.Tests.CrossInv
import Move.Tests.EnumPatterns
import Move.Tests.EnumPayloads
import Move.Tests.Enums
import Move.Tests.ExportAccount
import Move.Tests.GenericStorage
import Move.Tests.Generics
import Move.Tests.GlobalBorrows
import Move.Tests.GlobalInv
import Move.Tests.Integers
import Move.Tests.InlinePackage
import Move.Tests.Invariants
import Move.Tests.Literals
import Move.Tests.Loops
import Move.Tests.LowLevel.ModuleVerification
import Move.Tests.LowLevel.MultipleModules
import Move.Tests.LowLevel.Rejections
import Move.Tests.LowLevel.SourceVerification
import Move.Tests.ModelDomain
import Move.Tests.Modules.Math
import Move.Tests.MultipleModules
import Move.Tests.OrderedMap
import Move.Tests.PositionalStructs
import Move.Tests.Quicksort
import Move.Tests.Read
import Move.Tests.ResourceComposition
import Move.Tests.Signed
import Move.Tests.Tuples
import Move.Tests.VectorOperations
import Move.Tests.Vectors

/-! Aggregate root for the Leaner Move regression suite: source verification
and compiler lowering.  Lake needs a root module per test library, so a new
test file is added here too. -/

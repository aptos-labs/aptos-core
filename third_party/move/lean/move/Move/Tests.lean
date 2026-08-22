-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Tests.Compiler.ExportAccount
import Move.Tests.Compiler.InlinePackage
import Move.Tests.Compiler.LowLevel.ModuleVerification
import Move.Tests.Compiler.LowLevel.MultipleModules
import Move.Tests.Compiler.LowLevel.SourceVerification
import Move.Tests.Compiler.ModelDomain
import Move.Tests.Compiler.MultipleModules
import Move.Tests.Language.Abilities
import Move.Tests.Language.Addresses
import Move.Tests.Language.Arithmetic
import Move.Tests.Language.Attributes
import Move.Tests.Language.BorrowChecker
import Move.Tests.Language.ControlForms
import Move.Tests.Language.EnumPatterns
import Move.Tests.Language.EnumPayloads
import Move.Tests.Language.Enums
import Move.Tests.Language.Generics
import Move.Tests.Language.Integers
import Move.Tests.Language.Literals
import Move.Tests.Language.Loops
import Move.Tests.Language.PositionalStructs
import Move.Tests.Language.Signed
import Move.Tests.Language.Tuples
import Move.Tests.Language.VectorOperations
import Move.Tests.Language.Vectors
import Move.Tests.Negative.Lowering
import Move.Tests.Negative.Borrows
import Move.Tests.Negative.BorrowGlobals
import Move.Tests.Negative.Specifications
import Move.Tests.Negative.Surface
import Move.Tests.Negative.Verification
import Move.Tests.Verification.Account
import Move.Tests.Verification.Callees
import Move.Tests.Verification.BorrowCertificates
import Move.Tests.Verification.Calls
import Move.Tests.Verification.CorePrimitives
import Move.Tests.Verification.CrossInv
import Move.Tests.Verification.GenericStorage
import Move.Tests.Verification.GlobalBorrows
import Move.Tests.Verification.GlobalInv
import Move.Tests.Verification.Invariants
import Move.Tests.Verification.OrderedMap
import Move.Tests.Verification.Quicksort
import Move.Tests.Verification.Read
import Move.Tests.Verification.ResourceComposition

/-! Aggregate root for the categorized Leaner Move regression suite. Lake
needs a root module per test library, so every new test file is added here. -/

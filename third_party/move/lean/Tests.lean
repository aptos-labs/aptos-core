-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Tests.Interp.Arith
import Tests.Interp.ControlFlow
import Tests.Interp.Structs
import Tests.Interp.Vectors
import Tests.Interp.Mutations
import Tests.Interp.References
import Tests.Interp.CrossCallRefs
import Tests.Interp.Globals
import Tests.Interp.RefElimAgree
import Tests.IR.Mono
import Tests.Frontend.Enums
import Tests.Frontend.Generics
import Tests.Frontend.Vectors
import Tests.Move.Account
import Tests.Move.Abilities
import Tests.Move.Arithmetic
import Tests.Move.ResourceComposition
import Tests.Move.Calls
import Tests.Move.Loops
import Tests.Move.Enums
import Tests.Move.Generics
import Tests.Move.EnumPatterns
import Tests.Move.EnumPayloads
import Tests.Move.Read
import Tests.Move.Vectors
import Tests.Move.Quicksort
import Tests.Move.OrderedMap
import Tests.Move.VectorOperations
import Tests.Move.XIR
import Tests.Move.LowLevel.Rejections
import Tests.Move.LowLevel.SourceVerification
import Tests.Move.MultipleModules
import Tests.Move.LowLevel.MultipleModules
import Tests.Move.LowLevel.ModuleVerification

/-!
# Test Suite

Unit tests for the computable parts of the formalization, primarily the
bytecode interpreter.  Programs are authored as embedded Move source
(`move%`) or as attributed Lean declarations compiled through `Leaner`.  A
test case is a `#test` (or plain `#guard`) command: it is evaluated at
elaboration time and fails the build on failure, so running the tests is

```
lake test
```
-/

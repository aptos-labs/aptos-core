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

/-!
# Test Suite

Unit tests for the computable parts of the formalization, primarily the
bytecode interpreter. Programs are authored as embedded Move source
(`move%`). A test case is a `#test` (or plain `#guard`) command: it is
evaluated at elaboration time and fails the build on failure, so running
the tests is

```
APTOS_CLI=<path-to-aptos> lake build Tests
```
-/

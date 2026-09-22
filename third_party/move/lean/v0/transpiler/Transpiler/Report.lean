-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Leaner backend report

Report data is independent of the transitional XAST printer. LIR analysis and
backend printing contribute to the same per-module value without making the
report contract depend on a frontend AST.
-/

namespace Transpiler

/-- A Leaner extension the generated source relies on. -/
inductive Extension where
  | nestedNamespace
  | loopInvariant
  | specStatement
  | receiverVectorSpec
  deriving Repr, BEq, Inhabited

def Extension.describe : Extension → String
  | .nestedNamespace => "E1 nested namespace"
  | .loopInvariant => "E12 loop invariant"
  | .specStatement => "E14 spec statement"
  | .receiverVectorSpec => "E11 receiver-style vector operation in spec"

/-- The per-module transpilation report. -/
structure Report where
  /-- Leaner extensions the output relies on. -/
  extensions : List Extension := []
  /-- Constructs dropped with a disposition (`what: why`). -/
  dropped : List String := []
  /-- Declarations emitted commented out (`name: reason`). -/
  unsupported : List String := []
  /-- Axioms emitted (intrinsic models of natives, `axiom` conditions). -/
  axioms : List String := []
  deriving Inhabited

end Transpiler

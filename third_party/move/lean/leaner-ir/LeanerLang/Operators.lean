-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR
import LeanerLang.Ast

/-!
# Shared LeanerLang operator metadata

This table is the presentation boundary shared by source producers and the
AST pretty printer. Checked primitives and operations whose symbolic spelling
would be ambiguous remain in their explicit `core.prim.*` form.
-/

namespace LeanerLang

inductive OperatorFixity where
  | prefix
  | infix
  deriving Repr, BEq, DecidableEq

inductive OperatorAssociativity where
  | left
  | right
  | none
  deriving Repr, BEq, DecidableEq

inductive CoreOperator where
  | range
  | add
  | subtract
  | multiply
  | divide
  | modulo
  | bitwiseOr
  | bitwiseAnd
  | bitwiseXor
  | bitwiseNot
  | shiftLeft
  | shiftRight
  | logicalAnd
  | logicalOr
  | logicalNot
  | equal
  | notEqual
  | less
  | greater
  | lessEqual
  | greaterEqual
  | negate
  | implies
  | equivalent
  deriving Repr, BEq, DecidableEq

structure OperatorInfo where
  operation : CoreOperator
  symbol : String
  precedence : Nat
  fixity : OperatorFixity
  associativity : OperatorAssociativity := .left
  deriving Repr, BEq

namespace Operators

/-- The shared symbolic spelling and precedence table. Precedences follow the
Move expression ladder so profile-oriented source can use the same grouping. -/
def table : Array OperatorInfo := #[
  { operation := .implies, symbol := "==>", precedence := 2, fixity := .infix },
  { operation := .equivalent, symbol := "<==>", precedence := 2, fixity := .infix },
  { operation := .logicalOr, symbol := "||", precedence := 3, fixity := .infix },
  { operation := .logicalAnd, symbol := "&&", precedence := 4, fixity := .infix },
  { operation := .equal, symbol := "==", precedence := 5, fixity := .infix },
  { operation := .notEqual, symbol := "!=", precedence := 5, fixity := .infix },
  { operation := .less, symbol := "<", precedence := 5, fixity := .infix },
  { operation := .greater, symbol := ">", precedence := 5, fixity := .infix },
  { operation := .lessEqual, symbol := "<=", precedence := 5, fixity := .infix },
  { operation := .greaterEqual, symbol := ">=", precedence := 5, fixity := .infix },
  { operation := .range, symbol := "..", precedence := 6, fixity := .infix },
  { operation := .bitwiseOr, symbol := "|", precedence := 7, fixity := .infix },
  { operation := .bitwiseXor, symbol := "^", precedence := 8, fixity := .infix },
  { operation := .bitwiseAnd, symbol := "&", precedence := 9, fixity := .infix },
  { operation := .shiftLeft, symbol := "<<", precedence := 10, fixity := .infix },
  { operation := .shiftRight, symbol := ">>", precedence := 10, fixity := .infix },
  { operation := .add, symbol := "+", precedence := 11, fixity := .infix },
  { operation := .subtract, symbol := "-", precedence := 11, fixity := .infix },
  { operation := .multiply, symbol := "*", precedence := 12, fixity := .infix },
  { operation := .divide, symbol := "/", precedence := 12, fixity := .infix },
  { operation := .modulo, symbol := "%", precedence := 12, fixity := .infix },
  { operation := .logicalNot, symbol := "!", precedence := 13,
    fixity := .prefix, associativity := .none },
  { operation := .bitwiseNot, symbol := "~", precedence := 13,
    fixity := .prefix, associativity := .none },
  { operation := .negate, symbol := "-", precedence := 13,
    fixity := .prefix, associativity := .none }
]

def info? (operation : CoreOperator) : Option OperatorInfo :=
  table.find? (·.operation == operation)

def ofPrimitive : Primitive → Option CoreOperator
  | .range => some .range
  | .add => some .add
  | .subtract => some .subtract
  | .multiply => some .multiply
  | .divide => some .divide
  | .modulo => some .modulo
  | .bitwiseOr => some .bitwiseOr
  | .bitwiseAnd => some .bitwiseAnd
  | .bitwiseXor => some .bitwiseXor
  | .bitwiseNot => some .bitwiseNot
  | .shiftLeft => some .shiftLeft
  | .shiftRight => some .shiftRight
  | .logicalAnd => some .logicalAnd
  | .logicalOr => some .logicalOr
  | .logicalNot => some .logicalNot
  | .equal => some .equal
  | .notEqual => some .notEqual
  | .less => some .less
  | .greater => some .greater
  | .lessEqual => some .lessEqual
  | .greaterEqual => some .greaterEqual
  | .negate => some .negate
  | .implies => some .implies
  | .equivalent => some .equivalent
  | .profileAdd => some .add
  | .profileSubtract => some .subtract
  | .profileMultiply => some .multiply
  | .profileDivide => some .divide
  | .profileModulo => some .modulo
  | .profileShiftLeft => some .shiftLeft
  | .profileShiftRight => some .shiftRight
  | .profileNegate => some .negate
  | _ => none

def toPrimitive : CoreOperator → Primitive
  | .range => .range
  | .add => .profileAdd
  | .subtract => .profileSubtract
  | .multiply => .profileMultiply
  | .divide => .profileDivide
  | .modulo => .profileModulo
  | .bitwiseOr => .bitwiseOr
  | .bitwiseAnd => .bitwiseAnd
  | .bitwiseXor => .bitwiseXor
  | .bitwiseNot => .bitwiseNot
  | .shiftLeft => .profileShiftLeft
  | .shiftRight => .profileShiftRight
  | .logicalAnd => .logicalAnd
  | .logicalOr => .logicalOr
  | .logicalNot => .logicalNot
  | .equal => .equal
  | .notEqual => .notEqual
  | .less => .less
  | .greater => .greater
  | .lessEqual => .lessEqual
  | .greaterEqual => .greaterEqual
  | .negate => .profileNegate
  | .implies => .implies
  | .equivalent => .equivalent

/-- LIR-side lookup for the semantic source printer. Keeping it here ensures
that both printer layers select the same table entry when symbolic output is
enabled there. -/
def ofPrimitiveOperation : LeanerIR.PrimitiveOperation → Option CoreOperator
  | .range => some .range
  | .add => some .add
  | .subtract => some .subtract
  | .multiply => some .multiply
  | .divide => some .divide
  | .modulo => some .modulo
  | .bitwiseOr => some .bitwiseOr
  | .bitwiseAnd => some .bitwiseAnd
  | .bitwiseXor => some .bitwiseXor
  | .bitwiseNot => some .bitwiseNot
  | .shiftLeft => some .shiftLeft
  | .shiftRight => some .shiftRight
  | .logicalAnd => some .logicalAnd
  | .logicalOr => some .logicalOr
  | .logicalNot => some .logicalNot
  | .equal => some .equal
  | .notEqual => some .notEqual
  | .less => some .less
  | .greater => some .greater
  | .lessEqual => some .lessEqual
  | .greaterEqual => some .greaterEqual
  | .negate => some .negate
  | .implies => some .implies
  | .equivalent => some .equivalent
  | _ => none

def infoForPrimitive? (operation : Primitive) : Option OperatorInfo :=
  (ofPrimitive operation).bind info?

def infoForPrimitiveOperation? (operation : LeanerIR.PrimitiveOperation) :
    Option OperatorInfo :=
  (ofPrimitiveOperation operation).bind info?

end Operators
end LeanerLang

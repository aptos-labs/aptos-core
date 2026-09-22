-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Xast

/-!
# Names

Name policy of the transpiler: legalizing Move identifiers for Lean (keyword
escaping with guillemets), the root namespace derived from a module's named
address, Lean module paths of generated files, and the table of standard-library
modules that map onto Leaner primitives instead of being transpiled.
-/

namespace LeanerMove.Frontend.Names

open LeanerMove.Frontend.Xast

/-- Lean keywords and Leaner's module-scoped and spec-scoped keywords.  An
identifier in this list is escaped as `«name»`, which preserves the exact
name. -/
def reserved : List String := [
  -- Lean
  "fun", "let", "in", "do", "if", "then", "else", "match", "with", "end", "at", "by",
  "have", "show", "from", "where", "deriving", "structure", "inductive", "def",
  "theorem", "lemma", "instance", "class", "namespace", "section", "open", "import",
  "export", "variable", "universe", "axiom", "opaque", "abbrev", "macro", "syntax",
  "notation", "infix", "infixl", "infixr", "prefix", "postfix", "mutual", "partial",
  "private", "protected", "noncomputable", "unsafe", "local", "scoped", "set_option",
  "return", "for", "unless", "break", "continue", "try", "catch", "finally", "mut",
  "calc", "Type", "Sort", "Prop", "example", "termination_by", "decreasing_by",
  "nomatch", "nofun", "attribute", "omit", "include", "extends", "using", "suffices",
  "obtain", "exact", "intro", "forall", "exists", "fun", "assert", "true", "false",
  "sorry", "rfl", "id", "pure", "bind", "some", "none", "List", "Nat", "Int",
  -- Leaner items and clauses
  "struct", "enum", "entry", "friend", "spec", "verify", "module", "address_alias",
  "native", "inline", "package", "public", "requires", "ensures", "aborts_if",
  "modifies", "invariant", "pragma", "old", "existsAt", "result", "initial",
  "final", "abortCode", "this", "abort", "freeze", "vector", "has", "update",
  "global", "all", "moveTo", "moveFrom"
]

/-- Whether `name` is a legal Lean identifier without escaping: Move
identifiers are ASCII `[A-Za-z_][A-Za-z0-9_]*`, which Lean accepts. -/
def isPlainIdent (name : String) : Bool :=
  match name.toList with
  | [] => false
  | c :: rest => (c.isAlpha || c == '_') && rest.all fun c => c.isAlphanum || c == '_' || c == '\''

/-- Legalizes a Move identifier for Lean: keywords and anything that is not a
plain identifier are wrapped in guillemets. -/
def legalize (name : String) : String :=
  if reserved.contains name || !isPlainIdent name then "«" ++ name ++ "»" else name

/-- `snake_case` → `PascalCase`. -/
def pascalCase (name : String) : String :=
  String.join <| name.splitOn "_" |>.filter (· ≠ "") |>.map String.capitalize

/-- The root Lean namespace of a module: the PascalCase form of the named
address it was declared under (`aptos_framework` → `AptosFramework`); a module
declared under a numeric address has no root. -/
def rootNamespace (m : ModuleRef) : Option String :=
  m.addressAlias.map pascalCase

/-- The Lean namespace of a module's declarations: root (if any) and the Move
module name verbatim (`AptosFramework.coin`). -/
def moduleNamespace (m : ModuleRef) : String :=
  match rootNamespace m with
  | some root => root ++ "." ++ legalize m.name
  | none => legalize m.name

/-- The Lean module (file) path of a generated module: PascalCase per Lake's
file convention (`AptosFramework.Coin`), under `Generated` when there is no
root. -/
def leanModulePath (m : ModuleRef) : String :=
  match rootNamespace m with
  | some root => root ++ "." ++ pascalCase m.name
  | none => pascalCase m.name

/-- The standard-library module that maps onto Leaner primitives instead of
being transpiled: `std::vector` onto `Move.Vector` and the vector operations.
(`std::signer` is transpiled like any module; its natives get their
specification versions from the intrinsic models.) -/
def isPrimitiveModule (m : ModuleRef) : Bool :=
  m.address == "0x1" && m.name == "vector"

/-- The Move function a companion spec function is the specification version
of: the spec rewriter of compiler v2 names the companion of `f` `$f`. -/
def companionFunction? (name : String) : Option String :=
  if name.startsWith "$" then some (name.drop 1).toString else none

/-- A module Leaner provides itself, so the transpiler emits no file for it:
`std::vector` is `Move.Vector` (its natives and helpers are the vector
operation set). -/
def curatedModule? (m : ModuleRef) : Option String :=
  if m.address == "0x1" && m.name == "vector" then some "Move.Vector" else none

/-- How a `std::vector` function renders in Leaner. -/
inductive VecOp where
  /-- `Move.Vector.empty` (pure, ascribed). -/
  | empty
  /-- `Move.Vector.singleton x` (pure). -/
  | singleton
  /-- `v.push x` (pure value update of the borrowed local). -/
  | pushBack
  /-- `v.length` / `r.length` (pure). -/
  | length
  /-- `v.isEmpty` (pure). -/
  | isEmpty
  /-- `&v[i]` (element borrow); `*borrow` is `v.get i`. -/
  | borrow
  /-- `&mut v[i]` (element borrow). -/
  | borrowMut
  /-- `Move.Vector.destroyEmpty v` (pure). -/
  | destroyEmpty
  /-- `Move.Vector.contains vr xr` over references (pure). -/
  | contains
  /-- `Move.Vector.indexOf vr xr` over references (pure, tuple result). -/
  | indexOf
  /-- `Move.Vector.<op> r args` on a `&mut Vector` (Action). -/
  | mutRef (leanName : String)
  deriving Repr, BEq, Inhabited

/-- The rendering of a `std::vector` function by its Move name. -/
def vecOp (name : String) : Option VecOp :=
  match name with
  | "empty" => some .empty
  | "singleton" => some .singleton
  | "push_back" => some .pushBack
  | "length" => some .length
  | "is_empty" => some .isEmpty
  | "borrow" => some .borrow
  | "borrow_mut" => some .borrowMut
  | "destroy_empty" => some .destroyEmpty
  | "contains" => some .contains
  | "index_of" => some .indexOf
  | "pop_back" => some (.mutRef "popBack")
  | "swap" => some (.mutRef "swap")
  | "swap_remove" => some (.mutRef "swapRemove")
  | "append" => some (.mutRef "append")
  | "reverse" => some (.mutRef "reverse")
  | "reverse_slice" => some (.mutRef "reverseSlice")
  | "trim" => some (.mutRef "trim")
  | "trim_reverse" => some (.mutRef "trimReverse")
  | "rotate" => some (.mutRef "rotate")
  | "rotate_slice" => some (.mutRef "rotateSlice")
  | "insert" => some (.mutRef "insert")
  | "remove" => some (.mutRef "remove")
  | _ => none

/-- Whether a vector operation's rendering needs `Action`. -/
def VecOp.isAction : VecOp → Bool
  | .borrow | .borrowMut | .mutRef _ => true
  | _ => false

/-- The Leaner spelling of a module-qualified name seen from `current`: bare
inside the same module, `mod.name` for a module under the same root, and the
full path otherwise. -/
def qualify (current : ModuleRef) (target : QualifiedName) : String :=
  if target.module == current then legalize target.name
  else if rootNamespace target.module == rootNamespace current then
    legalize target.module.name ++ "." ++ legalize target.name
  else
    moduleNamespace target.module ++ "." ++ legalize target.name

/-- Whether a struct is positional: compiler v2 names the fields of a
positional struct `0`, `1`, ... -/
def isPositionalFields (fields : List Field) : Bool :=
  !fields.isEmpty && fields.all fun f => f.name.all Char.isDigit

/-- A fresh local name that does not clash with `taken`. -/
def fresh (base : String) (taken : List String) : String :=
  if !taken.contains base then base
  else
    let rec go (i : Nat) (fuel : Nat) : String :=
      match fuel with
      | 0 => base ++ "_" ++ toString i
      | fuel + 1 =>
        let candidate := base ++ "_" ++ toString i
        if taken.contains candidate then go (i + 1) fuel else candidate
    go 1 1000

end LeanerMove.Frontend.Names

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Std.Data.HashSet

/-!
# Reserved words

The words both LeanerLang printers escape (`«ensures»`) when they print an
identifier: keywords that begin a clause, declaration, or statement, and the
builtin type names, where a bare identifier would be read as the keyword.
-/

namespace LeanerLang

private def reservedWords : Std.HashSet String := Std.HashSet.ofList [
  "Address", "Bool", "Bytes", "Char", "Copy", "Drop", "Fn", "IPtr", "Int", "Key", "Nat",
  "Never", "Range", "SInt", "Signer", "Store", "UInt", "UPtr", "Unit", "Vector", "abort",
  "aborts_if", "aborts_with", "as", "assert", "assume", "break", "const", "continue", "copy",
  "deprecated", "discriminant", "do", "drop", "else", "ensures", "entry", "enum",
  "evidence", "exists", "false", "for", "forall", "friend", "fun", "function", "has",
  "i128", "i16", "i256", "i32", "i64", "i8", "if", "immutable", "in", "invariant",
  "invoke", "isize", "let", "let_post", "let_pre", "lifetime", "loop", "match", "modifies",
  "module", "move", "mut", "namespace", "native", "old", "opaque", "package", "panic",
  "pragma", "private", "public", "reads", "requires", "return", "rust", "spec", "string",
  "struct", "then", "true", "type", "u128", "u16", "u256", "u32", "u64", "u8", "use",
  "using", "usize", "view", "where", "while", "with"]

/-- Whether an identifier must be escaped when printed. -/
def isReservedWord (word : String) : Bool := reservedWords.contains word

end LeanerLang

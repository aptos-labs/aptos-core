--# publish

import Move

namespace LeanerTxnRejectIndexedEnum

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

@[move_enum]
inductive Indexed : Bool → Type where
  | yes : Indexed true
  | no : Indexed false
  deriving Copy, Drop, Store

/-! ## Tests -/

#export_leaner "LeanerRejectIndexedEnum" structs [Indexed] functions []

end LeanerTxnRejectIndexedEnum

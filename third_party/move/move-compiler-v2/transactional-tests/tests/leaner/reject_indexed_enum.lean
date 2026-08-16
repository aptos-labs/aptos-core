--# publish

import Move

namespace LeanerTxnRejectIndexedEnum

open Move
open scoped Move Move.Compiler

@[move_enum]
inductive Indexed : Bool → Type where
  | yes : Indexed true
  | no : Indexed false
  deriving Copy, Drop, Store

#export_leaner "LeanerRejectIndexedEnum" structs [Indexed] functions []

end LeanerTxnRejectIndexedEnum

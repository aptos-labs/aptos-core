--# publish

import Move

namespace LeanerTxnRejectEmptyEnum

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

@[move_enum]
inductive Empty
  deriving Copy, Drop, Store

/-! ## Tests -/

#export_leaner "LeanerRejectEmptyEnum" structs [Empty] functions []

end LeanerTxnRejectEmptyEnum

--# publish

import Move

namespace LeanerTxnRejectEmptyEnum

open Move
open scoped Move Move.Compiler

@[move_enum]
inductive Empty
  deriving Copy, Drop, Store

#export_leaner "LeanerRejectEmptyEnum" structs [Empty] functions []

end LeanerTxnRejectEmptyEnum

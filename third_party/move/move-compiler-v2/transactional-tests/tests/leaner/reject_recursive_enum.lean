--# publish

import Move

namespace LeanerTxnRejectRecursiveEnum

open Move
open scoped Move Move.Compiler

@[move_enum]
inductive Chain where
  | end_
  | link (next : Chain)
  deriving Copy, Drop, Store

#export_leaner "LeanerRejectRecursiveEnum" structs [Chain] functions []

end LeanerTxnRejectRecursiveEnum

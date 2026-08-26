--# publish

import Move

namespace LeanerTxnRejectRecursiveType

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

@[move_struct]
structure RecursiveType where
  next : RecursiveType
  deriving Copy, Drop, Store

/-! ## Tests -/

#export_leaner "LeanerRejectRecursiveType" structs [RecursiveType] functions []

end LeanerTxnRejectRecursiveType

--# publish

import Move

namespace LeanerTxnRejectRecursiveGenericType

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

mutual
  @[move_struct]
  structure Left (T : Type) where
    right : Right T
    deriving Copy, Drop, Store

  @[move_struct]
  structure Right (T : Type) where
    left : Left T
    deriving Copy, Drop, Store
end

/-! ## Tests -/

#export_leaner "LeanerRejectRecursiveGenericType" structs [Left, Right] functions []

end LeanerTxnRejectRecursiveGenericType

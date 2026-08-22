--# publish

import Move

namespace LeanerTxnRejectUnsupportedType

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

@[move_fun]
def unsupported (_value : Nat) : U64 :=
  1

/-! ## Tests -/

#export_leaner "LeanerRejectUnsupportedType" structs [] functions [unsupported]

end LeanerTxnRejectUnsupportedType

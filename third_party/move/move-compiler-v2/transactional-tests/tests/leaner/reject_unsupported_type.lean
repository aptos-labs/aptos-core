--# publish

import Move

namespace LeanerTxnRejectUnsupportedType

open Move
open scoped Move Move.Compiler

@[move_fun]
def unsupported (_value : U8) : U64 :=
  1

#export_leaner "LeanerRejectUnsupportedType" structs [] functions [unsupported]

end LeanerTxnRejectUnsupportedType

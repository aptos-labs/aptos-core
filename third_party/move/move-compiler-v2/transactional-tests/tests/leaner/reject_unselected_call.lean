--# publish

import Move

namespace LeanerTxnRejectUnselectedCall

open Move
open scoped Move Move.Compiler

/-! ## Functions -/

@[move_fun]
def helper (value : U64) : U64 :=
  value + value

@[move_fun]
def caller (value : U64) : U64 :=
  helper value

/-! ## Tests -/

#export_leaner "LeanerRejectUnselectedCall" structs [] functions [caller]

end LeanerTxnRejectUnselectedCall

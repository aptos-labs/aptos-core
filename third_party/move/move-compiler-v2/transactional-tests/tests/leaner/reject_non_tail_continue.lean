--# publish

import Move

move_module LeanerRejectNonTailContinue where

  /-! ## Functions -/

  partial fun sumDown (value : U64) : U64 :=
    if value < 1 then 0 else value + continue sumDown (value - 1)

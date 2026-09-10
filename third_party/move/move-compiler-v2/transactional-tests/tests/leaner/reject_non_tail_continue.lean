--# publish

import Move

module LeanerRejectNonTailContinue where

  /-! ## Functions -/

  partial fun sum_down (value : U64) : U64 :=
    if value < 1 then 0 else value + continue sum_down (value - 1)

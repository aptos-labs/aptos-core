--# publish

import Move

move_module LeanerRejectOrdinaryCall where

  /-- Ordinary Lean helpers are available to proofs, but are not Move code. -/
  def ordinaryHelper (value : U64) : U64 :=
    value + value

  fun caller (value : U64) : U64 :=
    ordinaryHelper value

--# publish

import Move

module LeanerRejectOrdinaryCall where

  /-! ## Functions -/

  /-- Ordinary Lean helpers are available to proofs, but are not Move code. -/
  def ordinary_helper (value : U64) : U64 :=
    value + value

  fun caller (value : U64) : U64 :=
    ordinary_helper value

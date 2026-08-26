--# publish

import Move

open scoped Move

module LeanerRejectInvalidAbility where

  /-! ## Functions -/

  @[move_struct]
  structure Resource where
    value : U64
    deriving Key

  @[move_struct]
  structure InvalidCopy where
    resource : Resource
    deriving Copy

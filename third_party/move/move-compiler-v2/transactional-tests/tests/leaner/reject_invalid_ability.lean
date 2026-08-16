--# publish

import Move

open scoped Move

move_module LeanerRejectInvalidAbility where

  @[move_struct]
  structure Resource where
    value : U64
    deriving Key

  @[move_struct]
  structure InvalidCopy where
    resource : Resource
    deriving Copy

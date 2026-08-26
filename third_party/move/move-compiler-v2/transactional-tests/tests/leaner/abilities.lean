--# publish

import Move

open scoped Move

module LeanerAbilities where

  /-! ## Functions -/

  @[move_struct]
  structure Plain where
    value : U64

  @[move_struct]
  structure CopyDrop where
    value : U64
    deriving Copy, Drop

  @[move_struct]
  structure Stored (T : Type) where
    value : T
    deriving Store

  @[move_struct]
  structure Resource where
    value : U64
    deriving Key

  @[move_enum]
  inductive Droppable where
    | empty
    | value (inner : U64)
    deriving Drop

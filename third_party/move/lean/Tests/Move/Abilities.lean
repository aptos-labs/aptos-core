-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-! Explicit Move abilities declared through Lean's `deriving` syntax. -/

namespace Tests.MovePrograms

open Move
open MoveModel.IR
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler

move_module Abilities where

  /-! ## Functions -/

  @[move_struct]
  structure Plain where
    value : U64

  @[move_struct]
  structure CopyDrop where
    value : U64
    deriving Copy, Drop

  @[move_struct]
  structure Stored where
    value : U64
    deriving Store

  @[move_struct]
  structure Resource where
    value : U64
    deriving Key

  @[move_struct]
  structure GenericValue (T : Type) where
    value : T
    deriving Copy, Drop, Store

  @[move_struct]
  structure GenericResource (T : Type) where
    value : T
    deriving Key, Drop

  @[move_struct]
  structure Phantom (T : Type) where
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Droppable where
    | first
    | second (value : U64)
    deriving Drop

  /-! ## Tests -/

  def compiled : MModule := move_module% "AbilitiesTest"

namespace Abilities

private def info (name : String) :=
  compiled.structMeta.find? (fun candidate => candidate.name == name)

#guard (info "Plain").map (fun value => value.abilities) == some {}
#guard (info "CopyDrop").map (fun value => value.abilities) ==
  some { copy := true, drop := true }
#guard (info "Stored").map (fun value => value.abilities) ==
  some { store := true }
#guard (info "Resource").map (fun value => value.abilities) ==
  some { key := true }
#guard (info "Droppable").map (fun value => value.abilities) ==
  some { drop := true }

#guard match compiled.structs.find? (fun candidate => candidate.name == "GenericValue") with
  | some value => value.typeParams ==
      [{ name := "T", abilities := { copy := true, drop := true, store := true } }]
  | none => false

#guard match compiled.structs.find? (fun candidate => candidate.name == "GenericResource") with
  | some value => value.typeParams ==
      [{ name := "T", abilities := { drop := true, store := true } }]
  | none => false

#guard match compiled.structs.find? (fun candidate => candidate.name == "Phantom") with
  | some value => value.typeParams ==
      [{ name := "T", abilities := {}, phantom := true }]
  | none => false

end Abilities

end Tests.MovePrograms

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-! Explicit Move abilities declared through Lean's `deriving` syntax. -/

namespace Tests.MovePrograms

open Move
open MoveModel.IR
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler

module Abilities where

  /-! ## Functions -/

  struct Plain where
    value : U64

  struct CopyDrop has Copy, Drop where
    value : U64

  struct Stored has Store where
    value : U64

  struct Resource has Key where
    value : U64

  struct GenericValue (T) has Copy, Drop, Store where
    value : T

  struct GenericResource (T) has Key, Drop where
    value : T

  struct Phantom (T) has Copy, Drop, Store where

  enum Droppable has Drop where
    | First
    | Second (value : U64)

  /-! ## Tests -/

  def compiled : MModule := lowerToIR ``Tests.MovePrograms.Abilities

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

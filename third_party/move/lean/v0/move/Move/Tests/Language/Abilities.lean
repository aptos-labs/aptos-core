-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move

/-! Explicit Move abilities declared through Lean's `deriving` syntax. -/

namespace Tests.MovePrograms

open Move
open MoveModel.IR
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

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Abilities

namespace Abilities

private def info (name : String) :=
  compiled.structMeta? name

#guard (info "Plain").map (fun value => value.abilities) == some {}
#guard (info "CopyDrop").map (fun value => value.abilities) ==
  some { copy := true, drop := true }
#guard (info "Stored").map (fun value => value.abilities) ==
  some { store := true }
#guard (info "Resource").map (fun value => value.abilities) ==
  some { key := true }
#guard (info "Droppable").map (fun value => value.abilities) ==
  some { drop := true }

#guard match compiled.structDecl? "GenericValue" with
  | some value => value.typeParams ==
      [{ name := "T", abilities := { copy := true, drop := true, store := true } }]
  | none => false

#guard match compiled.structDecl? "GenericResource" with
  | some value => value.typeParams ==
      [{ name := "T", abilities := { drop := true, store := true } }]
  | none => false

#guard match compiled.structDecl? "Phantom" with
  | some value => value.typeParams ==
      [{ name := "T", abilities := {}, phantom := true }]
  | none => false

end Abilities

end Tests.MovePrograms

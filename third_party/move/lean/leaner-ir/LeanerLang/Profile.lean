-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Lower

/-!
# Leaner frontend validation profile

The source package registers only the profile-neutral/core subset it emits.
Full Move and Rust profile packages may validate the same `RawUnit` with their
richer registries; this sibling library does not create a reverse dependency
on either package.
-/

namespace LeanerLang

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def unsupportedExtension (profile kind : String)
    (value : ProfileValue) : Array LeanerIR.Validation.Diagnostic :=
  #[.error "LEANER-PROFILE-EXTENSION"
    s!"the core-only LeanerLang {profile} registry does not admit {kind} extension `{value.tag}`"]

/-- Fields of a canonical string-array profile payload, if it is one. -/
private def stringArrayPayload? (payload : String) : Option (Array String) :=
  match Lean.Json.parse payload with
  | .ok (.arr fields) => fields.mapM fun
      | .str value => some value
      | _ => none
  | _ => none

/-- A `friend` namespace relation names a module by address, optional address
alias, and name. -/
private def isFriendPayload (payload : String) : Bool :=
  match stringArrayPayload? payload with
  | some #[_, alias, _] => (stringArrayPayload? alias).any (·.size <= 1)
  | _ => false

private def checkMoveProperty (value : ProfileValue) :
    Array LeanerIR.Validation.Diagnostic :=
  if value.tag == "struct.variants" && value.payload.isEmpty then #[]
  else if value.tag == "typeParameter.phantom" && value.payload.isEmpty then #[]
  else if value.tag == "metadata.friend" && isFriendPayload value.payload then #[]
  else unsupportedExtension "Move" "property" value

def moveSchema : ProfileSchema where
  profile := .move
  name := "move"
  version := ProfileName.move.config.version
  checkReference := fun _ => #[]
  checkType := unsupportedExtension "Move" "type"
  checkOperation := unsupportedExtension "Move" "operation"
  checkSurface := unsupportedExtension "Move" "surface"
  checkProperty := checkMoveProperty

def rustSchema : ProfileSchema where
  profile := .rust
  name := "rust"
  version := ProfileName.rust.config.version
  checkReference := fun _ => #[]
  checkType := unsupportedExtension "Rust" "type"
  checkOperation := unsupportedExtension "Rust" "operation"
  checkSurface := unsupportedExtension "Rust" "surface"
  checkProperty := unsupportedExtension "Rust" "property"

def profileRegistry : ProfileRegistry := #[moveSchema, rustSchema]

/-- Apply the ordinary shared LIR validation boundary to LeanerLang output. -/
def validate (unit : RawUnit) :
    Except (Array LeanerIR.Validation.Diagnostic) ValidatedUnit :=
  LeanerIR.Validation.validate profileRegistry unit

inductive CompileError where
  | frontend (diagnostics : Array LeanerLang.Diagnostic)
  | lir (diagnostics : Array LeanerIR.Validation.Diagnostic)
  deriving Repr

/-- Lower and validate authored Leaner source without bypassing the public raw
frontend boundary. -/
def compile (unit : CompilationUnit) : Except CompileError ValidatedUnit := do
  let raw ← lower unit |>.mapError .frontend
  validate raw |>.mapError .lir

end LeanerLang

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
  checkOperation := fun value =>
    if value.tag == "runtime.vector_error" && value.payload.isEmpty then #[]
    else unsupportedExtension "Move" "operation" value
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

/-! ## Native models

A Move native without a specification is read as the Move Prover reads it:
by the model its Boogie prelude or its translator gives the native
(`prelude.bpl`, `native.bpl`, `aptos-natives.bpl`, the reflection calls of
`bytecode_translator.rs`), and otherwise not at all — the Prover rejects a
call to a native it does not model. The models mirror the Prover case by
case. -/

/-- The Prover's model of a native without a specification. -/
inductive NativeModel where
  /-- The result is an uninterpreted function of the type parameters and
  arguments — one value for equal arguments, the value a clause applying
  it denotes: the specification function `function` when given, else the
  native itself. The native aborts exactly when the specification predicate
  `abortsUnless`, applied to the type parameters and arguments, is false,
  and without one it does not abort. `resultLength` is the length the
  prelude states for a vector result. -/
  | uninterpreted (resultLength : Option Nat := none) (function : Option String := none)
      (abortsUnless : Option String := none)
  /-- The result is the variant of the native's three-variant result enum
  at the position of `RuntimeValue.order` of its two operands (less, equal,
  greater in declaration order), without abort. -/
  | structuralOrder
  deriving Repr, Inhabited

/-- The Move Prover's models, by qualified native name.
`hash::sha2_256`, `hash::sha3_256`: uninterpreted functions of the input
with a 32-byte result, called by procedures that do not abort.
`bcs::to_bytes`: `bcs::serialize` of the value, without abort.
`type_info::type_name` and `type_info::type_of`: functions of the type,
the latter aborting unless the type is a struct (`spec_is_struct`).
`cmp::compare`: the structural order, as the translator's per-type
`$1_cmp_$compare` (`aptos-natives.bpl`), which it refines for type
parameters with the runtime's order.
The prelude's injectivity and length axioms of `serialize` and the hashes,
and the concrete names the Prover computes for concrete types, are not
mirrored. -/
def moveNativeModels : List (String × NativeModel) := [
  ("0x1::hash::sha2_256", .uninterpreted (some 32)),
  ("0x1::hash::sha3_256", .uninterpreted (some 32)),
  ("0x1::bcs::to_bytes", .uninterpreted (function := some "0x1::bcs::serialize")),
  ("0x1::type_info::type_name", .uninterpreted),
  ("0x1::type_info::type_of",
    .uninterpreted (abortsUnless := some "0x1::type_info::spec_is_struct")),
  ("0x1::cmp::compare", .structuralOrder)]

/-- The model of a native of `profile` without a specification, if its
prelude gives one. -/
def nativeModel? (profile : Option Profile) (key : String) : Option NativeModel :=
  match profile with
  | some .move => (moveNativeModels.find? (·.1 == key)).map (·.2)
  | _ => none

/-- Whether a native is read by its prelude model: it has no specification,
or one without conditions that declares it intrinsic, deferring to the
prelude. -/
def readsNativeModel {β : Type} (declaration : LeanerIR.FunctionDecl β) : Bool :=
  declaration.contract.conditions.isEmpty &&
    (declaration.contract.loc.isNone || declaration.pragmas.any fun
      | .assign "intrinsic" (.constant (.bool false)) _ => false
      | .assign "intrinsic" _ _ => true
      | _ => false)

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

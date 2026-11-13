-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Lower
import VerexSL.Modifiers

/-!
# Verex frontend validation profile

The source package registers only the profile-neutral/core subset it emits.
Full Move and Rust profile packages may validate the same `RawUnit` with their
richer registries; this sibling library does not create a reverse dependency
on either package.
-/

namespace VerexSL

open VerexIR
open VerexIR.Import
open VerexIR.Validation

private def unsupportedExtension (profile kind : String)
    (value : ProfileValue) : Array VerexIR.Validation.Diagnostic :=
  #[.error "VEREX-PROFILE-EXTENSION"
    s!"the core-only VerexSL {profile} registry does not admit {kind} extension `{value.tag}`"]

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
    Array VerexIR.Validation.Diagnostic :=
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
    if VerexIR.moveTrapTags.contains value.tag && value.payload.isEmpty then #[]
    else unsupportedExtension "Move" "operation" value
  checkSurface := unsupportedExtension "Move" "surface"
  checkProperty := checkMoveProperty
  storableTarget := moveStorableTarget

def rustSchema : ProfileSchema where
  profile := .rust
  name := "rust"
  version := ProfileName.rust.config.version
  checkReference := fun _ => #[]
  checkType := unsupportedExtension "Rust" "type"
  checkOperation := unsupportedExtension "Rust" "operation"
  checkSurface := unsupportedExtension "Rust" "surface"
  checkProperty := fun value =>
    if value.tag == rustClosureTag || value.tag == rustRefusedTag ||
        (value.tag == rustOpaqueTag || value.tag == rustDependencyTag || value.tag == rustShapeOnlyTag ||
          value.tag == rustOpaqueFunctionTag || value.tag == destructorTag) &&
          value.payload.isEmpty
    then #[]
    else unsupportedExtension "Rust" "property" value
  opaqueNominal := fun properties => properties.any (·.tag == rustOpaqueTag)
  bodiless := fun properties => some (properties.any (·.tag == rustDependencyTag))

def profileRegistry : ProfileRegistry := #[moveSchema, rustSchema]

/-! ## Native models

A Move native without a specification is read as the Move Prover reads it:
by the model its Boogie prelude or its translator gives the native
(`prelude.bpl`, `native.bpl`, `aptos-natives.bpl`, the reflection calls of
`bytecode_translator.rs`), and otherwise not at all — the Prover rejects a
call to a native it does not model. The models mirror the Prover case by
case. -/

/-- What a native reads of its type argument (`VerexIR.Proofs.Denote.reflectedTypeName`). -/
inductive TypeReflection where
  /-- The canonical name, as a `string::String`. -/
  | name
  /-- The address, module, and name with type arguments, as a `TypeInfo`. -/
  | info
  deriving Repr, Inhabited

/-- An `aggregator_v2` native the prelude implements. -/
inductive AggregatorOperation where
  | create
  | createUnbounded
  | tryAdd
  | trySub
  | isAtLeast
  | read
  deriving Repr, Inhabited, BEq

/-- The Prover's model of a native without a specification. -/
inductive NativeModel where
  /-- The result is an uninterpreted function of the type parameters and
  arguments — one value for equal arguments, the value a clause applying
  it denotes: the specification function `function` when given, else the
  native itself. The native aborts exactly when the specification predicate
  `abortsUnless`, applied to the type parameters and arguments, is false,
  and without one it does not abort. `resultLength` is the length the
  prelude states for a vector result. With `reflection`, the value at a type
  mentioning no parameter is the one the translator computes of the type. -/
  | uninterpreted (resultLength : Option Nat := none) (function : Option String := none)
      (abortsUnless : Option String := none) (reflection : Option TypeReflection := none)
  /-- The result is the variant of the native's three-variant result enum
  at the position of `RuntimeValue.order` of its two operands (less, equal,
  greater in declaration order), without abort. -/
  | structuralOrder
  /-- The prelude's intrinsic of `aggregator_v2`, by the element type: at
  `u64` and `u128` exact arithmetic over the `value` and `max_value` fields;
  it aborts unless the element's `type_name` is one of those two, which at
  any other concrete type it is not; at a type parameter that does not abort
  the result and the referent are unconstrained. -/
  | aggregator (operation : AggregatorOperation)
  deriving Repr, Inhabited

/-- The Move Prover's models, by qualified native name.
`hash::sha2_256`, `hash::sha3_256`: uninterpreted functions of the input
with a 32-byte result, called by procedures that do not abort.
`bcs::to_bytes`: `bcs::serialize` of the value, without abort.
`type_info::type_name` and `type_info::type_of`: functions of the type,
computed at a type mentioning no parameter as the translator computes them,
the latter aborting unless the type is a struct (`spec_is_struct`).
`cmp::compare`: the structural order, as the translator's per-type
`$1_cmp_$compare` (`aptos-natives.bpl`), which it refines for type
parameters with the runtime's order.
A model holds beside what the native's specification states, as the
prelude defines a native whatever its specification adds. The prelude's
injectivity of the hashes is contradictory with their fixed length; a
function assumes it of its parameters instead (`moveCollisionFreeHashes`).
`aggregator_v2`: the `{% for instance in aggregator_v2_instances %}` procedures
of `aptos-natives.bpl`. -/
def moveNativeModels : List (String × NativeModel) := [
  ("0x1::hash::sha2_256", .uninterpreted (some 32)),
  ("0x1::hash::sha3_256", .uninterpreted (some 32)),
  ("0x1::bcs::to_bytes", .uninterpreted (function := some "0x1::bcs::serialize")),
  ("0x1::type_info::type_name", .uninterpreted (reflection := some .name)),
  ("0x1::type_info::type_of",
    .uninterpreted (abortsUnless := some "0x1::type_info::spec_is_struct")
      (reflection := some .info)),
  ("0x1::cmp::compare", .structuralOrder),
  ("0x1::aggregator_v2::create_aggregator", .aggregator .create),
  ("0x1::aggregator_v2::create_unbounded_aggregator", .aggregator .createUnbounded),
  ("0x1::aggregator_v2::try_add", .aggregator .tryAdd),
  ("0x1::aggregator_v2::try_sub", .aggregator .trySub),
  ("0x1::aggregator_v2::is_at_least_impl", .aggregator .isAtLeast),
  ("0x1::aggregator_v2::read", .aggregator .read)]

/-- The native the aggregator model reads an element type's name by. -/
def moveTypeNameNative : String := "0x1::type_info::type_name"

/-- What the prelude states of a field: an argument at a position, or zero. -/
inductive StatedField where
  | argument (index : Nat)
  | zero
  deriving Repr, Inhabited

/-- The meaning the Move Prover's prelude gives a specification function
beyond an uninterpreted one. -/
inductive SpecificationModel where
  /-- A byte vector of the value that is never empty, and nothing else
  (`VerexIR.Proofs.serializedBytes`). The prelude also states the serialization
  injective and every address of one length, which no function into
  bounded byte vectors is of every value; a function assumes those of its
  parameters instead (`Contract.serializeFacts`). -/
  | serialization
  /-- Whether a type is a struct or enum, an uninterpreted value of a type
  parameter (`VerexIR.Proofs.Denote.reflectedIsStruct`). -/
  | isStruct
  /-- The field at a position of a structure operand. -/
  | field (index : Nat)
  /-- A structure operand with its field at a position the second operand. -/
  | setField (index : Nat)
  /-- An uninterpreted value of a structure at its arguments, with fields at
  positions the prelude states of every such value. -/
  | stated (fields : List (Nat × StatedField))
  deriving Repr, Inhabited

/-- The Move Prover's meanings of specification functions, by qualified
name: `bcs::serialize`, of which the prelude states every value's
serialization not empty, `type_info::spec_is_struct`, which the
translator computes, the fields `aggregator_v2::spec_get_value` and
`spec_get_max_value` read, and the operations of `aggregator`'s model
(`Frontend/VIR/Encode.lean`, `moveModelFields`): those `spec_get_handle`, `spec_get_key`, `spec_get_limit`,
`spec_read` and `spec_aggregator_get_val` read, the one `spec_aggregator_set_val`
writes, and the limit and value the prelude's axioms state of `spec_new_aggregator`,
which hold of its value by construction. -/
def moveSpecificationModels : List (String × SpecificationModel) := [
  ("0x1::bcs::serialize", .serialization),
  ("0x1::type_info::spec_is_struct", .isStruct),
  ("0x1::aggregator_v2::spec_get_value", .field 0),
  ("0x1::aggregator_v2::spec_get_max_value", .field 1),
  ("0x1::aggregator::spec_get_handle", .field 0),
  ("0x1::aggregator::spec_get_key", .field 1),
  ("0x1::aggregator::spec_get_limit", .field 2),
  ("0x1::aggregator::spec_read", .field 3),
  ("0x1::aggregator::spec_aggregator_get_val", .field 3),
  ("0x1::aggregator::spec_aggregator_set_val", .setField 3),
  ("0x1::aggregator_factory::spec_new_aggregator", .stated [(2, .argument 0), (3, .zero)])]

/-- The prelude's meaning of a specification function of `profile`, if it
gives one beyond an uninterpreted one. -/
def specificationModel? (profile : Option Profile) (key : String) : Option SpecificationModel :=
  match profile with
  | some .move => (moveSpecificationModels.find? (·.1 == key)).map (·.2)
  | _ => none

/-- The hashes a Move function that applies them assumes collision-free on
its byte-vector parameters (decided 2026-10-02). The Move Prover assumes
them injective,
which no function with fixed-length results is; stated of the values a
function takes, the assumption is consistent. -/
def moveCollisionFreeHashes : List String := ["0x1::hash::sha2_256", "0x1::hash::sha3_256"]

/-- The model of a native of `profile` without a specification, if its
prelude gives one. -/
def nativeModel? (profile : Option Profile) (key : String) : Option NativeModel :=
  match profile with
  | some .move => (moveNativeModels.find? (·.1 == key)).map (·.2)
  | _ => none

/-- Whether a native is read by its prelude model: it has no specification,
or one without conditions that declares it intrinsic, deferring to the
prelude. -/
def readsNativeModel {β : Type} (declaration : VerexIR.FunctionDecl β) : Bool :=
  declaration.contract.conditions.all isImplicitNoPanic &&
    (declaration.contract.loc.isNone || declaration.pragmas.any fun
      | .assign "intrinsic" (.constant (.bool false)) _ => false
      | .assign "intrinsic" _ _ => true
      | _ => false)

/-- Apply the ordinary shared VIR validation boundary to VerexSL output. -/
def validate (unit : RawUnit) :
    Except (Array VerexIR.Validation.Diagnostic) ValidatedUnit :=
  VerexIR.Validation.validate profileRegistry unit

inductive CompileError where
  | frontend (diagnostics : Array VerexSL.Diagnostic)
  | vir (diagnostics : Array VerexIR.Validation.Diagnostic)
  deriving Repr

/-- Lower and validate authored Verex source without bypassing the public raw
frontend boundary. -/
def compile (unit : CompilationUnit) : Except CompileError ValidatedUnit := do
  let raw ← lower unit |>.mapError .frontend
  validate raw |>.mapError .vir

end VerexSL

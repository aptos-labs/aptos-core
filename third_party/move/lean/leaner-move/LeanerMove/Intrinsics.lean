-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR

/-!
# Move intrinsic role schemas

This module owns the closed Move `map` intrinsic vocabulary. The neutral LIR
checker owns declaration identity and graph integrity; this profile layer owns
model names, role categories, required roles, dependencies, owner shape, and
the physical signature alternatives accepted by each role.
-/

namespace LeanerIR.Move.Intrinsics

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

inductive MapRole where
  | addAll | addNoOverride | addOverrideIfExists | append | appendDisjoint
  | backKey | borrow | borrowBack | borrowFront | borrowMut
  | borrowMutWithDefault | borrowWithDefault | delMustExist | delReturnKey
  | destroyEmpty | frontKey | get | hasKey | isEmpty | iterBorrowMut | keys
  | len | new_ | newFrom | newWithConfig | nextKey | popBack | popFront
  | prevKey | removeOrNone | replaceKeyInplace | toOrderedMap | toVecPair
  | trim | upsert | upsertAll | values
  | specAbortsAdd | specAbortsAddAll | specAbortsAppendDisjoint
  | specAbortsBorrow | specAbortsDel | specAbortsDestroyEmpty | specAbortsEmpty
  | specAbortsIterBorrowMut | specAbortsNewFrom | specAbortsNewWithConfig
  | specAbortsReplaceKeyInplace | specAbortsTrim | specAbortsUpsertAll
  | specDel | specGet | specHasKey | specIterPreserved | specIterValid
  | specKeyAt | specLeafIterValid | specLeafOffset | specLen | specNew
  | specRank | specSet
  deriving Repr, BEq, DecidableEq, Inhabited

inductive TargetKind where
  | executable
  | specification
  deriving Repr, BEq, DecidableEq, Inhabited

inductive Presence where
  | required
  | optional
  deriving Repr, BEq, DecidableEq, Inhabited

inductive NominalScope where
  | ownerNamespace
  | namespace (address module : String)
  deriving Repr, BEq, Inhabited

/-- A physical type position in a map-role signature. `owner`, `key`, and
`value` are interpreted relative to the intrinsic declaration; named nominal
patterns deliberately match declaration identity as well as exact type
arguments. -/
inductive TypePattern where
  | unit
  | bool
  | unsigned (bits : Nat)
  | num
  | key
  | value
  | owner
  | vector (element : TypePattern)
  | tuple (elements : Array TypePattern)
  | reference (kind : ReferenceKind) (referent : TypePattern)
  | nominal (scope : NominalScope) (name : String) (arguments : Array TypePattern)
  deriving Repr, BEq, Inhabited

structure SignaturePattern where
  parameters : Array TypePattern
  result : TypePattern
  deriving Repr, BEq, Inhabited

private def signature (parameters : Array TypePattern)
    (result : TypePattern) : SignaturePattern :=
  { parameters, result }

private abbrev key := TypePattern.key
private abbrev value := TypePattern.value
private abbrev owner := TypePattern.owner
private abbrev unit := TypePattern.unit
private abbrev bool := TypePattern.bool
private abbrev u16 := TypePattern.unsigned 16
private abbrev u64 := TypePattern.unsigned 64
private abbrev num := TypePattern.num
private abbrev refKey := TypePattern.reference .shared .key
private abbrev refValue := TypePattern.reference .shared .value
private abbrev mutValue := TypePattern.reference .mutable .value
private abbrev refOwner := TypePattern.reference .shared .owner
private abbrev mutOwner := TypePattern.reference .mutable .owner
private abbrev keys := TypePattern.vector .key
private abbrev values := TypePattern.vector .value
private abbrev optionKey := TypePattern.nominal (.namespace "0x1" "option") "Option" #[.key]
private abbrev optionValue := TypePattern.nominal (.namespace "0x1" "option") "Option" #[.value]
private abbrev iterator := TypePattern.nominal .ownerNamespace "IteratorPtr" #[]
private abbrev keyIterator := TypePattern.nominal .ownerNamespace "IteratorPtr" #[.key]
private abbrev leafIterator := TypePattern.nominal .ownerNamespace "LeafNodeIteratorPtr" #[]
private abbrev orderedMap :=
  TypePattern.nominal (.namespace "0x1" "ordered_map") "OrderedMap" #[.key, .value]

/-- Exact signature alternatives observed across the six supported map owners.
Specification-function positions use Move's logical integer domain, so direct
fixed-width integer parameters are represented by `num`. The two alternatives
on lookup-like roles encode the intentional difference between handles which
copy keys and containers which borrow them. -/
def signaturePatterns : MapRole → Array SignaturePattern
  | .addAll => #[signature #[mutOwner, keys, values] unit]
  | .addNoOverride | .addOverrideIfExists => #[signature #[mutOwner, key, value] unit]
  | .append | .appendDisjoint => #[signature #[mutOwner, owner] unit]
  | .backKey | .frontKey => #[signature #[refOwner] key]
  | .borrow => #[
      signature #[refOwner, refKey] refValue,
      signature #[refOwner, key] refValue]
  | .borrowBack | .borrowFront => #[
      signature #[refOwner] (.tuple #[key, refValue]),
      signature #[refOwner] (.tuple #[refKey, refValue])]
  | .borrowMut => #[
      signature #[mutOwner, refKey] mutValue,
      signature #[mutOwner, key] mutValue]
  | .borrowMutWithDefault => #[signature #[mutOwner, key, value] mutValue]
  | .borrowWithDefault => #[signature #[refOwner, key, refValue] refValue]
  | .delMustExist => #[
      signature #[mutOwner, refKey] value,
      signature #[mutOwner, key] value]
  | .delReturnKey => #[signature #[mutOwner, refKey] (.tuple #[key, value])]
  | .destroyEmpty => #[signature #[owner] unit]
  | .get => #[signature #[refOwner, refKey] optionValue]
  | .hasKey => #[
      signature #[refOwner, refKey] bool,
      signature #[refOwner, key] bool]
  | .isEmpty => #[signature #[refOwner] bool]
  | .iterBorrowMut => #[
      signature #[iterator, mutOwner] mutValue,
      signature #[keyIterator, mutOwner] mutValue]
  | .keys => #[signature #[refOwner] keys]
  | .len => #[signature #[refOwner] u64]
  | .new_ => #[signature #[] owner]
  | .newFrom => #[signature #[keys, values] owner]
  | .newWithConfig => #[signature #[u16, u16, bool] owner]
  | .nextKey | .prevKey => #[signature #[refOwner, refKey] optionKey]
  | .popBack | .popFront => #[signature #[mutOwner] (.tuple #[key, value])]
  | .removeOrNone => #[signature #[mutOwner, refKey] optionValue]
  | .replaceKeyInplace => #[signature #[mutOwner, refKey, key] unit]
  | .toOrderedMap => #[signature #[refOwner] orderedMap]
  | .toVecPair => #[signature #[owner] (.tuple #[keys, values])]
  | .trim => #[signature #[mutOwner, u64] owner]
  | .upsert => #[signature #[mutOwner, key, value] optionValue]
  | .upsertAll => #[signature #[mutOwner, keys, values] unit]
  | .values => #[signature #[refOwner] values]
  | .specAbortsAdd => #[signature #[owner, key, value] bool]
  | .specAbortsAddAll => #[signature #[owner, keys, values] bool]
  | .specAbortsAppendDisjoint => #[signature #[owner, owner] bool]
  | .specAbortsBorrow | .specAbortsDel => #[signature #[owner, key] bool]
  | .specAbortsDestroyEmpty | .specAbortsEmpty => #[signature #[owner] bool]
  | .specAbortsIterBorrowMut => #[
      signature #[iterator, owner] bool,
      signature #[keyIterator, owner] bool]
  | .specAbortsNewFrom => #[signature #[keys, values] bool]
  | .specAbortsNewWithConfig => #[signature #[num, num, bool] bool]
  | .specAbortsReplaceKeyInplace => #[signature #[owner, key, key] bool]
  | .specAbortsTrim => #[signature #[owner, num] bool]
  | .specAbortsUpsertAll => #[signature #[owner, keys, values] bool]
  | .specDel => #[signature #[owner, key] owner]
  | .specGet => #[signature #[owner, key] value]
  | .specHasKey => #[signature #[owner, key] bool]
  | .specIterPreserved => #[signature #[owner, owner] bool]
  | .specIterValid => #[signature #[keyIterator, owner] bool]
  | .specKeyAt => #[signature #[owner, num] key]
  | .specLeafIterValid => #[signature #[leafIterator, owner] bool]
  | .specLeafOffset => #[signature #[leafIterator, owner] num]
  | .specLen => #[signature #[owner] num]
  | .specNew => #[signature #[] owner]
  | .specRank => #[signature #[owner, key] num]
  | .specSet => #[signature #[owner, key, value] owner]

structure RoleSchema where
  role : MapRole
  sourceName : String
  kind : TargetKind
  presence : Presence := .optional
  dependencies : Array MapRole := #[]
  signatures : Array SignaturePattern
  deriving Repr, BEq, Inhabited

private def executable (role : MapRole) (sourceName : String)
    (dependencies : Array MapRole := #[]) : RoleSchema :=
  { role, sourceName, kind := .executable, dependencies,
    signatures := signaturePatterns role }

private def specification (role : MapRole) (sourceName : String)
    (presence : Presence := .optional)
    (dependencies : Array MapRole := #[]) : RoleSchema :=
  { role, sourceName, kind := .specification, presence, dependencies,
    signatures := signaturePatterns role }

/-- The complete role vocabulary transported by the six Aptos map fixtures.
Dependencies name logical observations needed to interpret a bound executable
role; optional abort predicates refine summaries but are not prerequisites. -/
def roleSchemas : Array RoleSchema := #[
  executable .addAll "map_add_all" #[.specHasKey, .specSet],
  executable .addNoOverride "map_add_no_override" #[.specHasKey, .specSet],
  executable .addOverrideIfExists "map_add_override_if_exists" #[.specSet],
  executable .append "map_append" #[.specHasKey, .specSet],
  executable .appendDisjoint "map_append_disjoint" #[.specHasKey, .specSet],
  executable .backKey "map_back_key" #[.specKeyAt, .specLen],
  executable .borrow "map_borrow" #[.specGet],
  executable .borrowBack "map_borrow_back" #[.specGet, .specKeyAt, .specLen],
  executable .borrowFront "map_borrow_front" #[.specGet, .specKeyAt],
  executable .borrowMut "map_borrow_mut" #[.specGet, .specSet],
  executable .borrowMutWithDefault "map_borrow_mut_with_default" #[.specGet, .specSet],
  executable .borrowWithDefault "map_borrow_with_default" #[.specGet],
  executable .delMustExist "map_del_must_exist" #[.specHasKey, .specDel],
  executable .delReturnKey "map_del_return_key" #[.specHasKey, .specDel],
  executable .destroyEmpty "map_destroy_empty",
  executable .frontKey "map_front_key" #[.specKeyAt],
  executable .get "map_get" #[.specGet],
  executable .hasKey "map_has_key" #[.specHasKey],
  executable .isEmpty "map_is_empty" #[.specLen],
  executable .iterBorrowMut "map_iter_borrow_mut" #[.specGet, .specSet],
  executable .keys "map_keys" #[.specKeyAt, .specLen],
  executable .len "map_len" #[.specLen],
  executable .new_ "map_new",
  executable .newFrom "map_new_from" #[.specHasKey, .specSet],
  executable .newWithConfig "map_new_with_config",
  executable .nextKey "map_next_key" #[.specKeyAt, .specRank],
  executable .popBack "map_pop_back" #[.specGet, .specDel, .specKeyAt, .specLen],
  executable .popFront "map_pop_front" #[.specGet, .specDel, .specKeyAt],
  executable .prevKey "map_prev_key" #[.specKeyAt, .specRank],
  executable .removeOrNone "map_remove_or_none" #[.specHasKey, .specDel],
  executable .replaceKeyInplace "map_replace_key_inplace" #[.specHasKey, .specDel, .specSet],
  executable .toOrderedMap "map_to_ordered_map" #[.specGet, .specKeyAt, .specLen],
  executable .toVecPair "map_to_vec_pair" #[.specGet, .specKeyAt, .specLen],
  executable .trim "map_trim" #[.specDel, .specLen],
  executable .upsert "map_upsert" #[.specSet],
  executable .upsertAll "map_upsert_all" #[.specSet],
  executable .values "map_values" #[.specGet, .specKeyAt, .specLen],
  specification .specAbortsAdd "map_spec_aborts_add",
  specification .specAbortsAddAll "map_spec_aborts_add_all",
  specification .specAbortsAppendDisjoint "map_spec_aborts_append_disjoint",
  specification .specAbortsBorrow "map_spec_aborts_borrow",
  specification .specAbortsDel "map_spec_aborts_del",
  specification .specAbortsDestroyEmpty "map_spec_aborts_destroy_empty",
  specification .specAbortsEmpty "map_spec_aborts_empty",
  specification .specAbortsIterBorrowMut "map_spec_aborts_iter_borrow_mut",
  specification .specAbortsNewFrom "map_spec_aborts_new_from",
  specification .specAbortsNewWithConfig "map_spec_aborts_new_with_config",
  specification .specAbortsReplaceKeyInplace "map_spec_aborts_replace_key_inplace",
  specification .specAbortsTrim "map_spec_aborts_trim",
  specification .specAbortsUpsertAll "map_spec_aborts_upsert_all",
  specification .specDel "map_spec_del" .required,
  specification .specGet "map_spec_get" .required,
  specification .specHasKey "map_spec_has_key" .required,
  specification .specIterPreserved "map_spec_iter_preserved",
  specification .specIterValid "map_spec_iter_valid",
  specification .specKeyAt "map_spec_key_at",
  specification .specLeafIterValid "map_spec_leaf_iter_valid",
  specification .specLeafOffset "map_spec_leaf_offset",
  specification .specLen "map_spec_len",
  specification .specNew "map_spec_new",
  specification .specRank "map_spec_rank",
  specification .specSet "map_spec_set" .required
]

def allRoles : Array MapRole := #[
  .addAll, .addNoOverride, .addOverrideIfExists, .append, .appendDisjoint,
  .backKey, .borrow, .borrowBack, .borrowFront, .borrowMut,
  .borrowMutWithDefault, .borrowWithDefault, .delMustExist, .delReturnKey,
  .destroyEmpty, .frontKey, .get, .hasKey, .isEmpty, .iterBorrowMut, .keys,
  .len, .new_, .newFrom, .newWithConfig, .nextKey, .popBack, .popFront,
  .prevKey, .removeOrNone, .replaceKeyInplace, .toOrderedMap, .toVecPair,
  .trim, .upsert, .upsertAll, .values,
  .specAbortsAdd, .specAbortsAddAll, .specAbortsAppendDisjoint,
  .specAbortsBorrow, .specAbortsDel, .specAbortsDestroyEmpty, .specAbortsEmpty,
  .specAbortsIterBorrowMut, .specAbortsNewFrom, .specAbortsNewWithConfig,
  .specAbortsReplaceKeyInplace, .specAbortsTrim, .specAbortsUpsertAll,
  .specDel, .specGet, .specHasKey, .specIterPreserved, .specIterValid,
  .specKeyAt, .specLeafIterValid, .specLeafOffset, .specLen, .specNew,
  .specRank, .specSet
]

def roleSchema? (name : String) : Option RoleSchema :=
  roleSchemas.find? (fun schema => schema.sourceName == name)

def registryComplete : Bool :=
  roleSchemas.size == 62 && allRoles.size == 62 &&
    roleSchemas.all (fun schema => allRoles.contains schema.role) &&
    roleSchemas.all (fun schema => !schema.signatures.isEmpty) &&
    allRoles.all (fun role => roleSchemas.countP (fun schema => schema.role == role) == 1) &&
    roleSchemas.all (fun schema =>
      roleSchemas.countP (fun other => other.sourceName == schema.sourceName) == 1) &&
    roleSchemas.countP (fun schema => schema.kind == .executable) == 37 &&
    roleSchemas.countP (fun schema => schema.kind == .specification) == 25

private def diagnostic (code message : String) (loc : LocId) : Array Diagnostic :=
  #[.at code message loc]

private def checkOwnerShape (ns : RawNamespace) (intrinsic : IntrinsicDecl) : Array Diagnostic :=
  match ns.structs.find? (fun declaration => declaration.name == intrinsic.owner) with
  | none => #[]
  | some owner =>
      if owner.generics.size == 2 && owner.generics.all (fun binder => binder.kind == .typeArg) then
        #[]
      else diagnostic "LIR-MOVE-INTRINSIC-OWNER-SHAPE"
        "Move map intrinsic owner must declare exactly two type parameters" intrinsic.loc

private partial def matchesType (unit : RawUnit) (ownerName : NameId)
    (pattern : TypePattern) (typeId : TypeId) : Bool :=
  let matchesPatterns (patterns : Array TypePattern) (types : Array TypeId) : Bool :=
    patterns.size == types.size &&
      (patterns.zip types).all fun (pattern, typeId) =>
        matchesType unit ownerName pattern typeId
  let matchesArguments (patterns : Array TypePattern)
      (arguments : Array GenericArgument) : Bool :=
    patterns.size == arguments.size &&
      (patterns.zip arguments).all fun (pattern, argument) =>
        match argument with
        | .typeArg typeUse => matchesType unit ownerName pattern typeUse.typeId
        | _ => false
  let matchesScope (scope : NominalScope) (actual : QualifiedName) : Bool :=
    match scope with
    | .ownerNamespace =>
        unit.tables.names[ownerName.index]?.any fun owner =>
          actual.namespaceId == owner.namespaceId
    | .namespace address module =>
        unit.tables.namespaces[actual.namespaceId.index]?.any fun namespaceRef =>
          namespaceRef.segments[0]? == some address &&
            namespaceRef.segments.back? == some module
  match unit.tables.types[typeId.index]? with
  | none => false
  | some actual => match pattern, actual with
      | .unit, .unit => true
      | .unit, .tuple elements => elements.isEmpty
      | .bool, .bool => true
      | .unsigned bits, .integer (.bits actualBits) false => bits == actualBits
      | .num, .integer .unbounded true => true
      | .key, .typeParameter 0 | .value, .typeParameter 1 => true
      | .owner, .nominal name arguments =>
          name == ownerName && matchesArguments #[.key, .value] arguments
      | .vector element, .vector actualElement none =>
          matchesType unit ownerName element actualElement
      | .tuple elements, .tuple actualElements =>
          matchesPatterns elements actualElements
      | .reference kind referent, .reference actualReference =>
          actualReference.profile == .move && actualReference.kind == kind &&
            matchesType unit ownerName referent actualReference.referent
      | .nominal scope expected arguments, .nominal name actualArguments =>
          match unit.tables.names[name.index]? with
          | some actualName =>
              matchesScope scope actualName && actualName.name == expected &&
                matchesArguments arguments actualArguments
          | none => false
      | _, _ => false

private def matchesSignature (unit : RawUnit) (ownerName : NameId)
    (pattern : SignaturePattern) (actual : Signature) : Bool :=
  actual.generics.size == 2 &&
    actual.generics.all (fun binder => binder.kind == .typeArg) &&
    pattern.parameters.size == actual.parameters.size &&
    (pattern.parameters.zip actual.parameters).all (fun (expected, parameter) =>
      matchesType unit ownerName expected parameter.typeUse.typeId) &&
    actual.results.size == 1 &&
    matchesType unit ownerName pattern.result actual.results[0]!.typeId

private def targetSignature? (unit : RawUnit) (expected : TargetKind)
    (binding : IntrinsicBinding) : Option (Signature × LocId) := do
  let targetNamespace ← unit.namespaces.find? (fun ns => ns.identity == binding.target.namespaceId)
  match expected with
  | .executable =>
      let declaration ← targetNamespace.functions.find? (fun declaration =>
        declaration.name == binding.target.name)
      return (declaration.signature, declaration.loc)
  | .specification =>
      let declaration ← targetNamespace.specFunctions.find? (fun declaration =>
        declaration.name == binding.target.name)
      return (declaration.signature, declaration.loc)

private def checkSignature (unit : RawUnit) (intrinsic : IntrinsicDecl)
    (schema : RoleSchema) (binding : IntrinsicBinding) : Array Diagnostic :=
  match targetSignature? unit schema.kind binding with
  | none => #[]
  | some (actual, targetLoc) =>
      if schema.signatures.any (fun expected =>
          matchesSignature unit intrinsic.owner expected actual) then
        #[]
      else
        #[{
          code := "LIR-MOVE-INTRINSIC-SIGNATURE"
          message := s!"Move map intrinsic role `{binding.role}` target has an incompatible physical signature"
          primary := some binding.loc
          related := #[{ loc := targetLoc, message := "bound declaration" }] }]

private def checkBinding (unit : RawUnit) (intrinsic : IntrinsicDecl)
    (expected : TargetKind) (binding : IntrinsicBinding) : Array Diagnostic :=
  match roleSchema? binding.role with
  | none => diagnostic "LIR-MOVE-INTRINSIC-ROLE"
      s!"unknown Move map intrinsic role `{binding.role}`" binding.loc
  | some schema =>
      if schema.kind == expected then
        checkSignature unit intrinsic schema binding
      else
        diagnostic "LIR-MOVE-INTRINSIC-ROLE-KIND"
          s!"Move map intrinsic role `{binding.role}` is bound to the wrong declaration kind"
          binding.loc

private structure BoundRole where
  schema : RoleSchema
  loc : LocId

private def boundRoleSchemas (intrinsic : IntrinsicDecl) : Array BoundRole :=
  (intrinsic.executableBindings ++ intrinsic.specBindings).filterMap fun binding => do
    let schema ← roleSchema? binding.role
    return { schema, loc := binding.loc }

private def checkRequiredRoles (intrinsic : IntrinsicDecl)
    (bound : Array BoundRole) : Array Diagnostic :=
  roleSchemas.foldl (fun ds schema =>
    if schema.presence == .required &&
        !bound.any (fun present => present.schema.role == schema.role) then
      ds ++ diagnostic "LIR-MOVE-INTRINSIC-ROLE-REQUIRED"
        s!"Move map intrinsic is missing required role `{schema.sourceName}`" intrinsic.loc
    else ds) #[]

private def checkDependencies (bound : Array BoundRole) : Array Diagnostic :=
  bound.foldl (fun ds present =>
    present.schema.dependencies.foldl (fun ds dependency =>
      if bound.any (fun candidate => candidate.schema.role == dependency) then ds
      else
        let dependencyName := (roleSchemas.find? (fun candidate =>
          candidate.role == dependency)).map (·.sourceName) |>.getD (repr dependency)
        ds ++ diagnostic "LIR-MOVE-INTRINSIC-ROLE-DEPENDENCY"
          s!"Move map intrinsic role `{present.schema.sourceName}` requires `{dependencyName}`"
          present.loc) ds) #[]

def check (unit : RawUnit) (ns : RawNamespace)
    (intrinsic : IntrinsicDecl) : Array Diagnostic :=
  if intrinsic.model != "map" then
    diagnostic "LIR-MOVE-INTRINSIC-MODEL"
      s!"unknown Move intrinsic model `{intrinsic.model}`" intrinsic.loc
  else
    let bindingErrors := intrinsic.executableBindings.foldl (fun ds binding =>
        ds ++ checkBinding unit intrinsic .executable binding) #[] ++
      intrinsic.specBindings.foldl (fun ds binding =>
        ds ++ checkBinding unit intrinsic .specification binding) #[]
    let bound := boundRoleSchemas intrinsic
    checkOwnerShape ns intrinsic ++ bindingErrors ++
      checkRequiredRoles intrinsic bound ++ checkDependencies bound

end LeanerIR.Move.Intrinsics

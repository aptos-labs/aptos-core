-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR

/-!
# Move intrinsic role schemas

This module owns the closed Move `map` intrinsic vocabulary. The neutral VIR
checker owns declaration identity and graph integrity; this profile layer owns
model names, role categories, required roles, dependencies, owner shape, and
the physical signature alternatives accepted by each role.
-/

namespace VerexIR.Move.Intrinsics

open VerexIR
open VerexIR.Import
open VerexIR.Validation

-- The roles are the shared vocabulary of the intrinsic (`VerexIR.MapRole`);
-- this profile owns their presence, dependencies, owner shape, and physical
-- signatures.
export VerexIR (MapRole MapOperation MapSpecRole)

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
  /-- Any nominal declared in the owner's namespace: the iterator of a validity
  predicate, whose shape the predicate's own binding fixes. -/
  | ownerNominal
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
private abbrev anyIterator := TypePattern.ownerNominal
private abbrev orderedMap :=
  TypePattern.nominal (.namespace "0x1" "ordered_map") "OrderedMap" #[.key, .value]

/-- Exact signature alternatives observed across the six supported map owners.
Specification-function positions use Move's logical integer domain, so direct
fixed-width integer parameters are represented by `num`. The two alternatives
on lookup-like roles encode the intentional difference between handles which
copy keys and containers which borrow them. -/
def operationSignatures : MapOperation → Array SignaturePattern
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
  | .delReturnKey => #[
      signature #[mutOwner, refKey] (.tuple #[key, value]),
      signature #[mutOwner, key] (.tuple #[key, value])]
  | .destroyEmpty | .destroyUnchecked => #[signature #[owner] unit]
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

def specSignatures : MapSpecRole → Array SignaturePattern
  | .abortsAdd => #[signature #[owner, key, value] bool]
  | .abortsAddAll => #[signature #[owner, keys, values] bool]
  | .abortsAppendDisjoint => #[signature #[owner, owner] bool]
  | .abortsBorrow | .abortsDel => #[signature #[owner, key] bool]
  | .abortsDestroyEmpty | .abortsEmpty | .isEmpty => #[signature #[owner] bool]
  | .abortsIterBorrowMut => #[
      signature #[iterator, owner] bool,
      signature #[keyIterator, owner] bool]
  | .abortsNewFrom => #[signature #[keys, values] bool]
  | .abortsNewWithConfig => #[signature #[num, num, bool] bool]
  | .abortsReplaceKeyInplace => #[signature #[owner, key, key] bool]
  | .abortsTrim => #[signature #[owner, num] bool]
  | .abortsUpsertAll => #[signature #[owner, keys, values] bool]
  | .del => #[signature #[owner, key] owner]
  | .get => #[signature #[owner, key] value]
  | .hasKey => #[signature #[owner, key] bool]
  | .iterPreserved => #[signature #[owner, owner] bool]
  | .iterValid => #[signature #[anyIterator, owner] bool]
  | .keyAt | .insertionKeyAt => #[signature #[owner, num] key]
  | .leafIterValid => #[signature #[anyIterator, owner] bool]
  | .leafOffset => #[signature #[anyIterator, owner] num]
  | .len => #[signature #[owner] num]
  | .new_ => #[signature #[] owner]
  | .rank | .insertionRank => #[signature #[owner, key] num]
  | .set => #[signature #[owner, key, value] owner]

def signaturePatterns : MapRole → Array SignaturePattern
  | .executable operation => operationSignatures operation
  | .specification role => specSignatures role

structure RoleSchema where
  role : MapRole
  presence : Presence := .optional
  /-- The logical observations needed to interpret a bound role. -/
  dependencies : Array MapSpecRole := #[]
  signatures : Array SignaturePattern
  deriving Repr, BEq, Inhabited

def RoleSchema.kind (schema : RoleSchema) : TargetKind :=
  match schema.role with
  | .executable _ => .executable
  | .specification _ => .specification

def RoleSchema.sourceName (schema : RoleSchema) : String := schema.role.name

private def executable (operation : MapOperation)
    (dependencies : Array MapSpecRole := #[]) : RoleSchema :=
  { role := .executable operation, dependencies,
    signatures := operationSignatures operation }

private def specification (role : MapSpecRole) (presence : Presence := .optional) : RoleSchema :=
  { role := .specification role, presence, signatures := specSignatures role }

/-- The complete role vocabulary transported by the six Aptos map fixtures.
Dependencies name logical observations needed to interpret a bound executable
role; optional abort predicates refine summaries but are not prerequisites. -/
def roleSchemas : Array RoleSchema := #[
  executable .addAll #[.hasKey, .set],
  executable .addNoOverride #[.hasKey, .set],
  executable .addOverrideIfExists #[.set],
  executable .append #[.hasKey, .set],
  executable .appendDisjoint #[.hasKey, .set],
  executable .backKey #[.keyAt, .len],
  executable .borrow #[.get],
  executable .borrowBack #[.get, .keyAt, .len],
  executable .borrowFront #[.get, .keyAt],
  executable .borrowMut #[.get, .set],
  executable .borrowMutWithDefault #[.get, .set],
  executable .borrowWithDefault #[.get],
  executable .delMustExist #[.hasKey, .del],
  executable .delReturnKey #[.hasKey, .del],
  executable .destroyEmpty,
  executable .destroyUnchecked,
  executable .frontKey #[.keyAt],
  executable .get #[.get],
  executable .hasKey #[.hasKey],
  executable .isEmpty #[.len],
  executable .iterBorrowMut #[.get, .set],
  executable .keys #[.keyAt, .len],
  executable .len #[.len],
  executable .new_,
  executable .newFrom #[.hasKey, .set],
  executable .newWithConfig,
  executable .nextKey #[.keyAt, .rank],
  executable .popBack #[.get, .del, .keyAt, .len],
  executable .popFront #[.get, .del, .keyAt],
  executable .prevKey #[.keyAt, .rank],
  executable .removeOrNone #[.hasKey, .del],
  executable .replaceKeyInplace #[.hasKey, .del, .set],
  executable .toOrderedMap #[.get, .keyAt, .len],
  executable .toVecPair #[.get, .keyAt, .len],
  executable .trim #[.del, .len],
  executable .upsert #[.set],
  executable .upsertAll #[.set],
  executable .values #[.get, .keyAt, .len],
  specification .abortsAdd,
  specification .abortsAddAll,
  specification .abortsAppendDisjoint,
  specification .abortsBorrow,
  specification .abortsDel,
  specification .abortsDestroyEmpty,
  specification .abortsEmpty,
  specification .abortsIterBorrowMut,
  specification .abortsNewFrom,
  specification .abortsNewWithConfig,
  specification .abortsReplaceKeyInplace,
  specification .abortsTrim,
  specification .abortsUpsertAll,
  specification .del .required,
  specification .get .required,
  specification .hasKey .required,
  specification .iterPreserved,
  specification .iterValid,
  specification .insertionKeyAt,
  specification .insertionRank,
  specification .isEmpty,
  specification .keyAt,
  specification .leafIterValid,
  specification .leafOffset,
  specification .len,
  specification .new_,
  specification .rank,
  specification .set .required
]

def roleSchema? (name : String) : Option RoleSchema :=
  (MapRole.ofName? name).bind fun role => roleSchemas.find? (·.role == role)

def registryComplete : Bool :=
  roleSchemas.size == MapRole.all.size &&
    MapRole.all.all (fun role => roleSchemas.countP (fun schema => schema.role == role) == 1) &&
    roleSchemas.all (fun schema => !schema.signatures.isEmpty) &&
    (MapRole.all.map (·.name)).toList.Nodup &&
    roleSchemas.countP (fun schema => schema.kind == .executable) == MapOperation.all.size &&
    roleSchemas.countP (fun schema => schema.kind == .specification) == MapSpecRole.all.size

private def diagnostic (code message : String) (loc : LocId) : Array Diagnostic :=
  #[.at code message loc]

private def checkOwnerShape (ns : RawNamespace) (intrinsic : IntrinsicDecl) : Array Diagnostic :=
  match ns.structs.find? (fun declaration => declaration.name == intrinsic.owner) with
  | none => #[]
  | some owner =>
      if owner.generics.size == 2 && owner.generics.all (fun binder => binder.kind == .typeArg) then
        #[]
      else diagnostic "VIR-MOVE-INTRINSIC-OWNER-SHAPE"
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
      | .ownerNominal, .nominal name _ =>
          (unit.tables.names[name.index]?).any (matchesScope .ownerNamespace ·)
      | _, _ => false

/-- A parameter matches its pattern; a fixed-width integer one matches `num`,
since its type only bounds a specification function's domain (G15). -/
private def matchesParameter (unit : RawUnit) (ownerName : NameId)
    (pattern : TypePattern) (typeId : TypeId) : Bool :=
  matchesType unit ownerName pattern typeId ||
    (pattern == .num && unit.tables.types[typeId.index]?.any fun
      | .integer (.bits _) _ => true
      | _ => false)

private def matchesSignature (unit : RawUnit) (ownerName : NameId)
    (pattern : SignaturePattern) (actual : Signature) : Bool :=
  actual.generics.size == 2 &&
    actual.generics.all (fun binder => binder.kind == .typeArg) &&
    pattern.parameters.size == actual.parameters.size &&
    (pattern.parameters.zip actual.parameters).all (fun (expected, parameter) =>
      matchesParameter unit ownerName expected parameter.typeUse.typeId) &&
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
          code := "VIR-MOVE-INTRINSIC-SIGNATURE"
          message := s!"Move map intrinsic role `{binding.role}` target has an incompatible physical signature"
          primary := some binding.loc
          related := #[{ loc := targetLoc, message := "bound declaration" }] }]

private def checkBinding (unit : RawUnit) (intrinsic : IntrinsicDecl)
    (expected : TargetKind) (binding : IntrinsicBinding) : Array Diagnostic :=
  match roleSchema? binding.role with
  | none => diagnostic "VIR-MOVE-INTRINSIC-ROLE"
      s!"unknown Move map intrinsic role `{binding.role}`" binding.loc
  | some schema =>
      if schema.kind == expected then
        checkSignature unit intrinsic schema binding
      else
        diagnostic "VIR-MOVE-INTRINSIC-ROLE-KIND"
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
      ds ++ diagnostic "VIR-MOVE-INTRINSIC-ROLE-REQUIRED"
        s!"Move map intrinsic is missing required role `{schema.sourceName}`" intrinsic.loc
    else ds) #[]

private def checkDependencies (bound : Array BoundRole) : Array Diagnostic :=
  bound.foldl (fun ds present =>
    present.schema.dependencies.foldl (fun ds dependency =>
      if bound.any (fun candidate => candidate.schema.role == .specification dependency) then ds
      else
        ds ++ diagnostic "VIR-MOVE-INTRINSIC-ROLE-DEPENDENCY"
          s!"Move map intrinsic role `{present.schema.sourceName}` requires `{dependency.name}`"
          present.loc) ds) #[]

def check (unit : RawUnit) (ns : RawNamespace)
    (intrinsic : IntrinsicDecl) : Array Diagnostic :=
  if intrinsic.model != "map" then
    diagnostic "VIR-MOVE-INTRINSIC-MODEL"
      s!"unknown Move intrinsic model `{intrinsic.model}`" intrinsic.loc
  else
    let bindingErrors := intrinsic.executableBindings.foldl (fun ds binding =>
        ds ++ checkBinding unit intrinsic .executable binding) #[] ++
      intrinsic.specBindings.foldl (fun ds binding =>
        ds ++ checkBinding unit intrinsic .specification binding) #[]
    let bound := boundRoleSchemas intrinsic
    checkOwnerShape ns intrinsic ++ bindingErrors ++
      checkRequiredRoles intrinsic bound ++ checkDependencies bound

end VerexIR.Move.Intrinsics

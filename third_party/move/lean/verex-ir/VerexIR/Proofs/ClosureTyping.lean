-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.GlobalTyping

/-!
# Closures typed semantically from their carriers

A closure the native typing admits (`NTy.admits`) inhabits, at the runtime
family, the semantic function type its native type reads as
(`HasType.closure`), where the unit's readings agree (`TypesAgree`):
its instantiation is faithful (`closureFaithful`), its target's signature
reads alike over its own environment, and the arguments its instantiation
gives the target's type parameters read alike, so its rows at the
instantiation read as the target's semantic signature there.
-/

namespace VerexIR.Proofs

open VerexIR.Validation
open Denote
open SemanticOperations

private theorem mapM_some_of_forall {α β : Type} {f g : α → Option β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      (∀ x ∈ xs, ∀ y, f x = some y → g x = some y) → xs.mapM g = some ys
  | [], ys, h, _ => by simpa using h
  | x :: xs, ys, h, hfg => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h ⊢
      obtain ⟨y, hy, rest, hrest, rfl⟩ := h
      exact ⟨y, hfg x (by simp) y hy, rest,
        mapM_some_of_forall hrest (fun x' mem => hfg x' (by simp [mem])), rfl⟩

/-- An environment without a type for any parameter resolves as the empty
one: a type it resolves mentions no parameter. -/
theorem resolveFuel_untyped (tables : Tables) {env : Array SemArg}
    (untyped : ∀ (index : Nat) (type : SemTy), env[index]? ≠ some (SemArg.type type)) :
    ∀ fuel typeId type, SemTy.resolveFuel tables env fuel typeId = some type →
      SemTy.resolveFuel tables #[] fuel typeId = some type := by
  intro fuel
  induction fuel with
  | zero => intro _ _ resolved; simp [SemTy.resolveFuel] at resolved
  | succ fuel ih =>
      intro typeId type resolved
      simp only [SemTy.resolveFuel, Option.bind_eq_bind] at resolved ⊢
      cases found : tables.types[typeId.index]? with
      | none => simp [found] at resolved
      | some node =>
          simp only [found, Option.bind_some] at resolved ⊢
          cases node with
          | tuple elements =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved ⊢
              obtain ⟨types, row, rfl⟩ := resolved
              exact ⟨types, mapM_some_of_forall row fun id _ type found => ih id type found, rfl⟩
          | vector element bound =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved ⊢
              obtain ⟨inner, innerFound, rfl⟩ := resolved
              exact ⟨inner, ih _ _ innerFound, rfl⟩
          | nominal name arguments =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at resolved ⊢
              obtain ⟨qualified, named, resolvedArguments, argumentsFound, rfl⟩ := resolved
              refine ⟨qualified, named, resolvedArguments, ?_, rfl⟩
              refine mapM_some_of_forall argumentsFound fun argument _ result found => ?_
              cases argument with
              | typeArg value =>
                  simp only [Option.map_eq_map, Option.map_eq_some_iff] at found ⊢
                  obtain ⟨inner, innerFound, rfl⟩ := found
                  exact ⟨inner, ih _ _ innerFound, rfl⟩
              | const | lifetime | evidence => exact found
          | function parameters result abilities =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at resolved ⊢
              obtain ⟨resolvedParameters, parametersFound, resolvedResult, resultFound, rfl⟩ :=
                resolved
              exact ⟨resolvedParameters,
                mapM_some_of_forall parametersFound fun id _ type found => ih id type found,
                resolvedResult, ih _ _ resultFound, rfl⟩
          | reference borrowed =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved ⊢
              obtain ⟨inner, innerFound, rfl⟩ := resolved
              exact ⟨inner, ih _ _ innerFound, rfl⟩
          | typeParameter index =>
              simp only at resolved
              split at resolved
              · next type typed => exact absurd typed (untyped index type)
              · cases resolved
          | _ => exact resolved

/-- A call's arguments under the empty outer instantiation are themselves. -/
theorem instantiateGenericArguments_empty (arguments : Array GenericArgument) :
    instantiateGenericArguments #[] arguments = arguments := by
  simp only [instantiateGenericArguments]
  conv => rhs; rw [← Array.map_id arguments]
  congr 1
  funext argument
  cases argument <;> simp [instantiatedTypeId_empty]

/-- The semantic arguments a runtime type instantiation gives a function's
generic binders: each type argument's closed semantic type. -/
def frameSemArguments (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (targetNs : ValidatedNamespace)
    (generics : Array GenericBinder) : Array SemArg :=
  (frameArguments unit function typeInstantiation generics).map fun argument =>
    match argument with
    | .typeArg value => match StaticTyping.resolveIn targetNs #[] value.typeId with
      | some type => .type type
      | none => .erased
    | _ => .erased

theorem frameArguments_size (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (generics : Array GenericBinder) :
    (frameArguments unit function typeInstantiation generics).size = generics.size := by
  simp [frameArguments]

/-- A faithful instantiation is one its target's frame reads faithfully, at
the semantic arguments it gives the target's type parameters. -/
theorem frameInstantiation_of_faithful {unit : ValidatedUnit} {function : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {targetNs : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody}
    (namespace_eq : unit.namespaces[function.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.functions[function.functionId.index]? = some declaration)
    (faithful : closureFaithful unit function typeInstantiation = true) :
    FrameInstantiation targetNs (requiredAt unit function) typeInstantiation
      (frameEnv declaration.signature.generics
        (frameSemArguments unit function typeInstantiation targetNs
          declaration.signature.generics)) := by
  simp only [closureFaithful, namespace_eq, declaration_eq] at faithful
  split at faithful
  · next noTypes =>
      left
      refine ⟨by simpa [Array.isEmpty_iff] using faithful, fun typeId _ type resolved => ?_⟩
      obtain ⟨fuel, found⟩ := resolved
      refine ⟨fuel, resolveFuel_untyped targetNs.tables (fun index semType at_index => ?_) fuel
        typeId type found⟩
      simp only [frameEnv, StaticTyping.staticEnv, Array.getElem?_map,
        Array.getElem?_mapIdx, Option.map_eq_some_iff] at at_index
      obtain ⟨_, ⟨binder, found, rfl⟩, typed⟩ := at_index
      obtain ⟨bounded, binder_eq⟩ := Array.getElem?_eq_some_iff.mp found
      have kind := Array.all_eq_true.mp noTypes index bounded
      simp only [binder_eq] at kind
      cases binderKind : binder.kind with
      | typeArg => rw [binderKind] at kind; exact absurd kind (by decide)
      | const | lifetime | evidence => simp [binderKind, SemArg.subst] at typed
  · simp only [Bool.and_eq_true, beq_iff_eq, Array.all_eq_true] at faithful
    obtain ⟨⟨same, closed⟩, interned⟩ := faithful
    right
    refine ⟨#[], frameArguments unit function typeInstantiation declaration.signature.generics,
      same, ?_, ?_⟩
    · rw [instantiateGenericArguments_empty]
      intro index value argument
      obtain ⟨bounded, element⟩ := Array.getElem?_eq_some_iff.mp argument
      have atIndex := closed index bounded
      simp only [element] at atIndex
      split at atIndex
      · next type resolvedType =>
          refine ⟨type, ?_, ⟨_, resolvedType⟩⟩
          simp only [frameArguments, Array.getElem?_mapIdx, Option.map_eq_some_iff] at argument
          obtain ⟨binder, binder_eq, made⟩ := argument
          split at made
          · next node kind found =>
              simp only [GenericArgument.typeArg.injEq] at made
              subst made
              simp only [frameEnv, StaticTyping.staticEnv, Array.getElem?_map,
                Array.getElem?_mapIdx, binder_eq, Option.map_some, kind, SemArg.subst,
                SemTy.subst, frameSemArguments, frameArguments, found, resolvedType]
          · cases made
      · cases atIndex
    · rw [instantiateGenericArguments_empty]
      intro typeId member
      obtain ⟨index, bounded, element⟩ := Array.mem_iff_getElem.mp member
      rw [← element]
      exact interned index bounded

/-- What `closureSignature?` reads: a function's parameter and result types,
its types mentioning only its own type parameters
with nodes. -/
theorem closureSignature?_eq_some {unit : ValidatedUnit} {function : FunctionHandle}
    {parameters results : List NTy}
    (signature : closureSignature? unit function = some (parameters, results)) :
    ∃ targetNs declaration,
      unit.namespaces[function.namespaceId.index]? = some targetNs ∧
      targetNs.functions[function.functionId.index]? = some declaration ∧
      declaration.signature.parameters.toList.mapM
        (fun parameter => ntyOf unit function.namespaceId parameter.typeUse.typeId) =
          some parameters ∧
      declaration.signature.results.toList.mapM
        (fun result => ntyOf unit function.namespaceId result.typeId) = some results ∧
      (parameters ++ results).all (fun τ => τ.paramsAll fun index =>
        (declaration.signature.generics[index]?.any fun binder => binder.kind matches .typeArg) &&
          (paramNodeIn? unit function.namespaceId index).isSome) = true := by
  simp only [closureSignature?, Option.bind_eq_bind, Option.bind_eq_some_iff, guard,
    Option.pure_def] at signature
  obtain ⟨targetNs, namespace_eq, declaration, declaration_eq, read, readParameters,
    readResults, readResults_eq, _, bounded, final⟩ := signature
  split at bounded
  · next mentions =>
      simp only [Option.some.injEq, Prod.mk.injEq] at final
      obtain ⟨rfl, rfl⟩ := final
      exact ⟨targetNs, declaration, namespace_eq, declaration_eq, readParameters,
        readResults_eq, mentions⟩
  · cases bounded

mutual
/-- A type substituted at its allowed parameters by closed types is closed. -/
theorem NTy.paramFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (closed : ∀ index, allowed index = true → (types index).paramFree = true) :
    (τ : NTy) → τ.paramsAll allowed = true → (τ.substWith types).paramFree = true
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _ | .bytes, _ => rfl
  | .param index, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      exact closed index mentioned
  | .tuple elements, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.paramFree, NRow.paramFree_substWith closed elements mentioned]
  | .struct _ arguments fields, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NTy.paramFree,
        NRow.paramFree_substWith closed arguments mentioned.1,
        NRow.paramFree_substWith closed fields mentioned.2, Bool.and_self]
  | .enum _ arguments _ rows _, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NTy.paramFree,
        NRow.paramFree_substWith closed arguments mentioned.1,
        NRows.paramFree_substWith closed rows mentioned.2, Bool.and_self]
  | .vector element, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.paramFree, NTy.paramFree_substWith closed element mentioned]
  | .table key value, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NTy.paramFree, NTy.paramFree_substWith closed key mentioned.1,
        NTy.paramFree_substWith closed value mentioned.2, Bool.and_self]
  | .ref referent, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.paramFree, NTy.paramFree_substWith closed referent mentioned]
  | .ghost slot, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.paramFree, NTy.paramFree_substWith closed slot mentioned]
  | .function parameters _ results, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NTy.paramFree, Bool.and_eq_true,
        NRow.paramFree_substWith closed parameters mentioned.1,
        NRow.paramFree_substWith closed results mentioned.2, and_self]

theorem NRow.paramFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (closed : ∀ index, allowed index = true → (types index).paramFree = true) :
    (row : NRow) → row.paramsAll allowed = true → (row.substWith types).paramFree = true
  | .nil, _ => rfl
  | .cons τ rest, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRow.substWith, NRow.paramFree, Bool.and_eq_true,
        NTy.paramFree_substWith closed τ mentioned.1,
        NRow.paramFree_substWith closed rest mentioned.2, and_self]

theorem NRows.paramFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (closed : ∀ index, allowed index = true → (types index).paramFree = true) :
    (rows : NRows) → rows.paramsAll allowed = true → (rows.substWith types).paramFree = true
  | .nil, _ => rfl
  | .cons fields rest, mentioned => by
      simp only [NRows.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRows.substWith, NRows.paramFree, Bool.and_eq_true,
        NRow.paramFree_substWith closed fields mentioned.1,
        NRows.paramFree_substWith closed rest mentioned.2, and_self]
end

mutual
/-- A type holding no reference, substituted at its allowed parameters by
types holding none, holds none. -/
theorem NTy.refFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (plain : ∀ index, allowed index = true → (types index).refFree = true) :
    (τ : NTy) → τ.paramsAll allowed = true → τ.refFree = true →
      (τ.substWith types).refFree = true
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ => rfl
  | .function _ _ _, _, _ => rfl
  | .param index, mentioned, _ => by
      simp only [NTy.paramsAll] at mentioned
      exact plain index mentioned
  | .ref _, _, free => by simp [NTy.refFree] at free
  | .tuple elements, mentioned, free => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.refFree] at free
      simp only [NTy.substWith, NTy.refFree, NRow.refFree_substWith plain elements mentioned free]
  | .struct _ _ fields, mentioned, free => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.refFree] at free
      simp only [NTy.substWith, NTy.refFree,
        NRow.refFree_substWith plain fields mentioned.2 free]
  | .enum _ _ _ rows _, mentioned, free => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.refFree] at free
      simp only [NTy.substWith, NTy.refFree,
        NRows.refFree_substWith plain rows mentioned.2 free]
  | .vector element, mentioned, free => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.refFree] at free
      simp only [NTy.substWith, NTy.refFree, NTy.refFree_substWith plain element mentioned free]
  | .table key value, mentioned, free => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.refFree, Bool.and_eq_true] at free
      simp only [NTy.substWith, NTy.refFree, NTy.refFree_substWith plain key mentioned.1 free.1,
        NTy.refFree_substWith plain value mentioned.2 free.2, Bool.and_self]
  | .ghost slot, mentioned, free => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.refFree] at free
      simp only [NTy.substWith, NTy.refFree, NTy.refFree_substWith plain slot mentioned free]

theorem NRow.refFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (plain : ∀ index, allowed index = true → (types index).refFree = true) :
    (row : NRow) → row.paramsAll allowed = true → row.refFree = true →
      (row.substWith types).refFree = true
  | .nil, _, _ => rfl
  | .cons τ rest, mentioned, free => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [NRow.substWith, NRow.refFree, Bool.and_eq_true,
        NTy.refFree_substWith plain τ mentioned.1 free.1,
        NRow.refFree_substWith plain rest mentioned.2 free.2, and_self]

theorem NRows.refFree_substWith {allowed : Nat → Bool} {types : Nat → NTy}
    (plain : ∀ index, allowed index = true → (types index).refFree = true) :
    (rows : NRows) → rows.paramsAll allowed = true → rows.refFree = true →
      (rows.substWith types).refFree = true
  | .nil, _, _ => rfl
  | .cons fields rest, mentioned, free => by
      simp only [NRows.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRows.refFree, Bool.and_eq_true] at free
      simp only [NRows.substWith, NRows.refFree, Bool.and_eq_true,
        NRow.refFree_substWith plain fields mentioned.1 free.1,
        NRows.refFree_substWith plain rest mentioned.2 free.2, and_self]
end

theorem resolveIn_subst {ns : ValidatedNamespace} {env : Array SemArg} {typeId : TypeId}
    {type : SemTy} (arguments : Array SemArg)
    (resolved : StaticTyping.resolveIn ns env typeId = some type) :
    StaticTyping.resolveIn ns (env.map (·.subst arguments)) typeId =
      some (type.subst arguments) :=
  SemTy.resolveFuel_subst ns.tables env arguments _ typeId type resolved

private theorem mapM_resolveIn_subst {ns : ValidatedNamespace} {env : Array SemArg}
    (arguments : Array SemArg) {α : Type} (typeOf : α → TypeId) :
    {items : List α} → {types : List SemTy} →
      items.mapM (fun item => StaticTyping.resolveIn ns env (typeOf item)) = some types →
      items.mapM (fun item => StaticTyping.resolveIn ns (env.map (·.subst arguments))
        (typeOf item)) = some (types.map (·.subst arguments))
  | [], _, resolved => by simpa using resolved
  | item :: items, types, resolved => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at resolved ⊢
      obtain ⟨head, headResolved, tail, tailResolved, rfl⟩ := resolved
      exact ⟨_, resolveIn_subst arguments headResolved, _,
        mapM_resolveIn_subst arguments typeOf tailResolved, by simp⟩

/-- A signature resolved under substituted arguments is the substitution of
its resolution. -/
theorem signatureTypes?_subst {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    {env : Array SemArg} {parameters results : List SemTy} (arguments : Array SemArg)
    (signature : StaticTyping.signatureTypes? ns declaration env = some (parameters, results)) :
    StaticTyping.signatureTypes? ns declaration (env.map (·.subst arguments)) =
      some (parameters.map (·.subst arguments), results.map (·.subst arguments)) := by
  simp only [StaticTyping.signatureTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.some.injEq, Prod.mk.injEq] at signature ⊢
  obtain ⟨resolvedParameters, parametersFound, resolvedResults, resultsFound, rfl, rfl⟩ :=
    signature
  exact ⟨_, mapM_resolveIn_subst arguments (fun (parameter : Parameter) => parameter.typeUse.typeId)
    parametersFound, _,
    mapM_resolveIn_subst arguments (fun (result : TypeUse) => result.typeId) resultsFound, rfl, rfl⟩

theorem NRow.ofList_substWith (types : Nat → NTy) :
    (items : List NTy) → (NRow.ofList items).substWith types =
      NRow.ofList (items.map (·.substWith types))
  | [] => rfl
  | τ :: items => by simp [NRow.ofList, NRow.substWith, NRow.ofList_substWith types items]

theorem extract_map {α β : Type} (f : α → β) : (mask : Nat) → (captured : Bool) →
    (items : List α) →
    ClosureMask.extract mask captured (items.map f) = (ClosureMask.extract mask captured items).map f
  | _, _, [] => rfl
  | mask, captured, item :: items => by
      simp only [List.map_cons, ClosureMask.extract, extract_map f (mask / 2) captured items]
      split <;> rfl

theorem extract_length {α β : Type} : (mask : Nat) → (captured : Bool) → (left : List α) →
    (right : List β) → left.length = right.length →
      (ClosureMask.extract mask captured left).length =
        (ClosureMask.extract mask captured right).length
  | _, _, [], [], _ => rfl
  | mask, captured, _ :: left, _ :: right, same => by
      simp only [List.length_cons, Nat.add_right_cancel_iff] at same
      simp only [ClosureMask.extract]
      have rest := extract_length (mask / 2) captured left right same
      split <;> simp [rest]

/-- A parameter row's reading splits by a mask. -/
theorem ParametersTypedAs.extract {unit : ValidatedUnit} : (mask : Nat) → (captured : Bool) →
    {items : List NTy} → {shared : List Bool} → {types : List SemTy} →
    (NRow.ofList items).ParametersTypedAs unit shared types →
    (NRow.ofList (ClosureMask.extract mask captured items)).ParametersTypedAs unit
      (ClosureMask.extract mask captured shared) (ClosureMask.extract mask captured types)
  | _, _, [], _, _, typed => by
      cases typed
      exact .nil
  | mask, captured, τ :: items, _, _, typed => by
      cases typed with
      | value head observed later =>
          have rest := ParametersTypedAs.extract (mask / 2) captured later
          simp only [ClosureMask.extract]
          split
          · exact .value head observed rest
          · exact rest
      | shared head observed later =>
          have rest := ParametersTypedAs.extract (mask / 2) captured later
          simp only [ClosureMask.extract]
          split
          · exact .shared head observed rest
          · exact rest

/-- A parameter row reads as what its parameters observe. -/
theorem ParametersTypedAs.observed {unit : ValidatedUnit} :
    {row : NRow} → {shared : List Bool} → {types : List SemTy} →
    row.ParametersTypedAs unit shared types → row.TypedAs unit (types.map observedType)
  | _, _, _, .nil => .nil
  | _, _, _, .value (type := type) head observed later => by
      have same : observedType type = type := by
        cases type with
        | reference kind referent =>
            cases kind with
            | shared =>
                have bounded := SemTy.sizeOf_unshared_le referent
                simp only [SemTy.unshared] at observed
                rw [observed] at bounded
                simp only [SemTy.reference.sizeOf_spec] at bounded
                omega
            | mutable => rfl
        | _ => rfl
      simp only [List.map_cons, same]
      exact .cons head (ParametersTypedAs.observed later)
  | _, _, _, .shared head _ later => by
      simp only [List.map_cons, observedType]
      exact .cons head (ParametersTypedAs.observed later)

/-- The type parameters a closure target's types may mention: its own type
binders, each with a node. -/
def targetParameter (unit : ValidatedUnit) (function : FunctionHandle)
    (generics : Array GenericBinder) (index : Nat) : Bool :=
  (generics[index]?.any fun binder => binder.kind matches .typeArg) &&
    (paramNodeIn? unit function.namespaceId index).isSome

/-- What a faithful instantiation gives one of its target's type
parameters: a closed type, stating no reference, whose closed native type
holding no reference is the parameter's argument. -/
theorem faithful_argument {unit : ValidatedUnit} {function : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {targetNs : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody}
    (namespace_eq : unit.namespaces[function.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.functions[function.functionId.index]? = some declaration)
    (faithful : closureFaithful unit function typeInstantiation = true) {index : Nat}
    (parameter : targetParameter unit function declaration.signature.generics index = true) :
    ∃ type τ,
      (frameSemArguments unit function typeInstantiation targetNs
        declaration.signature.generics)[index]? = some (.type type) ∧
      closureArgument unit function typeInstantiation index = τ ∧
      ntyOf unit function.namespaceId
          (instantiatedTypeId typeInstantiation
            ((paramNodeIn? unit function.namespaceId index).get!)) = some τ ∧
      StaticTyping.resolveIn targetNs #[]
          (instantiatedTypeId typeInstantiation
            ((paramNodeIn? unit function.namespaceId index).get!)) = some type ∧
      type.referenceFree = true ∧ τ.paramFree = true ∧ τ.refFree = true := by
  simp only [targetParameter, Bool.and_eq_true, Option.any_eq_true, Option.isSome_iff_exists]
    at parameter
  obtain ⟨⟨binder, binder_eq, kind⟩, node, found⟩ := parameter
  obtain ⟨bounded, element⟩ := Array.getElem?_eq_some_iff.mp binder_eq
  simp only [closureFaithful, namespace_eq, declaration_eq] at faithful
  split at faithful
  · next noTypes =>
      have notType := Array.all_eq_true.mp noTypes index bounded
      simp only [element] at notType
      cases binderKind : binder.kind with
      | typeArg => rw [binderKind] at notType; exact absurd notType (by decide)
      | const | lifetime | evidence => rw [binderKind] at kind; simp at kind
  · simp only [Bool.and_eq_true, beq_iff_eq, Array.all_eq_true] at faithful
    obtain ⟨⟨_, closed⟩, _⟩ := faithful
    have argument : (frameArguments unit function typeInstantiation
        declaration.signature.generics)[index]? =
          some (.typeArg ⟨instantiatedTypeId typeInstantiation node, ⟨0⟩⟩) := by
      simp only [frameArguments, Array.getElem?_mapIdx, binder_eq, Option.map_some]
      cases binderKind : binder.kind with
      | typeArg => simp [found]
      | const | lifetime | evidence => rw [binderKind] at kind; simp at kind
    obtain ⟨inArguments, argumentElement⟩ := Array.getElem?_eq_some_iff.mp argument
    have atIndex := closed index inArguments
    simp only [argumentElement] at atIndex
    split at atIndex
    · next type resolved =>
        simp only [Bool.and_eq_true, Option.any_eq_true] at atIndex
        obtain ⟨free, τ, native, closedType, plainType⟩ := atIndex
        refine ⟨type, τ, ?_, ?_, by simpa [found] using native, by simpa [found] using resolved,
          free, closedType, plainType⟩
        · simp only [frameSemArguments, Array.getElem?_map, argument, Option.map_some, resolved]
        · simp only [closureArgument, found, native, Option.getD_some]
    · cases atIndex

/-- A semantic type stating no reference observes itself. -/
theorem SemTy.unshared_of_referenceFree {type : SemTy} (free : type.referenceFree = true) :
    type.unshared = type := by
  cases type with
  | reference => simp [SemTy.referenceFree] at free
  | _ => rfl

/-- At a faithful instantiation, in a unit whose readings agree, the
arguments it gives a target's type parameters agree with their semantic
arguments. -/
theorem argumentsAgree_of_faithful {unit : ValidatedUnit} (agree : TypesAgree unit)
    {function : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)}
    {targetNs : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    (namespace_eq : unit.namespaces[function.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.functions[function.functionId.index]? = some declaration)
    (faithful : closureFaithful unit function typeInstantiation = true) :
    ArgumentsAgree unit (targetParameter unit function declaration.signature.generics)
      (closureArgument unit function typeInstantiation)
      (frameSemArguments unit function typeInstantiation targetNs
        declaration.signature.generics) := by
  intro index parameter
  obtain ⟨type, τ, semantic, argument, native, resolved, free, _, _⟩ :=
    faithful_argument namespace_eq declaration_eq faithful parameter
  refine ⟨type, semantic, ?_, SemTy.unshared_of_referenceFree free⟩
  rw [argument]
  exact agree.types function.namespaceId _ targetNs τ type namespace_eq native resolved free

private theorem ofList_paramsAll {allowed : Nat → Bool} :
    (types : List NTy) → types.all (·.paramsAll allowed) = true →
      (NRow.ofList types).paramsAll allowed = true
  | [], _ => rfl
  | τ :: types, mentioned => by
      simp only [List.all_cons, Bool.and_eq_true] at mentioned
      simp only [NRow.ofList, NRow.paramsAll, mentioned.1, ofList_paramsAll types mentioned.2,
        Bool.and_self]

private theorem extract_all {α : Type} {keep : α → Bool} :
    (mask : Nat) → (captured : Bool) → (items : List α) → items.all keep = true →
      (ClosureMask.extract mask captured items).all keep = true
  | _, _, [], _ => rfl
  | mask, captured, item :: items, kept => by
      simp only [List.all_cons, Bool.and_eq_true] at kept
      simp only [ClosureMask.extract]
      split
      · simp only [List.all_cons, kept.1, extract_all (mask / 2) captured items kept.2,
          Bool.and_self]
      · exact extract_all (mask / 2) captured items kept.2

private theorem mapM_length {α β : Type} {f : α → Option β} :
    {items : List α} → {results : List β} → items.mapM f = some results →
      results.length = items.length
  | [], _, found => by simp at found; subst found; rfl
  | item :: items, results, found => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at found
      obtain ⟨_, _, rest, restFound, rfl⟩ := found
      simp [mapM_length restFound]

/-- A closure the native typing admits at a function type, in a unit whose
readings agree: its target's signature at the semantic arguments its faithful
instantiation gives, the captures typed at the captured parameters wherever
the captured row's values are, and the function type's rows reading as the
supplied parameters and the results. -/
theorem closureReading {unit : ValidatedUnit} (agree : TypesAgree unit) {loans : LoanTypes}
    {function : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {captures : Array RuntimeValue} {parameters : NRow} {shared : Sharing} {results : NRow}
    (admitted : NTy.admits unit (.function parameters shared results)
      (.closure function mask typeInstantiation captures) = true)
    (typedCaptures : ∀ (row : NRow) (types : List SemTy),
      NRow.admits unit row captures.toList = true → row.TypedAs unit types →
        row.paramFree = true → row.refFree = true →
        ∃ represented : List RuntimeValue, represented.map (normalize unit) = captures.toList ∧
          HasTypes unit loans represented types) :
    ∃ targetNs declaration arguments allParameters targetResults,
      unit.namespaces[function.namespaceId.index]? = some targetNs ∧
      targetNs.functions[function.functionId.index]? = some declaration ∧
      StaticTyping.signatureTypes? targetNs declaration
        (frameEnv declaration.signature.generics arguments) =
          some (allParameters, targetResults) ∧
      FrameInstantiation targetNs (requiredAt unit function) typeInstantiation
        (frameEnv declaration.signature.generics arguments) ∧
      mask < 2 ^ declaration.signature.parameters.size ∧
      (∃ represented : Array RuntimeValue, represented.map (normalize unit) = captures ∧
        HasTypes unit loans represented.toList (ClosureMask.extract mask true allParameters)) ∧
      parameters.ParametersTypedAs unit shared.parameters
        (ClosureMask.extract mask false allParameters) ∧
      results.ParametersTypedAs unit shared.results targetResults := by
  unfold NTy.admits at admitted
  simp only [Bool.and_eq_true] at admitted
  obtain ⟨faithful, rowsAdmitted⟩ := admitted
  split at rowsAdmitted
  · next capturedAt suppliedAt returnedAt rowsAt =>
      simp only [Bool.and_eq_true, beq_iff_eq] at rowsAdmitted
      obtain ⟨⟨⟨suppliedEq, sharingEq⟩, returnedEq⟩, capturesAdmitted⟩ := rowsAdmitted
      simp only [closureRowsIn?, Option.map_eq_some_iff, Prod.mk.injEq] at rowsAt
      obtain ⟨⟨captured, supplied, returned⟩, rows, rfl, rfl, rfl⟩ := rowsAt
      obtain ⟨parameterNatives, resultNatives, signature, bounded, capturedFree, rfl, rfl, rfl⟩ :=
        closureRows?_eq_some.mp rows
      obtain ⟨targetNs, declaration, namespace_eq, declaration_eq, readParameters, readResults,
        mentions⟩ := closureSignature?_eq_some signature
      have agreesAt := agree.signatures function
      simp only [signatureAgrees, namespace_eq, declaration_eq, signature] at agreesAt
      split at agreesAt
      · next genericParameters genericResults genericSignature =>
          simp only [Bool.and_eq_true] at agreesAt
          have parametersTyped := NRow.parametersTypedAs_of_check _ _ _ agreesAt.1
          have resultsTyped := NRow.parametersTypedAs_of_check _ _ _ agreesAt.2
          let semArguments := frameSemArguments unit function typeInstantiation targetNs
            declaration.signature.generics
          have arguments := argumentsAgree_of_faithful agree namespace_eq declaration_eq faithful
          have allMentioned := mentions
          rw [List.all_append, Bool.and_eq_true] at allMentioned
          have parametersMentioned := ofList_paramsAll _ allMentioned.1
          have resultsMentioned := ofList_paramsAll _ allMentioned.2
          have parametersAt := NRow.ParametersTypedAs.subst arguments parametersTyped
            parametersMentioned
          have resultsAt := NRow.ParametersTypedAs.subst arguments resultsTyped resultsMentioned
          rw [NRow.ofList_substWith] at parametersAt resultsAt
          have suppliedTyped := ParametersTypedAs.extract mask false parametersAt
          rw [extract_map, ← NRow.ofList_substWith] at suppliedTyped
          have sharing : closureShared unit function mask =
              { parameters := ClosureMask.extract mask false
                  (parameterSharing unit function declaration)
                results := resultSharing unit function declaration } := by
            simp [closureShared, namespace_eq, declaration_eq, parameterSharing, resultSharing]
          rw [sharingEq] at sharing
          subst sharing
          rw [suppliedEq] at suppliedTyped
          rw [← NRow.ofList_substWith, returnedEq] at resultsAt
          have closedArguments : ∀ index,
              targetParameter unit function declaration.signature.generics index = true →
                (closureArgument unit function typeInstantiation index).paramFree = true :=
              fun index parameter => by
            obtain ⟨_, τ, _, same, _, _, _, closed, _⟩ :=
              faithful_argument namespace_eq declaration_eq faithful parameter
            rw [same]; exact closed
          have plainArguments : ∀ index,
              targetParameter unit function declaration.signature.generics index = true →
                (closureArgument unit function typeInstantiation index).refFree = true :=
              fun index parameter => by
            obtain ⟨_, τ, _, same, _, _, _, _, plain⟩ :=
              faithful_argument namespace_eq declaration_eq faithful parameter
            rw [same]; exact plain
          have capturedMentioned := ofList_paramsAll _
            (extract_all mask true parameterNatives allMentioned.1)
          obtain ⟨represented, normalized, representedTyped⟩ := typedCaptures _ _ capturesAdmitted
            (by rw [NRow.ofList_substWith, ← extract_map]
                exact ParametersTypedAs.observed (ParametersTypedAs.extract mask true
                  parametersAt))
            (NRow.paramFree_substWith closedArguments _ capturedMentioned)
            (NRow.refFree_substWith plainArguments _ capturedMentioned capturedFree)
          have capturesTyped : ∃ represented : Array RuntimeValue,
              represented.map (normalize unit) = captures ∧
              HasTypes unit loans represented.toList
                (ClosureMask.extract mask true (genericParameters.map (·.subst semArguments))) :=
            ⟨represented.toArray, by simpa using congrArg List.toArray normalized,
              by simpa using HasTypes.ofObserved representedTyped⟩
          refine ⟨targetNs, declaration, semArguments,
            genericParameters.map (·.subst semArguments), genericResults.map (·.subst semArguments),
            namespace_eq, declaration_eq,
            by simpa only [frameEnv] using signatureTypes?_subst semArguments genericSignature,
            frameInstantiation_of_faithful namespace_eq declaration_eq faithful, ?_, capturesTyped,
            suppliedTyped, resultsAt⟩
          rw [← Array.length_toList, ← mapM_length readParameters]
          exact bounded
      · cases agreesAt
  · cases rowsAdmitted

private theorem sizeOf_toList_lt (values : Array RuntimeValue) :
    sizeOf values.toList < 1 + sizeOf values := by
  cases values
  simp only [Array.mk.sizeOf_spec]
  omega

/-- The variant row an enum reading reads a variant's fields at. -/
theorem VariantsTypedAs.find {unit : ValidatedUnit} {source : StructHandle}
    {arguments : List SemArg} :
    {names : List String} → {rows : NRows} → NRows.VariantsTypedAs unit source arguments names rows →
      {name : String} → {row : NRow} → NRows.variant? names rows name = some row →
      rows.paramFree = true → rows.refFree = true →
      ∃ ns declared fieldTypes ghostTypes,
        SemanticOperations.handleFields? unit source (some name) = some (ns, declared) ∧
        ResolvesAll ns.tables arguments.toArray (declared.toList.map (·.type.typeId)) fieldTypes ∧
        ResolvesAll ns.tables arguments.toArray (ghostTypeIds unit source) ghostTypes ∧
        row.TypedAs unit (fieldTypes ++ ghostTypes.map .ghost) ∧ row.paramFree = true ∧
        row.refFree = true
  | [], .nil, .nil, _, _, found, _, _ => by simp [NRows.variant?] at found
  | candidate :: names, .cons fields rest, .cons declaration_eq resolve ghostsResolve typed later,
      name, row, found, closed, plain => by
      simp only [NRows.paramFree, NRows.refFree, Bool.and_eq_true] at closed plain
      simp only [NRows.variant?] at found
      split at found
      · next same =>
          simp only [beq_iff_eq] at same
          subst same
          cases found
          exact ⟨_, _, _, _, declaration_eq, resolve, ghostsResolve, typed, closed.1, plain.1⟩
      · exact VariantsTypedAs.find later found closed.2 plain.2

theorem admits_tuple {unit : ValidatedUnit} {elements : NRow} {value : RuntimeValue}
    (admitted : NTy.admits unit (.tuple elements) value = true) :
    ∃ values, value = .tuple values ∧ NRow.admits unit elements values.toList = true := by
  cases value <;> simp only [NTy.admits, Bool.false_eq_true] at admitted
  exact ⟨_, rfl, admitted⟩

theorem admits_vector {unit : ValidatedUnit} {element : NTy} {value : RuntimeValue}
    (admitted : NTy.admits unit (.vector element) value = true) :
    ∃ values, value = .vector values ∧ NTy.admitsEach unit element values.toList = true := by
  cases value <;> simp only [NTy.admits, Bool.false_eq_true] at admitted
  exact ⟨_, rfl, admitted⟩

theorem admits_table {unit : ValidatedUnit} {key value : NTy} {map : RuntimeValue}
    (admitted : NTy.admits unit (.table key value) map = true) :
    ∃ entries, map = .vector entries ∧ NTy.admitsEntries unit key value entries.toList = true := by
  cases map <;> simp only [NTy.admits, Bool.false_eq_true] at admitted
  exact ⟨_, rfl, admitted⟩

theorem admits_struct {unit : ValidatedUnit} {source : StructHandle} {arguments fields : NRow}
    {value : RuntimeValue}
    (admitted : NTy.admits unit (.struct source arguments fields) value = true) :
    ∃ values, value = .nominal source none values ∧
      NRow.admits unit fields values.toList = true := by
  cases value with
  | nominal actual variant values =>
      cases variant with
      | none =>
          simp only [NTy.admits, Bool.and_eq_true, beq_iff_eq] at admitted
          obtain ⟨rfl, admitted⟩ := admitted
          exact ⟨_, rfl, admitted⟩
      | some _ => simp [NTy.admits] at admitted
  | _ => simp [NTy.admits] at admitted

theorem admits_enum {unit : ValidatedUnit} {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {value : RuntimeValue}
    (admitted : NTy.admits unit (.enum source arguments names rows distinct) value = true) :
    ∃ name row values, value = .nominal source (some name) values ∧
      NRows.variant? names rows name = some row ∧ NRow.admits unit row values.toList = true := by
  cases value with
  | nominal actual variant values =>
      cases variant with
      | some name =>
          simp only [NTy.admits, Bool.and_eq_true, beq_iff_eq] at admitted
          obtain ⟨rfl, admitted⟩ := admitted
          split at admitted
          · next row found => exact ⟨name, row, _, rfl, found, admitted⟩
          · cases admitted
      | none => simp [NTy.admits] at admitted
  | _ => simp [NTy.admits] at admitted

theorem admits_function {unit : ValidatedUnit} {parameters : NRow} {shared : Sharing}
    {results : NRow} {value : RuntimeValue}
    (admitted : NTy.admits unit (.function parameters shared results) value = true) :
    ∃ function mask typeInstantiation captures,
      value = .closure function mask typeInstantiation captures ∧
      NTy.admits unit (.function parameters shared results)
        (.closure function mask typeInstantiation captures) = true := by
  cases value with
  | closure function mask typeInstantiation captures => exact ⟨_, _, _, _, rfl, admitted⟩
  | _ => simp [NTy.admits] at admitted

/-- A frame's function value is admitted at the frame's resolution of its
function type: its transport to the runtime family keeps its encoding. -/
theorem Skolems.admits_carrier {unit : ValidatedUnit} [Θ : Skolems unit]
    {parameters results : NRow} {shared : Sharing}
    (function : (NTy.function parameters shared results).carrier) :
    NTy.admits unit (.function parameters.resolved shared results.resolved)
      ((Carriers.functions parameters shared results).closure function).encode = true := by
  have admitted : NTy.admits unit (Θ.resolve (.function parameters shared results))
      (@NTy.encode (Carriers.runtime unit) _ (Θ.toRuntime _ function)) = true :=
    NTy.admits_encode unit _ _
  rw [Θ.encode_toRuntime, NTy.encode_function, Θ.resolve_eq] at admitted
  simpa only [NTy.substWith, NRow.resolved_eq_substWith, Skolems.type] using admitted

/-- A scalar row reads only as its scalar types. -/
theorem Denote.NRow.TypedAs.scalarTypes_eq {unit : ValidatedUnit} :
    {row : NRow} → {types scalars : List SemTy} → row.TypedAs unit types →
      row.scalarTypes? = some scalars → types = scalars
  | _, _, _, .nil, scalar => by simpa [NRow.scalarTypes?] using scalar
  | .cons τ _, _, _, .cons head tail, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at scalar
      obtain ⟨type, found, rest, restFound, rfl⟩ := scalar
      rw [NRow.TypedAs.scalarTypes_eq tail restFound, NTy.readsUnique τ head (.scalar found)]

/-- A frame's function value at rows the frame resolves to scalars is typed
by its carrier where the unit's readings agree, as the rule for an unseen
invocation takes it. -/
theorem ClosureTypedAt.ofCarrier {unit : ValidatedUnit} [Skolems unit] (agree : TypesAgree unit)
    (representable : OwnersRepresentable unit) {parameters results : NRow} {shared : Sharing}
    (function : (NTy.function parameters shared results).carrier)
    (resultsFree : results.refFree = true) (parametersScalar : ScalarAt unit parameters)
    (resultsScalar : ScalarAt unit results) :
    ClosureTypedAt unit parameters shared results function := by
  intro _ _
  obtain ⟨parameterTypes, parametersScalar⟩ := Option.isSome_iff_exists.mp parametersScalar
  obtain ⟨resultTypes, resultsScalar⟩ := Option.isSome_iff_exists.mp resultsScalar
  have admitted := Skolems.admits_carrier function
  generalize (Carriers.functions parameters shared results).closure function = closure at admitted ⊢
  obtain ⟨function, mask, typeInstantiation, captures, plain⟩ := closure
  simp only [ClosureValue.encode] at admitted
  obtain ⟨targetNs, declaration, arguments, allParameters, targetResults, namespace_eq,
      declaration_eq, signature_eq, faithful, bound, capturesTyped, parametersTyped,
      resultsTyped⟩ := closureReading agree (loans := fun _ => none) admitted
    fun row _ capturesAdmitted rowTyped rowClosed rowPlain =>
      NRow.represented_of_admits representable agree captures.toList row capturesAdmitted rowTyped
        rowClosed rowPlain
  have observed := (ParametersTypedAs.observed parametersTyped).scalarTypes_eq parametersScalar
  have results_eq := (ParametersTypedAs.observed resultsTyped).scalarTypes_eq resultsScalar
  subst results_eq
  refine ⟨parameterTypes, targetResults, ClosureMask.extract mask false allParameters, observed,
    ⟨targetNs, declaration, arguments, allParameters, namespace_eq, declaration_eq, signature_eq,
      faithful, bound, capturesTyped, rfl⟩,
    RowTyped.ofResolved parametersScalar, fun shape row => ?_⟩
  subst row
  exact RowDecodes.ofObserved (RowDecodes.ofResolved shape resultsFree resultsScalar)

/-- Every runtime encoding of a memory has a typed representation, for a unit
whose resource types correspond to their keys' types and whose typed values
are representable. -/
theorem memoryTyped (unit : ValidatedUnit) (resources : ResourcesTyped unit)
    (representable : OwnersRepresentable unit) (memory : Memory unit) :
    MemoryTyped unit memory := by
  intro start encodes
  have represented : ∀ slot ∈ start.globals.entries, ∃ representation,
      normalize unit representation = slot.value ∧ ∃ ns type,
        unit.namespaces[slot.key.namespaceId.index]? = some ns ∧
        Resolves ns.tables #[] slot.key.typeId type ∧
        HasType unit (fun _ => none) representation type := by
    intro slot member
    have found := encodes.2 slot.key.namespaceId slot.key.typeId slot.key.key
    rw [show (⟨slot.key.namespaceId, slot.key.typeId, slot.key.key⟩ : GlobalKey) = slot.key from rfl,
      encodes.1.lookup_of_mem member] at found
    split at found
    · next resource named =>
        have ordinary := runtimeResourceOf_kind named
        rcases resource with ⟨native, arguments, kind⟩
        cases ordinary
        obtain ⟨value, -, encoded⟩ := Option.map_eq_some_iff.mp found.symm
        obtain ⟨ns, type, namespace_eq, resolves, typed, closed, plain, closures⟩ :=
          resources _ _ _ named
        obtain ⟨representation, normalized, typedRepresentation⟩ :=
          representable _ _ _ typed closed plain closures (NTy.admits_encode unit _ value)
        exact ⟨representation, encoded ▸ normalized, ns, type, namespace_eq, resolves,
          typedRepresentation⟩
    · cases found
  classical
  let represent (slot : GlobalSlot) : GlobalSlot :=
    if member : slot ∈ start.globals.entries then
      { slot with value := (represented slot member).choose }
    else slot
  refine ⟨⟨start.globals.entries.map represent⟩, ?_, ?_⟩
  · simp only [GlobalMap.normalize, Array.map_map]
    congr 1
    rw [Array.map_congr_left (fun slot member => ?_), Array.map_id']
    simp only [Function.comp, represent, dif_pos member]
    cases slot
    simp only [(represented _ member).choose_spec.1]
  · intro slot member
    simp only [Array.mem_map] at member
    obtain ⟨slot, member, rfl⟩ := member
    obtain ⟨-, ns, type, namespace_eq, resolves, typed⟩ := (represented slot member).choose_spec
    simp only [represent, dif_pos member]
    exact ⟨ns, type, namespace_eq, resolves, typed⟩

end VerexIR.Proofs

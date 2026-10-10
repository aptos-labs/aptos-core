-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanTyping
import LeanerIR.Validation.InstantiationCertificate

/-!
# Faithful frames

A generic frame reads its instantiation at the types it requires
(`FrameInstantiation`): through a table of the closed type arguments it was
created with, built by searching the type arena (`invocationTypeInstantiation`).
A call from a faithful frame creates a faithful frame (`FrameInstantiation.call`):
the callee's search at the caller-rewritten arguments repeats, node by node,
the caller's search for the instance of the callee's type at the call's own
arguments (`instantiate_repeat`), which the checker requires at each call
(`StaticTyping.Context.edgeClosed`).
-/

namespace LeanerIR

open Validation
open StaticTyping
open SemanticOperations

private theorem findSome?_range_single {α : Type} (f : Nat → Option α) (key : Nat)
    (others : ∀ index, index ≠ key → f index = none) :
    ∀ count, (List.range count).findSome? f = if key < count then f key else none := by
  intro count
  induction count with
  | zero => simp
  | succ count ih =>
      rw [List.range_succ, List.findSome?_append, ih]
      by_cases below : key < count
      · simp [below, Nat.lt_succ_of_lt below, others count (by omega)]
      · by_cases same : key = count
        · subst same
          simp
        · simp [below, others count (Ne.symm same)]
          omega

private theorem find?_fold {step : Array (TypeId × TypeId) → Nat → Array (TypeId × TypeId)}
    {entry : Nat → Option (TypeId × TypeId)} {typeId : TypeId}
    (stepped : ∀ result index, (step result index).find? (·.1 == typeId) =
      (result.find? (·.1 == typeId)).or (entry index)) :
    ∀ (indices : List Nat) (result : Array (TypeId × TypeId)),
      (indices.foldl step result).find? (·.1 == typeId) =
        (result.find? (·.1 == typeId)).or (indices.findSome? entry) := by
  intro indices
  induction indices with
  | nil => intro result; simp
  | cons index rest ih =>
      intro result
      rw [List.foldl_cons, ih, stepped, List.findSome?_cons, Option.or_assoc]
      cases entry index <;> simp

/-- A frame's instantiation table maps each type its arguments instantiate
to the instance. -/
theorem instantiatedTypeId_invocation {ns : ValidatedNamespace}
    {outer : Array (TypeId × TypeId)} {arguments : Array GenericArgument}
    {typeId instance_ : TypeId}
    (found : instantiatePlaceFieldType? ns (instantiateGenericArguments outer arguments) typeId =
      some instance_) :
    instantiatedTypeId (invocationTypeInstantiation ns outer arguments) typeId = instance_ := by
  have bound : typeId.index < ns.tables.types.size := by
    unfold instantiatePlaceFieldType? instantiatePlaceFieldTypeFuel? at found
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at found
    obtain ⟨_, entry, -⟩ := found
    exact (Array.getElem?_eq_some_iff.mp entry).1
  unfold instantiatedTypeId invocationTypeInstantiation
  rw [← Array.foldl_toList, Array.toList_range]
  dsimp only
  rw [find?_fold (entry := fun index => if (⟨index⟩ : TypeId) = typeId then
      (instantiatePlaceFieldType? ns (instantiateGenericArguments outer arguments) ⟨index⟩).bind
        fun concrete => if concrete = ⟨index⟩ then none else some (⟨index⟩, concrete)
    else none)]
  · rw [findSome?_range_single _ typeId.index]
    · simp only [Array.find?_empty, Option.none_or, bound, if_true]
      rw [found]
      by_cases same : instance_ = typeId
      · subst same
        simp
      · simp [same]
    · intro index different
      have : (⟨index⟩ : TypeId) ≠ typeId := fun same => different (congrArg TypeId.index same)
      simp [this]
  · intro result index
    split
    · rename_i concrete concrete_eq
      split
      · rename_i unchanged
        have : concrete = ⟨index⟩ := beq_iff_eq.mp unchanged
        simp [concrete_eq, this]
      · rename_i changed
        have : concrete ≠ ⟨index⟩ := fun same => changed (beq_iff_eq.mpr same)
        rw [Array.find?_push]
        by_cases same : (⟨index⟩ : TypeId) = typeId
        · subst same
          simp [concrete_eq, this]
        · have different : ((⟨index⟩ : TypeId) == typeId) = false := beq_false_of_ne same
          simp [different, same]
    · rename_i none_eq
      simp [none_eq]

theorem Array.mapM_some_of_forall {α β : Type} {f g : α → Option β}
    {xs : Array α} {ys : Array β} (mapped : xs.mapM f = some ys)
    (step : ∀ x ∈ xs, ∀ y, f x = some y → g x = some y) : xs.mapM g = some ys := by
  rw [Array.mapM_eq_mapM_toList] at mapped ⊢
  simp only [Functor.map, Option.map_eq_some_iff] at mapped ⊢
  obtain ⟨list, list_eq, rfl⟩ := mapped
  refine ⟨list, ?_, rfl⟩
  have : ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys → (∀ x ∈ xs, ∀ y, f x = some y →
      g x = some y) → xs.mapM g = some ys := by
    intro xs
    induction xs with
    | nil => intro ys mapped _; simpa using mapped
    | cons x rest ih =>
        intro ys mapped step
        simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
          Option.pure_def, Option.some.injEq] at mapped ⊢
        obtain ⟨y, y_eq, rest', rest'_eq, rfl⟩ := mapped
        exact ⟨y, step x (by simp) y y_eq, rest',
          ih rest'_eq (fun x member => step x (by simp [member])), rfl⟩
  exact this list_eq fun x member => step x (Array.mem_toList_iff.mp member)

/-- More fuel finds the same instance. -/
theorem instantiatePlaceFieldTypeFuel?_mono (ns : ValidatedNamespace)
    (arguments : Array GenericArgument) :
    ∀ (fuel : Nat) (typeId instance_ : TypeId),
      instantiatePlaceFieldTypeFuel? ns arguments fuel typeId = some instance_ →
        instantiatePlaceFieldTypeFuel? ns arguments (fuel + 1) typeId = some instance_ := by
  intro fuel
  induction fuel with
  | zero => intro _ _ h; simp [instantiatePlaceFieldTypeFuel?] at h
  | succ fuel ih =>
      intro typeId instance_ h
      rw [instantiatePlaceFieldTypeFuel?] at h ⊢
      cases entry : ns.tables.types[typeId.index]? with
      | none => simp [entry] at h
      | some node =>
          simp only [entry, Option.bind_eq_bind, Option.bind_some] at h ⊢
          have lift := fun typeId instance_ => ih typeId instance_
          cases node with
          | tuple elements =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, found⟩ := h
              exact ⟨instantiated, Array.mapM_some_of_forall mapped fun x _ y => lift x y, found⟩
          | vector element length =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, found⟩ := h
              exact ⟨instantiated, lift _ _ mapped, found⟩
          | typeDomain nested =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, found⟩ := h
              exact ⟨instantiated, lift _ _ mapped, found⟩
          | resourceDomain resource arguments' =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, found⟩ := h
              refine ⟨instantiated, ?_, found⟩
              cases arguments' with
              | none => exact mapped
              | some row =>
                  simp only [Option.mapM, Option.map_eq_map, Option.map_eq_some_iff] at mapped ⊢
                  obtain ⟨row', row'_eq, rfl⟩ := mapped
                  exact ⟨row', Array.mapM_some_of_forall row'_eq fun x _ y => lift x y, rfl⟩
          | nominal name arguments' =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, found⟩ := h
              refine ⟨instantiated, Array.mapM_some_of_forall mapped ?_, found⟩
              intro argument _ result step
              cases argument with
              | typeArg value =>
                  simp only [Option.bind_eq_some_iff] at step ⊢
                  obtain ⟨inner, inner_eq, result_eq⟩ := step
                  exact ⟨inner, lift _ _ inner_eq, result_eq⟩
              | lifetime value => exact step
              | const value => exact step
              | evidence value => exact step
          | function arguments' result abilities =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨instantiated, mapped, result', result_eq, found⟩ := h
              exact ⟨instantiated, Array.mapM_some_of_forall mapped fun x _ y => lift x y,
                result', lift _ _ result_eq, found⟩
          | reference reference =>
              simp only [Option.bind_eq_some_iff] at h ⊢
              obtain ⟨referent, referent_eq, lifetime, lifetime_eq, found⟩ := h
              exact ⟨referent, lift _ _ referent_eq, lifetime, lifetime_eq, found⟩
          | _ => exact h

theorem instantiatePlaceFieldTypeFuel?_mono_le (ns : ValidatedNamespace)
    (arguments : Array GenericArgument) {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {typeId instance_ : TypeId}
    (found : instantiatePlaceFieldTypeFuel? ns arguments fuel typeId = some instance_) :
    instantiatePlaceFieldTypeFuel? ns arguments fuel' typeId = some instance_ := by
  induction le with
  | refl => exact found
  | step _ ih => exact instantiatePlaceFieldTypeFuel?_mono ns arguments _ typeId instance_ ih

theorem List.mapM_compose {α β γ : Type} {f : α → Option β} {g : β → Option γ}
    {h : α → Option γ} :
    ∀ {xs : List α} {ys : List β} {zs : List γ}, xs.mapM f = some ys → ys.mapM g = some zs →
      (∀ x ∈ xs, ∀ y ∈ ys, ∀ z, f x = some y → g y = some z → h x = some z) →
      xs.mapM h = some zs
  | [], ys, zs, first, second, _ => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at first
      subst first
      simpa using second
  | x :: xs, ys, zs, first, second, step => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at first
      obtain ⟨y, y_eq, ys', ys'_eq, rfl⟩ := first
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at second ⊢
      obtain ⟨z, z_eq, zs', zs'_eq, rfl⟩ := second
      exact ⟨z, step x (by simp) y (by simp) z y_eq z_eq, zs',
        List.mapM_compose ys'_eq zs'_eq fun x member y member' =>
          step x (by simp [member]) y (by simp [member']), rfl⟩

theorem Array.mapM_compose {α β γ : Type} {f : α → Option β} {g : β → Option γ}
    {h : α → Option γ} {xs : Array α} {ys : Array β} {zs : Array γ}
    (first : xs.mapM f = some ys) (second : ys.mapM g = some zs)
    (step : ∀ x ∈ xs, ∀ y ∈ ys, ∀ z, f x = some y → g y = some z → h x = some z) :
    xs.mapM h = some zs := by
  rw [Array.mapM_eq_mapM_toList] at first second ⊢
  simp only [Functor.map, Option.map_eq_some_iff] at first second ⊢
  obtain ⟨ys', ys'_eq, rfl⟩ := first
  obtain ⟨zs', zs'_eq, rfl⟩ := second
  exact ⟨zs', List.mapM_compose ys'_eq (by simpa using zs'_eq)
    fun x member y member' => step x (Array.mem_toList_iff.mp member) y
      (by simpa using member'), rfl⟩

private theorem List.mapM_erased {gA gA' gC : GenericArgument → Option GenericArgument} :
    ∀ {args found found' outcome : List GenericArgument},
      args.mapM gA = some found → found'.mapM gC = some outcome →
      found'.map GenericArgument.eraseLoc = found.map GenericArgument.eraseLoc →
      (∀ a ∈ args, ∀ x, ∀ ac ∈ found', ∀ y, gA a = some x → ac.eraseLoc = x.eraseLoc →
        gC ac = some y → ∃ z, gA' a = some z ∧ z.eraseLoc = y.eraseLoc) →
      ∃ result, args.mapM gA' = some result ∧
        result.map GenericArgument.eraseLoc = outcome.map GenericArgument.eraseLoc
  | [], found, found', outcome, first, second, same, _ => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at first
      subst first
      simp only [List.map_nil, List.map_eq_nil_iff] at same
      subst same
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at second
      subst second
      exact ⟨[], rfl, rfl⟩
  | a :: args, found, found', outcome, first, second, same, step => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at first
      obtain ⟨x, x_eq, rest, rest_eq, rfl⟩ := first
      cases found' with
      | nil => simp at same
      | cons ac rest' =>
          simp only [List.map_cons, List.cons.injEq] at same
          simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
            Option.pure_def, Option.some.injEq] at second
          obtain ⟨y, y_eq, outcome', outcome'_eq, rfl⟩ := second
          obtain ⟨z, z_eq, z_erased⟩ := step a (by simp) x ac (by simp) y x_eq same.1 y_eq
          obtain ⟨result, result_eq, result_erased⟩ := List.mapM_erased rest_eq outcome'_eq same.2
            fun a member x ac member' => step a (by simp [member]) x ac (by simp [member'])
          refine ⟨z :: result, ?_, by simp [z_erased, result_erased]⟩
          simp [z_eq, result_eq]

private theorem Array.mapM_erased {gA gA' gC : GenericArgument → Option GenericArgument}
    {args found found' outcome : Array GenericArgument}
    (first : args.mapM gA = some found) (second : found'.mapM gC = some outcome)
    (same : found'.map GenericArgument.eraseLoc = found.map GenericArgument.eraseLoc)
    (step : ∀ a ∈ args, ∀ x, ∀ ac ∈ found', ∀ y, gA a = some x → ac.eraseLoc = x.eraseLoc →
      gC ac = some y → ∃ z, gA' a = some z ∧ z.eraseLoc = y.eraseLoc) :
    ∃ result, args.mapM gA' = some result ∧
      result.map GenericArgument.eraseLoc = outcome.map GenericArgument.eraseLoc := by
  rw [Array.mapM_eq_mapM_toList] at first second ⊢
  simp only [Functor.map, Option.map_eq_some_iff] at first second ⊢
  obtain ⟨found, found_eq, rfl⟩ := first
  obtain ⟨outcome, outcome_eq, rfl⟩ := second
  obtain ⟨result, result_eq, result_erased⟩ := List.mapM_erased found_eq outcome_eq
    (by simpa using congrArg Array.toList same) fun a member x ac member' =>
      step a (Array.mem_toList_iff.mp member) x ac (by simpa using member')
  exact ⟨result.toArray, ⟨result, result_eq, rfl⟩, by simpa using result_erased⟩

private theorem findIdx?_node {types : Array Ty} {accepts : Ty → Bool} {index : Nat}
    (found : types.findIdx? accepts = some index) :
    ∃ node, types[index]? = some node ∧ accepts node = true := by
  have holds := Array.of_findIdx?_eq_some found
  split at holds
  · exact ⟨_, ‹_›, holds⟩
  · simp at holds

private theorem Array.any_false_mem {α : Type} {p : α → Bool} {xs : Array α} {x : α}
    (none_ : xs.any p = false) (member : x ∈ xs) : p x = false := by
  cases holds : p x
  · rfl
  · have : xs.any p = true := Array.any_eq_true'.mpr ⟨x, member, holds⟩
    rw [none_] at this
    cases this

/-- A lifetime that is not a parameter instantiates to itself. -/
private theorem instantiateLifetime?_fixed {ns : ValidatedNamespace}
    {arguments : Array GenericArgument} {lifetime result : LifetimeId}
    (fixed : StaticTyping.lifetimeParameter ns lifetime = false)
    (found : instantiateLifetime? ns arguments lifetime = some result) : result = lifetime := by
  simp only [instantiateLifetime?, Option.bind_eq_bind, Option.bind_eq_some_iff] at found
  obtain ⟨declaration, declaration_eq, found⟩ := found
  simp only [StaticTyping.lifetimeParameter, declaration_eq, Option.any_some] at fixed
  cases kind : declaration.kind with
  | parameter index => rw [kind] at fixed; simp at fixed
  | _ =>
      rw [kind] at found
      simp only [Option.some.injEq] at found
      exact found.symm

/-- Rewriting type arguments leaves lifetime instantiation alone. -/
private theorem instantiateLifetime?_rewritten {ns : ValidatedNamespace}
    {table : Array (TypeId × TypeId)} {arguments : Array GenericArgument} {lifetime : LifetimeId} :
    instantiateLifetime? ns (instantiateGenericArguments table arguments) lifetime =
      instantiateLifetime? ns arguments lifetime := by
  simp only [instantiateLifetime?, instantiateGenericArguments, Array.getElem?_map]
  cases ns.tables.lifetimes[lifetime.index]? with
  | none => rfl
  | some declaration =>
      simp only [Option.bind_eq_bind, Option.bind_some]
      cases declaration.kind with
      | parameter index =>
          dsimp only
          cases arguments[index]? with
          | none => rfl
          | some argument => cases argument <;> rfl
      | _ => rfl

/-- A callee's search for the instance of a type at the caller-rewritten
arguments repeats, node by node, the caller's search: where the type
instantiates at the call's own arguments to a node the caller's arguments
instantiate, free of the caller's lifetime parameters, the rewritten
arguments instantiate it to the same node. -/
theorem instantiate_repeat {ns callerNs : ValidatedNamespace}
    (types_eq : callerNs.tables.types = ns.tables.types)
    {arguments callerArguments : Array GenericArgument} {table : Array (TypeId × TypeId)}
    (table_eq : ∀ typeId instance_,
      instantiatePlaceFieldType? callerNs callerArguments typeId = some instance_ →
        instantiatedTypeId table typeId = instance_) :
    ∀ (fuel : Nat), fuel ≤ callerNs.tables.types.size + 1 →
      ∀ (typeId instance_ result : TypeId),
        instantiatePlaceFieldTypeFuel? ns arguments fuel typeId = some instance_ →
        instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel instance_ = some result →
        StaticTyping.mentionsLifetimeParameter callerNs fuel instance_ = false →
        instantiatePlaceFieldTypeFuel? ns (instantiateGenericArguments table arguments) fuel
          typeId = some result := by
  intro fuel
  induction fuel with
  | zero => intro _ _ _ _ h; simp [instantiatePlaceFieldTypeFuel?] at h
  | succ fuel ih =>
  intro bound typeId instance_ result first second fixed
  have ih' := fun typeId instance_ result => ih (by omega) typeId instance_ result
  rw [instantiatePlaceFieldTypeFuel?] at first ⊢
  cases entry : ns.tables.types[typeId.index]? with
  | none => simp [entry] at first
  | some node =>
  simp only [entry, Option.bind_eq_bind, Option.bind_some] at first ⊢
  -- The node the call's arguments found, and the caller's search from it.
  have caller : ∀ found, callerNs.tables.types[instance_.index]? = some found →
      instantiatePlaceFieldTypeFuel? callerNs callerArguments (fuel + 1) instance_ =
        (match found with
          | .typeParameter index => match callerArguments[index]? with
              | some (.typeArg value) => some value.typeId
              | _ => none
          | .tuple elements => do
              let instantiated ← elements.mapM
                (instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel)
              let index ← callerNs.tables.types.findIdx? (fun candidate =>
                candidate == .tuple instantiated)
              some ⟨index⟩
          | .vector element length => do
              let element ← instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel element
              let index ← callerNs.tables.types.findIdx? fun candidate =>
                candidate == .vector element length
              some ⟨index⟩
          | .typeDomain nested => do
              let nested ← instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel nested
              let index ← callerNs.tables.types.findIdx? (fun candidate =>
                candidate == .typeDomain nested)
              some ⟨index⟩
          | .resourceDomain resource arguments => do
              let arguments ← arguments.mapM fun arguments =>
                arguments.mapM (instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel)
              let index ← callerNs.tables.types.findIdx? fun candidate =>
                candidate == .resourceDomain resource arguments
              some ⟨index⟩
          | .nominal name arguments => do
              let arguments ← arguments.mapM fun argument => match argument with
                | .typeArg value => do
                    let typeId ←
                      instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel value.typeId
                    some (.typeArg { value with typeId })
                | .lifetime value => .lifetime <$>
                    instantiateLifetime? callerNs callerArguments value
                | .const value => some (.const value)
                | .evidence value => some (.evidence value)
              let index ← callerNs.tables.types.findIdx? fun candidate => match candidate with
                | .nominal candidateName candidateArguments =>
                    candidateName == name &&
                      sameGenericArgumentValues candidateArguments arguments
                | _ => false
              some ⟨index⟩
          | .function arguments result abilities => do
              let arguments ← arguments.mapM
                (instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel)
              let result ← instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel result
              let index ← callerNs.tables.types.findIdx? fun candidate =>
                candidate == .function arguments result abilities
              some ⟨index⟩
          | .reference reference => do
              let referent ← instantiatePlaceFieldTypeFuel? callerNs callerArguments fuel
                reference.referent
              let lifetime ← instantiateLifetime? callerNs callerArguments reference.lifetime
              let instantiated := .reference { reference with referent, lifetime }
              let index ← callerNs.tables.types.findIdx? (fun candidate =>
                candidate == instantiated)
              some ⟨index⟩
          | _ => some instance_) := by
    intro found found_eq
    rw [instantiatePlaceFieldTypeFuel?, found_eq]
    rfl
  -- The caller's view of the node the call's arguments found.
  have located : ∀ {index : Nat} {accepts : Ty → Bool},
      ns.tables.types.findIdx? accepts = some index →
        ∃ found, callerNs.tables.types[index]? = some found ∧ accepts found = true := by
    intro index accepts found
    obtain ⟨found, found_eq, holds⟩ := findIdx?_node found
    exact ⟨found, types_eq ▸ found_eq, holds⟩
  have unmentioned : ∀ {index : Nat} {found : Ty},
      callerNs.tables.types[index]? = some found →
      StaticTyping.mentionsLifetimeParameter callerNs (fuel + 1) ⟨index⟩ = false →
        (match found with
          | .tuple elements => elements.any (StaticTyping.mentionsLifetimeParameter callerNs fuel)
          | .vector element _ | .typeDomain element =>
              StaticTyping.mentionsLifetimeParameter callerNs fuel element
          | .resourceDomain _ arguments =>
              arguments.any (·.any (StaticTyping.mentionsLifetimeParameter callerNs fuel))
          | .nominal _ arguments => arguments.any fun
              | .typeArg value => StaticTyping.mentionsLifetimeParameter callerNs fuel value.typeId
              | .lifetime value => StaticTyping.lifetimeParameter callerNs value
              | .const _ | .evidence _ => false
          | .function arguments result _ =>
              arguments.any (StaticTyping.mentionsLifetimeParameter callerNs fuel) ||
                StaticTyping.mentionsLifetimeParameter callerNs fuel result
          | .reference borrowed => StaticTyping.lifetimeParameter callerNs borrowed.lifetime ||
              StaticTyping.mentionsLifetimeParameter callerNs fuel borrowed.referent
          | _ => false) = false := by
    intro index found found_eq mentions
    rw [StaticTyping.mentionsLifetimeParameter, found_eq] at mentions
    exact mentions
  cases node with
  | typeParameter index =>
      simp only at first ⊢
      split at first
      · rename_i value value_eq
        simp only [Option.some.injEq] at first
        subst first
        simp only [instantiateGenericArguments, Array.getElem?_map, value_eq, Option.map_some]
        exact congrArg some (table_eq _ _ (instantiatePlaceFieldTypeFuel?_mono_le _ _ bound second))
      · cases first
  | tuple elements =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨instantiated', mapped', index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      refine ⟨instantiated', Array.mapM_compose mapped mapped' fun x _ y member z first second =>
        ih' x y z first second (Array.any_false_mem fixed' member), index', ?_, rfl⟩
      rw [types_eq] at found''
      exact found''
  | vector element length =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨instantiated', mapped', index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      refine ⟨instantiated', ih' _ _ _ mapped mapped' fixed', index', ?_, rfl⟩
      rw [types_eq] at found''
      exact found''
  | typeDomain nested =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨instantiated', mapped', index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      refine ⟨instantiated', ih' _ _ _ mapped mapped' fixed', index', ?_, rfl⟩
      rw [types_eq] at found''
      exact found''
  | resourceDomain resource arguments' =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨instantiated', mapped', index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      refine ⟨instantiated', ?_, index', by rw [types_eq] at found''; exact found'', rfl⟩
      cases arguments' with
      | none =>
          simp only [Option.mapM] at mapped ⊢
          cases mapped
          simpa [Option.mapM] using mapped'
      | some row =>
          simp only [Option.mapM, Option.map_eq_map, Option.map_eq_some_iff] at mapped ⊢
          obtain ⟨row', row'_eq, rfl⟩ := mapped
          simp only [Option.mapM, Option.map_eq_map, Option.map_eq_some_iff] at mapped'
          obtain ⟨row'', row''_eq, rfl⟩ := mapped'
          simp only [Option.any_some] at fixed'
          exact ⟨row'', Array.mapM_compose row'_eq row''_eq fun x _ y member z first second =>
            ih' x y z first second (Array.any_false_mem fixed' member), rfl⟩
  | function arguments' result' abilities =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, image, image_eq, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨instantiated', mapped', image', image'_eq, index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      simp only [Bool.or_eq_false_iff] at fixed'
      refine ⟨instantiated', Array.mapM_compose mapped mapped' fun x _ y member z first second =>
        ih' x y z first second (Array.any_false_mem fixed'.1 member), image',
        ih' _ _ _ image_eq image'_eq fixed'.2, index', ?_, rfl⟩
      rw [types_eq] at found''
      exact found''
  | reference reference =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨referent, referent_eq, lifetime, lifetime_eq, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases beq_iff_eq.mp accepts
      rw [instantiatePlaceFieldTypeFuel?] at second
      simp only [found'_eq, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff] at second
      obtain ⟨referent', referent'_eq, lifetime', lifetime'_eq, index', found'', result_eq⟩ := second
      simp only [Option.some.injEq] at result_eq
      subst result_eq
      have fixed' := unmentioned found'_eq fixed
      simp only [Bool.or_eq_false_iff] at fixed'
      have same := instantiateLifetime?_fixed fixed'.1 lifetime'_eq
      subst same
      refine ⟨referent', ih' _ _ _ referent_eq referent'_eq fixed'.2, _,
        by rw [instantiateLifetime?_rewritten]; exact lifetime_eq, index', ?_, rfl⟩
      rw [types_eq] at found''
      exact found''
  | nominal name arguments' =>
      simp only [Option.bind_eq_some_iff] at first ⊢
      obtain ⟨instantiated, mapped, index, found, instance_eq⟩ := first
      simp only [Option.some.injEq] at instance_eq
      subst instance_eq
      obtain ⟨found', found'_eq, accepts⟩ := located found
      cases found' with
      | nominal name' argumentsC =>
          simp only [Bool.and_eq_true, beq_iff_eq, sameGenericArgumentValues_eq,
            decide_eq_true_eq] at accepts
          obtain ⟨rfl, same⟩ := accepts
          have fixed' := unmentioned found'_eq fixed
          rw [instantiatePlaceFieldTypeFuel?] at second
          simp only [found'_eq, Option.bind_eq_bind, Option.bind_some,
            Option.bind_eq_some_iff] at second
          obtain ⟨instantiated', mapped', index', found'', result_eq⟩ := second
          simp only [Option.some.injEq] at result_eq
          subst result_eq
          obtain ⟨rewritten, rewritten_eq, erased⟩ := Array.mapM_erased
            (gA' := fun argument => match argument with
              | .typeArg value => do
                  let typeId ← instantiatePlaceFieldTypeFuel? ns
                    (instantiateGenericArguments table arguments) fuel value.typeId
                  some (.typeArg { value with typeId })
              | .lifetime value => .lifetime <$>
                  instantiateLifetime? ns (instantiateGenericArguments table arguments) value
              | .const value => some (.const value)
              | .evidence value => some (.evidence value))
            mapped mapped' same
            (fun argument _ x argumentC member y first erased second => by
              have fixedC := Array.any_false_mem fixed' member
              cases argument with
              | typeArg value =>
                  simp only [Option.bind_eq_some_iff] at first
                  obtain ⟨instance_, instance_eq, x_eq⟩ := first
                  simp only [Option.some.injEq] at x_eq
                  subst x_eq
                  cases argumentC with
                  | typeArg valueC =>
                      simp only [GenericArgument.eraseLoc, GenericArgument.typeArg.injEq,
                        TypeUse.mk.injEq, and_true] at erased
                      simp only [Option.bind_eq_some_iff] at second
                      obtain ⟨result_, result_eq, y_eq⟩ := second
                      simp only [Option.some.injEq] at y_eq
                      subst y_eq
                      simp only at fixedC
                      rw [erased] at result_eq fixedC
                      refine ⟨.typeArg { value with typeId := result_ }, ?_, by
                        simp [GenericArgument.eraseLoc]⟩
                      simp [ih' _ _ _ instance_eq result_eq fixedC]
                  | _ => simp [GenericArgument.eraseLoc] at erased
              | lifetime value =>
                  simp only [Functor.map, Option.map_eq_some_iff] at first
                  obtain ⟨lifetime, lifetime_eq, rfl⟩ := first
                  cases argumentC with
                  | lifetime valueC =>
                      simp only [GenericArgument.eraseLoc, GenericArgument.lifetime.injEq] at erased
                      subst erased
                      simp only [Functor.map, Option.map_eq_some_iff] at second
                      obtain ⟨lifetime', lifetime'_eq, rfl⟩ := second
                      have same := instantiateLifetime?_fixed fixedC lifetime'_eq
                      subst same
                      refine ⟨.lifetime lifetime', ?_, rfl⟩
                      simp only [Functor.map, instantiateLifetime?_rewritten, lifetime_eq,
                        Option.map_some]
                  | _ => simp [GenericArgument.eraseLoc] at erased
              | const value =>
                  simp only [Option.some.injEq] at first
                  subst first
                  cases argumentC with
                  | const valueC =>
                      simp only [GenericArgument.eraseLoc, GenericArgument.const.injEq] at erased
                      subst erased
                      simp only [Option.some.injEq] at second
                      subst second
                      exact ⟨_, rfl, rfl⟩
                  | _ => simp [GenericArgument.eraseLoc] at erased
              | evidence value =>
                  simp only [Option.some.injEq] at first
                  subst first
                  cases argumentC with
                  | evidence valueC =>
                      simp only [GenericArgument.eraseLoc, GenericArgument.evidence.injEq] at erased
                      subst erased
                      simp only [Option.some.injEq] at second
                      subst second
                      exact ⟨_, rfl, rfl⟩
                  | _ => simp [GenericArgument.eraseLoc] at erased)
          refine ⟨rewritten, rewritten_eq, index', ?_, rfl⟩
          rw [types_eq] at found''
          rw [← found'']
          congr 1
          funext candidate
          cases candidate <;> simp [sameGenericArgumentValues_eq, erased]
      | _ => simp at accepts
  | _ =>
      simp only [Option.some.injEq] at first
      subst first
      rw [caller _ (types_eq ▸ entry)] at second
      simpa using second

/-- Resolution reads an environment only at its type entries. -/
theorem SemTy.resolveFuel_congr {tables : Tables} {left right : Array SemArg}
    (agree : ∀ (index : Nat) (type : SemTy), left[index]? = some (SemArg.type type) ↔
      right[index]? = some (SemArg.type type)) :
    ∀ fuel, SemTy.resolveFuel tables left fuel = SemTy.resolveFuel tables right fuel := by
  intro fuel
  induction fuel with
  | zero => funext typeId; rfl
  | succ fuel ih =>
      funext typeId
      rw [SemTy.resolveFuel, SemTy.resolveFuel, ih]
      congr 1
      funext node
      cases node with
      | typeParameter index =>
          dsimp only
          cases left_eq : left[index]? with
          | none =>
              cases right_eq : right[index]? with
              | none => rfl
              | some argument =>
                  cases argument with
                  | type type => exact absurd ((agree index type).mpr right_eq) (by simp [left_eq])
                  | _ => rfl
          | some argument =>
              cases argument with
              | type type => simp [(agree index type).mp left_eq]
              | _ =>
                  cases right_eq : right[index]? with
                  | none => rfl
                  | some argument' =>
                      cases argument' with
                      | type type =>
                          exact absurd ((agree index type).mpr right_eq) (by simp [left_eq])
                      | _ => rfl
      | _ => rfl

theorem Resolves.congr {tables : Tables} {left right : Array SemArg}
    (agree : ∀ (index : Nat) (type : SemTy), left[index]? = some (SemArg.type type) ↔
      right[index]? = some (SemArg.type type))
    {typeId : TypeId} {type : SemTy} (resolved : Resolves tables left typeId type) :
    Resolves tables right typeId type := by
  obtain ⟨fuel, found⟩ := resolved
  exact ⟨fuel, by rw [← SemTy.resolveFuel_congr agree]; exact found⟩

/-- Resolution reads a namespace's tables only at their types and names. -/
theorem SemTy.resolveFuel_tables {left right : Tables} (types_eq : left.types = right.types)
    (names_eq : left.names = right.names) (env : Array SemArg) :
    ∀ fuel, SemTy.resolveFuel left env fuel = SemTy.resolveFuel right env fuel := by
  intro fuel
  induction fuel with
  | zero => funext typeId; rfl
  | succ fuel ih =>
      funext typeId
      rw [SemTy.resolveFuel, SemTy.resolveFuel, ih, types_eq, names_eq]

theorem Resolves.tables {left right : Tables} (types_eq : left.types = right.types)
    (names_eq : left.names = right.names) {env : Array SemArg} {typeId : TypeId}
    {type : SemTy} (resolved : Resolves left env typeId type) : Resolves right env typeId type := by
  obtain ⟨fuel, found⟩ := resolved
  exact ⟨fuel, by rw [← SemTy.resolveFuel_tables types_eq names_eq]; exact found⟩

/-- A faithful frame reads each type it requires as the type the type
resolves to in its environment. -/
theorem FrameInstantiation.read {ns : ValidatedNamespace} {required : Array TypeId}
    {instantiation : Array (TypeId × TypeId)} {env : Array SemArg}
    (faithful : FrameInstantiation ns required instantiation env) {typeId : TypeId}
    (member : typeId ∈ required) {type : SemTy} (resolved : Resolves ns.tables env typeId type) :
    Resolves ns.tables #[] (instantiatedTypeId instantiation typeId) type := by
  rcases faithful with ⟨rfl, closed⟩ | ⟨outer, typeArguments, rfl, bound, found⟩
  · rw [instantiatedTypeId_empty]
    exact closed typeId member type resolved
  · obtain ⟨instance_, instance_eq⟩ := Option.isSome_iff_exists.mp (found typeId member)
    rw [instantiatedTypeId_invocation instance_eq]
    exact (Resolves.instantiate bound _ typeId instance_ instance_eq type).mp resolved

theorem List.mapM_length {α β : Type} {f : α → Option β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys → ys.length = xs.length
  | [], ys, mapped => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at mapped
      simp [← mapped]
  | _ :: _, ys, mapped => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at mapped
      obtain ⟨y, -, rest', rest'_eq, rfl⟩ := mapped
      simp [List.mapM_length rest'_eq]

theorem Array.mapM_size {α β : Type} {f : α → Option β} {xs : Array α} {ys : Array β}
    (mapped : xs.mapM f = some ys) : ys.size = xs.size := by
  rw [Array.mapM_eq_mapM_toList] at mapped
  simp only [Functor.map, Option.map_eq_some_iff] at mapped
  obtain ⟨list, list_eq, rfl⟩ := mapped
  simpa using List.mapM_length list_eq

theorem Array.mapM_getElem? {α β : Type} {f : α → Option β} {xs : Array α} {ys : Array β}
    (mapped : xs.mapM f = some ys) {index : Nat} {x : α} (x_eq : xs[index]? = some x) :
    ∃ y, ys[index]? = some y ∧ f x = some y := by
  rw [Array.mapM_eq_mapM_toList] at mapped
  simp only [Functor.map, Option.map_eq_some_iff] at mapped
  obtain ⟨list, list_eq, rfl⟩ := mapped
  have : ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys → ∀ {index : Nat} {x : α},
      xs[index]? = some x → ∃ y, ys[index]? = some y ∧ f x = some y := by
    intro xs
    induction xs with
    | nil => intro ys _ index x x_eq; simp at x_eq
    | cons head rest ih =>
        intro ys mapped index x x_eq
        simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
          Option.pure_def, Option.some.injEq] at mapped
        obtain ⟨y, y_eq, rest', rest'_eq, rfl⟩ := mapped
        cases index with
        | zero =>
            simp only [List.getElem?_cons_zero, Option.some.injEq] at x_eq
            subst x_eq
            exact ⟨y, rfl, y_eq⟩
        | succ index => simpa using ih rest'_eq (by simpa using x_eq)
  simpa using this list_eq (by simpa using x_eq)

/-- A frame's environment at a type binder holds the type argument there. -/
theorem frameEnv_type {generics : Array GenericBinder} {arguments : Array SemArg}
    {index : Nat} {binder : GenericBinder} {type : SemTy}
    (binder_eq : generics[index]? = some binder) (kind : binder.kind = .typeArg)
    (argument_eq : arguments[index]? = some (.type type)) :
    (frameEnv generics arguments)[index]? = some (.type type) := by
  simp only [frameEnv, staticEnv, Array.getElem?_map, Array.getElem?_mapIdx, binder_eq,
    Option.map_some, kind]
  simp [SemArg.subst, SemTy.subst, argument_eq]

/-- Rewriting through the empty instantiation leaves arguments alone. -/
theorem instantiateGenericArguments_empty (arguments : Array GenericArgument) :
    instantiateGenericArguments #[] arguments = arguments := by
  simp only [instantiateGenericArguments, instantiatedTypeId_empty]
  conv => rhs; rw [← Array.map_id arguments]
  congr 1
  funext argument
  cases argument <;> rfl

/-- A call from a faithful frame creates a faithful frame: the target's
required types instantiate at the call's arguments to types the caller
requires, free of the caller's lifetime parameters, and its type arguments
are required of the caller. -/
theorem FrameInstantiation.call {unit : ValidatedUnit} {callerNs targetNs : ValidatedNamespace}
    {callerRequired targetRequired : Array TypeId}
    {callerInstantiation : Array (TypeId × TypeId)} {callerEnv : Array SemArg}
    {handle : FunctionHandle} {generics : Array GenericBinder}
    {instantiations : Array GenericArgument} {arguments : Array SemArg}
    (caller : FrameInstantiation callerNs callerRequired callerInstantiation callerEnv)
    (types_eq : callerNs.tables.types = targetNs.tables.types)
    (names_eq : callerNs.tables.names = targetNs.tables.names)
    (target_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (consulted : ∀ (index : Nat) (value : TypeUse),
      instantiations[index]? = some (.typeArg value) → value.typeId ∈ callerRequired)
    (edge : ∀ typeId ∈ targetRequired, ∃ instance_,
      instantiatePlaceFieldType? targetNs instantiations typeId = some instance_ ∧
        instance_ ∈ callerRequired ∧
        StaticTyping.mentionsLifetimeParameter callerNs (callerNs.tables.types.size + 1)
          instance_ = false)
    (resolved : StaticTyping.resolveArguments callerNs callerEnv instantiations = some arguments)
    (kinds : ∀ (index : Nat) (binder : GenericBinder) (argument : GenericArgument),
      generics[index]? = some binder → instantiations[index]? = some argument →
        (binder.kind = .typeArg ↔ ∃ value, argument = .typeArg value))
    (arity : instantiations.size = generics.size) :
    FrameInstantiation targetNs targetRequired
      (callTypeInstantiation unit handle callerInstantiation instantiations)
      (frameEnv generics arguments) := by
  by_cases empty : instantiations.isEmpty
  · left
    have instantiations_eq : instantiations = #[] := Array.isEmpty_iff.mp empty
    have generics_eq : generics = #[] := by
      apply Array.eq_empty_of_size_eq_zero
      rw [← arity, instantiations_eq]
      rfl
    subst generics_eq
    refine ⟨by simp [callTypeInstantiation, empty], fun typeId _ type resolved => ?_⟩
    simpa [frameEnv, staticEnv] using resolved
  · right
    have instantiation_eq : callTypeInstantiation unit handle callerInstantiation instantiations =
        invocationTypeInstantiation targetNs callerInstantiation instantiations := by
      simp [callTypeInstantiation, empty, target_eq]
    rw [instantiation_eq]
    refine ⟨callerInstantiation, instantiations, rfl, ?_, ?_⟩
    · intro index value rewritten
      simp only [instantiateGenericArguments, Array.getElem?_map, Option.map_eq_some_iff] at rewritten
      obtain ⟨argument, argument_eq, image⟩ := rewritten
      cases argument with
      | typeArg original =>
          simp only [GenericArgument.typeArg.injEq] at image
          subst image
          have bound : index < generics.size := by
            rw [← arity]
            exact (Array.getElem?_eq_some_iff.mp argument_eq).1
          obtain ⟨binder, binder_eq⟩ : ∃ binder, generics[index]? = some binder :=
            ⟨_, Array.getElem?_eq_getElem bound⟩
          have kind := (kinds index binder _ binder_eq argument_eq).mpr ⟨original, rfl⟩
          simp only [StaticTyping.resolveArguments] at resolved
          obtain ⟨semantic, semantic_eq, image⟩ := Array.mapM_getElem? resolved argument_eq
          simp only [Functor.map, Option.map_eq_some_iff] at image
          obtain ⟨type, type_eq, rfl⟩ := image
          exact ⟨type, frameEnv_type binder_eq kind semantic_eq, Resolves.tables types_eq names_eq
            (caller.read (consulted index original argument_eq) ⟨_, type_eq⟩)⟩
      | _ => simp at image
    · intro typeId member
      obtain ⟨instance_, instance_eq, required, fixed⟩ := edge typeId member
      rcases caller with ⟨rfl, -⟩ | ⟨outer, typeArguments, rfl, -, found⟩
      · rw [instantiateGenericArguments_empty]
        simp [instance_eq]
      · obtain ⟨result, result_eq⟩ := Option.isSome_iff_exists.mp (found instance_ required)
        have repeated := instantiate_repeat (ns := targetNs) (callerNs := callerNs) types_eq
          (fun typeId instance_ found => instantiatedTypeId_invocation found)
          (callerNs.tables.types.size + 1) (Nat.le_refl _) typeId instance_ result
          (by simpa [instantiatePlaceFieldType?, types_eq] using instance_eq) result_eq fixed
        simp [instantiatePlaceFieldType?, ← types_eq, repeated]

end LeanerIR

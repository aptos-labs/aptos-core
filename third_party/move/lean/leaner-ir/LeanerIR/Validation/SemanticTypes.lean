-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Instantiate

/-!
# Semantic types

The type a value inhabits, independent of the table that spells it: a type
of the table with its names spelled out, its type parameters replaced by
the generic arguments of the use, and what does not shape a value erased —
lifetimes, abilities, a reference's profile. Types of different namespaces,
of a generic frame and of its instantiation, and of a closure and its
target compare as semantic types.
-/

namespace LeanerIR

mutual
/-- A type as values inhabit it. -/
inductive SemTy where
  | unit | never | bool | character | string | bytes | address | signer
  | integer (width : IntWidth) (signed : Bool)
  | tuple (elements : List SemTy)
  | vector (element : SemTy) (length : Option ConstValue)
  | nominal (name : QualifiedName) (arguments : List SemArg)
  | function (arguments : List SemTy) (result : SemTy)
  | reference (kind : ReferenceKind) (referent : SemTy)
  | profile (value : ProfileValue)
  /-- A rigid type parameter: a generic body is typed once, under its
  parameters, for every instantiation. -/
  | param (index : Nat)
/-- A generic argument as values see it: a type, a constant, or an argument
that does not shape a value. -/
inductive SemArg where
  | type (type : SemTy)
  | const (value : ConstValue)
  | erased
end

mutual
private def SemTy.describe : SemTy → String
  | .unit => "unit" | .never => "never" | .bool => "bool" | .character => "char"
  | .string => "string" | .bytes => "bytes" | .address => "address" | .signer => "signer"
  | .integer (.bits width) signed => s!"{if signed then "i" else "u"}{width}"
  | .integer _ signed => s!"{if signed then "i" else "u"}size"
  | .tuple elements => s!"({SemTy.describeList elements})"
  | .vector element _ => s!"vector<{element.describe}>"
  | .nominal name arguments => s!"{name.name}<{SemArg.describeList arguments}>"
  | .function arguments result => s!"|{SemTy.describeList arguments}| {result.describe}"
  | .reference .shared referent => s!"&{referent.describe}"
  | .reference .mutable referent => s!"&mut {referent.describe}"
  | .profile value => s!"profile {value.tag}"
  | .param index => s!"#{index}"
private def SemTy.describeList : List SemTy → String
  | [] => ""
  | [type] => type.describe
  | type :: types => type.describe ++ ", " ++ SemTy.describeList types
private def SemArg.describe : SemArg → String
  | .type type => type.describe
  | .const _ => "const"
  | .erased => "_"
private def SemArg.describeList : List SemArg → String
  | [] => ""
  | [argument] => argument.describe
  | argument :: rest => argument.describe ++ ", " ++ SemArg.describeList rest
end

instance : Repr SemTy := ⟨fun type _ => .text (SemTy.describe type)⟩

/-! Equality of semantic types is decided structurally through the nested
lists, which a derived instance does not reach. -/
mutual
def SemTy.beq : SemTy → SemTy → Bool
  | .unit, .unit | .never, .never | .bool, .bool | .character, .character | .string, .string
  | .bytes, .bytes | .address, .address | .signer, .signer => true
  | .integer w s, .integer w' s' => decide (w = w') && s == s'
  | .tuple xs, .tuple ys => SemTy.beqList xs ys
  | .vector x l, .vector y l' => SemTy.beq x y && decide (l = l')
  | .nominal n xs, .nominal m ys => decide (n = m) && SemArg.beqList xs ys
  | .function xs r, .function ys r' => SemTy.beqList xs ys && SemTy.beq r r'
  | .reference k x, .reference k' y => decide (k = k') && SemTy.beq x y
  | .profile p, .profile q => decide (p = q)
  | .param i, .param j => i == j
  | _, _ => false
def SemTy.beqList : List SemTy → List SemTy → Bool
  | [], [] => true
  | x :: xs, y :: ys => SemTy.beq x y && SemTy.beqList xs ys
  | _, _ => false
def SemArg.beq : SemArg → SemArg → Bool
  | .type x, .type y => SemTy.beq x y
  | .const c, .const d => decide (c = d)
  | .erased, .erased => true
  | _, _ => false
def SemArg.beqList : List SemArg → List SemArg → Bool
  | [], [] => true
  | x :: xs, y :: ys => SemArg.beq x y && SemArg.beqList xs ys
  | _, _ => false
end

mutual
theorem SemTy.beq_iff : ∀ a b : SemTy, SemTy.beq a b = true ↔ a = b
  | .tuple xs, b => by
      cases b <;> simp only [SemTy.beq, Bool.false_eq_true, reduceCtorEq, SemTy.tuple.injEq]
      exact SemTy.beqList_iff _ _
  | .vector x l, b => by
      cases b <;> simp only [SemTy.beq, Bool.false_eq_true, reduceCtorEq, SemTy.vector.injEq,
        Bool.and_eq_true, decide_eq_true_eq]
      rw [SemTy.beq_iff]
  | .nominal n xs, b => by
      cases b <;> simp only [SemTy.beq, Bool.false_eq_true, reduceCtorEq, SemTy.nominal.injEq,
        Bool.and_eq_true, decide_eq_true_eq]
      rw [SemArg.beqList_iff]
  | .function xs r, b => by
      cases b <;> simp only [SemTy.beq, Bool.false_eq_true, reduceCtorEq, SemTy.function.injEq,
        Bool.and_eq_true]
      rw [SemTy.beqList_iff, SemTy.beq_iff]
  | .reference k x, b => by
      cases b <;> simp only [SemTy.beq, Bool.false_eq_true, reduceCtorEq, SemTy.reference.injEq,
        Bool.and_eq_true, decide_eq_true_eq]
      rw [SemTy.beq_iff]
  | .unit, b | .never, b | .bool, b | .character, b | .string, b | .bytes, b | .address, b
  | .signer, b | .integer _ _, b | .profile _, b | .param _, b => by
      cases b <;> simp [SemTy.beq]
theorem SemTy.beqList_iff : ∀ xs ys : List SemTy, SemTy.beqList xs ys = true ↔ xs = ys
  | [], ys => by cases ys <;> simp [SemTy.beqList]
  | x :: xs, ys => by
      cases ys <;> simp only [SemTy.beqList, Bool.false_eq_true, reduceCtorEq, Bool.and_eq_true,
        List.cons.injEq]
      rw [SemTy.beq_iff, SemTy.beqList_iff]
theorem SemArg.beq_iff : ∀ a b : SemArg, SemArg.beq a b = true ↔ a = b
  | .type x, b => by
      cases b <;> simp only [SemArg.beq, Bool.false_eq_true, reduceCtorEq, SemArg.type.injEq]
      exact SemTy.beq_iff _ _
  | .const c, b => by cases b <;> simp [SemArg.beq]
  | .erased, b => by cases b <;> simp [SemArg.beq]
theorem SemArg.beqList_iff : ∀ xs ys : List SemArg, SemArg.beqList xs ys = true ↔ xs = ys
  | [], ys => by cases ys <;> simp [SemArg.beqList]
  | x :: xs, ys => by
      cases ys <;> simp only [SemArg.beqList, Bool.false_eq_true, reduceCtorEq, Bool.and_eq_true,
        List.cons.injEq]
      rw [SemArg.beq_iff, SemArg.beqList_iff]
end

instance : DecidableEq SemTy := fun a b => decidable_of_iff _ (SemTy.beq_iff a b)
instance : DecidableEq SemArg := fun a b => decidable_of_iff _ (SemArg.beq_iff a b)

mutual
/-- Replace each rigid parameter by the type the arguments hold at its
position; a parameter without one stays. -/
def SemTy.subst (arguments : Array SemArg) : SemTy → SemTy
  | .tuple elements => .tuple (SemTy.substList arguments elements)
  | .vector element bound => .vector (element.subst arguments) bound
  | .nominal name nominalArguments => .nominal name (SemArg.substList arguments nominalArguments)
  | .function parameters result =>
      .function (SemTy.substList arguments parameters) (result.subst arguments)
  | .reference kind referent => .reference kind (referent.subst arguments)
  | .param index => match arguments[index]? with
      | some (.type type) => type
      | _ => .param index
  | type => type

def SemTy.substList (arguments : Array SemArg) : List SemTy → List SemTy
  | [] => []
  | type :: types => type.subst arguments :: SemTy.substList arguments types

def SemArg.subst (arguments : Array SemArg) : SemArg → SemArg
  | .type type => .type (type.subst arguments)
  | argument => argument

def SemArg.substList (arguments : Array SemArg) : List SemArg → List SemArg
  | [] => []
  | argument :: rest => argument.subst arguments :: SemArg.substList arguments rest
end

theorem SemTy.substList_eq_map (arguments : Array SemArg) :
    ∀ types : List SemTy, SemTy.substList arguments types = types.map (·.subst arguments)
  | [] => rfl
  | _ :: types => by simp [SemTy.substList, SemTy.substList_eq_map arguments types]

theorem SemArg.substList_eq_map (arguments : Array SemArg) :
    ∀ rest : List SemArg, SemArg.substList arguments rest = rest.map (·.subst arguments)
  | [] => rfl
  | _ :: rest => by simp [SemArg.substList, SemArg.substList_eq_map arguments rest]

/-- The semantic type of a type of the table under the semantic generic
arguments `env`, within `fuel` nested nodes. Specification-only types have
none. -/
def SemTy.resolveFuel (tables : Tables) (env : Array SemArg) : Nat → TypeId → Option SemTy
  | 0, _ => none
  | fuel + 1, typeId => do
      let type : Ty ← tables.types[typeId.index]?
      match type with
      | .unit => some .unit
      | .never => some .never
      | .bool => some .bool
      | .character => some .character
      | .string => some .string
      | .bytes => some .bytes
      | .address => some .address
      | .signer => some .signer
      | .integer width signed => some (.integer width signed)
      | .tuple elements => .tuple <$> elements.toList.mapM (resolveFuel tables env fuel)
      | .vector element bound => (.vector · bound) <$> resolveFuel tables env fuel element
      | .nominal name arguments => do
          let name ← tables.names[name.index]?
          let arguments ← arguments.toList.mapM fun
            | .typeArg value => SemArg.type <$> resolveFuel tables env fuel value.typeId
            | .const value => some (.const value)
            | .lifetime _ | .evidence _ => some .erased
          some (.nominal name arguments)
      | .function arguments result _ => do
          let arguments ← arguments.toList.mapM (resolveFuel tables env fuel)
          let result ← resolveFuel tables env fuel result
          some (.function arguments result)
      | .reference borrowed =>
          (.reference borrowed.kind ·) <$> resolveFuel tables env fuel borrowed.referent
      | .profile tag => some (.profile tag)
      | .typeParameter index => match env[index]? with
          | some (.type type) => some type
          | _ => none
      | .range | .eventStore | .typeDomain _ | .resourceDomain _ _ | .stateDomain => none

private theorem List.mapM_some_of_forall {α β : Type} {f g : α → Option β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      (∀ x ∈ xs, ∀ y, f x = some y → g x = some y) → xs.mapM g = some ys
  | [], ys, h, _ => by simpa using h
  | x :: xs, ys, h, hfg => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h ⊢
      obtain ⟨y, hy, rest, hrest, rfl⟩ := h
      exact ⟨y, hfg x (by simp) y hy, rest,
        List.mapM_some_of_forall hrest (fun x' mem => hfg x' (by simp [mem])), rfl⟩

/-- More fuel resolves to the same type. -/
theorem SemTy.resolveFuel_mono (tables : Tables) (env : Array SemArg) :
    ∀ (fuel : Nat) (typeId : TypeId) (type : SemTy),
      resolveFuel tables env fuel typeId = some type →
        resolveFuel tables env (fuel + 1) typeId = some type := by
  intro fuel
  induction fuel with
  | zero => intro _ _ h; simp [resolveFuel] at h
  | succ fuel ih =>
      intro typeId type h
      rw [resolveFuel] at h ⊢
      cases entry : tables.types[typeId.index]? with
      | none => simp [entry] at h
      | some ty =>
          simp only [entry, Option.bind_eq_bind, Option.bind_some] at h ⊢
          have lift := fun typeId type => ih typeId type
          cases ty with
          | tuple elements =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨types, resolved, rfl⟩ := h
              exact ⟨types, List.mapM_some_of_forall resolved fun x _ y => lift x y, rfl⟩
          | vector element bound =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨resolved, found, rfl⟩ := h
              exact ⟨resolved, lift _ _ found, rfl⟩
          | nominal name arguments =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h ⊢
              obtain ⟨spelled, named, resolved, found, rfl⟩ := h
              refine ⟨spelled, named, resolved, List.mapM_some_of_forall found ?_, rfl⟩
              intro argument _ result step
              cases argument with
              | typeArg value =>
                  simp only [Option.map_eq_map, Option.map_eq_some_iff] at step ⊢
                  obtain ⟨inner, resolvedInner, rfl⟩ := step
                  exact ⟨inner, lift _ _ resolvedInner, rfl⟩
              | const value => exact step
              | lifetime value => exact step
              | evidence value => exact step
          | function arguments result abilities =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h ⊢
              obtain ⟨resolved, found, image, resultFound, rfl⟩ := h
              exact ⟨resolved, List.mapM_some_of_forall found fun x _ y => lift x y, image,
                lift _ _ resultFound, rfl⟩
          | reference borrowed =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨resolved, found, rfl⟩ := h
              exact ⟨resolved, lift _ _ found, rfl⟩
          | _ => exact h

theorem SemTy.resolveFuel_mono_le (tables : Tables) (env : Array SemArg) {fuel fuel' : Nat}
    (le : fuel ≤ fuel') {typeId : TypeId} {type : SemTy}
    (resolved : resolveFuel tables env fuel typeId = some type) :
    resolveFuel tables env fuel' typeId = some type := by
  induction le with
  | refl => exact resolved
  | step _ ih => exact resolveFuel_mono tables env _ typeId type ih

/-- A type of the table resolves to a semantic type under an environment of
generic arguments, at some fuel. -/
def Resolves (tables : Tables) (env : Array SemArg) (typeId : TypeId) (type : SemTy) : Prop :=
  ∃ fuel, SemTy.resolveFuel tables env fuel typeId = some type

theorem Resolves.unique {tables : Tables} {env : Array SemArg} {typeId : TypeId}
    {left right : SemTy} : Resolves tables env typeId left → Resolves tables env typeId right →
      left = right := by
  rintro ⟨fuel, resolved⟩ ⟨fuel', resolved'⟩
  have := SemTy.resolveFuel_mono_le tables env (Nat.le_max_left fuel fuel') resolved
  have := SemTy.resolveFuel_mono_le tables env (Nat.le_max_right fuel fuel') resolved'
  simp_all

private theorem List.mapM_some_map {α β : Type} {f g : α → Option β} {h : β → β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      (∀ x ∈ xs, ∀ y, f x = some y → g x = some (h y)) → xs.mapM g = some (ys.map h)
  | [], ys, found, _ => by simp at found; subst found; rfl
  | x :: xs, ys, found, each => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at found ⊢
      obtain ⟨y, hy, rest, hrest, rfl⟩ := found
      exact ⟨h y, each x (by simp) y hy, rest.map h,
        List.mapM_some_map hrest (fun x' mem => each x' (by simp [mem])), rfl⟩

/-- Substituting the environment substitutes the type it resolves to: a
body typed under its rigid parameters is typed under every instantiation. -/
theorem SemTy.resolveFuel_subst (tables : Tables) (env : Array SemArg) (arguments : Array SemArg) :
    ∀ (fuel : Nat) (typeId : TypeId) (type : SemTy),
      resolveFuel tables env fuel typeId = some type →
        resolveFuel tables (env.map (·.subst arguments)) fuel typeId =
          some (type.subst arguments) := by
  intro fuel
  induction fuel with
  | zero => intro _ _ h; simp [resolveFuel] at h
  | succ fuel ih =>
      intro typeId type h
      rw [resolveFuel] at h ⊢
      cases entry : tables.types[typeId.index]? with
      | none => simp [entry] at h
      | some node =>
          simp only [entry, Option.bind_eq_bind, Option.bind_some] at h ⊢
          cases node with
          | tuple elements =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨types, found, rfl⟩ := h
              refine ⟨types.map (·.subst arguments),
                List.mapM_some_map found fun x _ y => ih x y, ?_⟩
              simp [SemTy.subst, SemTy.substList_eq_map]
          | vector element bound =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨inner, found, rfl⟩ := h
              exact ⟨_, ih _ _ found, by simp [SemTy.subst]⟩
          | nominal name nominalArguments =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h ⊢
              obtain ⟨spelled, named, resolved, found, rfl⟩ := h
              refine ⟨spelled, named, resolved.map (·.subst arguments),
                List.mapM_some_map found ?_, by simp [SemTy.subst, SemArg.substList_eq_map]⟩
              intro argument _ result step
              cases argument with
              | typeArg value =>
                  simp only [Option.map_eq_map, Option.map_eq_some_iff] at step ⊢
                  obtain ⟨inner, resolvedInner, rfl⟩ := step
                  exact ⟨_, ih _ _ resolvedInner, rfl⟩
              | const value => cases step; rfl
              | lifetime value => cases step; rfl
              | evidence value => cases step; rfl
          | function parameters result abilities =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h ⊢
              obtain ⟨types, found, resultType, resultFound, rfl⟩ := h
              refine ⟨types.map (·.subst arguments),
                List.mapM_some_map found fun x _ y => ih x y, _, ih _ _ resultFound, ?_⟩
              simp [SemTy.subst, SemTy.substList_eq_map]
          | reference borrowed =>
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at h ⊢
              obtain ⟨inner, found, rfl⟩ := h
              exact ⟨_, ih _ _ found, by simp [SemTy.subst]⟩
          | typeParameter index =>
              cases bound : env[index]? with
              | none => simp [bound] at h
              | some argument =>
                  cases argument <;> simp [bound] at h
                  subst h
                  simp [Array.getElem?_map, bound, SemArg.subst]
          | _ =>
              simp only [Option.some.injEq, reduceCtorEq] at h ⊢
              all_goals (subst h; rfl)

theorem Resolves.subst {tables : Tables} {env : Array SemArg} {typeId : TypeId} {type : SemTy}
    (arguments : Array SemArg) (resolved : Resolves tables env typeId type) :
    Resolves tables (env.map (·.subst arguments)) typeId (type.subst arguments) := by
  obtain ⟨fuel, found⟩ := resolved
  exact ⟨fuel, SemTy.resolveFuel_subst tables env arguments fuel typeId type found⟩

/-- Elementwise resolution of a row of types. -/
inductive ResolvesAll (tables : Tables) (env : Array SemArg) : List TypeId → List SemTy → Prop
  | nil : ResolvesAll tables env [] []
  | cons {typeId : TypeId} {type : SemTy} {typeIds : List TypeId} {types : List SemTy} :
      Resolves tables env typeId type → ResolvesAll tables env typeIds types →
        ResolvesAll tables env (typeId :: typeIds) (type :: types)

theorem ResolvesAll.of_mapM {tables : Tables} {env : Array SemArg} {fuel : Nat} :
    ∀ {typeIds : List TypeId} {types : List SemTy},
      typeIds.mapM (SemTy.resolveFuel tables env fuel) = some types →
        ResolvesAll tables env typeIds types
  | [], types, h => by simp at h; subst h; exact .nil
  | typeId :: rest, types, h => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h
      obtain ⟨head, found, tail, foundTail, rfl⟩ := h
      exact .cons ⟨fuel, found⟩ (ResolvesAll.of_mapM foundTail)

theorem ResolvesAll.to_mapM {tables : Tables} {env : Array SemArg} :
    ∀ {typeIds : List TypeId} {types : List SemTy}, ResolvesAll tables env typeIds types →
      ∃ fuel, ∀ fuel', fuel ≤ fuel' →
        typeIds.mapM (SemTy.resolveFuel tables env fuel') = some types
  | [], [], .nil => ⟨0, fun _ _ => rfl⟩
  | _ :: _, _ :: _, .cons ⟨fuel, found⟩ rest => by
      obtain ⟨fuelTail, foundTail⟩ := ResolvesAll.to_mapM rest
      refine ⟨max fuel fuelTail, fun fuel' le => ?_⟩
      simp only [List.mapM_cons, Option.bind_eq_bind,
        SemTy.resolveFuel_mono_le tables env (Nat.le_trans (Nat.le_max_left _ _) le) found,
        Option.bind_some, foundTail fuel' (Nat.le_trans (Nat.le_max_right _ _) le), Option.pure_def]

/-- Elementwise resolution of generic arguments: a type argument to its
type, a constant to itself, any other argument to `erased`. -/
inductive ArgumentsResolve (tables : Tables) (env : Array SemArg) :
    List GenericArgument → List SemArg → Prop
  | nil : ArgumentsResolve tables env [] []
  | type {value : TypeUse} {type : SemTy} {arguments : List GenericArgument}
      {resolved : List SemArg} :
      Resolves tables env value.typeId type → ArgumentsResolve tables env arguments resolved →
        ArgumentsResolve tables env (.typeArg value :: arguments) (.type type :: resolved)
  | const {value : ConstValue} {arguments : List GenericArgument} {resolved : List SemArg} :
      ArgumentsResolve tables env arguments resolved →
        ArgumentsResolve tables env (.const value :: arguments) (.const value :: resolved)
  | lifetime {value : LifetimeId} {arguments : List GenericArgument} {resolved : List SemArg} :
      ArgumentsResolve tables env arguments resolved →
        ArgumentsResolve tables env (.lifetime value :: arguments) (.erased :: resolved)
  | evidence {value : EvidenceId} {arguments : List GenericArgument}
      {resolved : List SemArg} :
      ArgumentsResolve tables env arguments resolved →
        ArgumentsResolve tables env (.evidence value :: arguments) (.erased :: resolved)

/-- What a node of the table resolves to, its children read by `Resolves`. -/
def NodeResolves (tables : Tables) (env : Array SemArg) : Ty → SemTy → Prop
  | .unit, type => type = .unit
  | .never, type => type = .never
  | .bool, type => type = .bool
  | .character, type => type = .character
  | .string, type => type = .string
  | .bytes, type => type = .bytes
  | .address, type => type = .address
  | .signer, type => type = .signer
  | .integer width signed, type => type = .integer width signed
  | .tuple elements, type =>
      ∃ resolved, ResolvesAll tables env elements.toList resolved ∧ type = .tuple resolved
  | .vector element bound, type =>
      ∃ resolved, Resolves tables env element resolved ∧ type = .vector resolved bound
  | .nominal name arguments, type =>
      ∃ spelled resolved, tables.names[name.index]? = some spelled ∧
        ArgumentsResolve tables env arguments.toList resolved ∧ type = .nominal spelled resolved
  | .function arguments result _, type =>
      ∃ resolved resolvedResult, ResolvesAll tables env arguments.toList resolved ∧
        Resolves tables env result resolvedResult ∧ type = .function resolved resolvedResult
  | .reference borrowed, type =>
      ∃ resolved, Resolves tables env borrowed.referent resolved ∧
        type = .reference borrowed.kind resolved
  | .profile tag, type => type = .profile tag
  | .typeParameter index, type => env[index]? = some (.type type)
  | .range, _ | .eventStore, _ | .typeDomain _, _ | .resourceDomain _ _, _ | .stateDomain, _ =>
      False

theorem ArgumentsResolve.of_mapM {tables : Tables} {env : Array SemArg} {fuel : Nat} :
    ∀ {arguments : List GenericArgument} {resolved : List SemArg},
      arguments.mapM (fun
          | .typeArg value => SemArg.type <$> SemTy.resolveFuel tables env fuel value.typeId
          | .const value => some (.const value)
          | .lifetime _ | .evidence _ => some .erased) = some resolved →
        ArgumentsResolve tables env arguments resolved
  | [], resolved, h => by simp at h; subst h; exact .nil
  | argument :: rest, resolved, h => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h
      obtain ⟨head, found, tail, foundTail, rfl⟩ := h
      cases argument with
      | typeArg value =>
          simp only [Option.map_eq_map, Option.map_eq_some_iff] at found
          obtain ⟨type, resolvedType, rfl⟩ := found
          exact .type ⟨fuel, resolvedType⟩ (ArgumentsResolve.of_mapM foundTail)
      | const value =>
          cases found
          exact .const (ArgumentsResolve.of_mapM foundTail)
      | lifetime value =>
          cases found
          exact .lifetime (ArgumentsResolve.of_mapM foundTail)
      | evidence value =>
          cases found
          exact .evidence (ArgumentsResolve.of_mapM foundTail)

theorem ArgumentsResolve.to_mapM {tables : Tables} {env : Array SemArg} :
    ∀ {arguments : List GenericArgument} {resolved : List SemArg},
      ArgumentsResolve tables env arguments resolved →
        ∃ fuel, ∀ fuel', fuel ≤ fuel' →
          arguments.mapM (fun
              | .typeArg value => SemArg.type <$> SemTy.resolveFuel tables env fuel' value.typeId
              | .const value => some (.const value)
              | .lifetime _ | .evidence _ => some .erased) = some resolved
  | [], [], .nil => ⟨0, fun _ _ => rfl⟩
  | _, _, .type ⟨fuel, found⟩ rest => by
      obtain ⟨fuelTail, foundTail⟩ := ArgumentsResolve.to_mapM rest
      refine ⟨max fuel fuelTail, fun fuel' le => ?_⟩
      have tail := foundTail fuel' (Nat.le_trans (Nat.le_max_right _ _) le)
      simp only [Option.map_eq_map] at tail
      simp only [List.mapM_cons, Option.bind_eq_bind,
        SemTy.resolveFuel_mono_le tables env (Nat.le_trans (Nat.le_max_left _ _) le) found,
        Option.map_eq_map, Option.map_some, Option.bind_some, tail, Option.pure_def]
  | _, _, .const rest | _, _, .lifetime rest | _, _, .evidence rest => by
      obtain ⟨fuelTail, foundTail⟩ := ArgumentsResolve.to_mapM rest
      refine ⟨fuelTail, fun fuel' le => ?_⟩
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_some, foundTail fuel' le,
        Option.pure_def]

/-- A node resolves as its children do. -/
theorem Resolves.node {tables : Tables} {env : Array SemArg} {typeId : TypeId} {node : Ty}
    (entry : tables.types[typeId.index]? = some node) (type : SemTy) :
    Resolves tables env typeId type ↔ NodeResolves tables env node type := by
  constructor
  · rintro ⟨fuel, resolved⟩
    cases fuel with
    | zero => simp [SemTy.resolveFuel] at resolved
    | succ fuel =>
        rw [SemTy.resolveFuel] at resolved
        simp only [entry, Option.bind_eq_bind, Option.bind_some] at resolved
        cases node with
        | tuple elements =>
            simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved
            obtain ⟨types, found, rfl⟩ := resolved
            exact ⟨types, ResolvesAll.of_mapM found, rfl⟩
        | vector element bound =>
            simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved
            obtain ⟨inner, found, rfl⟩ := resolved
            exact ⟨inner, ⟨fuel, found⟩, rfl⟩
        | nominal name arguments =>
            simp only [Option.bind_eq_some_iff, Option.some.injEq] at resolved
            obtain ⟨spelled, named, types, found, rfl⟩ := resolved
            exact ⟨spelled, types, named, ArgumentsResolve.of_mapM found, rfl⟩
        | function arguments result abilities =>
            simp only [Option.bind_eq_some_iff, Option.some.injEq] at resolved
            obtain ⟨types, found, resultType, resultFound, rfl⟩ := resolved
            exact ⟨types, resultType, ResolvesAll.of_mapM found, ⟨fuel, resultFound⟩, rfl⟩
        | reference borrowed =>
            simp only [Option.map_eq_map, Option.map_eq_some_iff] at resolved
            obtain ⟨inner, found, rfl⟩ := resolved
            exact ⟨inner, ⟨fuel, found⟩, rfl⟩
        | typeParameter index =>
            simp only [NodeResolves]
            cases bound : env[index]? with
            | none => simp [bound] at resolved
            | some argument => cases argument <;> simp_all
        | _ => simp_all [NodeResolves, eq_comm]
  · intro holds
    cases node with
    | tuple elements =>
        obtain ⟨types, all, rfl⟩ := holds
        obtain ⟨fuel, found⟩ := all.to_mapM
        refine ⟨fuel + 1, ?_⟩
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some,
          found fuel (Nat.le_refl _), Option.map_eq_map, Option.map_some]
    | vector element bound =>
        obtain ⟨inner, ⟨fuel, found⟩, rfl⟩ := holds
        refine ⟨fuel + 1, ?_⟩
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some, found,
          Option.map_eq_map, Option.map_some]
    | nominal name arguments =>
        obtain ⟨spelled, types, named, all, rfl⟩ := holds
        obtain ⟨fuel, found⟩ := all.to_mapM
        refine ⟨fuel + 1, ?_⟩
        have arguments := found fuel (Nat.le_refl _)
        simp only [Option.map_eq_map] at arguments
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some, named,
          arguments, Option.map_eq_map]
    | function arguments result abilities =>
        obtain ⟨types, resultType, all, ⟨resultFuel, resultFound⟩, rfl⟩ := holds
        obtain ⟨fuel, found⟩ := all.to_mapM
        refine ⟨max fuel resultFuel + 1, ?_⟩
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some,
          found _ (Nat.le_max_left _ _),
          SemTy.resolveFuel_mono_le tables env (Nat.le_max_right _ _) resultFound]
    | reference borrowed =>
        obtain ⟨inner, ⟨fuel, found⟩, rfl⟩ := holds
        refine ⟨fuel + 1, ?_⟩
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some, found,
          Option.map_eq_map, Option.map_some]
    | typeParameter index =>
        refine ⟨1, ?_⟩
        simp only [NodeResolves] at holds
        simp only [SemTy.resolveFuel, entry, Option.bind_eq_bind, Option.bind_some, holds]
    | _ =>
        refine ⟨1, ?_⟩
        simp_all [NodeResolves, SemTy.resolveFuel]

/-! ## Instantiation

A generic frame's types are the declaration's types with its type
parameters replaced: the runtime finds each instantiated node already
interned in the table (`Validation.instantiatePlaceFieldType?`). Resolving
the interned node without parameters gives the type resolving the
declaration's node under the arguments gives. -/

private theorem findIdx?_node {types : Array Ty} {accepts : Ty → Bool} {index : Nat}
    (found : types.findIdx? accepts = some index) :
    ∃ node, types[index]? = some node ∧ accepts node = true := by
  have holds := Array.of_findIdx?_eq_some found
  split at holds
  · exact ⟨_, ‹_›, holds⟩
  · simp at holds

private theorem resolvesAll_instantiate {tables : Tables} {env : Array SemArg}
    {instantiate : TypeId → Option TypeId}
    (each : ∀ typeId concrete, instantiate typeId = some concrete →
      ∀ type, Resolves tables env typeId type ↔ Resolves tables #[] concrete type) :
    ∀ {typeIds concretes : List TypeId}, typeIds.mapM instantiate = some concretes →
      ∀ types, ResolvesAll tables env typeIds types ↔ ResolvesAll tables #[] concretes types
  | [], concretes, h, types => by
      simp at h; subst h
      constructor <;> intro all <;> cases all <;> exact .nil
  | typeId :: rest, concretes, h, types => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h
      obtain ⟨concrete, found, tail, foundTail, rfl⟩ := h
      constructor
      · intro all
        cases all with
        | cons head tailAll =>
            exact .cons ((each _ _ found _).mp head)
              ((resolvesAll_instantiate each foundTail _).mp tailAll)
      · intro all
        cases all with
        | cons head tailAll =>
            exact .cons ((each _ _ found _).mpr head)
              ((resolvesAll_instantiate each foundTail _).mpr tailAll)

private theorem argumentsResolve_instantiate {tables : Tables} {env : Array SemArg}
    {instantiate : TypeId → Option TypeId} {lifetime : LifetimeId → Option LifetimeId}
    (each : ∀ typeId concrete, instantiate typeId = some concrete →
      ∀ type, Resolves tables env typeId type ↔ Resolves tables #[] concrete type) :
    ∀ {arguments concretes : List GenericArgument},
      arguments.mapM (fun argument => match argument with
        | .typeArg value => do
            let typeId ← instantiate value.typeId
            some (.typeArg { value with typeId })
        | .lifetime value => .lifetime <$> lifetime value
        | .const value => some (.const value)
        | .evidence value => some (.evidence value)) = some concretes →
      ∀ resolved, ArgumentsResolve tables env arguments resolved ↔
        ArgumentsResolve tables #[] concretes resolved
  | [], concretes, h, resolved => by
      simp at h; subst h
      constructor <;> intro all <;> cases all <;> exact .nil
  | argument :: rest, concretes, h, resolved => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at h
      obtain ⟨concrete, found, tail, foundTail, rfl⟩ := h
      have restIff := argumentsResolve_instantiate each foundTail
      cases argument with
      | typeArg value =>
          simp only [Option.bind_eq_some_iff, Option.some.injEq] at found
          obtain ⟨typeId, instantiated, rfl⟩ := found
          constructor
          · intro all
            cases all with
            | type head tailAll =>
                exact .type ((each _ _ instantiated _).mp head) ((restIff _).mp tailAll)
          · intro all
            cases all with
            | type head tailAll =>
                exact .type ((each _ _ instantiated _).mpr head) ((restIff _).mpr tailAll)
      | lifetime value =>
          simp only [Option.map_eq_map, Option.map_eq_some_iff] at found
          obtain ⟨instantiated, -, rfl⟩ := found
          constructor
          · intro all
            cases all with
            | lifetime tailAll => exact .lifetime ((restIff _).mp tailAll)
          · intro all
            cases all with
            | lifetime tailAll => exact .lifetime ((restIff _).mpr tailAll)
      | const value =>
          cases found
          constructor
          · intro all
            cases all with
            | const tailAll => exact .const ((restIff _).mp tailAll)
          · intro all
            cases all with
            | const tailAll => exact .const ((restIff _).mpr tailAll)
      | evidence value =>
          cases found
          constructor
          · intro all
            cases all with
            | evidence tailAll => exact .evidence ((restIff _).mp tailAll)
          · intro all
            cases all with
            | evidence tailAll => exact .evidence ((restIff _).mpr tailAll)

/-- Arguments the arena lookup identifies resolve alike: it compares type
arguments by their types, not their locations. -/
private theorem argumentsResolve_same {tables : Tables} {env : Array SemArg} :
    ∀ {left right : List GenericArgument}, left.length = right.length →
      ((left.zip right).all fun (left, right) =>
        Validation.sameGenericArgumentValue left right) = true →
      ∀ resolved, ArgumentsResolve tables env left resolved ↔
        ArgumentsResolve tables env right resolved
  | [], [], _, _, resolved => Iff.rfl
  | leftHead :: leftRest, rightHead :: rightRest, lengths, same, resolved => by
      simp only [List.zip_cons_cons, List.all_cons, Bool.and_eq_true] at same
      have restIff := argumentsResolve_same (env := env) (tables := tables)
        (by simpa using lengths) same.2
      cases leftHead <;> cases rightHead <;>
        simp [Validation.sameGenericArgumentValue] at same
      case typeArg.typeArg left right =>
        constructor
        · intro all
          cases all with
          | type head tailAll =>
              exact .type (by rw [← same.1]; exact head) ((restIff _).mp tailAll)
        · intro all
          cases all with
          | type head tailAll =>
              exact .type (by rw [same.1]; exact head) ((restIff _).mpr tailAll)
      case const.const left right =>
        obtain ⟨rfl, -⟩ := same
        constructor
        · intro all
          cases all with
          | const tailAll => exact .const ((restIff _).mp tailAll)
        · intro all
          cases all with
          | const tailAll => exact .const ((restIff _).mpr tailAll)
      case lifetime.lifetime left right =>
        constructor
        · intro all
          cases all with
          | lifetime tailAll => exact .lifetime ((restIff _).mp tailAll)
        · intro all
          cases all with
          | lifetime tailAll => exact .lifetime ((restIff _).mpr tailAll)
      case evidence.evidence left right =>
        constructor
        · intro all
          cases all with
          | evidence tailAll => exact .evidence ((restIff _).mp tailAll)
        · intro all
          cases all with
          | evidence tailAll => exact .evidence ((restIff _).mpr tailAll)

/-- The type an interned instantiation resolves to without parameters is the
type its declaration node resolves to under the arguments, where every type
argument resolves to the type the environment holds at its position. -/
theorem Resolves.instantiate {ns : Validation.ValidatedNamespace}
    {arguments : Array GenericArgument} {env : Array SemArg}
    (bound : ∀ (index : Nat) (value : TypeUse),
      arguments[index]? = some (GenericArgument.typeArg value) →
        ∃ type, env[index]? = some (SemArg.type type) ∧ Resolves ns.tables #[] value.typeId type) :
    ∀ fuel typeId concrete,
      Validation.instantiatePlaceFieldTypeFuel? ns arguments fuel typeId = some concrete →
        ∀ type, Resolves ns.tables env typeId type ↔ Resolves ns.tables #[] concrete type := by
  intro fuel
  induction fuel with
  | zero => intro _ _ h; simp [Validation.instantiatePlaceFieldTypeFuel?] at h
  | succ fuel ih =>
      intro typeId concrete h type
      rw [Validation.instantiatePlaceFieldTypeFuel?] at h
      cases entry : ns.tables.types[typeId.index]? with
      | none => simp [entry] at h
      | some node =>
          simp only [entry, Option.bind_eq_bind, Option.bind_some] at h
          rw [Resolves.node entry]
          cases node with
          | typeParameter index =>
              cases argument : arguments[index]? with
              | none => simp [argument] at h
              | some argument' =>
                  cases argument' <;> simp [argument] at h
                  rename_i value
                  subst h
                  obtain ⟨resolved, held, resolves⟩ := bound index value argument
                  simp only [NodeResolves, held, Option.some.injEq, SemArg.type.injEq]
                  constructor
                  · rintro rfl; exact resolves
                  · intro other; exact Resolves.unique resolves other
          | tuple elements =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, mapped, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .tuple instantiated := by simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              rw [Array.mapM_eq_mapM_toList] at mapped
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at mapped
              obtain ⟨list, mappedList, rfl⟩ := mapped
              have all := resolvesAll_instantiate (fun typeId concrete h => ih typeId concrete h)
                mappedList
              simp only [NodeResolves]
              constructor
              · rintro ⟨types, resolves, rfl⟩; exact ⟨types, (all types).mp resolves, rfl⟩
              · rintro ⟨types, resolves, rfl⟩; exact ⟨types, (all types).mpr resolves, rfl⟩
          | vector element bound =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, mapped, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .vector instantiated bound := by simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              simp only [NodeResolves]
              constructor
              · rintro ⟨inner, resolves, rfl⟩
                exact ⟨inner, (ih _ _ mapped inner).mp resolves, rfl⟩
              · rintro ⟨inner, resolves, rfl⟩
                exact ⟨inner, (ih _ _ mapped inner).mpr resolves, rfl⟩
          | nominal name nominalArguments =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, mapped, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              cases node with
              | nominal candidate candidateArguments =>
                  simp only [Bool.and_eq_true, beq_iff_eq] at same
                  obtain ⟨rfl, sameArguments⟩ := same
                  rw [Resolves.node at_index]
                  simp only [Validation.sameGenericArgumentValues, Bool.and_eq_true,
                    beq_iff_eq] at sameArguments
                  rw [Array.mapM_eq_mapM_toList] at mapped
                  simp only [Option.map_eq_map, Option.map_eq_some_iff] at mapped
                  obtain ⟨list, mappedList, rfl⟩ := mapped
                  have all := argumentsResolve_instantiate
                    (fun typeId concrete h => ih typeId concrete h) mappedList
                  have sameList : ((candidateArguments.toList.zip list).all fun (left, right) =>
                      Validation.sameGenericArgumentValue left right) = true := by
                    have zipped := sameArguments.2
                    rw [← Array.all_toList, Array.toList_zip] at zipped
                    simpa using zipped
                  have sameAll := argumentsResolve_same (tables := ns.tables) (env := #[])
                    (left := candidateArguments.toList) (right := list)
                    (by simpa using sameArguments.1) sameList
                  simp only [NodeResolves]
                  constructor
                  · rintro ⟨spelled, resolved, named, resolves, rfl⟩
                    exact ⟨spelled, resolved, named,
                      (sameAll resolved).mpr ((all resolved).mp resolves), rfl⟩
                  · rintro ⟨spelled, resolved, named, resolves, rfl⟩
                    exact ⟨spelled, resolved, named,
                      (all resolved).mpr ((sameAll resolved).mp resolves), rfl⟩
              | _ => simp at same
          | function functionArguments result abilities =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, mapped, instantiatedResult, mappedResult, index, found,
                rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .function instantiated instantiatedResult abilities := by
                simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              rw [Array.mapM_eq_mapM_toList] at mapped
              simp only [Option.map_eq_map, Option.map_eq_some_iff] at mapped
              obtain ⟨list, mappedList, rfl⟩ := mapped
              have all := resolvesAll_instantiate (fun typeId concrete h => ih typeId concrete h)
                mappedList
              simp only [NodeResolves]
              constructor
              · rintro ⟨types, resultType, resolves, resultResolves, rfl⟩
                exact ⟨types, resultType, (all types).mp resolves,
                  (ih _ _ mappedResult resultType).mp resultResolves, rfl⟩
              · rintro ⟨types, resultType, resolves, resultResolves, rfl⟩
                exact ⟨types, resultType, (all types).mpr resolves,
                  (ih _ _ mappedResult resultType).mpr resultResolves, rfl⟩
          | reference borrowed =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨referent, mapped, lifetime, -, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .reference { borrowed with referent, lifetime } := by
                simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              simp only [NodeResolves]
              constructor
              · rintro ⟨inner, resolves, rfl⟩
                exact ⟨inner, (ih _ _ mapped inner).mp resolves, rfl⟩
              · rintro ⟨inner, resolves, rfl⟩
                exact ⟨inner, (ih _ _ mapped inner).mpr resolves, rfl⟩
          | typeDomain nested =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, -, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .typeDomain instantiated := by simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              simp only [NodeResolves]
          | resourceDomain resource domainArguments =>
              simp only [Option.bind_eq_some_iff, Option.some.injEq] at h
              obtain ⟨instantiated, -, index, found, rfl⟩ := h
              obtain ⟨node, at_index, same⟩ := findIdx?_node found
              have node_eq : node = .resourceDomain resource instantiated := by simpa using same
              subst node_eq
              rw [Resolves.node at_index]
              simp only [NodeResolves]
          | _ =>
              simp only [Option.some.injEq] at h
              subst h
              rw [Resolves.node entry]
              simp only [NodeResolves]

end LeanerIR

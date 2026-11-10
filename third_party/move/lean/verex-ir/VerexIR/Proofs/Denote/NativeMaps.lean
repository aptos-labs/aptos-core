-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexIR.Proofs.Denote.Term
import VerexIR.Proofs.Denote.NominalQuote
import VerexIR.Proofs.Denote.FamilyTransport

/-!
# The entries of native tables

A map owner's native value holds its entries in a table carrier
(`SpecEntries`). A specification reads them as a typed map
(`SpecMap.ofEntries`), its keys and values read by their typed views:
integers by their values, other types as they are, a type parameter's as
its family reads them. The normal form states each observation of that map.
-/

namespace VerexIR.SpecMap

/-- The typed map of a table carrier, its keys and values read by `key` and
`value`. -/
def ofEntries {K V K' V' : Type} (key : K → K') (value : V → V') (table : SpecEntries (K × V)) :
    SpecMap K' V' :=
  table.entries.map fun entry => (key entry.1, value entry.2)

/-- The entries of a table's map are its own entries read. -/
theorem entriesSatisfy_ofEntries {K V K' V' : Type} {property : K' × V' → Prop}
    (key : K → K') (value : V → V') (table : SpecEntries (K × V))
    (read : ∀ entry ∈ table.entries, property (key entry.1, value entry.2)) :
    EntriesSatisfy property (ofEntries key value table) := by
  intro entry member
  simp only [ofEntries, List.mem_map] at member
  obtain ⟨stored, held, rfl⟩ := member
  exact read stored held

/-- A map written back into a table (`unread` each entry, as a
specification's map value is held) reads back as the map, where it fits a
table's length and each entry reads back as itself. -/
theorem ofEntries_ofList_map {K V K' V' : Type} (key : K → K') (value : V → V')
    (unread : K' × V' → K × V) (map : SpecMap K' V') (default : SpecEntries (K × V))
    (bounded : SpecMap.size map < 2 ^ 64)
    (roundtrip : EntriesSatisfy (fun entry => (key (unread entry).1, value (unread entry).2) = entry)
      map) :
    ofEntries key value (SpecEntries.ofList (map.map unread) default) = map := by
  have length : (map.map unread).length < 2 ^ 64 := by
    simp only [List.length_map]; simp only [SpecMap.size] at bounded; omega
  simp only [ofEntries, SpecEntries.entries_ofList default length, List.map_map]
  conv => rhs; rw [← List.map_id map]
  apply List.map_congr_left
  intro entry member
  exact roundtrip entry member

/-- A typed map written back into a table, as a specification's map value
is held: each entry unread, where the entries fit a table's length. -/
def writeBack {K V K' V' : Type} (unread : K' × V' → K × V) (map : SpecMap K' V') :
    SpecEntries (K × V) :=
  SpecEntries.ofList (map.map unread) default

/-- A written-back map reads back as the map (`ofEntries_ofList_map`). -/
theorem ofEntries_writeBack {K V K' V' : Type} (key : K → K') (value : V → V')
    (unread : K' × V' → K × V) (map : SpecMap K' V') (bounded : SpecMap.size map < 2 ^ 64)
    (roundtrip : EntriesSatisfy (fun entry => (key (unread entry).1, value (unread entry).2) = entry)
      map) :
    ofEntries key value (writeBack unread map) = map :=
  ofEntries_ofList_map key value unread map default bounded roundtrip

/-- A table is its typed map written back: the reads of its keys and values
read back. -/
theorem eq_writeBack_of_ofEntries {K V K' V' : Type} {key : K → K'} {value : V → V'}
    {unreadKey : K' → K} {unreadValue : V' → V}
    (keyBack : ∀ k, unreadKey (key k) = k) (valueBack : ∀ v, unreadValue (value v) = v)
    {table : SpecEntries (K × V)} {map : SpecMap K' V'} (defined : ofEntries key value table = map) :
    table = writeBack (fun entry => (unreadKey entry.1, unreadValue entry.2)) map := by
  subst defined
  have length : ((ofEntries key value table).map
      (fun entry => (unreadKey entry.1, unreadValue entry.2))).length < 2 ^ 64 := by
    simpa [ofEntries] using table.bounded
  apply SpecEntries.ext
  rw [writeBack, SpecEntries.entries_ofList default length]
  simp only [ofEntries, List.map_map]
  conv => lhs; rw [← List.map_id table.entries]
  apply List.map_congr_left
  intro entry _
  simp [keyBack, valueBack]

/-- The size of a table's map is the table's length, bounded by its carrier. -/
theorem size_ofEntries_bounds {K V K' V' : Type} (key : K → K') (value : V → V')
    (table : SpecEntries (K × V)) :
    0 ≤ SpecMap.size (SpecMap.ofEntries key value table) ∧
      SpecMap.size (SpecMap.ofEntries key value table) < 18446744073709551616 := by
  have := table.bounded
  simp only [SpecMap.size, SpecMap.ofEntries, List.length_map]
  omega

/-! The length a written-back map fits, structurally: a table's map fits its
carrier, an update at a key the map has keeps its size, a removal does not
grow it. -/

theorem size_ofEntries_lt {K V K' V' : Type} (key : K → K') (value : V → V')
    (table : SpecEntries (K × V)) : SpecMap.size (ofEntries key value table) < 2 ^ 64 :=
  (size_ofEntries_bounds key value table).2

theorem size_update_present_lt {K V : Type} [DecidableEq K] {discipline : Discipline K}
    {map : SpecMap K V} {key : K} {value : V} (valid : Valid discipline map)
    (present : hasKey map key = true) (bounded : SpecMap.size map < 2 ^ 64) :
    SpecMap.size (update discipline map key value) < 2 ^ 64 := by
  rw [size_update valid, if_pos present]; exact bounded

theorem size_update_lt {K V : Type} [DecidableEq K] {discipline : Discipline K}
    {map : SpecMap K V} {key : K} {value : V} (valid : Valid discipline map)
    (bounded : SpecMap.size map + 1 < 2 ^ 64) :
    SpecMap.size (update discipline map key value) < 2 ^ 64 := by
  rw [size_update valid]; split <;> omega

theorem size_remove_lt {K V : Type} [DecidableEq K] {discipline : Discipline K}
    {map : SpecMap K V} {key : K} (bounded : SpecMap.size map < 2 ^ 64) :
    SpecMap.size (remove discipline map key) < 2 ^ 64 := by
  rw [size_remove]; split <;> omega

/-- The keys of a table's map read from integer carriers fit the carriers'
range. -/
theorem keys_ofEntries_fit {V V' : Type} {width : IntWidth} {signed : Bool} (value : V → V')
    (table : SpecEntries (SpecInt width signed × V)) :
    SpecMap.KeysSatisfy (IntegerValueFits width signed)
      (SpecMap.ofEntries SpecInt.val value table) := by
  intro pair member
  obtain ⟨native, _, rfl⟩ := List.mem_map.mp member
  exact native.1.fits

/-- The values of a table's map read from integer carriers fit the
carriers' range. -/
theorem values_ofEntries_fit {K K' : Type} {width : IntWidth} {signed : Bool} (key : K → K')
    (table : SpecEntries (K × SpecInt width signed)) :
    SpecMap.ValuesSatisfy (IntegerValueFits width signed)
      (SpecMap.ofEntries key SpecInt.val table) := by
  intro pair member
  obtain ⟨native, _, rfl⟩ := List.mem_map.mp member
  exact native.2.fits

/-- Zero fits every width that has values. -/
theorem IntegerValueFits.zero_of {width : IntWidth} {signed : Bool} (value : SpecInt width signed) :
    IntegerValueFits width signed 0 := by
  have fits := value.fits
  cases width with
  | unbounded => rfl
  | pointer => simp [IntegerValueFits, Ty.integerValueFits?] at fits
  | bits width =>
      cases signed <;>
        simp only [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at fits ⊢ <;>
        split at fits <;> simp_all <;> omega

/-- A key of a table's map read from integer carriers, cut back to the
carriers' width, is the key: the keys fit, as the default does. -/
@[vir_denote_norm] theorem val_ofInt_keyAt_ofEntries {V V' : Type} {width : IntWidth}
    {signed : Bool} (default : SpecInt width signed) (value : V → V')
    (table : SpecEntries (SpecInt width signed × V)) (index : Int) :
    (SpecInt.ofInt default (SpecMap.keyAt (SpecMap.ofEntries SpecInt.val value table) index)).val =
      SpecMap.keyAt (SpecMap.ofEntries SpecInt.val value table) index :=
  SpecInt.val_ofInt default (SpecMap.keyAt_satisfies_all (keys_ofEntries_fit value table)
    (IntegerValueFits.zero_of default) index)

/-- A value of a table's map read from integer carriers, cut back to the
carriers' width, is the value. -/
@[vir_denote_norm] theorem val_ofInt_valueAt_ofEntries {K K' : Type} [DecidableEq K']
    {width : IntWidth} {signed : Bool} (default : SpecInt width signed) (key : K → K')
    (table : SpecEntries (K × SpecInt width signed)) (at_ : K') :
    (SpecInt.ofInt default (SpecMap.valueAt (SpecMap.ofEntries key SpecInt.val table) at_)).val =
      SpecMap.valueAt (SpecMap.ofEntries key SpecInt.val table) at_ :=
  SpecInt.val_ofInt default (SpecMap.valueAt_satisfies (values_ofEntries_fit key table)
    (IntegerValueFits.zero_of default) at_)

/-- Validity is decidable: distinct keys, or ascending ones; a closed map's
validity is then evaluated. -/
instance decidableValid {K V : Type} [DecidableEq K] (discipline : Discipline K)
    (map : SpecMap K V) : Decidable (Valid discipline map) := by
  cases discipline with
  | sequence =>
      unfold Valid Distinct
      exact inferInstance
  | ordered order =>
      unfold Valid Ascending
      exact inferInstance

end VerexIR.SpecMap

namespace VerexIR.Proofs.Denote.NativeMaps

open Lean Meta in
/-- Read a named field's certification at the field's own family boundary.
The owner's dependent certificate can prevent conversion of the whole selector
at an instantiated family. Its `ofDeepHolds` result depends only on the field,
so existing observations can use the canonical field operation without
expanding the owner's certificate or changing the stored selector. -/
def certificationView (value : Lean.Expr) : MetaM Lean.Expr := withDefault do
  unless value.isAppOfArity ``Var.selectCertified 7 do return value
  let some certified ← unfoldDefinition? value | return value
  return (← FamilyTransport.respellOperation? certified).getD certified

open Lean Meta in
/-- The inhabitants supplied by a named verification family keep that
family's projection spelling. A local runtime inhabitant can be convertible
to this projection, but retaining both spellings would produce two typed
readers of the same plain value. Other families retain their supplied
inhabitants; a replacement is accepted only by checked conversion. -/
def canonicalInhabitants (carriers supplied : Lean.Expr) : MetaM Lean.Expr := do
  let carriers := carriers.consumeMData
  unless carriers.isAppOfArity ``Skolems.toCarriers 3 do return supplied
  let canonical := mkAppN (mkConst ``Skolems.carriersNonempty) carriers.getAppArgs
  unless ← withNewMCtxDepth (withDefault (isDefEq supplied canonical)) do return supplied
  return canonical

/-! ## Typed reads

The keys and values of a table, as a typed map holds them: integers by
their values, other types as they are, a type parameter's as its family
reads them (`NTy.read`). -/

section NativeViews
variable [Carriers]

-- The typed map's removal lemmas: a removed key is absent, the others keep
-- their values, the size drops by the key's presence, the positions after
-- the key's rank shift, and the map stays valid.
attribute [vir_denote_norm, grind =] VerexIR.SpecMap.hasKey_remove VerexIR.SpecMap.valueAt_remove
  VerexIR.SpecMap.size_remove VerexIR.SpecMap.keyAt_remove_ordered
attribute [vir_denote_norm, grind .] VerexIR.SpecMap.valid_remove

/-! The typed reads at each type: integers by their values, and a parameter
of an instantiated family at its argument. -/

@[simp, vir_denote_norm] theorem NTy.read_int (width : Nat) (signed : Bool) :
    NTy.read (.int width signed) = @SpecInt.val (.bits width) signed := rfl
@[simp, vir_denote_norm] theorem NTy.read_num : NTy.read .num = @SpecInt.val .unbounded true := rfl
@[simp, vir_denote_norm] theorem NTy.read_unit :
    NTy.read (.unit) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_bool :
    NTy.read (.bool) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_address :
    NTy.read (.address) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_signer :
    NTy.read (.signer) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_string :
    NTy.read (.string) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_bytes :
    NTy.read (.bytes) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_tuple (elements : NRow) :
    NTy.read (.tuple elements) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_struct (source : StructHandle) (arguments fields : NRow) :
    NTy.read (.struct source arguments fields) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_enum (source : StructHandle) (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup) :
    NTy.read (.enum source arguments names rows distinct) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_vector (element : NTy) :
    NTy.read (.vector element) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_table (key value : NTy) :
    NTy.read (.table key value) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_ghost (slot : NTy) :
    NTy.read (.ghost slot) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_ref (referent : NTy) :
    NTy.read (.ref referent) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_function (parameters : NRow) (shared : Sharing) (results : NRow) :
    NTy.read (.function parameters shared results) = fun value => value := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NTy.read_param_instantiate (θ : TypeArgs) (Θ : Carriers)
    [@CarriersNonempty Θ] (index : Nat) :
    @NTy.read (Carriers.instantiate θ Θ) (.param index) =
      @NTy.read Θ ((NTy.param index).subst θ.1) := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NTy.unread_param_instantiate (θ : TypeArgs) (Θ : Carriers)
    [Ξ : @CarriersNonempty Θ] (index : Nat) (inhabitable : (NTy.param index).inhabitable = true) :
    @NTy.unread (Carriers.instantiate θ Θ) (CarriersNonempty.instantiate θ Θ) (.param index)
        inhabitable =
      @NTy.unread Θ Ξ ((NTy.param index).subst θ.1)
        (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl) := rfl
/-! The native equality of tables of scalar keys and values: the equality of
their typed maps. -/

/-- At a scalar type, the native equality is the equality of the values read. -/
theorem NTy.equal_scalar (handles : Bool) : (τ : NTy) → τ.isScalar = true →
    (left right : τ.carrier) → (NTy.equal handles τ left right = true ↔ NTy.read τ left = NTy.read τ right)
  | .unit, _, _, _ => by simp [NTy.equal, NTy.read]
  | .bool, _, _, _ => by simp [NTy.equal, NTy.read]
  | .int _ _, _, _, _ => by simp [NTy.equal, NTy.read]
  | .num, _, _, _ => by simp [NTy.equal, NTy.read]
  | .address, _, _, _ => by simp [NTy.equal, NTy.read]
  | .signer, _, _, _ => by simp [NTy.equal, NTy.read]
  | .string, _, _, _ => by simp [NTy.equal, NTy.read]
  | .bytes, _, _, _ => by simp [NTy.equal, NTy.read]
  | .tuple _, scalar, _, _ | .struct _ _ _, scalar, _, _ | .enum _ _ _ _ _, scalar, _, _
  | .vector _, scalar, _, _ | .table _ _, scalar, _, _ | .ghost _, scalar, _, _
  | .ref _, scalar, _, _ | .param _, scalar, _, _ | .function _ _ _, scalar, _, _ => by
      simp [NTy.isScalar] at scalar

omit [Carriers] in
/-- Lists related element by element as the equality of their images. -/
theorem listEqual_iff_map {α β : Type} (equal : α → α → Bool) (image : α → β)
    (same : ∀ a b, equal a b = true ↔ image a = image b) :
    (left right : List α) → (listEqual equal left right = true ↔ left.map image = right.map image)
  | [], [] => by simp [listEqual]
  | [], _ :: _ => by simp [listEqual]
  | _ :: _, [] => by simp [listEqual]
  | a :: as, b :: bs => by
      simp only [listEqual, Bool.and_eq_true, same, listEqual_iff_map equal image same as bs,
        List.map_cons, List.cons.injEq]

@[simp, vir_denote_norm] theorem NTy.equal_table_scalar (handles : Bool) (key value : NTy)
    (keyScalar : key.isScalar = true) (valueScalar : value.isScalar = true)
    (left right : (NTy.table key value).carrier) :
    NTy.equal handles (.table key value) left right = true ↔
      SpecMap.ofEntries (NTy.read key) (NTy.read value) left =
        SpecMap.ofEntries (NTy.read key) (NTy.read value) right := by
  rw [NTy.equal_table]
  apply listEqual_iff_map
  intro a b
  simp only [Bool.and_eq_true, NTy.equal_scalar handles key keyScalar,
    NTy.equal_scalar handles value valueScalar, Prod.mk.injEq]

attribute [vir_denote_norm] NTy.isScalar

/-- A typed read is injective: an integer is its value, a type parameter's
value is read back by its family, any other value is read as itself. -/
theorem NTy.read_injective : (τ : NTy) → Function.Injective (NTy.read τ)
  | .int _ _ => fun _ _ same => SpecInt.ext same
  | .num => fun _ _ same => SpecInt.ext same
  | .param index => fun a b same => by
      have := congrArg (Carriers.reads index).unread same
      simp only [NTy.read] at this
      rwa [(Carriers.reads index).unread_read, (Carriers.reads index).unread_read] at this
  | .unit | .bool | .address | .signer | .string | .bytes | .tuple _ | .struct _ _ _
  | .enum _ _ _ _ _ | .vector _ | .table _ _ | .ghost _ | .ref _ | .function _ _ _ =>
      fun _ _ same => same

/-- Tables are equal as their typed maps are. -/
theorem NTy.table_eq_iff (key value : NTy) (left right : (NTy.table key value).carrier) :
    left = right ↔
      SpecMap.ofEntries (NTy.read key) (NTy.read value) left =
        SpecMap.ofEntries (NTy.read key) (NTy.read value) right := by
  constructor
  · intro same; rw [same]
  · intro same
    apply SpecEntries.ext
    have injective : Function.Injective
        (fun entry : key.carrier × value.carrier => (NTy.read key entry.1, NTy.read value entry.2)) :=
      fun a b eq => Prod.ext (NTy.read_injective key (Prod.mk.inj eq).1)
        (NTy.read_injective value (Prod.mk.inj eq).2)
    exact (List.map_inj_right fun a b eq => injective eq).mp same

end NativeViews

/-! ## Family transports

A caller's table, carried to a generic callee's family, holds the same
entries: the typed map of a table of type parameters is that of the
caller's table. -/

/-- The typed map of a caller's table carried to a callee's family of type
parameters is that of the caller's table. -/
theorem ofEntries_toSkolem_params [Θ : Carriers] [@CarriersNonempty Θ] {K V : Type}
    (θ : TypeArgs) (i j : Nat)
    (key : @NTy.carrier (Carriers.instantiate θ Θ) (.param i) → K)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.param j) → V)
    (table : ((NTy.table (.param i) (.param j)).subst θ.1).carrier) :
    SpecMap.ofEntries key value (NTy.toSkolem θ (.table (.param i) (.param j)) table) =
      SpecMap.ofEntries key value table := by
  simp only [SpecMap.ofEntries]
  congr 1
  show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
  exact List.map_id'' (fun _ => rfl) _

/-- A table of type parameters carried between a caller and a callee's
family is the same table. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.ofSkolem_table_params
    [Θ : Carriers] [@CarriersNonempty Θ] (θ : TypeArgs) (i j : Nat)
    (table : @NTy.carrier (Carriers.instantiate θ Θ) (.table (.param i) (.param j))) :
    NTy.ofSkolem θ (.table (.param i) (.param j)) table = table :=
  SpecEntries.ext (by
    show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
    exact List.map_id'' (fun _ => rfl) _)

@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.toSkolem_table_params
    [Θ : Carriers] [@CarriersNonempty Θ] (θ : TypeArgs) (i j : Nat)
    (table : ((NTy.table (.param i) (.param j)).subst θ.1).carrier) :
    NTy.toSkolem θ (.table (.param i) (.param j)) table = table :=
  SpecEntries.ext (by
    show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
    exact List.map_id'' (fun _ => rfl) _)

theorem ofEntries_ofSkolem_params [Θ : Carriers] [@CarriersNonempty Θ] {K V : Type}
    (θ : TypeArgs) (i j : Nat)
    (key : ((NTy.param i).subst θ.1).carrier → K)
    (value : ((NTy.param j).subst θ.1).carrier → V)
    (table : @NTy.carrier (Carriers.instantiate θ Θ) (.table (.param i) (.param j))) :
    SpecMap.ofEntries key value (NTy.ofSkolem θ (.table (.param i) (.param j)) table) =
      SpecMap.ofEntries key value table := by
  simp only [SpecMap.ofEntries]
  congr 1
  show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
  exact List.map_id'' (fun _ => rfl) _

open Lean Meta in
/-- A transported table of type parameters, its family, the family's
inhabitants (`CarriersNonempty`, which the transports take), arguments,
indices, and the caller's table. -/
private def transportedParams? (table : Lean.Expr) :
    Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) := do
  guard (table.isAppOfArity ``NTy.toSkolem 5 || table.isAppOfArity ``NTy.ofSkolem 5)
  let type := table.getArg! 3
  guard (type.isAppOfArity ``NTy.table 2)
  let key := type.getArg! 0
  let value := type.getArg! 1
  guard (key.isAppOfArity ``NTy.param 1 && value.isAppOfArity ``NTy.param 1)
  return (table.getArg! 0, table.getArg! 1, table.getArg! 2, key.getArg! 0, value.getArg! 0,
    table.getArg! 4)

open Lean Meta in
/-- The typed map of a caller's table carried to a generic callee is the
caller's (`ofEntries_toSkolem_params`). -/
simproc ↓ [vir_denote, vir_denote_norm] ofEntriesTransport (SpecMap.ofEntries _ _ _) := fun e => do
  let_expr SpecMap.ofEntries _ _ keyType valueType key value table := e | return .continue
  let some (family, nonempty, θ, i, j, caller) := transportedParams? table | return .continue
  let result := mkAppN e.getAppFn (e.getAppArgs.set! 6 caller)
  let identity := if table.isAppOf ``NTy.toSkolem then ``ofEntries_toSkolem_params
    else ``ofEntries_ofSkolem_params
  let proof := mkAppN (mkConst identity)
    #[family, nonempty, keyType, valueType, θ, i, j, key, value, caller]
  return .visit { expr := result, proof? := some (← mkExpectedTypeHint proof (← mkEq e result)) }

open Lean Meta in
/-- The carrier types of a typed map's table, evaluated: a callee's contract
spells them at the family its call induces, the caller's statements at its
own, and the two spellings are one type. -/
dsimproc ↓ [vir_denote, vir_denote_norm] canonicalTableCarriers (SpecMap.ofEntries _ _ _) := fun e => do
  let_expr SpecMap.ofEntries keyType valueType _ _ _ _ _ := e | return .continue
  let evaluate (type : Lean.Expr) : MetaM Lean.Expr := do
    if type.isAppOf ``VerexIR.SpecInt then return type
    let reduced ← withTransparency .default (whnf type)
    if reduced.isAppOf ``VerexIR.SpecInt then return reduced else return type
  let key ← evaluate keyType
  let value ← evaluate valueType
  if key == keyType && value == valueType then return .continue
  let args := (e.getAppArgs.set! 0 key).set! 1 value
  return .done (mkAppN e.getAppFn args)

/-! ## Typed map lemmas

The lemmas of the typed map, and the entries of a table built from its
list. -/

attribute [simp, vir_denote_norm] SpecMap.hasKey_update SpecMap.valueAt_update
  SpecMap.size_update SpecMap.size_remove SpecMap.hasKey_remove SpecMap.valueAt_remove

-- Setting a key the map holds keeps the positions and the ranks.
attribute [simp, vir_denote_norm] SpecMap.keyAt_update_present SpecMap.rank_update_present

-- The validity of a construction rewrites where its input's validity is
-- discharged within the depth a chain of constructions sets; past that
-- depth it is a fact of the leaf (`typedMapPositionFacts`).
attribute [simp, vir_denote_norm] SpecMap.valid_update SpecMap.valid_remove
  SpecMap.keyAt_remove_ordered

-- The bulk operations' lemmas: sizes and keys of several entries set, of a
-- prefix or a suffix, and of a replaced key, as the roles' abort
-- conditions permit them.
attribute [simp, vir_denote_norm] SpecMap.valid_updateAll SpecMap.hasKey_updateAll_add_all
  SpecMap.hasKey_updateAll_upsert_all SpecMap.hasKey_updateAll_new_from
  SpecMap.hasKey_updateAll_append SpecMap.size_updateAll_add_all SpecMap.size_updateAll_new_from
  SpecMap.size_updateAll_append_disjoint SpecMap.valid_takeEntries SpecMap.valid_dropEntries
  SpecMap.size_takeEntries SpecMap.size_dropEntries SpecMap.size_replaceKey
  SpecMap.hasKey_replaceKey

attribute [simp, vir_denote_norm] SpecMap.length_keys SpecMap.length_values
attribute [vir_denote_norm] SpecMap.getElem_keys_toNat

open Lean Meta in
/-- An abort condition over literal lists, decided. -/
private def decideLiteralAborts (e : Lean.Expr) (lists : List Lean.Expr) : SimpM Simp.Step := do
  unless lists.all (·.listLit?.isSome) do return .continue
  -- Without a decision procedure at the lists' type, the condition stays.
  let some decided ← (do
      try pure (some (← withDefault (whnf (← mkDecide e)))) catch _ => pure none)
    | return .continue
  if decided.isConstOf ``Bool.true then
    return .visit { expr := mkConst ``True, proof? := some (← mkEqTrue (← mkDecideProof e)) }
  if decided.isConstOf ``Bool.false then
    return .visit { expr := mkConst ``False,
                    proof? := some (← mkEqFalse (← mkDecideProof (mkNot e))) }
  return .continue

open Lean Meta in
/-- Building a map from literal lists aborts or not, decided. -/
simproc [vir_denote, vir_denote_norm] literalAbortsNewFrom (SpecMap.AbortsNewFrom _ _) := fun e => do
  let_expr SpecMap.AbortsNewFrom _ _ keys values := e | return .continue
  decideLiteralAborts e [keys, values]

open Lean Meta in
/-- Updating from literal lists aborts or not, decided. -/
simproc [vir_denote, vir_denote_norm] literalAbortsUpsertAll (SpecMap.AbortsUpsertAll _ _) :=
    fun e => do
  let_expr SpecMap.AbortsUpsertAll _ _ keys values := e | return .continue
  decideLiteralAborts e [keys, values]

open Lean Meta in
/-- A closed value evaluated to its literal: an integer, a natural, a
Boolean, an ordering, a string, or an option, list, pair or structure of
such, its proof fields kept; `none` where a value has no literal. -/
partial def groundLiteral? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let type ← whnfD (← inferType e)
  let e ← whnfD e
  if type.isConstOf ``Int then
    match_expr e with
    | Int.ofNat n => return (← getNatValue? (← whnfD n)).map fun n => toExpr (n : Int)
    | Int.negSucc n => return (← getNatValue? (← whnfD n)).map fun n => toExpr (-(n : Int) - 1)
    | _ => return (← getIntValue? e).map toExpr
  if type.isConstOf ``Nat then return (← getNatValue? e).map toExpr
  if type.isConstOf ``Bool || type.isConstOf ``Ordering then
    return if e.isConst then some e else none
  if type.isConstOf ``String then return if e.isStringLit then some e else none
  let .const constructor _ := e.getAppFn | return none
  let some (.ctorInfo info) := (← getEnv).find? constructor | return none
  let mut args := e.getAppArgs
  unless args.size == info.numParams + info.numFields do return none
  for i in [info.numParams : args.size] do
    if ← isProof args[i]! then continue
    let some value ← groundLiteral? args[i]! | return none
    args := args.set! i value
  return some (mkAppN e.getAppFn args)

open Lean Meta in
/-- A closed typed map term evaluates to its literal: a map built from
literal lists, and its observations. -/
def evalGroundMap (e : Lean.Expr) : SimpM Simp.DStep := do
  if e.hasFVar || e.hasMVar || e.hasLooseBVars then return .continue
  -- Keys of a native type decide their equality by the family's instance,
  -- whose unfolding is not evaluation; their reads go by the lemmas.
  let some keyType := e.getAppArgs[0]? | return .continue
  unless [``Int, ``Nat, ``Bool, ``String].any keyType.isConstOf do return .continue
  let some value ← (try groundLiteral? e catch _ => pure none) | return .continue
  if value == e then return .continue
  return .done value

dsimproc [vir_denote_norm, vir_denote_eval] evalGroundUpdateAll (SpecMap.updateAll _ _ _ _) :=
  evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundUpdate (SpecMap.update _ _ _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundRemove (SpecMap.remove _ _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundKeyAt (SpecMap.keyAt _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundValueAt (SpecMap.valueAt _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundSize (SpecMap.size _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundHasKey (SpecMap.hasKey _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundRank (SpecMap.rank _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundNextKey (SpecMap.nextKey? _ _ _) := evalGroundMap
dsimproc [vir_denote_norm, vir_denote_eval] evalGroundPrevKey (SpecMap.prevKey? _ _ _) := evalGroundMap

/-- A literal compared with a conditional between two literals decides the
condition: a variant's name against the name the specification computes. -/
@[vir_denote_norm] theorem literal_eq_ite_left {α : Type} (c : Prop) [Decidable c] {a b : α}
    (ne : a ≠ b) : (a = if c then a else b) ↔ c := by
  split <;> simp_all

@[vir_denote_norm] theorem literal_eq_ite_right {α : Type} (c : Prop) [Decidable c] {a b : α}
    (ne : a ≠ b) : (b = if c then a else b) ↔ ¬c := by
  split <;> simp_all [eq_comm]

attribute [vir_denote_norm] String.reduceNe

/-! Reads of a literal map at a symbolic key, entry by entry, and equality
of literal typed values, component by component. -/

@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.hasKey_cons {K V : Type} [DecidableEq K]
    (entry : K × V) (rest : SpecMap K V) (key : K) :
    SpecMap.hasKey (entry :: rest) key = (decide (entry.1 = key) || SpecMap.hasKey rest key) := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.valueAt_nil {K V : Type} [DecidableEq K]
    [Inhabited V] (key : K) : SpecMap.valueAt ([] : SpecMap K V) key = default := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.valueAt_cons {K V : Type} [DecidableEq K]
    [Inhabited V] (entry : K × V) (rest : SpecMap K V) (key : K) :
    SpecMap.valueAt (entry :: rest) key =
      if entry.1 = key then entry.2 else SpecMap.valueAt rest key := rfl

@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.size_cons {K V : Type} (entry : K × V)
    (rest : SpecMap K V) : SpecMap.size (entry :: rest) = SpecMap.size rest + 1 := by
  simp [SpecMap.size]

/-- Equal array literals have equal lists. -/
@[vir_denote_norm] theorem toArray_eq_iff {α : Type} (a b : List α) :
    (a.toArray = b.toArray) = (a = b) :=
  propext ⟨List.toArray_inj, congrArg _⟩

/-- An array built from its list is the list's array. -/
@[vir_denote_norm] theorem array_mk_eq_toArray {α : Type} (l : List α) : Array.mk l = l.toArray := by
  simp

attribute [vir_denote_norm] Prod.mk.injEq SpecVector.mk.injEq SpecInt.mk.injEq Array.mk.injEq
  List.cons.injEq HList.cons HList.nil

/-- The size of a sequence map after an update, unconditionally. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.size_update_sequence {K V : Type}
    [DecidableEq K] (map : SpecMap K V) (key : K) (value : V) :
    SpecMap.size (SpecMap.update .sequence map key value) =
      if SpecMap.hasKey map key then SpecMap.size map else SpecMap.size map + 1 := by
  simp only [SpecMap.size, SpecMap.update, SpecMap.Discipline.set, SpecMap.length_seqSet,
    SpecMap.hasKey]
  split <;> simp

-- Tests of variant names and other Boolean equalities evaluate.
attribute [vir_denote_norm] beq_self_eq_true beq_iff_eq beq_eq_false_iff_ne

-- The decisions and literal comparisons the lemmas leave behind evaluate.
attribute [vir_denote_norm] Int.reduceEq Int.reduceLT Int.reduceLE Nat.reduceEqDiff
  decide_eq_true_eq decide_eq_false_iff_not decide_true decide_false Bool.or_false Bool.false_or
  Bool.or_true Bool.true_or Bool.and_true Bool.true_and Bool.and_false Bool.false_and
  Bool.false_eq_true Bool.true_eq_false Bool.not_true Bool.not_false ite_true ite_false
  eq_self_iff_true false_or or_false true_or or_true true_and and_true false_and and_false
  not_false_eq_true not_true_eq_false false_implies true_implies

@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.hasKey_nil {K V : Type} [DecidableEq K]
    (key : K) : SpecMap.hasKey ([] : SpecMap K V) key = false := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.size_nil {K V : Type} :
    SpecMap.size ([] : SpecMap K V) = 0 := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.valid_nil {K V : Type}
    (discipline : SpecMap.Discipline K) : SpecMap.Valid discipline ([] : SpecMap K V) := by
  cases discipline <;> simp [SpecMap.Valid, SpecMap.Distinct, SpecMap.Ascending]
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.ofEntries_mk_nil {K V K' V' : Type}
    (key : K → K') (value : V → V') (bounded : ([] : List (K × V)).length < 2 ^ 64) :
    SpecMap.ofEntries key value ⟨[], bounded⟩ = [] := rfl

@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.ofEntries_mk_cons {K V K' V' : Type}
    (key : K → K') (value : V → V') (entry : K × V) (rest : List (K × V))
    (bounded : (entry :: rest).length < 2 ^ 64) :
    SpecMap.ofEntries key value ⟨entry :: rest, bounded⟩ =
      (key entry.1, value entry.2) ::
        SpecMap.ofEntries key value ⟨rest, by simp at bounded; omega⟩ := rfl

/-- The typed map of the plain entries of a certified table, read as they
are, is the typed map of its certified entries read through `NTy.forget`:
one table read at its two layers gives one map under the map that forgets a
value's certificates.  This is what ties a clause about a table inside a
value of its type to a clause about the same table inside a mutable
reference's current, whose entries are plain while the loan lives
(`doc/designs/data-invariants.md`).  In the normal form, so that a
clause reading a table through a reference's current reaches the same map as
one reading it through a value of its type; `NTy.forget_table` is therefore
kept out of the normal form, since rewriting this equation's left side first
would leave the two readings in two shapes. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.SpecMap.ofEntries_forget_table [Carriers]
    (key value : NTy) (entries : (NTy.table key value).carrier) :
    SpecMap.ofEntries (fun k => k) (fun v => v) (NTy.forget (.table key value) entries) =
      SpecMap.ofEntries (NTy.forget key) (NTy.forget value) entries := by
  have forgotten : (NTy.forget (.table key value) entries).entries =
      entries.entries.map fun entry => (NTy.forget key entry.1, NTy.forget value entry.2) := rfl
  simp [SpecMap.ofEntries, forgotten]

/-! The plain reading at a type that carries no certificate, which is the
certified reading at that type (`NTy.read_int`, `NTy.read_unit`, …): one
term and one normal form at either layer, so that a clause reading a type
parameter's plain values reaches the argument's reading once the transport
has named the argument (`NTy.readPlain_instantiate`). -/

@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_int [Θ : Carriers]
    [CarriersNonempty Θ] (width : Nat) (signed : Bool) (held) :
    NTy.readPlain (.int width signed) held = @SpecInt.val (.bits width) signed := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_num [Θ : Carriers]
    [CarriersNonempty Θ] (held) :
    NTy.readPlain .num held = @SpecInt.val .unbounded true := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_unit [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .unit held = fun value => value := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_bool [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .bool held = fun value => value := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_address [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .address held = fun value => value := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_signer [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .signer held = fun value => value := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_string [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .string held = fun value => value := rfl
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_bytes [Θ : Carriers]
    [CarriersNonempty Θ] (held) : NTy.readPlain .bytes held = fun value => value := rfl

/-- A certified value's plain side reads as the value does: one value, one
typed value, at either layer.  Forgetting keeps the encoding
(`NTy.plainEncode_forget`) and a typed value is determined by it
(`NTy.readWrite_read`, `NTy.readBack_readWrite`). -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_forget [Θ : Carriers]
    [CarriersNonempty Θ] (τ : NTy) (held : τ.inhabitable = true) (value : τ.carrier) :
    NTy.readPlain τ held (NTy.forget τ value) = NTy.read τ value := by
  rw [NTy.readPlain_eq_readBack, NTy.plainEncode_forget, ← NTy.readWrite_read,
    NTy.readBack_readWrite]

/-- The typed map of a table certified by its owner's certificate is the
typed map of the plain entries it certifies, where the keys' and values'
readings agree on a certified value and on the plain one it certifies: a
certified struct carries one certificate for its whole plain row
(`NTy.carrier`), so a table field read out of one is the plain field
certified by the owner (`NTy.ofDeepHolds`), and a clause about that table
is a clause about the plain field.  The twin of
`SpecMap.ofEntries_forget_table`, where the certified value is the primary
one; the readings are the premises because a view spells them at the type
it reads (`typedView?`), not through `NTy.read`. -/
theorem _root_.VerexIR.SpecMap.ofEntries_ofDeepHolds_table [Θ : Carriers] {K V : Type}
    (key value : NTy) (readKey : @NTy.plain Θ key → K) (readValue : @NTy.plain Θ value → V)
    (certifiedKey : @NTy.carrier Θ key → K) (certifiedValue : @NTy.carrier Θ value → V)
    (keyAgrees : ∀ k holds, certifiedKey (NTy.ofDeepHolds key k holds) = readKey k)
    (valueAgrees : ∀ v holds, certifiedValue (NTy.ofDeepHolds value v holds) = readValue v)
    (entries : @NTy.plain Θ (.table key value))
    (holds : NTy.deepHolds (.table key value) entries) :
    SpecMap.ofEntries certifiedKey certifiedValue
        (NTy.ofDeepHolds (.table key value) entries holds) =
      SpecMap.ofEntries readKey readValue entries := by
  show List.map _ (List.map _ entries.entries.attach) = List.map _ entries.entries
  rw [List.map_map]
  conv => rhs; rw [← List.attach_map_val]
  refine List.map_congr_left fun entry _ => ?_
  simp only [Function.comp_apply, keyAgrees, valueAgrees]

/-- A plain table satisfying its certificates is determined by its typed
reading: certify its entries, recover the certified table through the read's
inverse, and forget once. The hypothesis is essential because a plain
reader's total decoder can identify invalid values. -/
theorem _root_.VerexIR.SpecMap.eq_forget_writeBack_of_ofEntries_readPlain
    [Θ : Carriers] [CarriersNonempty Θ] (key value : NTy)
    (keyHeld : key.inhabitable = true) (valueHeld : value.inhabitable = true)
    (entries : (NTy.table key value).plain)
    (holds : NTy.deepHolds (.table key value) entries)
    (map : SpecMap key.readCarrier value.readCarrier)
    (defined : SpecMap.ofEntries (NTy.readPlain key keyHeld)
      (NTy.readPlain value valueHeld) entries = map) :
    entries = NTy.forget (.table key value)
      (SpecMap.writeBack (fun entry =>
        (NTy.unread key keyHeld entry.1, NTy.unread value valueHeld entry.2)) map) := by
  have agreement (τ : NTy) (held : τ.inhabitable = true)
      (plain : τ.plain) (valid : NTy.deepHolds τ plain) :
      NTy.read τ (NTy.ofDeepHolds τ plain valid) = NTy.readPlain τ held plain := by
    rw [← NTy.readPlain_forget τ held, NTy.forget_ofDeepHolds]
  have certified : SpecMap.ofEntries (NTy.read key) (NTy.read value)
      (NTy.ofDeepHolds (.table key value) entries holds) = map := by
    rw [SpecMap.ofEntries_ofDeepHolds_table key value
      (NTy.readPlain key keyHeld) (NTy.readPlain value valueHeld)
      (NTy.read key) (NTy.read value) (agreement key keyHeld) (agreement value valueHeld)]
    exact defined
  have recovered := SpecMap.eq_writeBack_of_ofEntries
    (NTy.unread_read key keyHeld) (NTy.unread_read value valueHeld) certified
  exact (NTy.forget_ofDeepHolds (.table key value) entries holds).symm.trans
    (congrArg (NTy.forget (.table key value)) recovered)

open Lean Meta in
/-- A table certified by its owner's certificate, read at the plain entries
it certifies (`SpecMap.ofEntries_ofDeepHolds_table`): the readings a view
spells read no certificate where the two layers agree, and that agreement
is reflexivity. A certificate-bearing key or value uses its total plain
reader, related to its certified reading by the table's actual certificate
and `NTy.readPlain_forget`. -/
simproc [vir_denote_norm] ofEntriesOfDeepHoldsTable
    (VerexIR.SpecMap.ofEntries _ _ _) := fun e => do
  let_expr VerexIR.SpecMap.ofEntries _ _ _ _ certifiedKey certifiedValue certified := e
    | return .continue
  let certified ← certificationView certified
  let_expr NTy.ofDeepHolds carriers τ entries holds := certified | return .continue
  let_expr NTy.table key value := ← whnfR τ | return .continue
  -- The lemma's premises are readings of the plain layer, so a reading
  -- whose own layer is not the table's plain one is not one of them, and the
  -- map is left as it is.  The two layers of a type that carries no
  -- certificate are one type by unfolding the family at the key's and the
  -- value's types, which is a definitional fact of the family and not a
  -- reducible one, so the layers are decided — and the application below
  -- built — at the transparency that fact needs.
  let standsAtPlain (reader τ : Lean.Expr) : MetaM Bool :=
    withTransparency .default do
      let .forallE _ domain _ _ := ← whnfR (← inferType reader) | return false
      withNewMCtxDepth <| isDefEq domain (mkApp2 (mkConst ``NTy.plainOf)
        (mkApp (mkConst ``Carriers.toPlain) carriers) τ)
  -- Each reading reads the plain values of its type as well: the two
  -- layers are one type where the type carries no certificate, and the
  -- reading is then the plain one by reflexivity.  The plain layer is
  -- named in its canonical spelling (`PlainSpelling`).
  let reflexive (reader : Lean.Expr) (τ : Lean.Expr) : MetaM (Option Lean.Expr) :=
    withTransparency .default <|
    withLocalDeclD `value (mkApp2 (mkConst ``NTy.plainOf)
        (mkApp (mkConst ``Carriers.toPlain) carriers) τ) fun value =>
      withLocalDeclD `holds (mkApp3 (mkConst ``NTy.deepHolds) carriers τ value) fun held => do
        let certified := mkApp4 (mkConst ``NTy.ofDeepHolds) carriers τ value held
        let wanted ← mkEq (mkApp reader certified) (mkApp reader value)
        let proof ← (try
            let reflexivity ← mkEqRefl (mkApp reader value)
            unless ← withNewMCtxDepth (isDefEq (← inferType reflexivity) wanted) do failure
            pure (some (← mkExpectedTypeHint reflexivity wanted))
          catch _ => pure none)
        let some proof := proof | return none
        return some (← mkLambdaFVars #[value, held] proof)
  let agreement (reader τ : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
    if ← standsAtPlain reader τ then
      if let some proof ← reflexive reader τ then return some (reader, proof)
    observing? <| withDefault do
      let nonempty ← canonicalInhabitants carriers
        (← synthInstance (mkApp (mkConst ``CarriersNonempty) carriers))
      let inhabitable := mkApp (mkConst ``NTy.inhabitable) τ
      unless ← isDefEq inhabitable (mkConst ``Bool.true) do failure
      let inhabited ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable (mkConst ``Bool.true))
      let plainRead := mkAppN (mkConst ``NTy.readPlain) #[carriers, nonempty, τ, inhabited]
      let proof ← withLocalDeclD `value (mkApp2 (mkConst ``NTy.plain) carriers τ) fun value =>
        withLocalDeclD `holds (mkApp3 (mkConst ``NTy.deepHolds) carriers τ value) fun held => do
          let certified := mkApp4 (mkConst ``NTy.ofDeepHolds) carriers τ value held
          let reads := mkAppN (mkConst ``NTy.readPlain_forget)
            #[carriers, nonempty, τ, inhabited, certified]
          let forgets := mkAppN (mkConst ``NTy.forget_ofDeepHolds) #[carriers, τ, value, held]
          let same ← mkEqTrans (← mkEqSymm reads) (← mkCongrArg plainRead forgets)
          let wanted ← mkEq (mkApp reader certified) (mkApp plainRead value)
          unless ← isDefEq (← inferType same) wanted do failure
          mkLambdaFVars #[value, held] (← mkExpectedTypeHint same wanted)
      return (plainRead, proof)
  let some (readKey, keyAgrees) ← agreement certifiedKey key | return .continue
  let some (readValue, valueAgrees) ← agreement certifiedValue value | return .continue
  -- Building the application decides the layers once more, and a simproc
  -- that raised instead of declining would abort the whole stage that
  -- visits the node.
  let application : MetaM (Option Lean.Expr) := observing? <| withTransparency .default <|
    mkAppOptM ``VerexIR.SpecMap.ofEntries_ofDeepHolds_table
      #[carriers, none, none, key, value, readKey, readValue, certifiedKey,
        certifiedValue, keyAgrees, valueAgrees, entries, holds]
  let some proof ← application | return .continue
  let some (_, _, rhs) := (← whnfR (← inferType proof)).eq? | return .continue
  return .visit { expr := rhs, proof? := some proof }

/-- Reading keys injectively preserves a sequence map's distinctness.
Values do not participate in its validity. -/
theorem _root_.VerexIR.SpecMap.valid_sequence_ofEntries_read {K V K' V' : Type}
    (key : K → K') (value : V → V')
    (injective : ∀ {left right}, key left = key right → left = right)
    (entries : SpecEntries (K × V)) :
    SpecMap.Valid .sequence (SpecMap.ofEntries (fun k => k) (fun v => v) entries) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries key value entries) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries, List.Nodup, List.pairwise_map]
  constructor
  · intro held
    exact held.imp fun different same => different (injective same)
  · intro held
    exact held.imp fun different same => different (congrArg key same)

/-- The validity a map owner carries of its entries as they are, read as the
validity of the map a specification states: validity reads the keys alone,
and a reading of them that decides their order carries it either way.  The
values are read along, since a specification's map reads both. -/
theorem _root_.VerexIR.SpecMap.valid_ofEntries_read {K V K' V' : Type}
    (carrierOrder : SpecMap.KeyOrder K) (order : SpecMap.KeyOrder K')
    (key : K → K') (value : V → V')
    (decides : ∀ left right, carrierOrder.compare left right = order.compare (key left) (key right))
    (entries : SpecEntries (K × V)) :
    SpecMap.Valid (.ordered carrierOrder) (SpecMap.ofEntries (fun k => k) (fun v => v) entries) ↔
      SpecMap.Valid (.ordered order) (SpecMap.ofEntries key value entries) := by
  show SpecMap.Ascending _ _ ↔ SpecMap.Ascending _ _
  simp only [SpecMap.Ascending, SpecMap.ofEntries, List.pairwise_map, decides]

open Lean Meta in
/-- Read an integer-key map's validity at the same view as its operations.
The integer projection preserves distinctness for a sequence and comparison
for an ordered map; both therefore use the canonical integer-valued view. -/
simproc [vir_denote_norm] validOfEntriesRead (VerexIR.SpecMap.Valid _ _) := fun e => do
  let_expr VerexIR.SpecMap.Valid _ _ discipline map := e | return .continue
  let sequence := discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.sequence 1
  let carrierOrder? := (discipline.app2? ``VerexIR.SpecMap.Discipline.ordered).map Prod.snd
  unless sequence || carrierOrder?.isSome do return .continue
  let_expr VerexIR.SpecMap.ofEntries keyType valueType _ _ readKey readValue entries := map
    | return .continue
  -- A declaration carries its entries unchanged. Already-read maps keep
  -- their own view and do not pass through this boundary again.
  let identity (reader type : Lean.Expr) : MetaM Bool :=
    withLocalDeclD `value type fun value =>
      withNewMCtxDepth <| isDefEq (mkApp reader value) value
  unless ← identity readKey keyType do return .continue
  unless ← identity readValue valueType do return .continue
  let keyRepresentation ← whnfR keyType
  let_expr VerexIR.SpecInt width signed := keyRepresentation | return .continue
  let read := mkApp2 (mkConst ``VerexIR.SpecInt.val) width signed
  let valueRead ← do
    let value ← whnfR valueType
    if value.isAppOfArity ``VerexIR.SpecInt 2 then
      pure (mkApp2 (mkConst ``VerexIR.SpecInt.val) (value.getArg! 0) (value.getArg! 1))
    else withLocalDeclD `value valueType fun held => mkLambdaFVars #[held] held
  let application : MetaM (Option Lean.Expr) := observing? <| withTransparency .default <| do
    if sequence then
      return ← mkAppOptM ``VerexIR.SpecMap.valid_sequence_ofEntries_read
        #[keyType, valueType, none, none, read, valueRead,
          mkApp2 (mkConst ``VerexIR.SpecInt.ext) width signed, entries]
    let some carrierOrder := carrierOrder? | failure
    let_expr NTy.keyOrder _ _ τ := carrierOrder | failure
    let τ ← whnfR τ
    unless τ.isAppOfArity ``NTy.int 2 do failure
    let integers := mkConst ``VerexIR.SpecMap.intOrder
    let compared (order left right : Lean.Expr) : MetaM Lean.Expr := do
      return mkApp2 (← mkAppM ``VerexIR.SpecMap.KeyOrder.compare #[order]) left right
    let decides ← withLocalDeclD `left keyType fun left =>
      withLocalDeclD `right keyType fun right => do
        let wanted ← mkEq (← compared carrierOrder left right)
          (← compared integers (mkApp read left) (mkApp read right))
        let reflexivity ← mkEqRefl (← compared integers (mkApp read left) (mkApp read right))
        unless ← withNewMCtxDepth (isDefEq (← inferType reflexivity) wanted) do failure
        let proof ← mkExpectedTypeHint reflexivity wanted
        mkLambdaFVars #[left, right] proof
    mkAppOptM ``VerexIR.SpecMap.valid_ofEntries_read
      #[keyType, valueType, none, none, carrierOrder, integers, read, valueRead, decides, entries]
  let some equivalence ← application | return .continue
  let some (_, rhs) := (← whnfR (← inferType equivalence)).app2? ``Iff | return .continue
  return .visit { expr := rhs, proof? := some (← mkAppM ``propext #[equivalence]) }

/-- A type parameter's plain reading at the family a call's type arguments
induce is the outer family's reading at the argument's type: the induced
family stores the argument's canonical plain reader, so the parameter's
reader is definitionally the argument's reader. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.readPlain_instantiate
    (θ : TypeArgs) (outer : Carriers) [Ξ : @CarriersNonempty outer] (index : Nat)
    (held : (NTy.param index).inhabitable = true)
    (outerHeld : ((NTy.param index).subst θ.1).inhabitable = true) :
    @NTy.readPlain (Carriers.instantiate θ outer) inferInstance (.param index) held =
      @NTy.readPlain outer Ξ ((NTy.param index).subst θ.1) outerHeld := by
  rfl

/-- A type parameter's reading at the family a call's type arguments induce
is the outer family's reading at the argument's type. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.read_instantiate
    (θ : TypeArgs) (outer : Carriers) [@CarriersNonempty outer] (index : Nat)
    (value : @NTy.carrier (Carriers.instantiate θ outer) (.param index)) :
    @NTy.read (Carriers.instantiate θ outer) (.param index) value =
      @NTy.read outer ((NTy.param index).subst θ.1) value := rfl

open Lean Meta in
/-- Reduce a native type's head without unfolding its canonical descriptor. -/
def nativeTypeHead (type : Lean.Expr) : MetaM Lean.Expr := do
  let env ← getEnv
  withDefault <| whnfHeadPred type.consumeMData (fun type => pure (!isNominalType env type))

open Lean Meta in
/-- The family and type a view is decided on: the type's normal form, where a
parameter of an instantiated family is the argument it stands for, in the
outer family, with the proof that the resolved type has values where the
resolution supplies one.  The encoders agree by unfolding.  A frame's type
arguments are inhabitable by their own certificate
(`TypeArgs.inhabitable`), so a parameter resolved through one has values
even where the argument is symbolic and nothing evaluates: without the
proof a view of a declaration's own clause at a symbolic frame could not be
decided at all (`typedView?`). -/
private partial def resolvedType (carriers τ : Lean.Expr) :
    MetaM (Lean.Expr × Lean.Expr × Option Lean.Expr × Option Lean.Expr) := do
  let τ ← nativeTypeHead τ
  let_expr NTy.param index := τ | return (carriers, τ, none, none)
  -- The instantiated family, spelled by carriers or by skolems.
  let instantiation? : Option (Lean.Expr × Lean.Expr × Lean.Expr) := do
    let carriers := carriers.consumeMData
    if carriers.isAppOfArity ``Carriers.instantiate 3 then
      return (carriers.getArg! 0, carriers.getArg! 1, carriers.getArg! 2)
    let .app (.app (.app toCarriers unit) functions) skolems := carriers | none
    guard (toCarriers.isConstOf ``Skolems.toCarriers)
    let .app (.app (.app (.app instantiate _) _) θ) outer := skolems.consumeMData | none
    guard (instantiate.isConstOf ``Skolems.instantiate)
    return (θ, mkApp3 toCarriers unit functions outer,
      mkApp3 (mkConst ``Skolems.carriersNonempty) unit functions outer)
  let some (θ, outer, nonempty) := instantiation? | return (carriers, τ, none, none)
  let parameter := mkApp (mkConst ``NTy.param) index
  -- The row the frame substitutes: `TypeArgs` is an abbreviation of the
  -- subtype of a row by its inhabitability, so the row is that subtype's
  -- value and a projection of `TypeArgs` names nothing.
  let row := mkProj ``Subtype 0 θ
  let resolved := mkApp2 (mkConst ``NTy.subst) row parameter
  -- The argument's inhabitability, from the frame's: a parameter itself has
  -- values, so the row's default does (`NRow.getD_inhabitable`).
  let inhabitable := mkAppN (mkConst ``NRow.getD_inhabitable)
    #[row, mkApp (mkConst ``TypeArgs.inhabitable) θ, index, parameter,
      ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq (mkApp (mkConst ``NTy.inhabitable) parameter) (mkConst ``Bool.true))]
  let (outerCarriers, outerType, outerInhabitable?, outerNonempty?) ← resolvedType outer resolved
  let resolvedNonempty := outerNonempty?.orElse fun _ =>
    if outerCarriers == outer then some nonempty else none
  if let some outerInhabitable := outerInhabitable? then
    return (outerCarriers, outerType, some outerInhabitable, resolvedNonempty)
  let hinted ← mkExpectedTypeHint inhabitable
    (← mkEq (mkApp (mkConst ``NTy.inhabitable) outerType) (mkConst ``Bool.true))
  return (outerCarriers, outerType, some hinted, resolvedNonempty)

/-- The view of a native type's values in a typed map: integers by their
values and certified concrete values as they are, a type parameter's as its
family reads them, with the typed values' instances. A plain type that can
carry certificates uses its total typed reader (`NTy.readPlain`); its inverse
requires the table's actual certificate. A view reads one of the two
layers a value of the type has — the certified values of a table a value of
its type holds, or the plain values of a table a mutable reference's current
holds, which carry no certificate while the loan lives
(`doc/designs/data-invariants.md`). -/
structure TypedView where
  /-- The family and type the view is decided on (`resolvedType`). -/
  carriers : Lean.Expr
  type : Lean.Expr
  /-- The family's inhabitants (`CarriersNonempty`), which the default of a
  missing key and the write-back take. -/
  nonempty : Lean.Expr
  /-- The Lean type of the typed values. -/
  carrier : Lean.Expr
  /-- The read of a native value into its typed value. -/
  read : Lean.Expr
  /-- The `Inhabited` instance of the typed values. -/
  inhabited : Lean.Expr
  /-- The `DecidableEq` instance of the typed values. -/
  decEq : Lean.Expr
  /-- Whether the typed values are integers. -/
  integer : Bool
  /-- Whether the read is the identity: the values are held as they are. -/
  identity : Bool
  /-- Whether the view reads the plain values of its type (`NTy.plain`) —
  the entries of a table a mutable reference's current holds — rather than
  the certified ones.  An integer type's two layers are the same type, so
  its view is the same at both and the flag only records which was asked
  for. -/
  plain : Bool
  /-- The native value of a typed value, inverse to the read (`NTy.unread`). -/
  unread : Lean.Expr

open Lean Meta in
/-- The plain reader at a resolved certificate-free type. Decide the branch
at construction, while preserving the nominal type descriptor in its domain.
Unknown types and types carrying certificates retain their total codec reader. -/
def canonicalPlainReader? (carriers τ : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if carriers.hasLooseBVars || τ.hasLooseBVars then return none
  let resolved ← nativeTypeHead τ
  if resolved.hasLooseBVars || isNominalType (← getEnv) resolved then return none
  if resolved.isAppOfArity ``NTy.param 1 then
    -- Given sorts, including the declaration scaffold, share their plain
    -- and typed values and store the identity plain reader. Inspect that
    -- interpretation rather than choosing an identity for an unknown family.
    let domain := mkApp2 (mkConst ``NTy.plain) carriers τ
    let typed := mkApp2 (mkConst ``NTy.readCarrier) carriers τ
    unless ← withNewMCtxDepth (withDefault (isDefEq domain typed)) do return none
    let identity ← withLocalDeclD `value domain fun value => mkLambdaFVars #[value] value
    let reader := mkProj ``Subtype 0
      (mkApp2 (mkConst ``Carriers.plainRead) carriers (resolved.getArg! 0))
    unless ← withNewMCtxDepth (withDefault (isDefEq reader identity)) do return none
    return some identity
  let free ← withTransparency .default <| whnf
    (mkApp (mkConst ``NTy.certificateFree) resolved)
  unless free.isConstOf ``Bool.true do return none
  if resolved.isAppOfArity ``NTy.int 2 then
    return some (mkApp2 (mkConst ``SpecInt.val)
      (mkApp (mkConst ``IntWidth.bits) (resolved.getArg! 0)) (resolved.getArg! 1))
  if resolved.isConstOf ``NTy.num then
    return some (mkApp2 (mkConst ``SpecInt.val)
      (mkConst ``IntWidth.unbounded) (mkConst ``Bool.true))
  let domain := mkApp2 (mkConst ``NTy.carrier) carriers τ
  return some (← withLocalDeclD `value domain fun value => mkLambdaFVars #[value] value)

open Lean Meta in
/-- A certificate-free identity reader at the canonical native carrier.
Already-materialized plain identity lambdas are handled at the same boundary
as the family's readers, retaining the original native type descriptor. -/
private def canonicalIdentityReader (reader : Lean.Expr) : MetaM Lean.Expr := do
  let .lam _ domain body _ := reader.consumeMData | return reader
  unless body == .bvar 0 do return reader
  let domain := domain.consumeMData
  let familyType? : Option (Lean.Expr × Lean.Expr) := do
    if domain.isAppOfArity ``NTy.plain 2 then
      return (domain.getArg! 0, domain.getArg! 1)
    if domain.isAppOfArity ``NTy.plainOf 2 then
      let_expr Carriers.toPlain family := domain.getArg! 0 | none
      return (family, domain.getArg! 1)
    none
  let some (family, type) := familyType? | return reader
  let some canonical ← canonicalPlainReader? family type | return reader
  unless ← withNewMCtxDepth (withDefault (isDefEq reader canonical)) do return reader
  return canonical

open Lean Meta in
/-- Reconstruct a table field's named pair projections from the actual pair
at each layer. An instantiated owner schema can otherwise retain old
implicit field types even though its value is the caller's same pair. -/
private partial def tableFieldProjections (entries : Lean.Expr) : MetaM Lean.Expr := do
  let entries := entries.consumeMData
  let .const getter _ := entries.getAppFn | return entries
  unless (getter == ``Prod.fst || getter == ``Prod.snd) && entries.getAppNumArgs == 3 do
    return entries
  let pair ← tableFieldProjections (entries.getArg! 2)
  let rebuilt ← withDefault (mkAppM' entries.getAppFn #[pair])
  unless ← withNewMCtxDepth (withDefault (isDefEq entries rebuilt)) do return entries
  return rebuilt

open Lean Meta in
/-- Construct a typed map from its readers' canonical domains and codomains.
The explicit raw types keep a generic read specialized to a concrete type
identical to that concrete read, including identity readers already emitted
at the plain layer. The table is accepted only by checked conversion. -/
def typedMapOfEntries (key value entries : Lean.Expr) : MetaM Lean.Expr := withDefault do
  let entries ← tableFieldProjections entries
  let key ← canonicalIdentityReader key
  let value ← canonicalIdentityReader value
  let .forallE _ keyDomain keyRange _ ← whnfR (← inferType key)
    | throwError "the map's key reader is not a function"
  let .forallE _ valueDomain valueRange _ ← whnfR (← inferType value)
    | throwError "the map's value reader is not a function"
  if keyRange.hasLooseBVars || valueRange.hasLooseBVars then
    throwError "a typed map's reader has a dependent codomain"
  mkAppOptM ``VerexIR.SpecMap.ofEntries
    #[keyDomain, valueDomain, keyRange, valueRange, key, value, entries]

open Lean Meta in
/-- Rebuild a map observation at its reader-derived types after the readers
specialize at an instantiated family. -/
def canonicalMapOfEntries? (map : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr VerexIR.SpecMap.ofEntries _ _ _ _ key value entries := map | return none
  observing? <| withDefault do
    let canonical ← typedMapOfEntries key value entries
    if canonical == map then failure
    unless ← withNewMCtxDepth (isDefEq map canonical) do failure
    return canonical

open Lean Meta in
/-- The typed view of a native type's values: integers by their values for
integers and `num`, the family's reads for a type parameter, the values
themselves where the type has an inhabitant that evaluates; none
otherwise. `plain` asks for the view of the type's plain values, the
entries a table inside a mutable reference's current holds. Certificate-free
concrete values keep their identity view; concrete values that can carry a
certificate and unresolved parameters use the total typed reader
(`NTy.readPlain`). This is the same reader a generic parameter obtains at
instantiation. Recovering the plain entries from that reading requires their
actual `deepHolds` certificate, since invalid plain values can share the
decoder's fallback (`doc/designs/data-invariants.md`). -/
def typedView? (carriers τ : Lean.Expr) (plain : Bool := false) :
    MetaM (Option TypedView) := do
  let (carriers, τ, resolved?, resolvedNonempty?) ← resolvedType carriers τ
  let int := mkConst ``Int
  let inhabitable := mkApp (mkConst ``NTy.inhabitable) τ
  -- The type has values: evaluated where the type is known, and otherwise
  -- by the resolution's certificate, which a symbolic frame's type
  -- arguments carry (`resolvedType`).
  let decided? : Option Lean.Expr ←
    if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
      some <$> mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable (mkConst ``Bool.true))
    else pure resolved?
  let some decided := decided? | return none
  -- A read of a missing key returns the type's inhabitant, so every view
  -- needs the family's inhabitants; a family without them is a mistake in
  -- the goal's instances, not a type the view declines.
  let some nonempty ← (match resolvedNonempty? with
    | some supplied => pure (some supplied)
    | none => synthInstance? (mkApp (mkConst ``CarriersNonempty) carriers))
    | throwError "typedView?: the family {carriers} has no `CarriersNonempty` instance"
  let nonempty ← canonicalInhabitants carriers nonempty
  let unread := mkApp4 (mkConst ``NTy.unread) carriers nonempty τ decided
  let integers (bits signed : Lean.Expr) : MetaM TypedView := do
    return { carriers, type := τ, nonempty, carrier := int,
             read := mkApp2 (mkConst ``SpecInt.val) bits signed,
             inhabited := ← synthInstance (← mkAppM ``Inhabited #[int]),
             decEq := ← synthInstance (← mkAppM ``DecidableEq #[int]), integer := true,
             identity := false, plain, unread }
  if τ.isAppOfArity ``NTy.int 2 then
    return some (← integers (mkApp (mkConst ``IntWidth.bits) (τ.getArg! 0)) (τ.getArg! 1))
  if τ.isConstOf ``NTy.num then
    return some (← integers (mkConst ``IntWidth.unbounded) (mkConst ``Bool.true))
  -- A given parameter has one abstract sort for its native, plain and
  -- specification roles. Emit that sort's identity view directly rather than
  -- reintroducing the concrete read/unread path into generic clauses.
  if carriers.isAppOfArity ``GivenFrame.toCarriers 1 && τ.isAppOfArity ``NTy.param 1 then
    let carrier := mkApp2 (mkConst ``NTy.carrier) carriers τ
    let read ← withLocalDeclD `value carrier fun value => mkLambdaFVars #[value] value
    return some { carriers, type := τ, nonempty, carrier, read,
                  inhabited := mkApp4 (mkConst ``NTy.readInhabited) carriers nonempty τ decided,
                  decEq := mkApp2 (mkConst ``NTy.decEq) carriers τ, integer := false,
                  identity := true, plain, unread := read }
  let inhabitant := mkApp4 (mkConst ``NTy.inhabitantOf) carriers nonempty τ decided
  let env ← getEnv
  let knownShape := isNominalType env τ ||
    match τ.getAppFn.constName?.bind env.find? with
    | some (.ctorInfo info) => info.induct == ``NTy
    | _ => false
  let needsPlainReader ← if plain && knownShape then do
      let free ← withTransparency .default <| whnf (mkApp (mkConst ``NTy.certificateFree) τ)
      pure (!free.isConstOf ``Bool.true)
    else pure false
  if plain && (τ.isAppOfArity ``NTy.param 1 || !knownShape || needsPlainReader) then
    -- Keep the same total typed reader for a concrete certificate-bearing
    -- type and for a parameter instantiated at that type. An unresolved
    -- argument retains these operations until its family is known.
    -- The reader follows the argument's
    -- reader at an instantiation (`NTy.readPlain_instantiate`), preserving
    -- its typed sort and the total codec semantics. A typed value reads
    -- back to the plain layer by forgetting the certificate its unread
    -- operation supplies.
    let typedCarrier := mkApp2 (mkConst ``NTy.readCarrier) carriers τ
    let unreadPlain ← withLocalDeclD `typed typedCarrier fun typed => do
      mkLambdaFVars #[typed] (mkApp3 (mkConst ``NTy.forget) carriers τ (mkApp unread typed))
    return some { carriers, type := τ, nonempty, carrier := typedCarrier,
                  read := mkAppN (mkConst ``NTy.readPlain) #[carriers, nonempty, τ, decided],
                  inhabited := mkApp4 (mkConst ``NTy.readInhabited) carriers nonempty τ decided,
                  decEq := mkApp2 (mkConst ``NTy.readDecEq) carriers τ, integer := false,
                  identity := false, plain, unread := unreadPlain }
  if plain then
    -- Certificate-free concrete values have the same plain and native
    -- carrier. Use the native spelling for the identity reader and its
    -- instances, as the certified view and a specialized generic view do.
    let carrier := mkApp2 (mkConst ``NTy.carrier) carriers τ
    let read ← withLocalDeclD `value carrier fun value => mkLambdaFVars #[value] value
    return some { carriers, type := τ, nonempty, carrier, read,
                  inhabited := mkApp3 (mkConst ``nativeInhabited) carriers τ inhabitant,
                  decEq := mkApp2 (mkConst ``NTy.decEq) carriers τ, integer := false,
                  identity := true, plain, unread := read }
  if τ.isAppOfArity ``NTy.param 1 then
    return some { carriers, type := τ, nonempty,
                  carrier := mkApp2 (mkConst ``NTy.readCarrier) carriers τ,
                  read := mkApp2 (mkConst ``NTy.read) carriers τ,
                  inhabited := mkApp4 (mkConst ``NTy.readInhabited) carriers nonempty τ decided,
                  decEq := mkApp2 (mkConst ``NTy.readDecEq) carriers τ, integer := false,
                  identity := false, plain, unread }
  let inhabited := mkApp3 (mkConst ``nativeInhabited) carriers τ inhabitant
  let carrier := mkApp2 (mkConst ``NTy.carrier) carriers τ
  let read ← withLocalDeclD `value carrier fun value => mkLambdaFVars #[value] value
  return some { carriers, type := τ, nonempty, carrier, read, inhabited,
                decEq := mkApp2 (mkConst ``NTy.decEq) carriers τ, integer := false,
                identity := true, plain, unread := read }

open Lean Meta in
/-- The entry a table holds for a typed entry of its map, each part unread
by its view: the write-back of a specification's map (`SpecMap.writeBack`). -/
def TypedView.entryUnread (keys values : TypedView) : MetaM Lean.Expr := do
  let pairType ← mkAppM ``Prod #[keys.carrier, values.carrier]
  withLocalDeclD `entry pairType fun entry => do
    mkLambdaFVars #[entry] (← mkAppM ``Prod.mk
      #[keys.unread.beta #[← mkAppM ``Prod.fst #[entry]],
        values.unread.beta #[← mkAppM ``Prod.snd #[entry]]])

open Lean Meta in
/-- That a view's values read back (`NTy.unread_read`): `∀ x, unread (read x) = x`.
A plain view that reads its values as they are reads them back by
reflexivity; `NTy.unread_read` is a statement about the type's certified
values and does not reach the plain ones. -/
def TypedView.readBack (view : TypedView) : MetaM Lean.Expr := do
  if view.plain then
    unless view.identity do
      throwError "a total plain reader has no unconditional read-back: recovering its plain \
        values requires their actual certificate (`NTy.deepHolds`)"
    return ← withLocalDeclD `value view.carrier fun value => do
      mkLambdaFVars #[value] (← mkEqRefl value)
  let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq (mkApp (mkConst ``NTy.inhabitable) view.type) (mkConst ``Bool.true))
  withLocalDeclD `value (mkApp2 (mkConst ``NTy.carrier) view.carriers view.type) fun value => do
    let proof := mkAppN (mkConst ``NTy.unread_read)
      #[view.carriers, view.nonempty, view.type, decided, value]
    mkLambdaFVars #[value] (← mkExpectedTypeHint proof
      (← mkEq (view.unread.beta #[view.read.beta #[value]]) value))

open Lean Meta in
/-- The order a view's typed values compare by as an ordered map's keys, at
the given value ranks: the structural order of the type's values
(`NTy.keyOrder`) where the view reads certified values, and the runtime
order of the plain values' encodings where it reads plain ones — the same
order, since forgetting a value's certificates keeps its encoding
(`NTy.plainEncode_forget`), through the shared constructor
(`SpecMap.KeyOrder.byRuntime`), whose injectivity the plain codec gives. -/
def TypedView.plainKeyOrder (view : TypedView) (rank : Lean.Expr) : MetaM Lean.Expr := do
  let codec := mkApp2 (mkConst ``NTy.plainCodec) view.carriers view.type
  mkAppM ``SpecMap.KeyOrder.byRuntime
    #[← mkAppM ``Codec.encode #[codec], ← mkAppM ``Codec.encode_injective #[codec], rank]

open Lean Meta in
/-- Under typed values, a choice projecting nothing of a value is the
specification's test of the value's variant (`Which.project?_eq_none_iff`);
an enum's variant names are distinct. -/
simproc [vir_denote, vir_denote_norm] variantProjectionNone
    (@Eq _ _ (@Option.none _)) := fun e => do
  let_expr Eq _ lhs rhs := e | return .continue
  unless rhs.isAppOfArity ``Option.none 1 do return .continue
  let_expr Which.project? unit functions skolems names rows σs choice value := lhs
    | return .continue
  try
    let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[names])
    let proof := mkAppN (mkConst ``Which.project?_eq_none_iff)
      #[unit, functions, skolems, names, rows, σs, choice, value, distinct]
    let some (_, rewritten) := (← inferType proof).iff? | return .continue
    return .visit { expr := rewritten, proof? := some (← mkPropExt proof) }
  catch _ => return .continue

open Lean Meta in
/-- The `DecidableEq` instance of typed keys a lemma's right side alone
needs: a native type's own equality, or the synthesized one. -/
private def keyInstance? (type : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr DecidableEq carrier := type | return none
  if let .app (.app (.const ``NTy.carrier _) carriers) τ := carrier.consumeMData then
    return some (mkApp2 (mkConst ``NTy.decEq) carriers τ)
  try some <$> synthInstance type catch _ => return none

theorem forall_imp_congr {α : Sort u} {A B C : α → Prop} (same : ∀ k, A k → B k = C k) :
    (∀ k, A k → B k) = (∀ k, A k → C k) :=
  propext ⟨fun h k a => (same k a) ▸ h k a, fun h k a => (same k a).symm ▸ h k a⟩

open Lean Meta in
/-- Under the presence of an entry, its presence reads `true`: in
`∀ k, map.hasKey k = true → body`, the body's `map.hasKey k` is `true`, as
the invariant of a typed map's entries states it (`forall_entries_key`). -/
simproc [vir_denote, vir_denote_norm] presentKey (∀ _, SpecMap.hasKey _ _ = true → _) := fun e => do
  let .forallE name domain body info := e | return .continue
  let .forallE _ premise conclusion _ := body | return .continue
  if conclusion.hasLooseBVar 0 then return .continue
  let conclusion := conclusion.lowerLooseBVars 1 1
  let_expr Eq _ held true_ := premise | return .continue
  unless true_.isConstOf ``Bool.true && held.isAppOfArity ``SpecMap.hasKey 5 do return .continue
  withLocalDecl name info domain fun k => do
    let premise := premise.instantiate1 k
    let conclusion := conclusion.instantiate1 k
    let held := held.instantiate1 k
    let abstracted ← kabstract conclusion held
    unless abstracted.hasLooseBVars do return .continue
    let motive := Lean.mkLambda `present .default (mkConst ``Bool) abstracted
    let rewritten := abstracted.instantiate1 (mkConst ``Bool.true)
    let same ← withLocalDeclD `present premise fun h => do
      mkLambdaFVars #[k, h] (← mkCongrArg motive h)
    let proof := mkAppN (mkConst ``forall_imp_congr [← getLevel domain])
      #[domain, ← mkLambdaFVars #[k] premise, ← mkLambdaFVars #[k] conclusion,
        ← mkLambdaFVars #[k] rewritten, same]
    let e' ← mkForallFVars #[k] (← mkArrow premise rewritten)
    return .visit { expr := e', proof? := some (← mkExpectedTypeHint proof (← mkEq e e')) }

open Lean Meta in
/-- Rewrite by the first of the lemmas whose left side unifies with the
term at default transparency, where a native type's carrier converts to
its reduced form; every argument of the lemma must be fixed by the
unification. -/
private def rewriteByAny (e : Lean.Expr) (lemmas : List Name) : SimpM Simp.Step := do
  for lemma in lemmas do
    let saved ← Meta.saveState
    let step? ← try
        let theorem_ ← mkConstWithFreshMVarLevels lemma
        let (arguments, _, statement) ← forallMetaTelescopeReducing (← inferType theorem_)
        let some (lhs, rhs, iff) := (statement.eq?.map fun (_, lhs, rhs) => (lhs, rhs, false)) <|>
            (statement.iff?.map fun (lhs, rhs) => (lhs, rhs, true)) | pure none
        if ← withDefault (isDefEq lhs e) then
          for argument in arguments do
            unless ← argument.mvarId!.isAssigned do
              let type ← instantiateMVars (← inferType argument)
              if let some instance_ ← keyInstance? type then argument.mvarId!.assign instance_
          let proof ← instantiateMVars (mkAppN theorem_ arguments)
          let rhs ← instantiateMVars rhs
          if proof.hasExprMVar || rhs.hasExprMVar then pure none
          else
            let proof ← if iff then mkPropExt proof else pure proof
            pure (some (rhs, ← mkExpectedTypeHint proof (← mkEq e rhs)))
        else pure none
      catch _ => pure none
    match step? with
    | some (rhs, proof) => return .visit { expr := rhs, proof? := some proof }
    | none => saved.restore
  return .continue

open Lean Meta in
/-- A closed observation of a typed map built from literals evaluates: its
value, under typed maps. The lemmas need the map valid, which a chain of
literal constructions discharges only within simp's depth; evaluation needs
nothing. -/
private def evaluateLiteralObservation (e : Lean.Expr) : SimpM Simp.DStep := do
  if e.hasFVar || e.hasMVar || e.hasLooseBVars then return .continue
  let literal (sub : Lean.Expr) : Bool :=
    sub.isConstOf ``List.nil || sub.isConstOf ``List.cons ||
    sub.isConstOf ``SpecMap.update || sub.isConstOf ``SpecMap.remove ||
    sub.isConstOf ``SpecMap.Discipline.sequence || sub.isConstOf ``SpecMap.Discipline.ordered ||
    sub.isConstOf ``SpecMap.intOrder || sub.isConstOf ``OfNat.ofNat || sub.isConstOf ``Int ||
    sub.isConstOf ``Prod.mk || sub.isConstOf ``Prod
  -- Only observations spelled from integer literals, the constructions, and
  -- instances.
  for name in e.getUsedConstants do
    unless literal (mkConst name) || name.getPrefix == ``VerexIR.SpecMap || (← isInstance name) do
      return .continue
  -- Evaluated lazily, to the constructor: a full reduction would also
  -- normalize the instances and proofs inside.
  let value ← withDefault (whnf e)
  let result ← (do
    if value.isConstOf ``Bool.true || value.isConstOf ``Bool.false then return some value
    if let some n := value.rawNatLit? <|> value.nat? then return some (mkNatLit n)
    let_expr Int.ofNat n := value | pure none
    let n ← withDefault (whnf n)
    let some n := n.rawNatLit? <|> n.nat? | pure none
    pure (some (mkIntLit n)))
  let some result := result | return .continue
  return .done result

open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalHasKey (SpecMap.hasKey _ _) := fun e => do
  let_expr SpecMap.hasKey _ _ _ _ _ := e | return .continue
  evaluateLiteralObservation e
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalValueAt (SpecMap.valueAt _ _) := fun e => do
  let_expr SpecMap.valueAt _ _ _ _ _ _ := e | return .continue
  evaluateLiteralObservation e
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalSize (SpecMap.size _) := fun e => do
  let_expr SpecMap.size _ _ map := e | return .continue
  unless map.isAppOf ``SpecMap.update || map.isAppOf ``SpecMap.remove || map.isAppOf ``List.cons ||
    map.isAppOf ``List.nil do return .continue
  evaluateLiteralObservation e
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalKeyAt (SpecMap.keyAt _ _) := fun e => do
  let_expr SpecMap.keyAt _ _ _ _ _ := e | return .continue
  evaluateLiteralObservation e
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalRank (SpecMap.rank _ _) := fun e => do
  let_expr SpecMap.rank _ _ _ _ _ := e | return .continue
  evaluateLiteralObservation e

/-- A choice projecting fields of a value is the value's being that
variant with those fields (`Which.project?_eq_some_iff`), applied at
default transparency: the fields are a row's carrier, which the unifier
does not see as the tuple the leaf spells. -/
simproc [vir_denote, vir_denote_norm] variantProjectionSome
    (@Eq _ _ (@Option.some _ _)) := fun e => do
  let_expr Eq _ lhs _ := e | return .continue
  unless lhs.isAppOfArity ``Which.project? 8 do return .continue
  rewriteByAny e [``Which.project?_eq_some_iff]


end VerexIR.Proofs.Denote.NativeMaps

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
carriers' width, is the key: the keys fit, as the fallback does. -/
@[vir_denote_norm] theorem val_ofInt_keyAt_ofEntries {V V' : Type} {width : IntWidth}
    {signed : Bool} (default absent : SpecInt width signed) (value : V → V')
    (table : SpecEntries (SpecInt width signed × V)) (index : Int) :
    (SpecInt.ofInt default (@SpecMap.keyAt Int V' ⟨absent.val⟩
        (SpecMap.ofEntries SpecInt.val value table) index)).val =
      @SpecMap.keyAt Int V' ⟨absent.val⟩ (SpecMap.ofEntries SpecInt.val value table) index :=
  SpecInt.val_ofInt default (@SpecMap.keyAt_satisfies_all Int V' ⟨absent.val⟩ _ _
    (keys_ofEntries_fit value table) absent.fits index)

/-- A value of a table's map read from integer carriers, cut back to the
carriers' width, is the value. -/
@[vir_denote_norm] theorem val_ofInt_valueAt_ofEntries {K K' : Type} [DecidableEq K']
    {width : IntWidth} {signed : Bool} (default absent : SpecInt width signed) (key : K → K')
    (table : SpecEntries (K × SpecInt width signed)) (at_ : K') :
    (SpecInt.ofInt default (@SpecMap.valueAt K' Int _ ⟨absent.val⟩
        (SpecMap.ofEntries key SpecInt.val table) at_)).val =
      @SpecMap.valueAt K' Int _ ⟨absent.val⟩ (SpecMap.ofEntries key SpecInt.val table) at_ :=
  SpecInt.val_ofInt default (@SpecMap.valueAt_satisfies K' Int _ ⟨absent.val⟩ _ _
    (values_ofEntries_fit key table) absent.fits at_)

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
@[simp, vir_denote_norm] theorem NTy.read_char :
    NTy.read (.char) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_tuple (elements : NRow) :
    NTy.read (.tuple elements) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_struct (source : StructHandle) (arguments fields : NRow) :
    NTy.read (.struct source arguments fields) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup) :
    NTy.read (.enum source arguments names rows distinct) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_vector (element : NTy) :
    NTy.read (.vector element) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_slice (element : NTy) :
    NTy.read (.slice element) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_array (element : NTy) (length : Nat) :
    NTy.read (.array element length) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_table (key value : NTy) :
    NTy.read (.table key value) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_ghost (slot : NTy) :
    NTy.read (.ghost slot) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_ref (referent : NTy) :
    NTy.read (.ref referent) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_sliceRef (referent : NTy) :
    NTy.read (.sliceRef referent) = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.read_function (parameters : NRow) (shared : Sharing)
    (results : NRow) :
    NTy.read (.function parameters shared results) = fun value => value := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NTy.read_param_instantiate (θ : TypeArgs) (Θ : Carriers)
    (index : Nat) :
    @NTy.read (Carriers.instantiate θ Θ) (.param index) =
      @NTy.read Θ ((NTy.param index).subst θ.1) := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NTy.unread_param_instantiate (θ : TypeArgs) (Θ : Carriers)
    (index : Nat) (inhabitable : (NTy.param index).inhabitable = true) :
    @NTy.unread (Carriers.instantiate θ Θ) (.param index) inhabitable =
      @NTy.unread Θ ((NTy.param index).subst θ.1)
        ((NTy.inhabitable_unmark _).trans
          (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl)) := rfl

/-! The native equality of tables of scalar keys and values: the equality of
their typed maps. -/

/-- At a scalar type, the native equality is the equality of the values read. -/
theorem NTy.equal_scalar (handles : Bool) : (τ : NTy) → τ.isScalar = true →
    (left right : τ.carrier) →
      (NTy.equal handles τ left right = true ↔ NTy.read τ left = NTy.read τ right)
  | .unit, _, _, _ => by simp [NTy.equal, NTy.read]
  | .bool, _, _, _ => by simp [NTy.equal, NTy.read]
  | .int _ _, _, _, _ => by simp [NTy.equal, NTy.read]
  | .num, _, _, _ => by simp [NTy.equal, NTy.read]
  | .address, _, _, _ => by simp [NTy.equal, NTy.read]
  | .signer, _, _, _ => by simp [NTy.equal, NTy.read]
  | .string, _, _, _ => by simp [NTy.equal, NTy.read]
  | .bytes, _, _, _ => by simp [NTy.equal, NTy.read]
  | .char, _, _, _ => by simp [NTy.equal, NTy.read]
  | .tuple _, scalar, _, _ | .struct _ _ _, scalar, _, _ | .enum _ _ _ _ _, scalar, _, _
  | .vector _, scalar, _, _ | .slice _, scalar, _, _ | .array _ _, scalar, _, _
  | .table _ _, scalar, _, _ | .ghost _, scalar, _, _
  | .ref _, scalar, _, _ | .sliceRef _, scalar, _, _ | .param _, scalar, _, _
  | .function _ _ _, scalar, _, _
  | .nominal _ _ _, scalar, _, _ => by
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
  | .unit | .bool | .address | .signer | .string | .bytes | .char | .tuple _ | .struct _ _ _
  | .enum _ _ _ _ _ | .vector _ | .slice _ | .array _ _ | .table _ _ | .ghost _ | .ref _
  | .sliceRef _ | .function _ _ _
  | .nominal _ _ _ =>
      fun _ _ same => same
  | .observed referent => NTy.read_injective referent

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
theorem ofEntries_toSkolem_params [Θ : Carriers] {K V : Type}
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
    [Θ : Carriers] (θ : TypeArgs) (i j : Nat)
    (table : @NTy.carrier (Carriers.instantiate θ Θ) (.table (.param i) (.param j))) :
    NTy.ofSkolem θ (.table (.param i) (.param j)) table = table :=
  SpecEntries.ext (by
    show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
    exact List.map_id'' (fun _ => rfl) _)

@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.toSkolem_table_params
    [Θ : Carriers] (θ : TypeArgs) (i j : Nat)
    (table : ((NTy.table (.param i) (.param j)).subst θ.1).carrier) :
    NTy.toSkolem θ (.table (.param i) (.param j)) table = table :=
  SpecEntries.ext (by
    show List.map (fun entry => (entry.fst, entry.snd)) table.entries = table.entries
    exact List.map_id'' (fun _ => rfl) _)

theorem ofEntries_ofSkolem_params [Θ : Carriers] {K V : Type}
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
/-- A transported table of type parameters: its family, type arguments,
indices, and the caller's table. -/
private def transportedParams? (table : Lean.Expr) :
    Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) := do
  guard (table.isAppOfArity ``NTy.toSkolem 4 || table.isAppOfArity ``NTy.ofSkolem 4)
  let type := table.getArg! 2
  guard (type.isAppOfArity ``NTy.table 2)
  let key := type.getArg! 0
  let value := type.getArg! 1
  guard (key.isAppOfArity ``NTy.param 1 && value.isAppOfArity ``NTy.param 1)
  return (table.getArg! 0, table.getArg! 1, key.getArg! 0, value.getArg! 0, table.getArg! 3)

open Lean Meta in
/-- The typed map of a caller's table carried to a generic callee is the
caller's (`ofEntries_toSkolem_params`). -/
simproc ↓ [vir_denote, vir_denote_norm] ofEntriesTransport (SpecMap.ofEntries _ _ _) := fun e => do
  let_expr SpecMap.ofEntries _ _ keyType valueType key value table := e | return .continue
  let some (family, θ, i, j, caller) := transportedParams? table | return .continue
  let result := mkAppN e.getAppFn (e.getAppArgs.set! 6 caller)
  let identity := if table.isAppOf ``NTy.toSkolem then ``ofEntries_toSkolem_params
    else ``ofEntries_ofSkolem_params
  let proof := mkAppN (mkConst identity)
    #[family, keyType, valueType, θ, i, j, key, value, caller]
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

/-- A type parameter's reading at the family a call's type arguments induce
is the outer family's reading at the argument's type. -/
@[simp, vir_denote_norm] theorem _root_.VerexIR.Proofs.Denote.NTy.read_instantiate
    (θ : TypeArgs) (outer : Carriers) (index : Nat)
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
arguments are inhabitable by their own proof (`TypeArgs.inhabitable`), so a
parameter resolved through one has values even where the argument is symbolic
and nothing evaluates: without the proof a view of a declaration's own clause
at a symbolic frame could not be decided at all (`typedView?`). -/
private partial def resolvedType (carriers τ : Lean.Expr) :
    MetaM (Lean.Expr × Lean.Expr × Option Lean.Expr) := do
  let τ ← nativeTypeHead τ
  let_expr NTy.param index := τ | return (carriers, τ, none)
  -- The instantiated family, spelled by carriers or by skolems.
  let instantiation? : Option (Lean.Expr × Lean.Expr) := do
    let carriers := carriers.consumeMData
    if carriers.isAppOfArity ``Carriers.instantiate 2 then
      return (carriers.getArg! 0, carriers.getArg! 1)
    let .app (.app (.app toCarriers unit) functions) skolems := carriers | none
    guard (toCarriers.isConstOf ``Skolems.toCarriers)
    let .app (.app (.app (.app instantiate _) _) θ) outer := skolems.consumeMData | none
    guard (instantiate.isConstOf ``Skolems.instantiate)
    return (θ, mkApp3 toCarriers unit functions outer)
  let some (θ, outer) := instantiation? | return (carriers, τ, none)
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
  -- The substitution reads the entry with its mark dropped (`NTy.subst`),
  -- which keeps its inhabitants (`NTy.inhabitable_unmark`).
  let inhabitable ← mkAppM ``Eq.trans
    #[mkApp (mkConst ``NTy.inhabitable_unmark) (mkApp3 (mkConst ``NRow.getD) row index parameter),
      inhabitable]
  let (outerCarriers, outerType, outerInhabitable?) ← resolvedType outer resolved
  if let some outerInhabitable := outerInhabitable? then
    return (outerCarriers, outerType, some outerInhabitable)
  let hinted ← mkExpectedTypeHint inhabitable
    (← mkEq (mkApp (mkConst ``NTy.inhabitable) outerType) (mkConst ``Bool.true))
  return (outerCarriers, outerType, some hinted)

/-- The view of a native type's values in a typed map over the single
`Carriers` representation: integers by their values and concrete values as
they are, a type parameter's as its family reads them, with the typed
values' instances. -/
structure TypedView where
  /-- The family and type the view is decided on (`resolvedType`). -/
  carriers : Lean.Expr
  type : Lean.Expr
  /-- The Lean type of the typed values. -/
  carrier : Lean.Expr
  /-- The read of a native value into its typed value. -/
  read : Lean.Expr
  /-- The `Inhabited` instance of the typed values (`NTy.plainFallback` for
  observable off-domain reads). -/
  inhabited : Lean.Expr
  /-- The `DecidableEq` instance of the typed values. -/
  decEq : Lean.Expr
  /-- Whether the typed values are integers. -/
  integer : Bool
  /-- Whether the read is the identity: the values are held as they are. -/
  identity : Bool
  /-- The native value of a typed value, inverse to the read (`NTy.unread`). -/
  unread : Lean.Expr

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
identical to that concrete read. The table is accepted only by checked
conversion. -/
def typedMapOfEntries (key value entries : Lean.Expr) : MetaM Lean.Expr := withDefault do
  let entries ← tableFieldProjections entries
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
themselves where the type is inhabitable; `none` otherwise. Observable
off-domain lookups (`SpecMap.valueAt`, `SpecMap.keyAt`) use the uninterpreted
`NTy.plainFallback` witness rather than concrete structural defaults or
`NTy.readBack`. -/
def typedView? (carriers τ : Lean.Expr) : MetaM (Option TypedView) := do
  let (carriers, τ, resolved?) ← resolvedType carriers τ
  let int := mkConst ``Int
  let inhabitable := mkApp (mkConst ``NTy.inhabitable) τ
  -- The type has values: evaluated where the type is known, and otherwise
  -- by the resolution's proof, which a symbolic frame's type arguments
  -- carry (`resolvedType`).
  let decided? : Option Lean.Expr ←
    if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
      some <$> mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable (mkConst ``Bool.true))
    else pure resolved?
  let some decided := decided? | return none
  let fallback := mkApp3 (mkConst ``NTy.plainFallback) carriers τ decided
  let unread := mkApp3 (mkConst ``NTy.unread) carriers τ decided
  let integers (bits signed : Lean.Expr) : MetaM TypedView := do
    let val := mkApp3 (mkConst ``SpecInt.val) bits signed fallback
    let inhabited := mkApp2 (mkConst ``Inhabited.mk [Level.one]) int val
    return { carriers, type := τ, carrier := int,
             read := mkApp2 (mkConst ``SpecInt.val) bits signed,
             inhabited,
             decEq := ← synthInstance (← mkAppM ``DecidableEq #[int]), integer := true,
             identity := false, unread }
  if τ.isAppOfArity ``NTy.int 2 then
    return some (← integers (mkApp (mkConst ``IntWidth.bits) (τ.getArg! 0)) (τ.getArg! 1))
  if τ.isConstOf ``NTy.num then
    return some (← integers (mkConst ``IntWidth.unbounded) (mkConst ``Bool.true))
  -- A given parameter has one abstract sort for its native and
  -- specification roles. Emit that sort's identity view directly rather than
  -- reintroducing the concrete read/unread path into generic clauses.
  if carriers.isAppOfArity ``GivenFrame.toCarriers 1 && τ.isAppOfArity ``NTy.param 1 then
    let carrier := mkApp2 (mkConst ``NTy.carrier) carriers τ
    let read ← withLocalDeclD `value carrier fun value => mkLambdaFVars #[value] value
    let inhabited := mkApp3 (mkConst ``nativeInhabited) carriers τ fallback
    return some { carriers, type := τ, carrier, read, inhabited,
                  decEq := mkApp2 (mkConst ``NTy.decEq) carriers τ, integer := false,
                  identity := true, unread := read }
  if τ.isAppOfArity ``NTy.param 1 then
    let typedCarrier := mkApp2 (mkConst ``NTy.readCarrier) carriers τ
    let read := mkApp2 (mkConst ``NTy.read) carriers τ
    let inhabited := mkApp2 (mkConst ``Inhabited.mk [Level.one]) typedCarrier (mkApp read fallback)
    return some { carriers, type := τ, carrier := typedCarrier, read, inhabited,
                  decEq := mkApp2 (mkConst ``NTy.readDecEq) carriers τ, integer := false,
                  identity := false, unread }
  let inhabited := mkApp3 (mkConst ``nativeInhabited) carriers τ fallback
  let carrier := mkApp2 (mkConst ``NTy.carrier) carriers τ
  let read ← withLocalDeclD `value carrier fun value => mkLambdaFVars #[value] value
  return some { carriers, type := τ, carrier, read, inhabited,
                decEq := mkApp2 (mkConst ``NTy.decEq) carriers τ, integer := false,
                identity := true, unread := read }

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
/-- That a view's values read back (`NTy.unread_read`): `∀ x, unread (read x) = x`. -/
def TypedView.readBack (view : TypedView) : MetaM Lean.Expr := do
  if view.identity then
    return ← withLocalDeclD `value view.carrier fun value => do
      mkLambdaFVars #[value] (← mkEqRefl value)
  let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq (mkApp (mkConst ``NTy.inhabitable) view.type) (mkConst ``Bool.true))
  withLocalDeclD `value (mkApp2 (mkConst ``NTy.carrier) view.carriers view.type) fun value => do
    let proof := mkAppN (mkConst ``NTy.unread_read)
      #[view.carriers, view.type, decided, value]
    mkLambdaFVars #[value] (← mkExpectedTypeHint proof
      (← mkEq (view.unread.beta #[view.read.beta #[value]]) value))

open Lean Meta in
/-- Under typed values, a choice projecting nothing of a value is the
specification's test of the value's variant (`Which.project?_eq_none_iff`);
an enum's variant names are distinct. -/
simproc [vir_denote, vir_denote_norm] variantProjectionNone
    (@Eq _ _ (@Option.none _)) := fun e => do
  let_expr Eq _ lhs rhs := e | return .continue
  unless rhs.isAppOfArity ``Option.none 1 do return .continue
  let_expr Which.project? carriers names rows σs choice value := lhs
    | return .continue
  try
    let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[names])
    let proof := mkAppN (mkConst ``Which.project?_eq_none_iff)
      #[carriers, names, rows, σs, choice, value, distinct]
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
private def evaluateLiteralObservation (e : Lean.Expr) (operands : Array Lean.Expr) :
    SimpM Simp.DStep := do
  if e.hasFVar || e.hasMVar || e.hasLooseBVars then return .continue
  let literal (sub : Lean.Expr) : Bool :=
    sub.isConstOf ``List.nil || sub.isConstOf ``List.cons ||
    sub.isConstOf ``SpecMap.update || sub.isConstOf ``SpecMap.remove ||
    sub.isConstOf ``SpecMap.Discipline.sequence || sub.isConstOf ``SpecMap.Discipline.ordered ||
    sub.isConstOf ``SpecMap.intOrder || sub.isConstOf ``OfNat.ofNat || sub.isConstOf ``Int ||
    sub.isConstOf ``Prod.mk || sub.isConstOf ``Prod
  -- Only observations whose map and query operands are spelled from integer
  -- literals, the constructions, and instances. Absent reads reduce to an
  -- uninterpreted fallback and decline below without leaking a concrete value.
  for operand in operands do
    for name in operand.getUsedConstants do
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
  let_expr SpecMap.hasKey _ _ _ map key := e | return .continue
  evaluateLiteralObservation e #[map, key]
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalValueAt (SpecMap.valueAt _ _) := fun e => do
  let_expr SpecMap.valueAt _ _ _ _ map key := e | return .continue
  evaluateLiteralObservation e #[map, key]
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalSize (SpecMap.size _) := fun e => do
  let_expr SpecMap.size _ _ map := e | return .continue
  unless map.isAppOf ``SpecMap.update || map.isAppOf ``SpecMap.remove || map.isAppOf ``List.cons ||
    map.isAppOf ``List.nil do return .continue
  evaluateLiteralObservation e #[map]
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalKeyAt (SpecMap.keyAt _ _) := fun e => do
  let_expr SpecMap.keyAt _ _ _ map index := e | return .continue
  evaluateLiteralObservation e #[map, index]
open Lean Meta in
dsimproc [vir_denote, vir_denote_norm] literalRank (SpecMap.rank _ _) := fun e => do
  let_expr SpecMap.rank _ _ _ map key := e | return .continue
  evaluateLiteralObservation e #[map, key]

/-- A choice projecting fields of a value is the value's being that
variant with those fields (`Which.project?_eq_some_iff`), applied at
default transparency: the fields are a row's carrier, which the unifier
does not see as the tuple the leaf spells. -/
simproc [vir_denote, vir_denote_norm] variantProjectionSome
    (@Eq _ _ (@Option.some _ _)) := fun e => do
  let_expr Eq _ lhs _ := e | return .continue
  unless lhs.isAppOfArity ``Which.project? 6 do return .continue
  rewriteByAny e [``Which.project?_eq_some_iff]

end VerexIR.Proofs.Denote.NativeMaps

namespace VerexIR.SpecMap

open VerexIR.Proofs.Denote

/-- The value of a key's first entry, when present. -/
def get? {K V : Type} [DecidableEq K] : SpecMap K V → K → Option V
  | [], _ => none
  | entry :: rest, key => if entry.1 = key then some entry.2 else get? rest key

@[simp] theorem get?_nil {K V : Type} [DecidableEq K] (key : K) :
    get? ([] : SpecMap K V) key = none := rfl

@[simp] theorem get?_cons {K V : Type} [DecidableEq K] (entry : K × V)
    (rest : SpecMap K V) (key : K) :
    get? (entry :: rest) key = if entry.1 = key then some entry.2 else get? rest key := rfl

theorem mem_of_get? {K V : Type} [DecidableEq K] {map : SpecMap K V} {key : K} {value : V}
    (found : get? map key = some value) : (key, value) ∈ map := by
  induction map with
  | nil => cases found
  | cons entry rest ih =>
      simp only [get?] at found
      split at found
      · rename_i same
        cases Option.some.inj found
        exact same ▸ List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (ih found)

/-- Every entry of a typed map holds the key and value invariants once unread
back to the table's carrier types. -/
@[reducible] def EntriesHold [Carriers] {K V : Type} (κ ν : NTy)
    (unreadKey : K → κ.carrier) (unreadValue : V → ν.carrier) (map : SpecMap K V) : Prop :=
  EntriesSatisfy
    (fun entry => NTy.Holds κ (unreadKey entry.1) ∧ NTy.Holds ν (unreadValue entry.2)) map

theorem _root_.VerexIR.Proofs.Denote.NTy.Holds.unread_read [Carriers]
    {τ : NTy} (inhabitable : τ.inhabitable = true) {value : τ.carrier}
    (holds : NTy.Holds τ value) :
    NTy.Holds τ (NTy.unread τ inhabitable (NTy.read τ value)) := by
  rwa [NTy.unread_read τ inhabitable]

theorem entries_hold_ofEntries [Carriers] {K V : Type} {κ ν : NTy}
    {readKey : κ.carrier → K} {readValue : ν.carrier → V}
    {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier}
    {table : (NTy.table κ ν).carrier}
    (held : NTy.Holds (.table κ ν) table)
    (keyHolds : ∀ k, NTy.Holds κ k → NTy.Holds κ (unreadKey (readKey k)) := by
      intro _ h; first | exact h | exact ⟨trivial⟩)
    (valueHolds : ∀ v, NTy.Holds ν v → NTy.Holds ν (unreadValue (readValue v)) := by
      intro _ h; first | exact h | exact ⟨trivial⟩) :
    EntriesHold κ ν unreadKey unreadValue (ofEntries readKey readValue table) :=
  entriesSatisfy_ofEntries readKey readValue table fun entry member =>
    ⟨keyHolds entry.1 (held.of_mem_table_key member),
      valueHolds entry.2 (held.of_mem_table_value member)⟩

theorem _root_.VerexIR.Proofs.Denote.NTy.Holds.of_specMap_get? [Carriers]
    {K V : Type} [DecidableEq K] {κ ν : NTy}
    {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier} {map : SpecMap K V}
    (entries : EntriesHold κ ν unreadKey unreadValue map)
    {key : K} {value : V} (found : get? map key = some value) :
    NTy.Holds ν (unreadValue value) :=
  (entries (key, value) (mem_of_get? found)).2

theorem _root_.VerexIR.Proofs.Denote.NTy.Holds.of_specMap_valueAt [Carriers]
    {K V : Type} [DecidableEq K] [Inhabited V]
    {κ ν : NTy} {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier} {map : SpecMap K V}
    (entries : EntriesHold κ ν unreadKey unreadValue map)
    {key : K} (present : hasKey map key = true) :
    NTy.Holds ν (unreadValue (valueAt map key)) :=
  (entries (key, valueAt map key) (mem_of_get map key present)).2

theorem _root_.VerexIR.Proofs.Denote.NTy.Holds.of_specMap_keyAt [Carriers]
    {K V : Type} [Inhabited K]
    {κ ν : NTy} {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier} {map : SpecMap K V}
    (entries : EntriesHold κ ν unreadKey unreadValue map)
    {index : Int} (low : 0 ≤ index) (high : index < size map) :
    NTy.Holds κ (unreadKey (keyAt map index)) :=
  keyAt_satisfies (property := fun k => NTy.Holds κ (unreadKey k))
    (fun entry member => (entries entry member).1) low high

theorem entries_hold_update [Carriers] {K V : Type} [DecidableEq K]
    {κ ν : NTy} {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier}
    {discipline : Discipline K} {map : SpecMap K V} {key : K} {value : V}
    (before : EntriesHold κ ν unreadKey unreadValue map)
    (keyHeld : NTy.Holds κ (unreadKey key))
    (valueHeld : NTy.Holds ν (unreadValue value)) :
    EntriesHold κ ν unreadKey unreadValue (update discipline map key value) :=
  entriesSatisfy_update before ⟨keyHeld, valueHeld⟩

theorem entries_hold_remove [Carriers] {K V : Type} [DecidableEq K]
    {κ ν : NTy} {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier}
    {discipline : Discipline K} {map : SpecMap K V} {key : K}
    (before : EntriesHold κ ν unreadKey unreadValue map) :
    EntriesHold κ ν unreadKey unreadValue (remove discipline map key) :=
  entriesSatisfy_remove before

theorem _root_.VerexIR.Proofs.Denote.NTy.Holds.of_writeBack [Carriers]
    {K V : Type} {κ ν : NTy}
    {unreadKey : K → κ.carrier} {unreadValue : V → ν.carrier} {map : SpecMap K V}
    (entries : EntriesHold κ ν unreadKey unreadValue map) :
    NTy.Holds (.table κ ν)
      (writeBack (fun entry => (unreadKey entry.1, unreadValue entry.2)) map) := by
  rw [NTy.holds_table]
  intro stored member
  unfold writeBack SpecEntries.ofList at member
  split at member
  · obtain ⟨entry, inside, rfl⟩ := List.mem_map.mp member
    exact entries entry inside
  · exact nomatch member

end VerexIR.SpecMap

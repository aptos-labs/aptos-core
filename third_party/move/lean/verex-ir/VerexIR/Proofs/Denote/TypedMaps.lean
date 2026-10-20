-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
import VerexIR.Proofs.Maps.Encoding

/-! Typed map observations at the authored-proof boundary. -/

namespace VerexIR.Proofs.Denote
open Lean Elab Tactic Meta
open Maps.Encoding

/-- Transport duplicate-index specifications across integer-vector encoding. -/
private theorem duplicate_indices {α : Type} (xs : List α) :
    ¬xs.Nodup ↔ ∃ (i j : Nat) (hi : i < xs.length) (hj : j < xs.length),
      i ≠ j ∧ xs[i] = xs[j] := by
  classical
  rw [List.nodup_iff_pairwise_ne, List.pairwise_iff_getElem]
  constructor
  · intro h
    simp only [Classical.not_forall, Classical.not_not] at h
    obtain ⟨i, j, hi, hj, lt, eq⟩ := h
    exact ⟨i, j, hi, hj, by omega, eq⟩
  · rintro ⟨i, j, hi, hj, ne, eq⟩ h
    by_cases lt : i < j
    · exact h i j hi hj lt eq
    · exact h j i hj hi (by omega) eq.symm

theorem integer_duplicate_indices (width : Nat) (signed : Bool)
    (keys : SpecVector (SpecInt (.bits width) signed)) :
    ¬(keys.values.map (Codec.specInt (.bits width) signed).encode).toList.Nodup ↔
      ∃ i : Int, 0 ≤ i ∧ i < keys.values.size ∧
        ∃ j : Int, 0 ≤ j ∧ j < keys.values.size ∧ i ≠ j ∧
          ((keys.values[i.toNat]?.map (Codec.specInt (.bits width) signed).encode).getD .unit).asInt =
          ((keys.values[j.toNat]?.map (Codec.specInt (.bits width) signed).encode).getD .unit).asInt := by
  rw [duplicate_indices]
  simp only [Array.length_toList, Array.size_map, Array.getElem_toList, Array.getElem_map]
  constructor
  · rintro ⟨i, j, hi, hj, ne, eq⟩
    refine ⟨i, by omega, by omega, j, by omega, by omega, by omega, ?_⟩
    simpa [hi, hj, Codec.specInt, RuntimeValue.asInt] using eq
  · rintro ⟨i, ilo, ihi, j, jlo, jhi, ne, eq⟩
    have hi : i.toNat < keys.values.size := by omega
    have hj : j.toNat < keys.values.size := by omega
    refine ⟨i.toNat, j.toNat, hi, hj, by omega, ?_⟩
    simpa [hi, hj, Codec.specInt, RuntimeValue.asInt] using eq

private theorem integer_entries [Carriers] (owner : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (kw : Nat) (ks : Bool) (vw : Nat) (vs : Bool)
    (contents : (NTy.table (.int kw ks) (.int vw vs)).carrier) :
    Maps.entriesOf (.nominal owner variant (fields.push (NTy.encode (.table (.int kw ks) (.int vw vs)) contents))) =
      Maps.Encoding.entries integers integers (contents.entries.map fun pair => (pair.1.val, pair.2.val)) := by
  simp only [Maps.entriesOf, Array.back?_push, NTy.encode_table, List.map_map,
    Maps.Encoding.entries, Function.comp_def, RuntimeValue.field, NTy.encode_int,
    Maps.Encoding.integers]
  rfl

private theorem integer_size_bounds [Carriers] (kw : Nat) (ks : Bool) (vw : Nat) (vs : Bool)
    (contents : (NTy.table (.int kw ks) (.int vw vs)).carrier) :
    0 ≤ SpecMap.size (contents.entries.map fun pair => (pair.1.val, pair.2.val)) ∧
      SpecMap.size (contents.entries.map fun pair => (pair.1.val, pair.2.val)) < 2 ^ 64 := by
  simp only [SpecMap.size, List.length_map]
  have := contents.bounded
  omega

private theorem integer_keys_fit [Carriers] (kw : Nat) (ks : Bool) (vw : Nat) (vs : Bool)
    (contents : (NTy.table (.int kw ks) (.int vw vs)).carrier) :
    SpecMap.KeysSatisfy (IntegerValueFits (.bits kw) ks)
      (contents.entries.map fun pair => (pair.1.val, pair.2.val)) := by
  intro pair member
  obtain ⟨native, _, rfl⟩ := List.mem_map.mp member
  exact native.1.fits


/-- Choose the codec's observation of the absent sentinel as the model default. -/
private abbrev nativeInhabited [Carriers] (τ : NTy) (fallback : τ.carrier) : Inhabited τ.carrier :=
  ⟨(τ.codec.decode? .unit).getD fallback⟩

private theorem native_unit [Carriers] (τ : NTy) (fallback : τ.carrier) :
    (τ.codec.decode? .unit).getD (@default _ (nativeInhabited τ fallback)) =
      @default _ (nativeInhabited τ fallback) := by
  change (τ.codec.decode? .unit).getD ((τ.codec.decode? .unit).getD fallback) =
    (τ.codec.decode? .unit).getD fallback
  cases τ.codec.decode? .unit <;> rfl

/-- A native value codec, with its absent-read observation stated explicitly. -/
private def nativeView [Carriers] (τ : NTy) [Inhabited τ.carrier]
    (unit : (τ.codec.decode? .unit).getD default = default) : View τ.carrier where
  write := τ.encode
  read value := (τ.codec.decode? value).getD default
  read_write value := by simp only [NTy.decode_encode, Option.getD_some]
  read_unit := unit

private theorem native_entries [Carriers] (owner : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) (kw : Nat) (ks : Bool) (τ : NTy)
    [Inhabited τ.carrier] (unit : (τ.codec.decode? .unit).getD default = default)
    (contents : (NTy.table (.int kw ks) τ).carrier) :
    Maps.entriesOf (.nominal owner variant (fields.push (NTy.encode (.table (.int kw ks) τ) contents))) =
      Maps.Encoding.entries integers (nativeView τ unit) (contents.entries.map fun pair => (pair.1.val, pair.2)) := by
  simp only [Maps.entriesOf, Array.back?_push, NTy.encode_table, List.map_map,
    Maps.Encoding.entries, Function.comp_def, RuntimeValue.field, NTy.encode_int,
    Maps.Encoding.integers, nativeView]
  rfl

private theorem native_size_bounds [Carriers] (kw : Nat) (ks : Bool) (τ : NTy)
    (contents : (NTy.table (.int kw ks) τ).carrier) :
    0 ≤ SpecMap.size (contents.entries.map fun pair => (pair.1.val, pair.2)) ∧
      SpecMap.size (contents.entries.map fun pair => (pair.1.val, pair.2)) < 2 ^ 64 := by
  simp only [SpecMap.size, List.length_map]
  have := contents.bounded
  omega

private theorem native_keys_fit [Carriers] (kw : Nat) (ks : Bool) (τ : NTy)
    (contents : (NTy.table (.int kw ks) τ).carrier) :
    SpecMap.KeysSatisfy (IntegerValueFits (.bits kw) ks)
      (contents.entries.map fun pair => (pair.1.val, pair.2)) := by
  intro pair member
  obtain ⟨native, _, rfl⟩ := List.mem_map.mp member
  exact native.1.fits

private theorem integer_order (ranks : ValueRanks) :
    integers.order ranks = SpecMap.intOrder := by
  have equal : (integers.order ranks).compare = SpecMap.intOrder.compare :=
    funext fun a => funext fun b => RuntimeValue.order_integer ranks a b
  cases h : integers.order ranks
  cases h₂ : SpecMap.intOrder
  simp only [h, h₂] at equal
  cases equal
  rfl

private theorem integer_size {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) :
    Maps.size raw = SpecMap.size map := Maps.Encoding.size _ _ _ _ model

private theorem integer_hasKey {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (key : Int) :
    Maps.hasKey raw (.integer key) = SpecMap.hasKey map key := Maps.Encoding.hasKey _ _ _ _ model key

private theorem integer_rank {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (key : Int) :
    Maps.rank raw (.integer key) = SpecMap.rank map key := Maps.Encoding.rank _ _ _ _ model key

private theorem integer_valueAt {raw : RuntimeValue} {map : SpecMap Int Int}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers integers map) (key : Int) :
    (Maps.valueAt raw (.integer key)).asInt = SpecMap.valueAt map key := Maps.Encoding.valueAt _ _ _ _ model key

private theorem integer_valueAt_write {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (key : Int)
    (present : SpecMap.hasKey map key = true) :
    Maps.valueAt raw (.integer key) = values.write (SpecMap.valueAt map key) :=
  Maps.Encoding.valueAt_write _ _ _ _ model key present

private theorem integer_keyAt {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (index : Int) :
    (Maps.keyAt raw index).asInt = SpecMap.keyAt map index := Maps.Encoding.keyAt _ _ _ _ model index

private theorem integer_keyAt_write {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (index : Int)
    (low : 0 ≤ index) (high : index < SpecMap.size map) :
    Maps.keyAt raw index = .integer (SpecMap.keyAt map index) := Maps.Encoding.keyAt_write _ _ _ _ model index low high

private theorem integer_valid {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) (ranks : ValueRanks) :
    Maps.Valid (.ordered ranks) raw ↔ SpecMap.Valid (.ordered SpecMap.intOrder) map := by
  simpa only [integer_order] using Maps.Encoding.valid integers values _ _ model ranks

private theorem integer_update {raw : RuntimeValue} {map : SpecMap Int Int}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers integers map)
    (layout : Maps.Layout) (ranks : ValueRanks) (key value : Int) :
    Maps.entriesOf (Maps.update layout (.ordered ranks) raw (.integer key) (.integer value)) =
      Maps.Encoding.entries integers integers (SpecMap.update (.ordered SpecMap.intOrder) map key value) := by
  have transported := Maps.Encoding.update integers integers _ _ model layout ranks key value
  rw [integer_order] at transported
  exact transported

private theorem integer_remove {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map)
    (layout : Maps.Layout) (ranks : ValueRanks) (key : Int) :
    Maps.entriesOf (Maps.remove layout (.ordered ranks) raw (.integer key)) =
      Maps.Encoding.entries integers values (SpecMap.remove (.ordered SpecMap.intOrder) map key) := by
  have transported := Maps.Encoding.remove integers values _ _ model layout ranks key
  rw [integer_order] at transported
  exact transported

private def optionalInteger : RuntimeValue → Option Int
  | .nominal _ (some "Some") fields => some (((fields[0]?).getD RuntimeValue.unit).asInt)
  | _ => none

private theorem optionalInteger_none (owner : StructHandle) :
    optionalInteger (.nominal owner (some "None") #[]) = none := rfl

private theorem optionalInteger_some (owner : StructHandle) (value : RuntimeValue) :
    optionalInteger (.nominal owner (some "Some") #[value]) = some value.asInt := rfl

private theorem optionalInteger_elim (owner : StructHandle) (value : Option RuntimeValue) :
    optionalInteger (value.elim (.nominal owner (some "None") #[])
      (fun key => .nominal owner (some "Some") #[key])) = value.map RuntimeValue.asInt := by
  cases value <;> rfl

private theorem integer_nextKey {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map)
    (ranks : ValueRanks) (key : Int) :
    (Maps.nextKey? ranks raw (.integer key)).map RuntimeValue.asInt =
      SpecMap.nextKey? SpecMap.intOrder map key := by
  have transported := Maps.Encoding.nextKey integers values _ _ model ranks key
  rw [integer_order] at transported
  exact transported

private theorem integer_prevKey {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map)
    (ranks : ValueRanks) (key : Int) :
    (Maps.prevKey? ranks raw (.integer key)).map RuntimeValue.asInt =
      SpecMap.prevKey? SpecMap.intOrder map key := by
  have transported := Maps.Encoding.prevKey integers values _ _ model ranks key
  rw [integer_order] at transported
  exact transported

private def integerElements (value : RuntimeValue) : List Int :=
  (Maps.elementsOf value).map RuntimeValue.asInt

private theorem integer_vector_elements (width : Nat) (signed : Bool)
    (keys : SpecVector (SpecInt (.bits width) signed)) :
    integerElements (.vector (keys.values.map (Codec.specInt (.bits width) signed).encode)) =
      keys.values.toList.map SpecInt.val := by
  simp [integerElements, Maps.elementsOf_vector, Array.toList_map, List.map_map,
    Function.comp_def, Codec.specInt, RuntimeValue.asInt]

private theorem integer_keysOf {V : Type} [Inhabited V] {values : View V}
    {raw : RuntimeValue} {map : SpecMap Int V}
    (model : Maps.entriesOf raw = Maps.Encoding.entries integers values map) :
    integerElements (Maps.keysOf raw) = map.map Prod.fst := by
  simp [integerElements, Maps.elementsOf_keysOf, model, Maps.Encoding.entries,
    List.map_map, Function.comp_def, integers, RuntimeValue.asInt]

private theorem integer_entries_nil :
    ([] : Maps.Entries) = Maps.Encoding.entries integers integers ([] : SpecMap Int Int) := rfl

private theorem integer_entries_cons (key value : Int) (map : SpecMap Int Int) :
    (RuntimeValue.integer key, RuntimeValue.integer value) :: Maps.Encoding.entries integers integers map =
      Maps.Encoding.entries integers integers ((key, value) :: map) := rfl

private theorem integer_contents_eq {V : Type} [Inhabited V] (values : View V) (a b : SpecMap Int V) :
    Maps.Encoding.entries integers values a = Maps.Encoding.entries integers values b ↔ a = b :=
  (Maps.Encoding.entries_injective integers values).eq_iff

private def integerMap? (term : Lean.Expr) : Option (FVarId × Lean.Expr × Array Lean.Expr) := do
  guard (term.isAppOfArity ``RuntimeValue.nominal 3)
  let fields ← term.getArg! 2 |>.arrayLit?.map (·.2.toArray)
  let field ← fields.back?
  guard (field.isAppOfArity ``NTy.encode 3)
  let table := field.getArg! 1
  guard (table.isAppOfArity ``NTy.table 2)
  guard ((table.getArg! 0).isAppOfArity ``NTy.int 2)
  let .fvar carrier := field.getArg! 2 | none
  return (carrier, field, fields.pop)

private partial def integerMaps (term : Lean.Expr) (found : Array Lean.Expr := #[]) : Array Lean.Expr :=
  let found := if (integerMap? term).isSome && !found.contains term then found.push term else found
  match term with
  | .app f a => integerMaps a (integerMaps f found)
  | .lam _ d b _ | .forallE _ d b _ => integerMaps b (integerMaps d found)
  | .letE _ t v b _ => integerMaps b (integerMaps v (integerMaps t found))
  | .mdata _ b | .proj _ _ b => integerMaps b found
  | _ => found

private def mapExpression (term : Lean.Expr) : Bool :=
  (integerMap? term).isSome || [``Maps.update, ``Maps.remove, ``Maps.Layout.restamp,
    ``Maps.empty, ``Maps.updateAll, ``Maps.takeEntries, ``Maps.dropEntries].any term.isAppOf

private partial def integerObservation (term : Lean.Expr) : Bool :=
  if term.isAppOfArity ``RuntimeValue.eraseGhosts 1 then
    integerObservation (term.getArg! 0)
  else if term.isAppOfArity ``Maps.valueAt 2 then
    match integerMap? (term.getArg! 0) with
    | some (_, field, _) => ((field.getArg! 1).getArg! 1).isAppOfArity ``NTy.int 2
    | none => false
  else [``RuntimeValue.integer, ``Maps.keyAt].any term.isAppOf

/-- Observe a hypothesis through a projection. This is only a forward
implication: content equality never replaces allocation identity in a goal. -/
private partial def observeMapFact (proof : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let ty ← instantiateMVars (← inferType proof)
  if let .forallE name domain _ info := ty then
    return ← withLocalDecl name info domain fun arg => do
      let some result ← observeMapFact (mkApp proof arg) | return none
      return some (← mkLambdaFVars #[arg] result)
  if ty.isAppOfArity ``Exists 2 then
    return ← withLocalDeclD `witness (ty.getArg! 0) fun witness => do
      let prop := (ty.getArg! 1).beta #[witness]
      withLocalDeclD `fact prop fun fact => do
        let some result ← observeMapFact fact | return none
        let mapper ← mkLambdaFVars #[witness, fact] result
        return some (← mkAppM ``Exists.imp #[mapper, proof])
  if ty.isAppOfArity ``Or 2 then
    let (leftMap, leftChanged) ← withLocalDeclD `left (ty.getArg! 0) fun left => do
      let observed ← observeMapFact left
      return (← mkLambdaFVars #[left] (observed.getD left), observed.isSome)
    let (rightMap, rightChanged) ← withLocalDeclD `right (ty.getArg! 1) fun right => do
      let observed ← observeMapFact right
      return (← mkLambdaFVars #[right] (observed.getD right), observed.isSome)
    unless leftChanged || rightChanged do return none
    return some (← mkAppM ``Or.imp #[leftMap, rightMap, proof])
  if ty.isAppOfArity ``And 2 then
    let left ← mkAppM ``And.left #[proof]
    let right ← mkAppM ``And.right #[proof]
    let l ← observeMapFact left
    let r ← observeMapFact right
    if l.isNone && r.isNone then return none
    return some (← mkAppM ``And.intro #[l.getD left, r.getD right])
  let some (type, left, right) := ty.eq? | return none
  unless type.isConstOf ``RuntimeValue do return none
  if mapExpression left || mapExpression right then
    return some (← mkAppM ``congrArg #[mkConst ``Maps.entriesOf, proof])
  if [left, right].any (·.isAppOf ``Maps.keysOf) then
    return some (← mkAppM ``congrArg #[mkConst ``integerElements, proof])
  if [left, right].any (fun term => (term.find? fun part =>
      part.isAppOf ``Maps.nextKey? || part.isAppOf ``Maps.prevKey?).isSome) then
    return some (← mkAppM ``congrArg #[mkConst ``optionalInteger, proof])
  if [left, right].any integerObservation then
    return some (← mkAppM ``congrArg #[mkConst ``RuntimeValue.asInt, proof])
  return none

private def existingIntegerModel (map : Lean.Expr) : MetaM (Option (FVarId × Lean.Expr)) := do
  for decl in ← getLCtx do
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
    unless lhs.isAppOfArity ``Maps.entriesOf 1 && rhs.isAppOf ``Maps.Encoding.entries do continue
    let args := rhs.getAppArgs
    unless args[args.size - 3]!.isConstOf ``integers do continue
    if ← isDefEq lhs (← mkAppM ``Maps.entriesOf #[map]) then
      return some (decl.fvarId, args.back!)
  return none

/-- Keep the representation equation available for observations exposed later,
without putting it in the authored context or simplifying it away. -/
private def hideMapBridges (goal : MVarId) (bridges : Array FVarId) : MetaM MVarId := goal.withContext do
  let mut lctx ← getLCtx
  for bridge in bridges do lctx := lctx.setKind bridge .implDetail
  let next ← mkFreshExprMVarAt lctx (← getLocalInstances) (← goal.getType) .syntheticOpaque (← goal.getTag)
  goal.assign next
  return next.mvarId!

/-- Congruence for a guarded observation: bounds on the left are available
while the right-hand observation is transported. Used only by map lifting. -/
private theorem guardedAnd {a a' b b' : Prop} (left : a ↔ a') (right : a' → (b ↔ b')) :
    (a ∧ b) ↔ (a' ∧ b') := by
  constructor
  · rintro ⟨ha, hb⟩
    exact ⟨left.mp ha, (right (left.mp ha)).mp hb⟩
  · rintro ⟨ha, hb⟩
    exact ⟨left.mpr ha, (right ha).mpr hb⟩

/-- Replace encoded integer-map observations by operations on `SpecMap Int Int`.
The proof of every rewrite comes from the encoding boundary. -/
private def liftMaps (chosen : Option (Array Syntax)) : TacticM Unit := focusGoal do
  let goal ← getMainGoal
  let selected? ← chosen.mapM fun clause => goal.withContext do
    getFVarIds clause
  let mut observations := selected?.getD #[]
  let maps ← goal.withContext do
    let mut found := integerMaps (← instantiateMVars (← goal.getType))
    for decl in ← getLCtx do
      unless decl.isImplementationDetail || selected?.any (fun selected => !selected.contains decl.fvarId) do
        found := integerMaps (← instantiateMVars decl.type) found
    pure found
  let mut models : Array (Name × Lean.Expr) := #[]
  let mut bridges : Array FVarId := #[]
  let mut rules : Array (TSyntax ``Parser.Tactic.simpLemma) := #[]
  for map in maps do
    let some (carrier, field, leading) := integerMap? map | continue
    let goal ← getMainGoal
    let (name, proof, model, bridge, goal, bounds) ← match ← goal.withContext (existingIntegerModel map) with
    | some (bridge, model) => goal.withContext do
      let name ← if model.isFVar then pure (← model.fvarId!.getDecl).userName
        else pure (← carrier.getDecl).userName
      pure (name, mkFVar bridge, model, bridge, goal, #[])
    | none => do
      let (name, proof, model) ← goal.withContext do
        let name := (← carrier.getDecl).userName
        let table := field.getArg! 1
        let key := table.getArg! 0
        let value := table.getArg! 1
        let fields ← mkArrayLit (mkConst ``RuntimeValue) leading.toList
        let proof ← if value.isAppOfArity ``NTy.int 2 then
          mkAppOptM ``integer_entries #[field.getArg! 0, map.getArg! 0,
            map.getArg! 1, fields, key.getArg! 0, key.getArg! 1, value.getArg! 0,
            value.getArg! 1, field.getArg! 2]
        else do
          let carrier ← mkAppOptM ``NTy.carrier #[field.getArg! 0, value]
          let fallback ← try
            let inhabited ← synthInstance (← mkAppM ``Inhabited #[carrier])
            mkAppOptM ``default #[carrier, inhabited]
          catch _ => do
            let inhabitable ← mkAppM ``NTy.inhabitable #[value]
            unless ← isDefEq inhabitable (mkConst ``Bool.true) do
              throwError "map values need an inhabited specification carrier"
            mkAppOptM ``NTy.inhabitant #[field.getArg! 0, value, ← mkEqRefl inhabitable]
          let inhabited ← mkAppOptM ``nativeInhabited #[field.getArg! 0, value, fallback]
          let unit ← mkAppOptM ``native_unit #[field.getArg! 0, value, fallback]
          mkAppOptM ``native_entries #[field.getArg! 0, map.getArg! 0,
            map.getArg! 1, fields, key.getArg! 0, key.getArg! 1, value, inhabited,
            unit, field.getArg! 2]
        let model := (← inferType proof).getArg! 2 |>.getAppArgs.back!
        return (name, proof, model)
      let relation ← goal.withContext do
        mkEq (← mkAppM ``Maps.entriesOf #[map]) ((← inferType proof).getArg! 2)
      let (bridge, goal) ← (← goal.assert (Name.mkSimple s!"{name}_model") relation proof).intro1P
      let boundProof ← goal.withContext do
        let table := field.getArg! 1
        let key := table.getArg! 0
        let value := table.getArg! 1
        if value.isAppOfArity ``NTy.int 2 then
          mkAppOptM ``integer_size_bounds #[field.getArg! 0, key.getArg! 0, key.getArg! 1,
            value.getArg! 0, value.getArg! 1, field.getArg! 2]
        else
          mkAppOptM ``native_size_bounds #[field.getArg! 0, key.getArg! 0, key.getArg! 1,
            value, field.getArg! 2]
      let (sizeBound, goal) ← (← goal.assert (Name.mkSimple s!"{name}_size_bounds")
        (← goal.withContext (inferType boundProof)) boundProof).intro1P
      let keyProof ← goal.withContext do
        let table := field.getArg! 1
        let key := table.getArg! 0
        let value := table.getArg! 1
        if value.isAppOfArity ``NTy.int 2 then
          mkAppOptM ``integer_keys_fit #[field.getArg! 0, key.getArg! 0, key.getArg! 1,
            value.getArg! 0, value.getArg! 1, field.getArg! 2]
        else
          mkAppOptM ``native_keys_fit #[field.getArg! 0, key.getArg! 0, key.getArg! 1,
            value, field.getArg! 2]
      let (keyBound, goal) ← (← goal.assert (Name.mkSimple s!"{name}_keys")
        (← goal.withContext (inferType keyProof)) keyProof).intro1P
      pure (name, mkFVar bridge, model, bridge, goal, #[sizeBound, keyBound])
    observations := observations ++ bounds
    unless models.any (·.2 == model) do models := models.push (name, model)
    replaceMainGoal [goal]
    bridges := bridges.push bridge
    let bridgeTerm ← goal.withContext (Term.exprToSyntax proof)
    rules := rules.push (← `(Parser.Tactic.simpLemma| ↓$bridgeTerm:term))
    for theoremName in [``integer_size, ``integer_hasKey, ``integer_rank,
        ``integer_valueAt, ``integer_valueAt_write, ``integer_keyAt, ``integer_keyAt_write, ``integer_valid, ``integer_update, ``integer_remove,
        ``integer_nextKey, ``integer_prevKey, ``integer_keysOf] do
      let integerValues := ((field.getArg! 1).getArg! 1).isAppOfArity ``NTy.int 2
      if (theoremName == ``integer_valueAt || theoremName == ``integer_update) && !integerValues then continue
      if theoremName == ``integer_valueAt_write && integerValues then continue
      let rule ← goal.withContext do
        let args := (← inferType proof).getArg! 2 |>.getAppArgs
        let expr ← if theoremName == ``integer_valueAt || theoremName == ``integer_update then
          mkAppM theoremName #[proof]
        else
          mkAppOptM theoremName #[args[1]!, args[3]!, args[5]!, map, model, proof]
        Term.exprToSyntax expr
      -- Observe before traversing the encoding and its structural rank tables.
      -- Keep the bottom-up rule too: a nested key read may need rewriting first.
      rules := rules.push (← `(Parser.Tactic.simpLemma| ↓$rule:term))
      rules := rules.push (← `(Parser.Tactic.simpLemma| $rule:term))
  if models.isEmpty then return
  let facts ← (← getMainGoal).withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail || bridges.contains decl.fvarId ||
          selected?.any (fun selected => !selected.contains decl.fvarId) then return found
      let some observed ← observeMapFact (mkFVar decl.fvarId) | return found
      return found.push (decl.fvarId, decl.userName, observed)
  for (old, name, observed) in facts do
    let goal ← getMainGoal
    let (fresh, goal) ← (← goal.assert name (← goal.withContext (inferType observed)) observed).intro1P
    observations := observations.map fun id => if id == old then fresh else id
    replaceMainGoal [← hideMapBridges goal #[old]]
  replaceMainGoal [← hideMapBridges (← getMainGoal) bridges]
  for _ in [:2] do
    if (← getGoals).isEmpty then return
    let stx ← `(tactic| simp (config := { contextual := true, failIfUnchanged := false }) (disch := (first | assumption | exact SpecMap.rank_nonneg _ _ | verex_arithmetic_only | (simp (config := { failIfUnchanged := false }) only [*, SpecMap.size, List.length_cons, List.length_nil] <;> verex_arithmetic_only) | skip)) only [$rules,*,
      ↓Maps.entriesOf_restamp, ↓Maps.entriesOf_build, integer_contents_eq,
      integer_entries_nil, integer_entries_cons, Maps.asInt_integer,
      RuntimeValue.integer.injEq, exists_const, ↓asInt_eraseGhosts,
      RuntimeValue.order_integer, Int.compare_eq_lt, optionalInteger_none,
      optionalInteger_some, optionalInteger_elim, Option.some.injEq, integer_vector_elements] at *)
    observations ← withMainContext do
      let { ctx, simprocs, dischargeWrapper, .. } ← mkSimpContext stx (eraseLocal := false)
      let guarded ← mkSimpCongrTheorem ``guardedAnd 1000
      let ctx ← Simp.mkContext ctx.config ctx.simpTheorems
        (addSimpCongrTheoremEntry ctx.congrTheorems guarded) ctx.userConfig
      if selected?.isNone then
        discard <| dischargeWrapper.with fun discharge? =>
          simpLocation ctx simprocs discharge? .wildcard
        return observations
      else
        let (result?, _) ← dischargeWrapper.with fun discharge? => do
          simpGoal (← getMainGoal) ctx simprocs discharge? (fvarIdsToSimp := observations)
        match result? with
        | none => replaceMainGoal []; return #[]
        | some (fresh, next) =>
          let observations ← next.withContext do
            return observations.filter (← getLCtx).contains ++ fresh
          replaceMainGoal [next]
          return observations
  if (← getGoals).isEmpty then return
  -- A goal about representation equality still uses its original premise.
  -- Content equality is never used in the reverse direction.
  let goal ← getMainGoal
  for (original, _, _) in facts do
    let establishes ← goal.withContext do
      isDefEq (← inferType (mkFVar original)) (← goal.getType)
    if establishes then
      goal.assign (mkFVar original)
      replaceMainGoal []
      return
  for (name, model) in models do
    if model.isFVar then continue
    if (← getGoals).isEmpty then return
    let goal ← getMainGoal
    let hyps ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail && !bridges.contains decl.fvarId then return found
        let ty ← instantiateMVars decl.type
        return if (ty.find? (· == model)).isSome then found.push decl.fvarId else found
    let (reverted, goal) ← goal.revert hyps (preserveOrder := true)
    let (_, goal) ← goal.generalize #[{ expr := model, xName? := some name }]
    let (introduced, goal) ← goal.introNP reverted.size
    for old in reverted, fresh in introduced do
      if bridges.contains old then bridges := bridges.map fun id => if id == old then fresh else id
    replaceMainGoal [← hideMapBridges goal bridges]

elab "verex_lift_maps" : tactic => liftMaps none

elab "verex_lift_maps" " only" " [" chosen:ident,* "]" : tactic =>
  liftMaps (some (chosen.getElems.map (·.raw)))

/-- The inexpensive authored preparation, ending with typed map observations. -/
macro "verex_prepare" &"values" : tactic => `(tactic| (
  verex_paths
  all_goals verex_canonical_families
  all_goals verex_clear_computations
  all_goals try verex_unname
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals try verex_trivial
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_intro_any
  all_goals verex_type_bounds
  all_goals verex_finish_prepare
  all_goals try verex_trivial))

macro "verex_prepare" &"typed" : tactic => `(tactic| (
  verex_prepare values
  all_goals verex_lift_maps))

end VerexIR.Proofs.Denote

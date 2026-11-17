-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The generated bridges of declarations' own invariants

The bridge generator (`VerexSL.Verify.ensureOwnBridge`) on source modules: a
sorted map at its own parameters and at an instance row, an intrinsic table's
owner at a parameter row and at an instance row, a declaration used from
another namespace, and declarations it builds no bridge for, each recorded
with the operator the construction stopped at. The statements are checked for
the rank the unit's installed clause names, for the form of their right-hand
sides, and for the standard axioms; the form check is shown to reject the
installed clause and a key reader that is not the caller's typed view.
-/

open Lean Elab Command VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote

verex module 0x42::own_bridge_map where
  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >
  spec Map where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  -- A read no guard bounds, at an element whose carrier reads the frame.
  struct Firsts {K has Copy, Drop, Store} has Copy, Drop, Store where
    keys : Vector<K>
    probe : K
  spec Firsts where
    invariant core.prim.compare(keys[0], probe) <= 0

  -- The same read at an element whose carrier does not read the frame.
  struct Counts {K has Copy, Drop, Store} has Copy, Drop, Store where
    counts : Vector<u64>
    key : K
  spec Counts where
    invariant counts[0] <= 7

  -- A specification function without a body over a value of the parameter.
  opaque spec fun admitted {K}(key : K) : Bool
  struct Admitted {K has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
  spec Admitted where
    invariant admitted(key)

  fun size {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>) -> u64 :=
    map.entries.length
  spec size where
    aborts_if false

  fun size_at {T has Copy, Drop, Store}(map : &Map<u64, T>) -> u64 :=
    map.entries.length
  spec size_at where
    aborts_if false

  fun probe {K has Copy, Drop, Store}(firsts : &Firsts<K>, counts : &Counts<K>,
      admitted : &Admitted<K>) -> u64 := 0
  spec probe where
    aborts_if false

verify 0x42::own_bridge_map::size
verify 0x42::own_bridge_map::size_at
verify 0x42::own_bridge_map::probe

verex module 0x42::own_bridge_table where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  struct Cell {T} has Store, Copy, Drop where
    inner : T

  public fun lookup_generic {K has Copy, Drop} {V has Copy, Drop}
      (t : &Table<K, V>, k : K) -> V := *borrow(t, k)
  spec lookup_generic where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k)

  public fun lookup_cell {T has Copy, Drop}(t : &Table<u64, Cell<T> >, k : u64) -> Cell<T> :=
    *borrow(t, k)
  spec lookup_cell where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k)

verify 0x42::own_bridge_table::lookup_generic
verify 0x42::own_bridge_table::lookup_cell

/-- The bridge recorded for a declaration of a unit at a row, by the key a
leaf reads it by: the declaration's handle and the row as the instance's
descriptor states them. -/
private def recordedBridge? (segments : Array String) (unit : VerexIR.Validation.ValidatedUnit)
    (owner : VerexIR.StructHandle) (row : NRow) : CommandElabM (Option OwnBridge.Entry) := do
  let some declared := structNTy unit owner | throwError "no native type"
  let view ← liftTermElabM <| withNominalUnit (unitName segments) do
    nativeTypeView (← quoteNTy (declared.subst row))
  return OwnBridge.bridge? (← getEnv) (unitName segments) (view.getArg! 0) (view.getArg! 1)

private def recordedGap? (segments : Array String) (unit : VerexIR.Validation.ValidatedUnit)
    (owner : VerexIR.StructHandle) (row : NRow) : CommandElabM (Option OwnBridge.Gap) := do
  let some declared := structNTy unit owner | throwError "no native type"
  let view ← liftTermElabM <| withNominalUnit (unitName segments) do
    nativeTypeView (← quoteNTy (declared.subst row))
  return OwnBridge.gap? (← getEnv) (unitName segments) (view.getArg! 0) (view.getArg! 1)

/-- The rank expressions a term orders by. -/
private partial def ranksOf (term : Lean.Expr) (found : Array Lean.Expr := #[]) : Array Lean.Expr :=
  if term.isAppOfArity ``VerexIR.SemanticOperations.valueRanks 1 then
    if found.contains term then found else found.push term
  else match term with
    | .app f a => ranksOf a (ranksOf f found)
    | .lam _ d b _ | .forallE _ d b _ => ranksOf b (ranksOf d found)
    | .mdata _ b | .proj _ _ b => ranksOf b found
    | .letE _ t v b _ => ranksOf b (ranksOf v (ranksOf t found))
    | _ => found

-- The verified functions' values are a sorted map at its own parameters and
-- at `[u64, T]`, a table's owner at its own parameters and at
-- `[u64, Cell<T>]`: each has a bridge, recorded under the key a leaf reads.
run_cmd do
  let standard := [``propext, ``Classical.choice, ``Quot.sound]
  let segments := #["0x42", "own_bridge_map"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_map | throwError "missing unit"
  let map : VerexIR.StructHandle := ⟨⟨0⟩, 1⟩
  let installed ← getConstInfoDefn (dataOwnName segments ++ `case0)
  let installedRanks := ranksOf installed.value
  unless installedRanks.size == 1 do
    throwError "the installed clause names {installedRanks.size} ranks"
  for row in [NRow.cons (.param 0) (.cons (.param 1) .nil),
      NRow.cons (.int 64 false) (.cons (.param 0) .nil)] do
    let some entry ← recordedBridge? segments unit map row
      | throwError "no bridge of the map at {typeArgumentsSpelling row}"
    let statement := (← getConstInfo entry.theoremName).type
    -- The exact rank: the statement orders by the expression the installed
    -- clause names, and binds none.
    unless ranksOf statement == installedRanks do
      throwError "the bridge at {typeArgumentsSpelling row} orders by {ranksOf statement}, the \
        installed clause by {installedRanks}"
    if (statement.find? (·.isConstOf ``Exists)).isSome then
      throwError "the bridge at {typeArgumentsSpelling row} binds a witness"
    unless (← collectAxioms entry.theoremName).all (standard.contains ·) do
      throwError "unexpected axioms in {entry.theoremName}"
  -- At the integer row the order is the structural order at the integer
  -- type, which the leaf's normal form reads as the comparison of values.
  let some integer ← recordedBridge? segments unit map
      (NRow.cons (.int 64 false) (.cons (.param 0) .nil)) | throwError "no bridge"
  let integerStatement := (← getConstInfo integer.theoremName).type
  unless (integerStatement.find? fun part =>
      part.isAppOfArity ``NTy.order 5 && (part.getArg! 2).isAppOfArity ``NTy.int 2).isSome do
    throwError "the bridge at the integer row does not order at the integer type"
  let segments := #["0x42", "own_bridge_table"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_table | throwError "missing unit"
  let table : VerexIR.StructHandle := ⟨⟨0⟩, 0⟩
  let cell : NTy := .struct ⟨⟨0⟩, 1⟩ (.cons (.param 0) .nil) (.cons (.param 0) .nil)
  for (row, integerKey) in [(NRow.cons (.param 0) (.cons (.param 1) .nil), false),
      (NRow.cons (.int 64 false) (.cons cell .nil), true)] do
    let some entry ← recordedBridge? segments unit table row
      | throwError "no bridge of the table at {typeArgumentsSpelling row}"
    let statement := (← getConstInfo entry.theoremName).type
    -- The key reader is the caller's typed view: an integer's value, the
    -- family's read of the type parameter in constructor form.
    let some map := statement.find? (·.isAppOfArity ``VerexIR.SpecMap.ofEntries 7)
      | throwError "the table's bridge states no map"
    let reader := map.getArg! 4
    unless (if integerKey then reader.isAppOfArity ``VerexIR.SpecInt.val 2
        else reader.isAppOfArity ``NTy.read 2 && (reader.getArg! 1).isAppOfArity ``NTy.param 1) do
      throwError "the table's bridge at {typeArgumentsSpelling row} reads keys by {reader}"
    unless (← collectAxioms entry.theoremName).all (standard.contains ·) do
      throwError "unexpected axioms in {entry.theoremName}"

-- The form check rejects the installed clause, which names the induced frame
-- and the base function values, and a caller's reading whose key reader is
-- the identity at a type parameter.
run_cmd do
  let segments := #["0x42", "own_bridge_table"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_table | throwError "missing unit"
  let some entry ← recordedBridge? segments unit ⟨⟨0⟩, 0⟩ (.cons (.param 0) (.cons (.param 1) .nil))
    | throwError "no bridge"
  let statement := (← getConstInfo entry.theoremName).type
  let installed ← getConstInfoDefn (dataOwnName segments ++ `case0)
  liftTermElabM <| Meta.forallTelescope statement fun binders body => do
    let carriers := mkApp3 (mkConst ``Skolems.toCarriers) (mkConst (unitName segments))
      (mkConst (functionCarriersName segments)) binders[0]!
    let reading := body.getArg! 1
    let base := functionCarriersBaseName segments
    OwnBridge.checkOutputForm (fun _ => false) base carriers reading
    let rejected (reading : Lean.Expr) : MetaM Bool := do
      try OwnBridge.checkOutputForm (fun _ => false) base carriers reading; pure false
      catch _ => pure true
    unless ← rejected installed.value do
      throwError "the form check accepts the installed clause"
    let some map := reading.find? (·.isAppOfArity ``VerexIR.SpecMap.ofEntries 7)
      | throwError "no map"
    let key := mkApp2 (mkConst ``NTy.carrier) carriers (mkApp (mkConst ``NTy.param) (mkNatLit 0))
    let identity := Lean.Expr.lam `key key (.bvar 0) .default
    let other := reading.replace fun part =>
      if part == map.getArg! 4 then some identity else none
    unless ← rejected other do
      throwError "the form check accepts the identity key reader at a type parameter"

-- No bridge, and why: an unbounded read at a carrier that reads the frame,
-- and a specification function without a body over a value of the parameter.
-- The unbounded read at an integer element has one: the fallbacks agree.
run_cmd do
  let segments := #["0x42", "own_bridge_map"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_map | throwError "missing unit"
  let row := NRow.cons (.param 0) .nil
  let some firsts ← recordedGap? segments unit ⟨⟨0⟩, 2⟩ row | throwError "Firsts has no recorded gap"
  unless (firsts.reason.splitOn "a read with a fallback whose bound does not follow").length == 2 do
    throwError "Firsts: {firsts.reason}"
  if (← recordedBridge? segments unit ⟨⟨0⟩, 2⟩ row).isSome then throwError "Firsts has a bridge"
  unless (← recordedBridge? segments unit ⟨⟨0⟩, 3⟩ row).isSome do
    throwError "Counts has no bridge: {(← recordedGap? segments unit ⟨⟨0⟩, 3⟩ row).map (·.reason)}"
  let some admitted ← recordedGap? segments unit ⟨⟨0⟩, 4⟩ row
    | throwError "Admitted has no recorded gap"
  unless (admitted.reason.splitOn "opaqueSpec").length ≥ 2 do
    throwError "Admitted: {admitted.reason}"

-- A declaration used from another namespace: its clause orders by the ranks
-- of the namespace that declares it, at the installed side and at the
-- caller's alike.
verex module 0x42::own_bridge_orders where
  struct Ordered {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : K
    second : K
  spec Ordered where
    invariant core.prim.compare(first, second) < 0

verex module 0x42::own_bridge_user where
  struct Holder {K has Copy, Drop, Store} has Copy, Drop, Store where
    pair : 0x42::own_bridge_orders::Ordered<K>

  fun below {K has Copy, Drop, Store}(holder : &Holder<K>) -> Bool :=
    core.prim.compare(&holder.pair.first, &holder.pair.second) < 0
  spec below where
    ensures result
    aborts_if false

verify 0x42::own_bridge_user::below

run_cmd do
  let segments := #["0x42", "own_bridge_user"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_user | throwError "missing unit"
  let some owner := (List.range unit.namespaces.size).findSome? fun index => do
      let ns ← unit.namespaces[index]?
      let structId ← ns.structs.findIdx? fun declaration =>
        ((ns.nameOf declaration.name).map (·.name)) == some "Ordered"
      some (⟨⟨index⟩, structId⟩ : VerexIR.StructHandle)
    | throwError "the unit does not link the declaring namespace"
  if owner.namespaceId.index == 0 then throwError "the declaration is not from another namespace"
  let some entry ← recordedBridge? segments unit owner (.cons (.param 0) .nil)
    | throwError "no bridge of the declaration of the other namespace"
  let statement := (← getConstInfo entry.theoremName).type
  let installed ← getConstInfoDefn (dataOwnName segments ++ `case0)
  let ranks := ranksOf installed.value
  unless ranks.size == 1 && ranksOf statement == ranks do
    throwError "the bridge orders by {ranksOf statement}, the installed clause by {ranks}"

-- The leaf's normal form of a bridged clause is reached in one pass, on the
-- hypothesis path (the forward half of a fact in hand) and on the
-- certification path (the clause the backward half owes): a second pass
-- changes nothing.
example := fun Θ inhabitable value valid =>
  (show True by
    have clause :=
      ((«0x42».own_bridge_map.dataOwn.case0.«at_T0,T1».bridge Θ inhabitable value).mp valid).1
    simp only [vir_denote_norm] at clause
    fail_if_success (simp only [vir_denote_norm] at clause)
    trivial)

example (impossible : False) := fun Θ inhabitable value =>
  («0x42».own_bridge_map.dataOwn.case0.«at_T0,T1».bridge Θ inhabitable value).mpr
    ⟨by
      simp only [vir_denote_norm]
      fail_if_success (simp only [vir_denote_norm])
      exact impossible.elim, impossible.elim⟩

example := fun Θ inhabitable value valid =>
  (show True by
    have clause :=
      ((«0x42».own_bridge_table.dataOwn.case0.«at_T0,T1».bridge Θ inhabitable value).mp valid).1
    simp only [vir_denote_norm] at clause
    fail_if_success (simp only [vir_denote_norm] at clause)
    trivial)

-- An owner whose roles read its keys in order states an ordered map. Keyed
-- by a type parameter the caller reads the keys through the typed view, and
-- its order of the read keys is its order of the keys by the family's
-- coherence law (`NTy.readOrder_read_order`); keyed by an integer the two
-- orders agree by computation.
verex module 0x42::own_bridge_ordered where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_key_at (Table)]
  opaque spec fun key_at {K} {V}(t : Table<K, V>, i : Int) : K

  public fun lookup_generic {K has Copy, Drop} {V has Copy, Drop}
      (t : &Table<K, V>, k : K) -> V := *borrow(t, k)
  spec lookup_generic where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k)

  public fun lookup_integer {V has Copy, Drop}(t : &Table<u64, V>, k : u64) -> V :=
    *borrow(t, k)
  spec lookup_integer where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k)

verify 0x42::own_bridge_ordered::lookup_generic
verify 0x42::own_bridge_ordered::lookup_integer

run_cmd do
  let segments := #["0x42", "own_bridge_ordered"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_ordered | throwError "missing unit"
  for (row, typedView) in [(NRow.cons (.param 0) (.cons (.param 1) .nil), true),
      (NRow.cons (.int 64 false) (.cons (.param 0) .nil), false)] do
    let some entry ← recordedBridge? segments unit ⟨⟨0⟩, 0⟩ row
      | throwError "no bridge of the ordered table at {typeArgumentsSpelling row}: \
          {(← recordedGap? segments unit ⟨⟨0⟩, 0⟩ row).map (·.reason)}"
    let statement := (← getConstInfo entry.theoremName).type
    let some discipline := statement.find? (·.isAppOfArity ``VerexIR.SpecMap.Discipline.ordered 2)
      | throwError "the ordered table's bridge states no ordered map"
    let order := discipline.getArg! 1
    unless order.isAppOfArity ``NTy.readOrder 3 == typedView do
      throwError "the ordered table's bridge at {typeArgumentsSpelling row} orders keys by {order}"
    unless (← collectAxioms entry.theoremName).all
        ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {entry.theoremName}"

/-! The form check reads through the generated definitions a reading applies:
a definition's body is checked at the arguments of its application, so a
helper cannot hide a key reader or a family the check would reject inline. -/

namespace OwnBridgeFormHelpers

open VerexIR VerexIR.Proofs

/-- A helper whose map is keyed by the identity at a type parameter. -/
def hiddenIdentity
    (Θ : @Skolems «0x42».own_bridge_table.unit «0x42».own_bridge_table.functionCarriers)
    (table : @NTy.carrier Θ.toCarriers (.table (.param 0) (.param 1))) : Prop :=
  SpecMap.Valid .sequence
    (SpecMap.ofEntries (fun key : @NTy.carrier Θ.toCarriers (.param 0) => key)
      (fun value => value) table)

/-- A helper whose map is keyed by the family's typed read of the parameter. -/
def typedRead
    (Θ : @Skolems «0x42».own_bridge_table.unit «0x42».own_bridge_table.functionCarriers)
    (table : @NTy.carrier Θ.toCarriers (.table (.param 0) (.param 1))) : Prop :=
  SpecMap.Valid .sequence
    (SpecMap.ofEntries (@NTy.read Θ.toCarriers (.param 0)) (fun value => value) table)

/-- A helper that applies the helper with the identity reader. -/
def nestedIdentity
    (Θ : @Skolems «0x42».own_bridge_table.unit «0x42».own_bridge_table.functionCarriers)
    (table : @NTy.carrier Θ.toCarriers (.table (.param 0) (.param 1))) : Prop :=
  hiddenIdentity Θ table ∧ True

end OwnBridgeFormHelpers

run_cmd liftTermElabM do
  let segments := #["0x42", "own_bridge_table"]
  let frameType := mkApp2 (mkConst ``Skolems) (mkConst (unitName segments))
    (mkConst (functionCarriersName segments))
  let family (frame : Lean.Expr) := mkApp3 (mkConst ``Skolems.toCarriers)
    (mkConst (unitName segments)) (mkConst (functionCarriersName segments)) frame
  let tableType (frame : Lean.Expr) := mkApp2 (mkConst ``NTy.carrier) (family frame)
    (mkApp2 (mkConst ``NTy.table) (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))
      (mkApp (mkConst ``NTy.param) (toExpr (1 : Nat))))
  let helpers := [``OwnBridgeFormHelpers.hiddenIdentity, ``OwnBridgeFormHelpers.typedRead,
    ``OwnBridgeFormHelpers.nestedIdentity]
  let base := functionCarriersBaseName segments
  let accepted (generated : Name → Bool) (carriers reading : Lean.Expr) : MetaM Bool := do
    try OwnBridge.checkOutputForm generated base carriers reading; pure true
    catch _ => pure false
  let through (name : Name) : Bool := helpers.contains name
  Meta.withLocalDeclD `Θ frameType fun frame =>
    Meta.withLocalDeclD `table (tableType frame) fun table =>
    Meta.withLocalDeclD `Θ' frameType fun other =>
    Meta.withLocalDeclD `table' (tableType other) fun otherTable => do
      let applied (name : Name) := mkApp2 (mkConst name) frame table
      -- A valid helper passes, applied and as a function of its own frame.
      unless ← accepted through (family frame) (applied ``OwnBridgeFormHelpers.typedRead) do
        throwError "the form check rejects a helper keyed by the typed read"
      unless ← accepted through (family frame) (mkConst ``OwnBridgeFormHelpers.typedRead) do
        throwError "the form check rejects an unapplied helper keyed by the typed read"
      -- A helper-hidden identity reader at a parameter is rejected, at any
      -- depth, applied or not.
      for name in [``OwnBridgeFormHelpers.hiddenIdentity, ``OwnBridgeFormHelpers.nestedIdentity] do
        if ← accepted through (family frame) (applied name) then
          throwError "the form check accepts the identity key reader hidden in {name}"
        if ← accepted through (family frame) (mkConst name) then
          throwError "the form check accepts the identity key reader hidden in unapplied {name}"
      -- The rejection is of the helper's body: unread, the application passes.
      unless ← accepted (fun _ => false) (family frame)
          (applied ``OwnBridgeFormHelpers.hiddenIdentity) do
        throwError "the application of an unread helper is rejected for another reason"
      -- A helper applied at another frame states another family.
      if ← accepted through (family frame)
          (mkApp2 (mkConst ``OwnBridgeFormHelpers.typedRead) other otherTable) then
        throwError "the form check accepts a helper applied at another frame"

/-- Open the facts of three values of `0x42::own_bridge_map` by their
installed clause, twice each, under the given options: one at a row no bridge
was asked for, one at a row whose recorded clause has none, and one whose
recorded bridge does not apply to the fact as it is spelled. -/
private def openInstalledRoutes (options : Options → Options) : CommandElabM Unit := do
  let segments := #["0x42", "own_bridge_map"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_map | throwError "missing unit"
  liftTermElabM <| withNominalUnit (unitName segments) <| withOptions options do
    let frameType := mkApp2 (mkConst ``Skolems) (mkConst (unitName segments))
      (mkConst (functionCarriersName segments))
    Meta.withLocalDeclD `Θ frameType fun frame => do
      let carriers := mkApp3 (mkConst ``Skolems.toCarriers) (mkConst (unitName segments))
        (mkConst (functionCarriersName segments)) frame
      let some map := structNTy unit ⟨⟨0⟩, 1⟩ | throwError "no native type"
      let some firsts := structNTy unit ⟨⟨0⟩, 2⟩ | throwError "no native type"
      -- `Map<bool, T>` is the type of no value of the verified functions.
      let unasked ← quoteNTy (map.subst (.cons .bool (.cons (.param 0) .nil)))
      let unsupported ← quoteNTy (firsts.subst (.cons (.param 0) .nil))
      -- The bridged instance, spelled by its fields instead of its descriptor.
      let respelled ← nativeTypeView
        (← quoteNTy (map.subst (.cons (.param 0) (.cons (.param 1) .nil))))
      for type in [unasked, unsupported, respelled] do
        Meta.withLocalDeclD `value (mkApp2 (mkConst ``NTy.carrier) carriers type) fun value => do
          for _ in [0:2] do
            if (← ownBridge? carriers type value).isSome then
              throwError "a bridge is used at {type}"

/-- The three kinds a fact read by its installed clause is noted under. -/
private def installedRouteKinds : List String :=
  ["missing enumeration", "recorded unsupported clause", "recorded bridge declined"]

-- A fact read by its installed clause is noted once, with the reason: no
-- bridge was asked for at the row, the row's clause has no bridge, or the
-- recorded bridge does not apply to the fact as it is spelled.
run_cmd do
  let (_, path) ← IO.FS.createTempFile
  installedRouteNotes.set {}
  openInstalledRoutes (·.set `verex.stageLog path.toString)
  let lines ← IO.FS.lines path
  IO.FS.removeFile path
  for kind in installedRouteKinds do
    let count := (lines.filter fun line => (line.splitOn kind).length == 2).size
    unless count == 1 do throwError "`{kind}` is noted {count} times: {lines}"
  unless lines.size == 3 do throwError "{lines.size} notes: {lines}"

-- With no channel enabled an opening notes nothing: no line is held back and
-- the records are those the generator left.  A note the silence passed over
-- is written when a channel is enabled afterwards.
run_cmd do
  installedRouteNotes.set {}
  let (bridges, gaps) := OwnBridge.recorded (← getEnv)
  openInstalledRoutes fun options =>
    (options.set `verex.stageLog "").set `verex.solverDebug false
  let noted := (← installedRouteNotes.get).toList
  unless noted.isEmpty do
    throwError "an opening with no channel enabled noted {noted}"
  let (bridgesAfter, gapsAfter) := OwnBridge.recorded (← getEnv)
  unless bridges.size == bridgesAfter.size && gaps.size == gapsAfter.size do
    throwError "an opening with no channel enabled changed the records: \
      {bridges.size} → {bridgesAfter.size} bridges, {gaps.size} → {gapsAfter.size} gaps"
  let (_, path) ← IO.FS.createTempFile
  openInstalledRoutes (·.set `verex.stageLog path.toString)
  let lines ← IO.FS.lines path
  IO.FS.removeFile path
  for kind in installedRouteKinds do
    let count := (lines.filter fun line => (line.splitOn kind).length == 2).size
    unless count == 1 do throwError "after a silent opening `{kind}` is noted {count} times: {lines}"
  unless lines.size == 3 do throwError "{lines.size} notes after a silent opening: {lines}"


-- The descriptor a bridge is recorded with describes its theorem, and a
-- descriptor that does not is a producer error rather than a decline: each
-- recorded entry passes the registration check, and the same entry with
-- another instance type, another parameter row or another declaration's
-- theorem is rejected by it.
run_cmd liftTermElabM do
  let (bridges, _) := OwnBridge.recorded (← getEnv)
  unless bridges.size > 0 do throwError "no bridge is recorded"
  for entry in bridges do
    if let some wrong ← entry.describesTheorem? then
      throwError "the recorded descriptor of {entry.declaration} does not describe its \
        theorem: {wrong}"
  let entry := bridges[0]!
  let bool := mkConst ``VerexIR.Proofs.Denote.NTy.bool
  let mutations : Array (String × OwnBridge.Entry) := #[
    ("another instance type", { entry with instanceType := bool }),
    ("another parameter row", { entry with
      params := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.cons) bool entry.params }),
    ("a theorem that is not declared", { entry with theoremName := `no.such.bridge })]
  for (what, wrong) in mutations do
    if (← wrong.describesTheorem?).isNone then
      throwError "a descriptor with {what} was accepted"

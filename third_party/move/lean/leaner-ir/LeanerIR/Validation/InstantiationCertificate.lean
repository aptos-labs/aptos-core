-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Capability
import LeanerIR.Validation.IndexedArena
import LeanerIR.Semantics.Operations

/-!
# Instantiation certificates by witness

`invocationTypeInstantiation` locates every instantiated compound type by a
search over the namespace's types. A kernel certificate of its result
replays every search; here the searches become checks: a key map over the
types' fingerprints, decided once per namespace, answers every search by
one lookup, and a per-certificate witness names each type's instantiation,
which a checker verifies against the map with logarithmic reads.
-/

namespace LeanerIR

/-! ## Erasing type-argument locations

The nominal search compares type arguments by their type identifiers and
everything else structurally; erasing the locations of type arguments makes
every search an equality of erased types (`GenericArgument.eraseLoc`). -/

def Ty.eraseLocs : Ty → Ty
  | .nominal name arguments => .nominal name ⟨arguments.toList.map GenericArgument.eraseLoc⟩
  | type => type

theorem Ty.eraseLocs_nominal (name : NameId) (arguments : Array GenericArgument) :
    (Ty.nominal name arguments).eraseLocs = .nominal name (arguments.map GenericArgument.eraseLoc) := by
  simp only [Ty.eraseLocs, Ty.nominal.injEq, true_and]
  apply Array.ext' ; simp

/-! ## Fingerprints

The kernel's cost per evaluation step grows with the body of the
definition it unfolds; `Ty.beq` and `Ty.eraseLocs` are 21-way matchers, so a
comparison of two types costs it half a millisecond, whatever their size.
A search over a table therefore compares *fingerprints* — one natural per
erased type, packed from a prefix-free list of naturals below `2^32` — and
compares types structurally only at the entry it finds. Soundness needs no
injectivity: equal erasures have equal fingerprints by congruence, which is
all the arguments below use. The encoding is nevertheless injective, so a
certificate never fails on a collision: sequences carry their lengths, and
integers are split into 32-bit limbs. -/

/-- The 32-bit limbs of a natural, least significant first, behind their
count. The fuel is the value itself, which bounds the limb count. -/
def limbsFuel : Nat → Nat → List Nat
  | 0, _ => []
  | fuel + 1, value =>
      if value = 0 then [] else value % 4294967296 :: limbsFuel fuel (value / 4294967296)

def limbs (value : Nat) : List Nat :=
  let digits := limbsFuel value value
  digits.length :: digits

/-- Bytes behind their count. -/
def byteList (bytes : List UInt8) : List Nat := bytes.length :: bytes.map UInt8.toNat

mutual
def ConstValue.fingerprint : ConstValue → List Nat
  | .unit => [0]
  | .bool value => [1, if value then 1 else 0]
  | .character value => [2, value]
  | .integer value => 3 :: limbs value.toNat ++ limbs (-value).toNat
  | .address value => 4 :: byteList value.toUTF8.toList
  | .string value => 5 :: byteList value.toUTF8.toList
  | .bytes value => 6 :: byteList value.toList
  | .vector elements => 7 :: ConstValue.fingerprintList elements.toList
  | .tuple elements => 8 :: ConstValue.fingerprintList elements.toList
  | .profile _ => [9]
def ConstValue.fingerprintList : List ConstValue → List Nat
  | [] => [100]
  | value :: rest => 101 :: (ConstValue.fingerprint value ++ ConstValue.fingerprintList rest)
end

def GenericArgument.fingerprint : GenericArgument → List Nat
  | .typeArg value => [0, value.typeId.index, value.loc.index]
  | .const value => 1 :: value.fingerprint
  | .lifetime value => [2, value.index]
  | .evidence value => [3, value.index]

def IntWidth.fingerprint : IntWidth → Nat
  | .bits width => width + 2
  | .pointer => 0
  | .unbounded => 1

/-- A type's fingerprint, as a list: its constructor and the identities its
fields hold. -/
def Ty.fingerprintList : Ty → List Nat
  | .unit => [0]
  | .never => [1]
  | .bool => [2]
  | .character => [3]
  | .string => [4]
  | .bytes => [5]
  | .address => [6]
  | .signer => [7]
  | .integer width signed => [8, width.fingerprint, if signed then 1 else 0]
  | .tuple elements => 9 :: elements.toList.map TypeId.index
  | .vector element length => 10 :: element.index ::
      (match length with | some value => value.fingerprint | none => [])
  | .range => [11]
  | .eventStore => [12]
  | .typeDomain type => [13, type.index]
  | .resourceDomain resource arguments => 14 :: resource.index ::
      (match arguments with | some arguments => arguments.toList.map TypeId.index | none => [])
  | .stateDomain => [15]
  | .nominal name arguments => 16 :: name.index ::
      (arguments.toList.map GenericArgument.fingerprint).flatten
  | .function arguments result abilities => 17 :: result.index :: abilities.size ::
      arguments.toList.map TypeId.index
  | .typeParameter index => [18, index]
  | .reference value => [19, value.referent.index, value.lifetime.index]
  | .profile _ => [20]

/-- A list of naturals packed into one, in 32-bit chunks (bijective base
`2^32` numeration, injective on lists of naturals below `2^32`): one
natural, which the kernel compares in a single step. -/
def packNats (values : List Nat) : Nat :=
  values.foldl (fun packed value => packed * 4294967296 + value % 4294967296 + 1) 0

/-- A type's fingerprint: one natural. -/
def Ty.fingerprint (type : Ty) : Nat := packNats type.fingerprintList

end LeanerIR

namespace LeanerIR.Validation

/-- Equality up to the erasure of type-argument locations, decided. -/
def GenericArgument.sameErased (left right : GenericArgument) : Bool :=
  decide (left.eraseLoc = right.eraseLoc)

theorem sameGenericArgumentValue_eq (left right : GenericArgument) :
    sameGenericArgumentValue left right = GenericArgument.sameErased left right := by
  cases left <;> cases right <;> apply Bool.eq_iff_iff.mpr <;>
    simp [sameGenericArgumentValue, GenericArgument.sameErased, GenericArgument.eraseLoc,
      TypeUse.mk.injEq]

theorem sameGenericArgumentValues_eq (left right : Array GenericArgument) :
    sameGenericArgumentValues left right =
      decide (left.map GenericArgument.eraseLoc = right.map GenericArgument.eraseLoc) := by
  apply Bool.eq_iff_iff.mpr
  simp only [sameGenericArgumentValues, Bool.and_eq_true, beq_iff_eq, Array.all_eq_true,
    decide_eq_true_eq, sameGenericArgumentValue_eq, GenericArgument.sameErased]
  constructor
  · rintro ⟨sizes, same⟩
    apply Array.ext (by simp [sizes])
    intro i h1 _
    simp only [Array.size_map] at h1
    simp only [Array.getElem_map]
    have h := same i (by simp only [Array.size_zip]; omega)
    simpa [Array.getElem_zip] using h
  · intro equal
    have sizes : left.size = right.size := by simpa using congrArg Array.size equal
    refine ⟨sizes, fun i hi => ?_⟩
    simp only [Array.size_zip, sizes, Nat.min_self] at hi
    simp only [Array.getElem_zip]
    have := congrArg (·[i]?) equal
    simp only [Array.getElem?_map] at this
    rw [Array.getElem?_eq_getElem (by omega), Array.getElem?_eq_getElem (by omega)] at this
    simpa using this

/-- The nominal search's predicate is the equality of erased types. -/
theorem nominal_search_eq (name : NameId) (arguments : Array GenericArgument) (candidate : Ty) :
    (match candidate with
      | .nominal candidateName candidateArguments =>
          candidateName == name && sameGenericArgumentValues candidateArguments arguments
      | _ => false) =
      Ty.beq candidate.eraseLocs (Ty.nominal name arguments).eraseLocs := by
  cases candidate <;> apply Bool.eq_iff_iff.mpr <;>
    simp [Ty.eraseLocs, sameGenericArgumentValues_eq, Ty.beq_iff]
  intro _
  rw [← Array.toList_inj]
  simp

/-- A structural search for a type that is not nominal is the equality of
erased types as well: erasure changes only nominal types. -/
theorem structural_search_eq (shape candidate : Ty)
    (notNominal : ∀ name arguments, shape ≠ .nominal name arguments) :
    (candidate == shape) = Ty.beq candidate.eraseLocs shape.eraseLocs := by
  apply Bool.eq_iff_iff.mpr
  cases shape <;> (try exact absurd rfl (notNominal _ _)) <;> cases candidate <;>
    simp [Ty.eraseLocs, Ty.beq_iff]

theorem fingerprint_of_erase_eq {left right : Ty} (equal : left.eraseLocs = right.eraseLocs) :
    left.eraseLocs.fingerprint = right.eraseLocs.fingerprint := by rw [equal]

/-! ## The key map

Once per namespace, the elaborator publishes a search tree from the
fingerprint of every erased type to the least index carrying it. The tree
is only ever read: soundness uses that `lookup` is a function of the key,
never that the tree is ordered, so the certificate carries no invariant. -/

inductive KeyMap where
  | leaf
  | node (left : KeyMap) (key value : Nat) (right : KeyMap)
  deriving Repr, Inhabited

namespace KeyMap

def lookup : KeyMap → Nat → Option Nat
  | .leaf, _ => none
  | .node left key value right, probe =>
      if Nat.blt probe key then left.lookup probe
      else if Nat.blt key probe then right.lookup probe
      else some value

/-- A balanced tree over pairs sorted by key; the elaborator's side. -/
partial def ofSorted (pairs : Array (Nat × Nat)) : KeyMap :=
  go 0 pairs.size
where
  go (low high : Nat) : KeyMap :=
    if low < high then
      let middle := (low + high) / 2
      let (key, value) := pairs[middle]!
      .node (go low middle) key value (go (middle + 1) high)
    else .leaf

end KeyMap

/-- Whether the map answers entry `k`: the index it names for `k`'s
fingerprint lies at or before `k` and holds an equal erased type. -/
def correctAt (types : IndexedArena Ty) (map : KeyMap) (k : Nat) : Bool :=
  match types.get? k with
  | some target =>
      match map.lookup target.eraseLocs.fingerprint with
      | some found => Nat.ble found k &&
          (types.get? found).any fun candidate => Ty.beq candidate.eraseLocs target.eraseLocs
      | none => false
  | none => false

/-- The map answers every entry of a table of `count` types. -/
def Correct (types : IndexedArena Ty) (count : Nat) (map : KeyMap) : Prop :=
  ∀ k (_ : k < count), correctAt types map k = true

instance (types : IndexedArena Ty) (count : Nat) (map : KeyMap) :
    Decidable (Correct types count map) := by
  unfold Correct; infer_instance

/-- What the map answers at an entry is at or before it. -/
theorem lookup_of_correctAt {types : Array Ty} {map : KeyMap} {k : Nat} (bound : k < types.size)
    (at_k : correctAt (.ofArray types) map k = true) :
    ∃ found, map.lookup types[k].eraseLocs.fingerprint = some found ∧ found ≤ k := by
  unfold correctAt at at_k
  rw [IndexedArena.get?_ofArray, Array.getElem?_eq_getElem bound] at at_k
  dsimp only at at_k
  cases read : map.lookup types[k].eraseLocs.fingerprint with
  | none => rw [read] at at_k; exact absurd at_k (by simp)
  | some found =>
      rw [read] at at_k
      simp only [Bool.and_eq_true] at at_k
      exact ⟨found, rfl, Nat.le_of_ble_eq_true at_k.1⟩

/-! ## Witnesses

A certificate names, for every type of the table, its instantiation — the
index the search finds, or that there is none — and the depth of the
instantiation's recursion, which bounds the fuel the native definition
needs. A checker verifies every witness against the map by indexed reads
and one erasure comparison, linear in the table; its soundness carries the
witnesses to the native definition at every sufficient fuel. -/

structure Witness where
  result : Option TypeId
  depth : Nat
  deriving Repr, Inhabited, DecidableEq

/-- The kinds of a namespace's lifetimes, all an instantiation reads of
them, as a literal the kernel reads without deriving it. -/
def lifetimeKinds (ns : ValidatedNamespace) : Array LifetimeKind :=
  ⟨ns.tables.lifetimes.toList.map (·.kind)⟩

/-- `instantiateLifetime?` reading the lifetimes' kinds through an index. -/
def instantiateLifetimeIn? (kinds : IndexedArena LifetimeKind)
    (instantiations : Array GenericArgument) (lifetime : LifetimeId) : Option LifetimeId := do
  match ← kinds.get? lifetime.index with
  | .parameter index => match instantiations[index]? with
      | some (.lifetime value) => some value
      | _ => none
  | .static | .inference | .local => some lifetime

theorem instantiateLifetimeIn?_ofArray (ns : ValidatedNamespace) :
    instantiateLifetimeIn? (.ofArray (lifetimeKinds ns)) = instantiateLifetime? ns := by
  funext instantiations lifetime
  simp only [instantiateLifetimeIn?, instantiateLifetime?, IndexedArena.get?_ofArray,
    lifetimeKinds, List.getElem?_toArray, List.getElem?_map, Array.getElem?_toList]
  cases ns.tables.lifetimes[lifetime.index]? <;> rfl

/-- A component's witnessed instantiation: what the native definition
computes for it at any fuel above its depth. -/
def witnessed (witnesses : IndexedArena Witness) (component : TypeId) : Option TypeId :=
  (witnesses.get? component.index).bind (·.result)

/-- Whether a component has a witness shallower than `depth`. -/
def shallower (witnesses : IndexedArena Witness) (depth : Nat) (component : TypeId) : Bool :=
  (witnesses.get? component.index).any (·.depth < depth)

/-- The components an instantiation of a type recurses into. -/
def components : Ty → Array TypeId
  | .tuple elements => elements
  | .vector element _ => #[element]
  | .typeDomain nested => #[nested]
  | .resourceDomain _ arguments => arguments.getD #[]
  | .nominal _ arguments => arguments.filterMap fun argument => match argument with
      | .typeArg value => some value.typeId
      | _ => none
  | .function arguments result _ => arguments.push result
  | .reference reference => #[reference.referent]
  | _ => #[]

/-- The instantiated shape of a compound type from its components' witnesses:
the native definition's, with the witnessed instantiation for its
recursion; `none` when a component has none. -/
def shapeOf (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) : Ty → Option Ty
  | .tuple elements => do
      let instantiated ← elements.mapM (witnessed witnesses)
      some (.tuple instantiated)
  | .vector element length => do
      let element ← witnessed witnesses element
      some (.vector element length)
  | .typeDomain nested => do
      let nested ← witnessed witnesses nested
      some (.typeDomain nested)
  | .resourceDomain resource arguments => do
      let arguments ← arguments.mapM fun arguments => arguments.mapM (witnessed witnesses)
      some (.resourceDomain resource arguments)
  | .nominal name arguments => do
      let arguments ← arguments.mapM fun argument => match argument with
        | .typeArg value => do
            let typeId ← witnessed witnesses value.typeId
            some (.typeArg { value with typeId })
        | .lifetime value => .lifetime <$> instantiateLifetimeIn? kinds instantiations value
        | .const value => some (.const value)
        | .evidence value => some (.evidence value)
      some (.nominal name arguments)
  | .function arguments result abilities => do
      let arguments ← arguments.mapM (witnessed witnesses)
      let result ← witnessed witnesses result
      some (.function arguments result abilities)
  | .reference reference => do
      let referent ← witnessed witnesses reference.referent
      let lifetime ← instantiateLifetimeIn? kinds instantiations reference.lifetime
      some (.reference { reference with referent, lifetime })
  | _ => none

/-- Whether a type's instantiation searches the table for a shape. -/
def searches : Ty → Bool
  | .tuple _ | .vector .. | .typeDomain _ | .resourceDomain .. | .nominal .. | .function .. |
    .reference _ => true
  | _ => false

/-- Whether the witness of a compound type is what the search for `shape`
finds: the entry the map names for the shape's fingerprint, equal to the
shape up to erasure; or none, when the map has no entry for it. -/
def foundShape (types : IndexedArena Ty) (map : KeyMap) (result : Option TypeId) (shape : Ty) :
    Bool :=
  match map.lookup shape.eraseLocs.fingerprint, result with
  | some found, some ⟨j⟩ => Nat.beq found j &&
      (types.get? j).any fun candidate => Ty.beq candidate.eraseLocs shape.eraseLocs
  | none, none => true
  | _, _ => false

/-- Check one type's witness. -/
def checkAt (kinds : IndexedArena LifetimeKind) (types : IndexedArena Ty) (map : KeyMap)
    (instantiations : Array GenericArgument) (witnesses : IndexedArena Witness) (i : Nat) : Bool :=
  match witnesses.get? i, types.get? i with
  | none, _ => false
  | some witness, none => witness.result == none
  | some witness, some type =>
    if searches type then
      (components type).all (shallower witnesses witness.depth) &&
        match shapeOf kinds instantiations witnesses type with
        | some shape => foundShape types map witness.result shape
        | none => witness.result == none
    else match type with
      | .typeParameter index => witness.result == (match instantiations[index]? with
          | some (.typeArg value) => some value.typeId
          | _ => none)
      | _ => witness.result == some ⟨i⟩

/-- Check the witnesses of `count` types. -/
def checkAll (kinds : IndexedArena LifetimeKind) (types : IndexedArena Ty) (map : KeyMap)
    (instantiations : Array GenericArgument) (witnesses : IndexedArena Witness) (count : Nat) :
    Bool :=
  (List.range count).all (checkAt kinds types map instantiations witnesses)

/-! ## Soundness -/

/-- A monadic map over `Option` depends only on the function at the list's
members. -/
theorem List.mapM_congr_mem {α β : Type} {f g : α → Option β} :
    ∀ (xs : List α), (∀ x ∈ xs, f x = g x) → xs.mapM f = xs.mapM g
  | [], _ => rfl
  | x :: rest, agree => by
      simp only [List.mapM_cons]
      rw [agree x (by simp), List.mapM_congr_mem rest (fun y mem => agree y (by simp [mem]))]

/-- The same over arrays. -/
theorem mapM_congr_mem {α β : Type} {f g : α → Option β} (xs : Array α)
    (agree : ∀ x ∈ xs, f x = g x) : xs.mapM f = xs.mapM g := by
  rw [Array.mapM_eq_mapM_toList, Array.mapM_eq_mapM_toList,
    List.mapM_congr_mem xs.toList (fun x mem => agree x (Array.mem_toList_iff.mp mem))]

/-- An entry equal to a shape up to erasure is answered by the map's entry
for the shape's fingerprint. -/
theorem lookup_of_match {types : Array Ty} {map : KeyMap} (correct : Correct (.ofArray types) types.size map)
    (shape : Ty) (i : Nat) (bound : i < types.size)
    (same : Ty.beq types[i].eraseLocs shape.eraseLocs = true) :
    ∃ found, map.lookup shape.eraseLocs.fingerprint = some found ∧ found ≤ i := by
  obtain ⟨found, read, le⟩ := lookup_of_correctAt bound (correct i bound)
  exact ⟨found, by rw [← fingerprint_of_erase_eq ((Ty.beq_iff _ _).mp same)]; exact read, le⟩

/-- What the checker's search finds is the native definition's search. -/
theorem search_eq {types : Array Ty} {map : KeyMap}
    (correct : Correct (.ofArray types) types.size map)
    (shape : Ty) (result : Option TypeId)
    (found : foundShape (.ofArray types) map result shape = true)
    (predicate : Ty → Bool)
    (agrees : ∀ candidate, predicate candidate = Ty.beq candidate.eraseLocs shape.eraseLocs) :
    (types.findIdx? predicate).map (fun index => (⟨index⟩ : TypeId)) = result := by
  have predicateEq : predicate = fun candidate => Ty.beq candidate.eraseLocs shape.eraseLocs :=
    funext agrees
  rw [predicateEq, ← findIdx?_toList]
  unfold foundShape at found
  cases lookupRead : map.lookup shape.eraseLocs.fingerprint with
  | some answer =>
      rw [lookupRead] at found
      cases result with
      | none => simp at found
      | some jId =>
          obtain ⟨j⟩ := jId
          simp only [IndexedArena.get?_ofArray, Bool.and_eq_true, Option.any_eq_true] at found
          obtain ⟨answerEq, foundType, read, erased⟩ := found
          have answerEq : answer = j := Nat.eq_of_beq_eq_true answerEq
          subst answerEq
          have bound : answer < types.size := (Array.getElem?_eq_some_iff.mp read).1
          rw [Array.getElem?_eq_getElem bound, Option.some.injEq] at read
          subst read
          have search : types.toList.findIdx?
              (fun candidate => Ty.beq candidate.eraseLocs shape.eraseLocs) = some answer := by
            rw [List.findIdx?_eq_some_iff_getElem]
            refine ⟨by simpa using bound, by simpa using erased, ?_⟩
            intro i lt
            simp only [Array.getElem_toList, Bool.not_eq_true]
            cases h : Ty.beq types[i].eraseLocs shape.eraseLocs
            · rfl
            · obtain ⟨found', read', le⟩ := lookup_of_match correct shape i (by omega) h
              rw [lookupRead, Option.some.injEq] at read'
              omega
          rw [search]; rfl
  | none =>
      rw [lookupRead] at found
      cases result with
      | some _ => simp at found
      | none =>
          have none : types.toList.findIdx?
              (fun candidate => Ty.beq candidate.eraseLocs shape.eraseLocs) = none := by
            rw [List.findIdx?_eq_none_iff]
            intro candidate member
            obtain ⟨i, bound, read⟩ := List.mem_iff_getElem.mp member
            simp only [Array.getElem_toList] at read
            subst read
            simp only [Array.length_toList] at bound
            cases h : Ty.beq types[i].eraseLocs shape.eraseLocs
            · rfl
            · obtain ⟨_, read', _⟩ := lookup_of_match correct shape i bound h
              rw [lookupRead] at read'
              exact absurd read' (by simp)
          rw [none]; rfl

/-- Every component of a checked compound type has a shallower witness, so
the native definition agrees with the witnessed instantiation at it. -/
theorem shallower_of_all {witnesses : Array Witness} {tree : IndexedArena Witness}
    (treeEq : IndexedArena.ofArray witnesses = tree) {depth : Nat} {cs : Array TypeId}
    (all : cs.all (shallower tree depth) = true) (c : TypeId) (member : c ∈ cs) :
    ∃ h : c.index < witnesses.size, witnesses[c.index].depth < depth := by
  have := (Array.all_eq_true'.mp all) c member
  simp only [shallower, ← treeEq, IndexedArena.get?_ofArray, Option.any_eq_true] at this
  obtain ⟨witness, read, lt⟩ := this
  obtain ⟨bound, eq⟩ := Array.getElem?_eq_some_iff.mp read
  exact ⟨bound, by rw [eq]; exact of_decide_eq_true lt⟩

theorem witnessed_eq {witnesses : Array Witness} {tree : IndexedArena Witness}
    (treeEq : IndexedArena.ofArray witnesses = tree) (c : TypeId) (h : c.index < witnesses.size) :
    witnessed tree c = witnesses[c.index].result := by
  simp [witnessed, ← treeEq, IndexedArena.get?_ofArray, Array.getElem?_eq_getElem h]

theorem tuple_not_nominal (elements : Array TypeId) :
    ∀ name arguments, Ty.tuple elements ≠ .nominal name arguments := by intros; simp
theorem vector_not_nominal (element : TypeId) (length : Option ConstValue) :
    ∀ name arguments, Ty.vector element length ≠ .nominal name arguments := by intros; simp
theorem typeDomain_not_nominal (nested : TypeId) :
    ∀ name arguments, Ty.typeDomain nested ≠ .nominal name arguments := by intros; simp
theorem resourceDomain_not_nominal (resource : NameId) (arguments' : Option (Array TypeId)) :
    ∀ name arguments, Ty.resourceDomain resource arguments' ≠ .nominal name arguments := by
  intros; simp
theorem function_not_nominal (arguments' : Array TypeId) (result : TypeId) (abilities : Array Ability) :
    ∀ name arguments, Ty.function arguments' result abilities ≠ .nominal name arguments := by
  intros; simp
theorem reference_not_nominal (reference : ReferenceType) :
    ∀ name arguments, Ty.reference reference ≠ .nominal name arguments := by intros; simp

/-- The checker's verdict at a searching type: the native definition finds
what the witness says, given the components agree. `m` is the components'
instantiation, `build` the shape it is searched as. -/
theorem search_sound {ns : ValidatedNamespace} {map : KeyMap}
    (correct : Correct (.ofArray ns.tables.types) ns.tables.types.size map) {α : Type}
    (m : Option α) (build : α → Ty) (result : Option TypeId)
    (checked : (match m.map build with
      | some shape => foundShape (.ofArray ns.tables.types) map result shape
      | none => result == none) = true)
    (predicate : α → Ty → Bool)
    (agrees : ∀ x candidate, predicate x candidate =
      Ty.beq candidate.eraseLocs (build x).eraseLocs) :
    (m.bind fun x => (ns.tables.types.findIdx? (predicate x)).bind
      fun index => some (⟨index⟩ : TypeId)) = result := by
  cases m with
  | none =>
      simp only [Option.bind_none, Option.map_none] at checked ⊢
      simp only [beq_iff_eq] at checked; exact checked.symm
  | some x =>
      simp only [Option.bind_some, Option.map_some] at checked ⊢
      have := search_eq correct (build x) result checked (predicate x) (agrees x)
      rw [← this, Option.map_eq_bind]
      rfl

/-- The shape a checked compound type is searched as, by case. -/
theorem shapeOf_tuple (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (elements : Array TypeId) :
    shapeOf kinds instantiations witnesses (.tuple elements) =
      (elements.mapM (witnessed witnesses)).map .tuple := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind]; rfl

theorem shapeOf_vector (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (element : TypeId) (length : Option ConstValue) :
    shapeOf kinds instantiations witnesses (.vector element length) =
      (witnessed witnesses element).map fun element => .vector element length := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind]; rfl

theorem shapeOf_typeDomain (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (nested : TypeId) :
    shapeOf kinds instantiations witnesses (.typeDomain nested) =
      (witnessed witnesses nested).map .typeDomain := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind]; rfl

theorem shapeOf_resourceDomain (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (resource : NameId) (arguments : Option (Array TypeId)) :
    shapeOf kinds instantiations witnesses (.resourceDomain resource arguments) =
      (arguments.mapM fun (arguments : Array TypeId) => arguments.mapM (witnessed witnesses)).map
        fun arguments => .resourceDomain resource arguments := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind]; rfl

/-- The instantiation of a nominal type's arguments from the witnesses. -/
def nominalArguments (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (argument : GenericArgument) : Option GenericArgument :=
  match argument with
  | .typeArg value => (witnessed witnesses value.typeId).bind fun typeId =>
      some (GenericArgument.typeArg { value with typeId })
  | .lifetime value => GenericArgument.lifetime <$> instantiateLifetimeIn? kinds instantiations value
  | .const value => some (.const value)
  | .evidence value => some (.evidence value)

theorem shapeOf_nominal (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (name : NameId) (arguments : Array GenericArgument) :
    shapeOf kinds instantiations witnesses (.nominal name arguments) =
      (arguments.mapM (nominalArguments kinds instantiations witnesses)).map (.nominal name) := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind]; rfl

theorem shapeOf_function (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (arguments : Array TypeId) (result : TypeId)
    (abilities : Array Ability) :
    shapeOf kinds instantiations witnesses (.function arguments result abilities) =
      ((arguments.mapM (witnessed witnesses)).bind fun arguments =>
        (witnessed witnesses result).map fun result => (arguments, result)).map
        fun pair => .function pair.1 pair.2 abilities := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind, Option.bind_assoc]; rfl

theorem shapeOf_reference (kinds : IndexedArena LifetimeKind) (instantiations : Array GenericArgument)
    (witnesses : IndexedArena Witness) (reference : ReferenceType) :
    shapeOf kinds instantiations witnesses (.reference reference) =
      ((witnessed witnesses reference.referent).bind fun referent =>
        (instantiateLifetimeIn? kinds instantiations reference.lifetime).map
          fun lifetime => (referent, lifetime)).map
        fun pair => .reference { reference with referent := pair.1, lifetime := pair.2 } := by
  simp only [shapeOf, Option.map_eq_bind, Option.bind_eq_bind, Option.bind_assoc]; rfl

/-- Two binds in sequence are one bind over the pair. -/
theorem bind_bind_pair {α β γ : Type} (m₁ : Option α) (m₂ : Option β) (k : α → β → Option γ) :
    (m₁.bind fun a => m₂.bind fun b => k a b) =
      (m₁.bind fun a => m₂.map fun b => (a, b)).bind fun pair => k pair.1 pair.2 := by
  cases m₁ <;> cases m₂ <;> simp

theorem instantiate_eq_of_check {ns : ValidatedNamespace} {map : KeyMap}
    {instantiations : Array GenericArgument} {witnesses : Array Witness}
    {tree : IndexedArena Witness} (treeEq : IndexedArena.ofArray witnesses = tree)
    (correct : Correct (.ofArray ns.tables.types) ns.tables.types.size map)
    (checked : ∀ k, k < witnesses.size →
      checkAt (.ofArray (lifetimeKinds ns)) (.ofArray ns.tables.types) map instantiations tree k = true) :
    ∀ fuel i (hi : i < witnesses.size), witnesses[i].depth < fuel →
      instantiatePlaceFieldTypeFuel? ns instantiations fuel ⟨i⟩ = witnesses[i].result := by
  intro fuel
  induction fuel with
  | zero => intro i _ h; exact absurd h (Nat.not_lt_zero _)
  | succ fuel ih =>
    intro i hi depth
    have check := checked i hi
    unfold checkAt at check
    rw [show tree.get? i = witnesses[i]? from by rw [← treeEq, IndexedArena.get?_ofArray],
      Array.getElem?_eq_getElem hi, IndexedArena.get?_ofArray] at check
    -- The components' witnesses stand for their native instantiations.
    have agree {cs : Array TypeId} (all : cs.all (shallower tree witnesses[i].depth) = true) :
        ∀ c ∈ cs, instantiatePlaceFieldTypeFuel? ns instantiations fuel c = witnessed tree c := by
      intro c member
      obtain ⟨bound, lt⟩ := shallower_of_all treeEq all c member
      rw [witnessed_eq treeEq c bound]
      exact ih c.index bound (Nat.lt_of_lt_of_le lt (Nat.le_of_lt_succ depth))
    simp only [instantiatePlaceFieldTypeFuel?]
    cases read : ns.tables.types[i]? with
    | none =>
        simp only [read, Option.bind_eq_bind, Option.bind_none] at check ⊢
        simp only [beq_iff_eq] at check; exact check.symm
    | some type =>
        simp only [read, Option.bind_eq_bind, Option.bind_some] at check ⊢
        -- A type without a search: its witness is the native value itself.
        cases type <;> dsimp only at check ⊢ <;> first
          | (simp only [searches, Bool.false_eq_true, ↓reduceIte, beq_iff_eq] at check
             exact check.symm)
          | skip
        case tuple elements =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [mapM_congr_mem elements (agree all), shapeOf_tuple] at *
          exact search_sound correct _ _ _ found (fun x c => c == Ty.tuple x)
            (fun x c => structural_search_eq _ _ (tuple_not_nominal x))
        case vector element length =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [agree all element (by simp [components]), shapeOf_vector] at *
          exact search_sound correct _ _ _ found (fun x c => c == Ty.vector x length)
            (fun x c => structural_search_eq _ _ (vector_not_nominal x length))
        case typeDomain nested =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [agree all nested (by simp [components]), shapeOf_typeDomain] at *
          exact search_sound correct _ _ _ found (fun x c => c == Ty.typeDomain x)
            (fun x c => structural_search_eq _ _ (typeDomain_not_nominal x))
        case resourceDomain resource arguments =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [shapeOf_resourceDomain] at found
          have congruent : arguments.mapM (fun arguments =>
              arguments.mapM (instantiatePlaceFieldTypeFuel? ns instantiations fuel)) =
              arguments.mapM fun arguments => arguments.mapM (witnessed tree) := by
            cases arguments with
            | none => rfl
            | some arguments =>
                simp only [components, Option.getD_some] at all
                simp only [Option.mapM_some, mapM_congr_mem arguments (agree all)]
          rw [congruent]
          exact search_sound correct _ _ _ found (fun x c => c == Ty.resourceDomain resource x)
            (fun x c => structural_search_eq _ _ (resourceDomain_not_nominal resource x))
        case nominal name arguments =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [shapeOf_nominal] at found
          rw [mapM_congr_mem (g := nominalArguments (.ofArray (lifetimeKinds ns)) instantiations tree) arguments ?_]
          · exact search_sound correct _ _ _ found
              (fun x c => match c with
                | .nominal candidateName candidateArguments =>
                    candidateName == name && sameGenericArgumentValues candidateArguments x
                | _ => false)
              (fun x c => nominal_search_eq name x c)
          · intro argument member
            cases argument with
            | typeArg value =>
                dsimp only [nominalArguments]
                rw [agree all value.typeId (by
                  simp only [components, Array.mem_filterMap]
                  exact ⟨.typeArg value, member, rfl⟩)]
            | lifetime value =>
                simp only [nominalArguments, instantiateLifetimeIn?_ofArray]
            | const value => rfl
            | evidence value => rfl
        case function arguments result abilities =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [shapeOf_function] at found
          rw [mapM_congr_mem arguments (fun c member => agree all c (by
              simp only [components]; exact Array.mem_push_of_mem result member)),
            agree all result (by simp only [components]; exact Array.mem_push_self),
            bind_bind_pair]
          exact search_sound correct _ _ _ found
            (fun x c => c == Ty.function x.1 x.2 abilities)
            (fun x c => structural_search_eq _ _ (function_not_nominal x.1 x.2 abilities))
        case reference reference =>
          simp only [searches, ↓reduceIte, Bool.and_eq_true] at check
          obtain ⟨all, found⟩ := check
          rw [shapeOf_reference, instantiateLifetimeIn?_ofArray] at found
          rw [agree all reference.referent (by simp [components]), bind_bind_pair]
          exact search_sound correct _ _ _ found
            (fun x c => c == Ty.reference { reference with referent := x.1, lifetime := x.2 })
            (fun x c => structural_search_eq _ _ (reference_not_nominal _))

/-! ## The instantiation from the witnesses -/

/-- The pairs the fold of `invocationTypeInstantiation` collects, read off
the witnesses. -/
def foldWitnesses (witnesses : IndexedArena Witness) (count : Nat) : Array (TypeId × TypeId) :=
  (List.range count).foldl (init := #[]) fun result index =>
    match (witnesses.get? index).bind (·.result) with
    | some concrete =>
        if concrete == (⟨index⟩ : TypeId) then result else result.push (⟨index⟩, concrete)
    | none => result

theorem foldl_congr_mem {α β : Type} {f g : β → α → β} (xs : List α)
    (agree : ∀ x ∈ xs, ∀ acc, f acc x = g acc x) : ∀ init, xs.foldl f init = xs.foldl g init := by
  induction xs with
  | nil => intro _; rfl
  | cons x rest ih =>
      intro init
      simp only [List.foldl_cons]
      rw [agree x (by simp), ih (fun y mem acc => agree y (by simp [mem]) acc)]

/-- The runtime's instantiation, from a checked certificate: the map over
the table, and a witness per type of a depth the table's size bounds. -/
theorem invocationTypeInstantiation_eq_of_check {ns : ValidatedNamespace} {map : KeyMap}
    {witnesses : Array Witness} {tree : IndexedArena Witness} (outer : Array (TypeId × TypeId))
    (arguments : Array GenericArgument) (treeEq : IndexedArena.ofArray witnesses = tree)
    (correct : Correct (.ofArray ns.tables.types) ns.tables.types.size map)
    (checked : checkAll (.ofArray (lifetimeKinds ns)) (.ofArray ns.tables.types) map
      (SemanticOperations.instantiateGenericArguments outer arguments) tree witnesses.size = true)
    (sizes : witnesses.size = ns.tables.types.size)
    (depths : witnesses.toList.all (fun witness => decide (witness.depth ≤ ns.tables.types.size)) =
      true) :
    SemanticOperations.invocationTypeInstantiation ns outer arguments =
      foldWitnesses tree ns.tables.types.size := by
  unfold SemanticOperations.invocationTypeInstantiation foldWitnesses
  simp only [← Array.foldl_toList, Array.toList_range]
  have each : ∀ k, k < witnesses.size →
      checkAt (.ofArray (lifetimeKinds ns)) (.ofArray ns.tables.types) map
        (SemanticOperations.instantiateGenericArguments outer arguments) tree k = true := by
    intro k hk
    unfold checkAll at checked
    exact (List.all_eq_true.mp checked) k (List.mem_range.mpr hk)
  apply foldl_congr_mem
  intro index member acc
  have bound : index < witnesses.size := by rw [sizes]; exact List.mem_range.mp member
  have depth : witnesses[index].depth ≤ ns.tables.types.size :=
    of_decide_eq_true ((List.all_eq_true.mp depths) _ (Array.getElem_mem_toList bound))
  have := instantiate_eq_of_check treeEq correct each (ns.tables.types.size + 1) index bound
    (Nat.lt_succ_of_le depth)
  simp only [instantiatePlaceFieldType?, this, ← treeEq, IndexedArena.get?_ofArray,
    Array.getElem?_eq_getElem bound, Option.bind_some]
  rfl

/-! ## Computing the certificate

The elaborator computes the map and the witnesses natively; the kernel
checks them. -/

/-- The key map of a types table: every fingerprint to the least index
carrying it. -/
def computeKeyMap (types : Array Ty) : KeyMap :=
  let pairs := types.toList.zipIdx.foldl (init := (∅ : Std.HashMap Nat Nat)) fun pairs (type, index) =>
    let key := type.eraseLocs.fingerprint
    if pairs.contains key then pairs else pairs.insert key index
  KeyMap.ofSorted (pairs.toArray.qsort fun l r => l.1 < r.1)

/-- The recursion depth of a type's instantiation: one more than its deepest
component's, none for a leaf. Types are acyclic, so the search terminates
within the table's size. -/
partial def depthOf (types : Array Ty) (typeId : TypeId) (fuel : Nat) : Nat :=
  match fuel, types[typeId.index]? with
  | 0, _ => 0
  | _, none => 0
  | fuel + 1, some type =>
      let cs := components type
      if cs.isEmpty then 0 else
        cs.foldl (init := 0) (fun best c => max best (depthOf types c fuel + 1))

/-- The witnesses of one instantiation, from the native definition. -/
def computeWitnesses (ns : ValidatedNamespace) (instantiations : Array GenericArgument) :
    Array Witness :=
  let types := ns.tables.types
  (Array.range types.size).map fun index =>
    { result := instantiatePlaceFieldType? ns instantiations ⟨index⟩
      depth := depthOf types ⟨index⟩ (types.size + 1) }

end LeanerIR.Validation

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Spec.Semantics.Integers

/-!
# Runtime types and values

TRUSTED. Every value carries its intrinsic runtime type, so that a type
mismatch is observable (plan §5.2).

Status: plan Phase 4, stage B. Booleans, integers of every width, addresses,
signers, vectors, struct and enum types (Phase 2a), references to them (Phase
3): a reference is a location plus a mutability flag (plan §5.2,
"References"), instances of generic struct and enum types (Phase 4, stage A):
a struct type is a definition's name with closed type arguments, and function
types and closures (stage B), with the subsumption rule for function types
(`RuntimeType.assignable`). Natives with references (2026-11-03): a location
may designate the address inside a signer (`PathStep.signerAddress`). Table
natives (2026-11-04): a table holds its entries (`Value.table`), and a location
may designate one of them (`PathStep.tableEntry`).

**Tables** (language-owner rulings, 2026-11-04). "A table's entries are inside
the table": a value of `aptos_std::table::Table<K, V>` (`StructIdOf.isTable`)
holds its declared field, `handle`, and also its entries, a finite map from
keys, values of `K`, to entries, values of `Box<V>` (`tableEntryType`). Its
entries are part of the value, for ownership, for the abilities the value has,
and for locations; a reference to an entry designates a place inside the
table. `table.move` represents the items "by separate global state items",
tracked by the handle on the table struct; modeling them inside the value
agrees with that as long as no two tables share a handle, which the table
rules on the module ensure (`CompiledModule.followsTableRules`). The keys
select entries, but the spec does not say which entry a key selects; they are
part of the value for equality only (`Value.table`).

**Function values** (book, `functions.md`, "Function Values"). A function type
`|t1, .., tn| (r1, .., rk) has a` is a type like any other: "function values
can be stored in fields of structs or enums", and its abilities are the ones
it carries. A closure is a value: a function instance, a mask saying which of
its parameters are captured, the captured values, and the closure's type, the
remaining parameters, the results and the abilities, which the closure
records as a vector records its element type (`Value.closure`). "A function
value with more abilities may be used where a function type with fewer
abilities is expected" (plan §5.2), so a value's intrinsic type and the type of
the slot holding it may differ, in the abilities of a function type only.

A struct type is a name and type arguments; what it means (its fields, its
variants, its abilities) is declared in the program. The translation of
signature tokens into runtime types, abilities and conformance therefore live
in `Spec.Semantics.Program`. This file holds what needs no program: the types,
locations, the values, their intrinsic types, paths into values, and the
decoding of constants. Which locations exist and are valid depends on the
state (`Spec.Semantics.State`).

**Type parameters never appear at run time.** Every runtime type is closed: a
generic function runs as an instance, its frame holding closed type arguments
(`FunctionRef.typeArgs`), and a signature token is translated under them
(`RuntimeType.ofTokenWith`). "Generics can be used to define functions and
structs over different input data types", and type arguments are supplied
"when calling a generic function" and "when constructing or destructing
values of generic types" (book, `generics-and-abilities.md`, "Type
Arguments"), so by the time an instruction runs, every type it names is
closed.
-/

namespace MoveBytecode

/-- A module id: the address of the account that holds the module and the
module's name (`file_format.rs`, `ModuleHandle`). -/
structure ModuleId where
  address : Address
  name : String
  deriving DecidableEq, Repr, Inhabited

/-- The address of the standard library, `std` and `aptos_std`. -/
def stdAddress : Address := 0x1

/-- The module `aptos_std::table` (`table.move`). -/
def tableModuleId : ModuleId := ⟨stdAddress, "table"⟩

/-- A struct or enum type, generic in the type of its type arguments so that
runtime types can nest inside it; `StructId` is the instance at runtime types,
and the type arguments are runtime types (below). -/
structure StructIdOf (α : Type) where
  module : ModuleId
  name : String
  typeArgs : List α := []
  deriving DecidableEq, Repr, Inhabited

/-- A closed runtime type of the supported subset. `struct` covers enums too:
in the format both are struct definitions, the enum's with variants. -/
inductive RuntimeType where
  | bool
  | int (t : IntType)
  | address
  | signer
  | vector (elem : RuntimeType)
  | struct (s : StructIdOf RuntimeType)
  /-- A function type `|params| results has abilities` ("a function type is
  denoted, for example, as `|u64|bool`", "Function types can have associated
  abilities, written as `|u64|bool has copy`", book, `functions.md`,
  "Function Types"). Its parameters and results are slot types: they may be
  references ("the actual parameters of lambdas can be references"). A
  function type is a value type: a function value holds no reference. -/
  | function (params results : List RuntimeType) (abilities : AbilitySet)
  /-- `&T` (`mutable = false`) or `&mut T` (`mutable = true`), to a value type
  `T`: "references to references are not allowed", and references "cannot be
  stored as a field value of structs" (book, `references.md`); nor are they
  vector elements (`vector.md`: `vector[]: vector<T>` "where `T` is any single,
  non-reference type"). Reference types occur only as the types of locals,
  parameters, return values and operand-stack slots
  (`RuntimeType.ofSlotToken`). -/
  | ref (mutable : Bool) (target : RuntimeType)
  deriving Repr, Inhabited

/-- A struct or enum type: the definition's name, by its defining module and
its name, and its closed type arguments. "A type in a module is uniquely
identified by its name and as such the name is enough to perform resolution"
(`file_format.rs`, `StructHandle`), so `module` and `name` find the
definition (`Program.resolveStruct`, which ignores `typeArgs`). The type
arguments make the instance: `Coin<A>` and `Coin<B>` are different types, with
different abilities ("the abilities a type will have depend on both its
declaration **and** its type arguments", book, `generics-and-abilities.md`,
"Conditional Abilities and Generic Types"), stored separately in global
storage ("storage polymorphism", book, `global-storage.md`, "Global storage
operators with generics"). A non-generic type has no type arguments. -/
abbrev StructId := StructIdOf RuntimeType

/-- `s` is a table type: `aptos_std::table::Table` at some instance ("Type of
tables", `table.move`). Its values are tables (`Value.table`). -/
def StructIdOf.isTable {α : Type} (s : StructIdOf α) : Bool :=
  s.module == tableModuleId && s.name == "Table"

/-- The type of a table's entries at value type `v`: `Box<v>`, the "wrapper for
values" of `table.move`, a struct of `aptos_std::table`. A `Table<K, V>` maps
keys of type `K` to entries of type `Box<V>` (language-owner ruling 1,
2026-11-04). -/
def tableEntryType (v : RuntimeType) : RuntimeType :=
  .struct { module := tableModuleId, name := "Box", typeArgs := [v] }

/-! Structural equality of runtime types. As for `SignatureToken`,
`DecidableEq` cannot be derived for a nested inductive, so equality is defined
by structural recursion and shown lawful. -/

mutual
def RuntimeType.beq : RuntimeType → RuntimeType → Bool
  | .bool, .bool | .address, .address | .signer, .signer => true
  | .int t, .int t' => decide (t = t')
  | .vector a, .vector b => RuntimeType.beq a b
  | .struct ⟨m, n, as⟩, .struct ⟨m', n', as'⟩ =>
    decide (m = m') && decide (n = n') && RuntimeType.beqList as as'
  | .function ps rs a, .function ps' rs' a' =>
    RuntimeType.beqList ps ps' && RuntimeType.beqList rs rs' && decide (a = a')
  | .ref m a, .ref m' b => decide (m = m') && RuntimeType.beq a b
  | _, _ => false

def RuntimeType.beqList : List RuntimeType → List RuntimeType → Bool
  | [], [] => true
  | a :: as, b :: bs => RuntimeType.beq a b && RuntimeType.beqList as bs
  | _, _ => false
end

theorem RuntimeType.beq_iff (a b : RuntimeType) : RuntimeType.beq a b = true ↔ a = b := by
  induction a using RuntimeType.rec
    (motive_2 := fun s => ∀ b, RuntimeType.beq (.struct s) b = true ↔ .struct s = b)
    (motive_3 := fun as => ∀ bs, RuntimeType.beqList as bs = true ↔ as = bs) generalizing b with
  | int t => cases b <;> simp [RuntimeType.beq]
  | vector a ih => cases b <;> simp [RuntimeType.beq, ih]
  | struct s ih => exact ih b
  | function ps rs a ihp ihr => cases b <;> simp [RuntimeType.beq, ihp, ihr, and_assoc]
  | ref m a ih => cases b <;> simp [RuntimeType.beq, ih]
  | mk m n as ih =>
    rename_i b
    cases b with
    | struct s' => cases s'; simp [RuntimeType.beq, ih, and_assoc]
    | _ => simp [RuntimeType.beq]
  | nil => rename_i bs; cases bs <;> simp [RuntimeType.beqList]
  | cons x xs ihx ihxs => rename_i bs; cases bs <;> simp [RuntimeType.beqList, ihx, ihxs]
  | _ => cases b <;> simp [RuntimeType.beq]

instance : DecidableEq RuntimeType := fun a b =>
  decidable_of_iff _ (RuntimeType.beq_iff a b)

/-- A function instance in a program: a module index and a definition index
within that module, which name the definition, and the closed type arguments
it runs with, one per type parameter of a generic function and none for a
non-generic one. "A function within a module is uniquely identified by its
name" (`file_format.rs`, `FunctionHandle`), so `module` and `index` find the
definition (`Program.function?`, which ignores `typeArgs`); the type
arguments are those a `CallGeneric` supplies ("`ty_args = if func.is_generic
then func.ty_args else []`", `file_format.rs`, `Call`). A frame runs a
function instance (`Frame.function`), a native is called with one, and a
closure targets one. -/
structure FunctionRef where
  module : Nat
  index : FunctionDefinitionIndex
  typeArgs : List RuntimeType := []
  deriving DecidableEq, Repr, Inhabited

/-- A value type: one with no reference in it as a value: fields, vector
elements, constants, global storage and the targets of references hold values
of value types only. A struct type is one: it resolves only when its type
arguments are value types too (`Program.structType?`), so no value has a
struct type with a reference among its type arguments; and every type the
semantics builds has value type arguments (they are translated from value
tokens, `RuntimeType.ofTokenWith`, or checked,
`RuntimeType.argumentsWellFormed`). -/
def RuntimeType.isValue : RuntimeType → Bool
  | .bool | .int _ | .address | .signer | .struct _ | .function .. => true
  | .vector elem => elem.isValue
  | .ref _ _ => false

/-- **Assignability** (plan §5.2, "Values and typing"): a value of type `t`
may be used where type `u` is expected. "A function value with more abilities
may be used where a function type with fewer abilities is expected"; "an
immutable reference conforms covariantly in its target"; "otherwise types must
be equal". The parameters and results of function types, the targets of
mutable references, vector elements and type arguments compare by equality.
For a type without function types, assignability is equality. -/
def RuntimeType.assignable : RuntimeType → RuntimeType → Bool
  | .function ps rs a, .function ps' rs' a' => ps == ps' && rs == rs' && a'.subset a
  | .ref false t, .ref false u => t.assignable u
  | t, u => t == u

/-- Types `ts` are assignable, one for one, to types `us`. -/
def RuntimeType.assignableAll : List RuntimeType → List RuntimeType → Bool
  | [], [] => true
  | t :: ts, u :: us => t.assignable u && RuntimeType.assignableAll ts us
  | [], _ :: _ => false
  | _ :: _, [] => false

/-- Two values of these types may be compared with `Eq`: both types are
assignable to a common type (`RuntimeType.assignable`), as they are when both
values conform to the one static type the code compares them at.

- Two function types with the same parameters and results are, whatever
  their abilities: both are assignable to the function type with the
  abilities they share.
- Two immutable references are when their targets are: `&` is covariant in
  its target, and `Eq` compares the values the references designate.
- Otherwise the types must be equal. A mutable reference is invariant in its
  target, and a vector in its element type. -/
def RuntimeType.comparable : RuntimeType → RuntimeType → Bool
  | .function ps rs _, .function ps' rs' _ => ps == ps' && rs == rs'
  | .ref false t, .ref false u => t.comparable u
  | t, u => t == u

/-! ## Closure masks

"`mask` is a u64 bitset which describes which of the arguments of `fun` are
captured by the closure", and "if `i` is the `j`th bit set in the mask, then
`vj` has type `ti`" (`file_format.rs`, `PackClosure`): bit `i`, counting from
the least significant, captures parameter `i`, and the captured values are in
parameter order. -/

/-- Every bit `mask` sets names one of `n` parameters. A bit at or beyond `n`
names no parameter. -/
def maskWithin (mask : BitVec 64) (n : Nat) : Bool :=
  (List.range 64).all fun i => !mask.getLsbD i || decide (i < n)

/-- Split the parameters `xs` of a function into those `mask` captures and the
remaining ones, each in parameter order. -/
def maskSplit {α : Type} (mask : BitVec 64) (xs : List α) : List α × List α :=
  ((xs.zipIdx.filter fun (_, i) => mask.getLsbD i).map (·.1),
   (xs.zipIdx.filter fun (_, i) => !mask.getLsbD i).map (·.1))

/-- The arguments a closure passes to its function: for each parameter in
order, the next captured value if `mask` captures it, the next provided one
otherwise ("`CallClosure(PackClosure(f, mask, c1..cn), a1..am) ==
f(mask.compose(c1..cn, a1..am))`", `file_format.rs`, `CallClosure`). -/
def maskCompose {α : Type} (mask : BitVec 64) (captured provided : List α) : List α :=
  go ((List.range (captured.length + provided.length)).map mask.getLsbD) captured provided
where
  go : List Bool → List α → List α → List α
    | [], _, _ => []
    | true :: bits, c :: cs, ps => c :: go bits cs ps
    | false :: bits, cs, p :: ps => p :: go bits cs ps
    | true :: _, [], _ => []
    | false :: _, _, [] => []

/-! ## Locations

"A reference is a location plus a mutability flag", and "a location is a root
plus a path of field, variant-field or vector-index steps. The root is a local
slot of a specific frame instance, or a global (address, type)" (plan §5.2,
"References"). A path may also end in the address a signer holds, which the
native `signer::borrow_address` borrows (`PathStep.signerAddress`), or go into
an entry of a table, which the natives `table::borrow_box` and
`table::borrow_box_mut` borrow (`PathStep.tableEntry`). -/

/-- The identity of a frame instance. Every call creates a frame with an
identity that no frame had before, so a location in a local of a frame that
has returned designates nothing ever again. -/
abbrev FrameId := Nat

/-- Where a value is stored: a local slot of a frame instance, global storage
under an address and a struct type, type arguments included ("each `address`
can store at most one resource value of a given type", book,
`global-storage.md`; `Container<u64>` and `Container<T>` for another `T` are
different types there, "Global storage operators with generics"), or a value the
transaction holds for the whole execution and lends to the entry function by
reference: the signers of an entry function's `&signer` parameters ("Entry
functions can accept parameters of the following types: primitive types,
references to a `signer`, ...", book, `functions.md`). A value on an operand
stack has no location: no instruction borrows it. -/
inductive Root where
  | local_ (frame : FrameId) (index : LocalIndex)
  | global (address : Address) (type : StructId)
  | held (index : Nat)
  deriving DecidableEq, Repr, Inhabited

/-- One step from a value to a value directly inside it. -/
inductive PathStep where
  /-- Field `index` of a struct value (`MutBorrowField`, `ImmBorrowField`). -/
  | field (index : Nat)
  /-- Field `index` of an enum value of variant `tag` (`MutBorrowVariantField`,
  `ImmBorrowVariantField`). The step exists only while the value has that
  variant. -/
  | variantField (tag : VariantIndex) (index : Nat)
  /-- Element `index` of a vector (`VecMutBorrow`, `VecImmBorrow`). -/
  | element (index : Nat)
  /-- The address a signer holds: "VM Runtime representation is equivalent to
  ... `Master { account: address }`", and "`borrow_address` borrows this inner
  field" (`signer.move`; book, `primitive-types.md`, "Signer": "you can think of
  the native implementation as being `struct signer has drop { a: address }`").
  No instruction makes a reference here; the native `signer::borrow_address`
  does (`Spec.Semantics.ModeledNatives`), immutably. -/
  | signerAddress
  /-- Entry `index` of a table, in the order the table holds its entries
  (`Value.table`): "a reference to an entry designates a place inside the
  table, through a new path step" (language-owner ruling 1, 2026-11-04). No
  instruction makes a reference here; the natives `table::borrow_box` and
  `table::borrow_box_mut` do (`Spec.Semantics.ModeledNatives`). Which entry a
  key selects is the native's: the spec does not say. -/
  | tableEntry (index : Nat)
  deriving DecidableEq, Repr, Inhabited

structure Location where
  root : Root
  path : List PathStep
  deriving DecidableEq, Repr, Inhabited

/-- The location one step inside `l`. -/
def Location.child (l : Location) (step : PathStep) : Location :=
  { l with path := l.path ++ [step] }

/-- The location at `path` inside `l`. -/
def Location.descend (l : Location) (path : List PathStep) : Location :=
  { l with path := l.path ++ path }

/-- Locations `l` and `m` overlap: they have the same root and one path extends
the other, so one designates a value at or inside the other's. -/
def Location.overlaps (l m : Location) : Bool :=
  l.root == m.root && (l.path.isPrefixOf m.path || m.path.isPrefixOf l.path)

/-- A reference: a location, whether it is mutable, and the type of the value
it designates.

`since` is the number of invalidations the state had recorded when the
reference was created (`State.invalidations`). A location can stop existing
while a location of the same name exists again later: a local moved out and
assigned again, a resource moved out and published again, a vector element
popped and pushed again. A reference designates its location as it was when
the reference was created, and any invalidation recorded since then that
covers the location ends the reference's validity for good
(`State.deref?`). -/
structure Reference where
  mutable : Bool
  target : RuntimeType
  location : Location
  since : Nat
  deriving DecidableEq, Repr, Inhabited

/-- A runtime value.

- A signer holds the address it authorizes ("you can think of the native
  implementation as being `struct signer has drop { a: address }`", book,
  `primitive-types.md`, "Signer").
- A vector records its element type, so that an empty vector has a type.
- A struct value records its type, type arguments included, and holds its
  fields in declaration order.
- An enum value records its type and is one variant: its index in the
  declaration (its tag) and that variant's fields in declaration order.
- A reference is a value of its own, distinct from the value it designates:
  copying or dropping it copies or drops "the reference itself, not what they
  refer to" (book, `generics-and-abilities.md`, "Builtin Types").
- A closure is a function instance, the mask of the parameters it captured,
  the captured values in parameter order ("lambda expressions can capture
  context variables _by value_: those values are moved (or copied) into a
  _closure_", book, `functions.md`, "Lambda Expressions and Closures"), and
  its type: the remaining parameters, the results, and the abilities
  `PackClosure` derived for it (`file_format.rs`, `PackClosure`). A captured
  value is never a reference ("it is not possible to _capture_ reference
  values").
- A table, a value of a table type (`StructIdOf.isTable`), records its type and
  holds its declared fields in declaration order, and its entries: "a finite
  map from keys (values of `K`) to entries (values of `Box<V>`)"
  (language-owner ruling 1, 2026-11-04), kept as the keys and the entries in
  one order, `keys[i]` selecting `entries[i]` (`Value.wellFormed`: one key per
  entry, no key twice). Packing a table gives one with no entries; only the
  table natives add or remove entries. The entries are part of the value for
  ownership, abilities and locations; the keys, values of a type with `copy`
  and `drop`, for equality only: no reference designates a key, a key has no
  identity of its own, and a key takes no ability away from its table. -/
inductive Value where
  | bool (b : Bool)
  | int (t : IntType) (v : Int)
  | address (a : Address)
  | signer (a : Address)
  | vector (elem : RuntimeType) (elems : List Value)
  | struct (s : StructId) (fields : List Value)
  | variant (s : StructId) (tag : VariantIndex) (fields : List Value)
  | ref (r : Reference)
  | closure (fn : FunctionRef) (mask : BitVec 64) (captured : List Value)
      (params results : List RuntimeType) (abilities : AbilitySet)
  | table (s : StructId) (fields keys entries : List Value)
  deriving Repr, Inhabited

/-! Structural equality of values, which is also the semantics of `Eq` on
values of value types (book, `equality-and-comparison.md`: `==` "returns
`true` if the two operands have the same value"). `Eq` on references compares
the values they designate, not the references (`Step.equality`). As for
`SignatureToken`, `DecidableEq` cannot be derived for a nested inductive, so
equality is defined by structural recursion and shown lawful.

Closures are equal when they are the same function instance with the same
mask and the same captured values (their types are then the same): "those
relations are based on the name of the underlying function behind a runtime
value, and do not reflect semantic equivalence", and closures "are
lexicographically ordered using first the name of the underlying function
..., and then the captured values" (book, `functions.md`).

Tables are equal when they have the same type, fields, keys and entries, in
the same order. -/

mutual
def Value.beq : Value → Value → Bool
  | .bool a, .bool b => decide (a = b)
  | .int t x, .int t' y => decide (t = t') && decide (x = y)
  | .address a, .address b => decide (a = b)
  | .signer a, .signer b => decide (a = b)
  | .vector e vs, .vector e' ws => decide (e = e') && Value.beqList vs ws
  | .struct s fs, .struct s' gs => decide (s = s') && Value.beqList fs gs
  | .variant s i fs, .variant s' j gs => decide (s = s') && decide (i = j) && Value.beqList fs gs
  | .ref r, .ref r' => decide (r = r')
  | .closure f m cs ps rs a, .closure f' m' cs' ps' rs' a' =>
    decide (f = f') && decide (m = m') && Value.beqList cs cs' && decide (ps = ps') &&
      decide (rs = rs') && decide (a = a')
  | .table s fs ks es, .table s' fs' ks' es' =>
    decide (s = s') && Value.beqList fs fs' && Value.beqList ks ks' && Value.beqList es es'
  | _, _ => false

def Value.beqList : List Value → List Value → Bool
  | [], [] => true
  | a :: as, b :: bs => Value.beq a b && Value.beqList as bs
  | _, _ => false
end

theorem Value.beq_iff (a b : Value) : Value.beq a b = true ↔ a = b := by
  induction a using Value.rec
    (motive_2 := fun as => ∀ bs, Value.beqList as bs = true ↔ as = bs) generalizing b with
  | int t x => cases b <;> simp [Value.beq, and_comm]
  | vector e vs ih => cases b <;> simp [Value.beq, ih]
  | struct s fs ih => cases b <;> simp [Value.beq, ih]
  | variant s i fs ih => cases b <;> simp [Value.beq, ih, and_assoc]
  | closure f m cs ps rs a ih => cases b <;> simp [Value.beq, ih, and_assoc]
  | table s fs ks es ihf ihk ihe => cases b <;> simp [Value.beq, ihf, ihk, ihe, and_assoc]
  | nil => rename_i bs; cases bs <;> simp [Value.beqList]
  | cons x xs ihx ihxs => rename_i bs; cases bs <;> simp [Value.beqList, ihx, ihxs]
  | _ => cases b <;> simp [Value.beq]

instance : DecidableEq Value := fun a b =>
  decidable_of_iff _ (Value.beq_iff a b)

namespace Value

/-- The intrinsic runtime type of a value. An enum value has its enum's type,
whatever its variant. A reference has the reference type of its mutability and
target. A closure has the function type it records, with the abilities it
actually has, which may be more than those of the slot holding it
(`RuntimeType.assignable`). A table has its table type. -/
def type : Value → RuntimeType
  | .bool _ => .bool
  | .int t _ => .int t
  | .address _ => .address
  | .signer _ => .signer
  | .vector elem _ => .vector elem
  | .struct s _ => .struct s
  | .variant s _ _ => .struct s
  | .ref r => .ref r.mutable r.target
  | .closure _ _ _ ps rs a => .function ps rs a
  | .table s _ _ _ => .struct s

/-! ### Paths into a value -/

/-- The value one step inside this one, if the step exists: a struct value's
field, an enum value's field while the value has the step's variant, a
vector's element, a signer's address, or a table's field or entry. -/
def child? : Value → PathStep → Option Value
  | .struct _ fields, .field i => fields[i]?
  | .variant _ tag fields, .variantField t i => if tag = t then fields[i]? else none
  | .vector _ elems, .element i => elems[i]?
  | .signer a, .signerAddress => some (.address a)
  | .table _ fields _ _, .field i => fields[i]?
  | .table _ _ _ entries, .tableEntry i => entries[i]?
  | _, _ => none

/-- This value with the value one step inside it replaced by `w`, if the step
exists (`child?`). A signer's address is never replaced: that would make a
signer for another address, which no instruction may (D3b), and no reference
to it is mutable (`PathStep.signerAddress`). -/
def withChild? : Value → PathStep → Value → Option Value
  | .struct s fields, .field i, w =>
    if i < fields.length then some (.struct s (fields.set i w)) else none
  | .variant s tag fields, .variantField t i, w =>
    if tag = t ∧ i < fields.length then some (.variant s tag (fields.set i w)) else none
  | .vector e elems, .element i, w =>
    if i < elems.length then some (.vector e (elems.set i w)) else none
  | .table s fields keys entries, .field i, w =>
    if i < fields.length then some (.table s (fields.set i w) keys entries) else none
  | .table s fields keys entries, .tableEntry i, w =>
    if i < entries.length then some (.table s fields keys (entries.set i w)) else none
  | _, _, _ => none

/-- The value at `path` inside this one. -/
def get? (v : Value) : List PathStep → Option Value
  | [] => some v
  | step :: path => do (← v.child? step).get? path

/-- This value with the value at `path` replaced by `w`. -/
def set? (v : Value) : List PathStep → Value → Option Value
  | [], w => some w
  | step :: path, w => do v.withChild? step (← (← v.child? step).set? path w)

end Value

/-! ## Constants

A constant is "a serialized value along with its type" (`file_format.rs`,
`Constant`). The serialization is BCS, the Binary Canonical Serialization of
the `bcs` crate, whose specification gives:

- integers: little-endian two's complement of their exact width;
- booleans: one byte, `0` or `1`;
- addresses: their 32 bytes, most significant first;
- sequences (vectors): the number of elements as a ULEB128 integer, then each
  element in order. The length must be encoded canonically, in as few bytes
  as possible, and is at most `2^31 - 1` (`MAX_SEQUENCE_LENGTH`).

Only the types a constant may have are decoded (L8: booleans, integers,
addresses, and vectors of these). Signers are never decoded: they "cannot be
created via literals or instructions — only by the Move VM" (book,
`primitive-types.md`, "Signer"; D3b). Struct and enum values are never
decoded: only their module's code creates them (D3a). References are never
decoded: they denote locations, which no byte string names. Function values
are never decoded: only `PackClosure` creates them (D3c). -/

/-- Little-endian unsigned interpretation of a byte string. -/
def natOfBytesLE (bytes : List UInt8) : Nat :=
  bytes.foldr (fun b acc => acc * 256 + b.toNat) 0

/-- Big-endian unsigned interpretation of a byte string. -/
def natOfBytesBE (bytes : List UInt8) : Nat :=
  bytes.foldl (fun acc b => acc * 256 + b.toNat) 0

/-- The greatest length of a BCS sequence, `2^31 - 1`. -/
def bcsMaxSequenceLength : Nat := 2 ^ 31 - 1

/-- Split off the 7-bit groups of a ULEB128 integer, least significant first,
and return the remaining bytes. A byte contributes its low seven bits; its
high bit says whether another byte follows. -/
def uleb128Groups : List UInt8 → Option (List Nat × List UInt8)
  | [] => none
  | b :: rest =>
    if b.toNat < 128 then some ([b.toNat], rest)
    else do
      let (groups, rest) ← uleb128Groups rest
      pure ((b.toNat - 128) :: groups, rest)

/-- Decode the length of a BCS sequence: a canonical ULEB128 integer (its last
group is not zero, unless it is the only group) of at most
`bcsMaxSequenceLength`. -/
def decodeLength (data : List UInt8) : Option (Nat × List UInt8) := do
  let (groups, rest) ← uleb128Groups data
  let n := groups.foldr (fun g acc => acc * 128 + g) 0
  if 1 < groups.length && groups.getLast? == some 0 then none
  else if bcsMaxSequenceLength < n then none
  else some (n, rest)

/-- Decode `n` items in sequence with `decode`, returning the remaining
bytes. -/
def decodeSeq {α : Type} (decode : List UInt8 → Option (α × List UInt8)) :
    Nat → List UInt8 → Option (List α × List UInt8)
  | 0, data => some ([], data)
  | n + 1, data => do
    let (a, data) ← decode data
    let (as, data) ← decodeSeq decode n data
    pure (a :: as, data)

/-- Decode one BCS value of the given type, returning the remaining bytes.
`none` for a type a constant cannot have. -/
def bcsDecode : RuntimeType → List UInt8 → Option (Value × List UInt8)
  | .bool, data =>
    match data with
    | b :: rest =>
      if b = 0 then some (.bool false, rest)
      else if b = 1 then some (.bool true, rest)
      else none
    | [] => none
  | .int it, data =>
    let n := it.width / 8
    if data.length < n then none
    else
      let raw := natOfBytesLE (data.take n)
      let v : Int :=
        if it.isSigned && raw ≥ 2 ^ (it.width - 1) then (raw : Int) - (2 : Int) ^ it.width
        else raw
      some (.int it v, data.drop n)
  | .address, data =>
    if data.length < 32 then none
    else some (.address (BitVec.ofNat 256 (natOfBytesBE (data.take 32))), data.drop 32)
  | .vector elem, data => do
    let (n, rest) ← decodeLength data
    let (elems, rest) ← decodeSeq (bcsDecode elem) n rest
    pure (.vector elem elems, rest)
  | .signer, _ => none
  | .struct _, _ => none
  | .function .., _ => none
  | .ref _ _, _ => none

/-- Decode a constant: one value of the type, with no bytes left over. -/
def decodeConstant (t : RuntimeType) (data : List UInt8) : Option Value :=
  match bcsDecode t data with
  | some (v, []) => some v
  | some (_, _ :: _) => none
  | none => none

end MoveBytecode

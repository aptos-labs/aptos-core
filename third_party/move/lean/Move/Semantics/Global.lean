-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Reference

/-!
# Typed global resources

A resource descriptor gives typed access to one family of resources inside a
source world. Keys normally contain an address and any runtime type tags. The
world stores resources, never references or outstanding mutations.
-/

namespace Move.Semantics

/-- Typed access to one resource family in a source world. -/
structure Resource (World Key Value : Type) where
  lookup : World → Key → Option Value
  insert : World → Key → Value → World
  erase : World → Key → World

/-- A compositional, typed view of one Move resource family in an otherwise
abstract global state. Verification theorems quantify over the state and one
instance per resource they use, so adding another module or resource does not
require rebuilding a monolithic `World` record. -/
class ResourceStore (State Value : Type) where
  lookup : State → Move.Address → Option Value
  insert : State → Move.Address → Value → State
  erase : State → Move.Address → State
  lookup_insert_eq : ∀ state address value,
    lookup (insert state address value) address = some value
  lookup_erase_eq : ∀ state address,
    lookup (erase state address) address = none

/-- Distinct typed resource families occupy disjoint portions of global
state. One instance per pair supplies the frame laws needed to compose
contracts which touch several resources. -/
class IndependentResourceStores (State Left Right : Type)
    [ResourceStore State Left] [ResourceStore State Right] where
  left_lookup_right_insert : ∀ state written query value,
    ResourceStore.lookup (State := State) (Value := Left)
        (ResourceStore.insert (State := State) (Value := Right)
          state written value) query =
      ResourceStore.lookup (State := State) (Value := Left) state query
  right_lookup_left_insert : ∀ state written query value,
    ResourceStore.lookup (State := State) (Value := Right)
        (ResourceStore.insert (State := State) (Value := Left)
          state written value) query =
      ResourceStore.lookup (State := State) (Value := Right) state query
  left_lookup_right_erase : ∀ state written query,
    ResourceStore.lookup (State := State) (Value := Left)
        (ResourceStore.erase (State := State) (Value := Right)
          state written) query =
      ResourceStore.lookup (State := State) (Value := Left) state query
  right_lookup_left_erase : ∀ state written query,
    ResourceStore.lookup (State := State) (Value := Right)
        (ResourceStore.erase (State := State) (Value := Left)
          state written) query =
      ResourceStore.lookup (State := State) (Value := Right) state query

namespace ResourceStore

/-- Descriptor generated automatically for the relational source semantics. -/
def descriptor [store : ResourceStore State Value] :
    Resource State Move.Address Value where
  lookup := store.lookup
  insert := store.insert
  erase := store.erase

def contains [store : ResourceStore State Value]
    (state : State) (address : Move.Address) : Prop :=
  (store.lookup state address).isSome

/-- Typed observation used by global-place contract syntax. Contracts normally
guard it with `exists<Resource>(address)`; the default only totalizes the term
on states outside that precondition. -/
def get [Inhabited Value] [store : ResourceStore State Value]
    (state : State) (address : Move.Address) : Value :=
  (store.lookup state address).getD default

@[simp] theorem descriptor_lookup [store : ResourceStore State Value]
    (state : State) (address : Move.Address) :
    (descriptor (State := State) (Value := Value)).lookup state address =
      store.lookup state address := rfl

@[simp] theorem descriptor_insert [store : ResourceStore State Value]
    (state : State) (address : Move.Address) (value : Value) :
    (descriptor (State := State) (Value := Value)).insert state address value =
      store.insert state address value := rfl

@[grind =, simp] theorem lookup_insert_same [store : ResourceStore State Value]
    (state : State) (address : Move.Address) (value : Value) :
    store.lookup (store.insert state address value) address = some value :=
  store.lookup_insert_eq state address value

@[simp] theorem get_insert_same [Inhabited Value]
    [store : ResourceStore State Value]
    (state : State) (address : Move.Address) (value : Value) :
    get (store.insert state address value) address = value := by
  simp [get, store.lookup_insert_eq]

end ResourceStore

namespace IndependentResourceStores

@[grind =, simp] theorem lookup_left_after_right_insert
    [ResourceStore State Left] [ResourceStore State Right]
    [independent : IndependentResourceStores State Left Right]
    (state : State) (written query : Move.Address) (value : Right) :
    ResourceStore.lookup (State := State) (Value := Left)
        (ResourceStore.insert (State := State) (Value := Right)
          state written value) query =
      ResourceStore.lookup (State := State) (Value := Left) state query :=
  independent.left_lookup_right_insert state written query value

@[grind =, simp] theorem lookup_right_after_left_insert
    [ResourceStore State Left] [ResourceStore State Right]
    [independent : IndependentResourceStores State Left Right]
    (state : State) (written query : Move.Address) (value : Left) :
    ResourceStore.lookup (State := State) (Value := Right)
        (ResourceStore.insert (State := State) (Value := Left)
          state written value) query =
      ResourceStore.lookup (State := State) (Value := Right) state query :=
  independent.right_lookup_left_insert state written query value

@[grind =, simp] theorem lookup_left_after_right_erase
    [ResourceStore State Left] [ResourceStore State Right]
    [independent : IndependentResourceStores State Left Right]
    (state : State) (written query : Move.Address) :
    ResourceStore.lookup (State := State) (Value := Left)
        (ResourceStore.erase (State := State) (Value := Right)
          state written) query =
      ResourceStore.lookup (State := State) (Value := Left) state query :=
  independent.left_lookup_right_erase state written query

@[grind =, simp] theorem lookup_right_after_left_erase
    [ResourceStore State Left] [ResourceStore State Right]
    [independent : IndependentResourceStores State Left Right]
    (state : State) (written query : Move.Address) :
    ResourceStore.lookup (State := State) (Value := Right)
        (ResourceStore.erase (State := State) (Value := Left)
          state written) query =
      ResourceStore.lookup (State := State) (Value := Right) state query :=
  independent.right_lookup_left_erase state written query

end IndependentResourceStores

namespace Resource

/-- Move's default VM failure code for a missing resource, duplicate
publication, bounds error, or arithmetic failure in this model. -/
def executionFailure : Nat := 0

def contains (resource : Resource World Key Value) (key : Key) : Txn World Bool :=
  fun world => .ok (resource.lookup world key |>.isSome) world

/-- Immutable global borrow after immutable-reference erasure. -/
def borrow (resource : Resource World Key Value) (key : Key) : Txn World (ImmRef Value) :=
  fun world =>
    match resource.lookup world key with
    | some value => .ok value world
    | none => .abort executionFailure

def moveFrom (resource : Resource World Key Value) (key : Key) : Txn World Value :=
  fun world =>
    match resource.lookup world key with
    | some value => .ok value (resource.erase world key)
    | none => .abort executionFailure

def moveTo (resource : Resource World Key Value) (key : Key)
    (value : Value) : Txn World Unit := fun world =>
  match resource.lookup world key with
  | some _ => .abort executionFailure
  | none => .ok () (resource.insert world key value)

/-! Verification-only relational global operations. -/

def containsSpec (resource : Resource World Key Value) (key : Key) : Spec World Bool where
  ok := fun initial result final =>
    result = (resource.lookup initial key).isSome ∧ final = initial
  aborts := fun _ _ => False

def borrowSpec (resource : Resource World Key Value) (key : Key) : Spec World Value where
  ok := fun initial result final =>
    resource.lookup initial key = some result ∧ final = initial
  aborts := fun initial code =>
    resource.lookup initial key = none ∧ code = executionFailure

def moveFromSpec (resource : Resource World Key Value) (key : Key) : Spec World Value where
  ok := fun initial result final =>
    resource.lookup initial key = some result ∧ final = resource.erase initial key
  aborts := fun initial code =>
    resource.lookup initial key = none ∧ code = executionFailure

def moveToSpec (resource : Resource World Key Value) (key : Key)
    (value : Value) : Spec World Unit where
  ok := fun initial result final =>
    resource.lookup initial key = none ∧ result = () ∧
      final = resource.insert initial key value
  aborts := fun initial code =>
    (∃ old, resource.lookup initial key = some old) ∧ code = executionFailure

/-- Scoped concrete mutable borrow. The checked-out value is passed explicitly
to `body`; only the computed final value is restored. A body abort has no
write-back, and transaction rollback hides all tentative changes. Borrow
legality prevents `body` from observing this same resource through another
operation while it is suspended. -/
def withBorrowMut (resource : Resource World Key Value) (key : Key)
    (body : Value → Txn World (Result × Value)) : Txn World Result := fun world =>
  match resource.lookup world key with
  | none => .abort executionFailure
  | some initial =>
      match body initial world with
      | .abort code => .abort code
      | .ok (result, final) bodyWorld =>
          .ok result (resource.insert bodyWorld key final)

/-- Verification-only relational mutable global borrow. No runtime reference
or outstanding-loan entry is added to the world. -/
def withBorrowMutSpec (resource : Resource World Key Value) (key : Key)
    (body : Value → Spec World (Result × Value)) : Spec World Result where
  ok := fun initial result finalWorld =>
    ∃ value bodyWorld finalValue,
      resource.lookup initial key = some value ∧
      (body value).ok initial (result, finalValue) bodyWorld ∧
      finalWorld = resource.insert bodyWorld key finalValue
  aborts := fun initial code =>
    (resource.lookup initial key = none ∧ code = executionFailure) ∨
      ∃ value, resource.lookup initial key = some value ∧ (body value).aborts initial code

/-- Verification semantics for a mutable borrow focused through one or more
structure fields. The global resource is checked out by ownership, the focus
is represented by a prophecy mutation for the lifetime of `body`, and the
reconciled focus is written back before the resource is restored. `set` is
generated from the source field path, so references never become part of the
world model. -/
def withBorrowMutFocusSpec (resource : Resource World Key Owner) (key : Key)
    (get : Owner → Focus) (set : Owner → Focus → Owner)
    (body : Mutation Focus → Spec World (Result × Mutation Focus)) :
    Spec World Result :=
  resource.withBorrowMutSpec key fun owner => do
    let output ← withMutation (get owner) body
    pure (output.1, set owner output.2)

@[simp] theorem borrow_missing (resource : Resource World Key Value) (key : Key)
    (world : World) (h : resource.lookup world key = none) :
    resource.borrow key world = .abort executionFailure := by
  simp [borrow, h]

@[simp] theorem borrow_present (resource : Resource World Key Value) (key : Key)
    (world : World) (value : Value) (h : resource.lookup world key = some value) :
    resource.borrow key world = .ok value world := by
  simp [borrow, h]

@[simp] theorem withBorrowMut_success
    (resource : Resource World Key Value) (key : Key) (world bodyWorld : World)
    (body : Value → Txn World (Result × Value)) (initial final : Value) (result : Result)
    (hlookup : resource.lookup world key = some initial)
    (hbody : body initial world = .ok (result, final) bodyWorld) :
    resource.withBorrowMut key body world =
      .ok result (resource.insert bodyWorld key final) := by
  simp [withBorrowMut, hlookup, hbody]

@[simp] theorem withBorrowMut_abort
    (resource : Resource World Key Value) (key : Key) (world : World)
    (body : Value → Txn World (Result × Value)) (initial : Value) (code : Nat)
    (hlookup : resource.lookup world key = some initial)
    (hbody : body initial world = .abort code) :
    resource.withBorrowMut key body world = .abort code := by
  simp [withBorrowMut, hlookup, hbody]

@[simp] theorem borrowSpec_present
    (resource : Resource World Key Value) (key : Key) (initial final : World)
    (value result : Value) (hlookup : resource.lookup initial key = some value) :
    (resource.borrowSpec key).ok initial result final ↔
      result = value ∧ final = initial := by
  simp [borrowSpec, hlookup, Option.some.injEq, eq_comm]

@[simp] theorem borrowSpec_present_not_aborts
    (resource : Resource World Key Value) (key : Key) (initial : World)
    (value : Value) (code : Nat) (hlookup : resource.lookup initial key = some value) :
    ¬(resource.borrowSpec key).aborts initial code := by
  simp [borrowSpec, hlookup]

@[simp] theorem borrowSpec_missing_not_ok
    (resource : Resource World Key Value) (key : Key) (initial final : World)
    (result : Value) (hlookup : resource.lookup initial key = none) :
    ¬(resource.borrowSpec key).ok initial result final := by
  simp [borrowSpec, hlookup]

@[simp] theorem borrowSpec_missing_aborts
    (resource : Resource World Key Value) (key : Key) (initial : World)
    (code : Nat) (hlookup : resource.lookup initial key = none) :
    (resource.borrowSpec key).aborts initial code ↔ code = executionFailure := by
  simp [borrowSpec, hlookup]

@[simp] theorem withBorrowMutSpec_present_ok
    (resource : Resource World Key Value) (key : Key) (initial finalWorld : World)
    (body : Value → Spec World (Result × Value)) (value : Value) (result : Result)
    (hlookup : resource.lookup initial key = some value) :
    (resource.withBorrowMutSpec key body).ok initial result finalWorld ↔
      ∃ bodyWorld finalValue,
        (body value).ok initial (result, finalValue) bodyWorld ∧
        finalWorld = resource.insert bodyWorld key finalValue := by
  simp [withBorrowMutSpec, hlookup]

@[simp] theorem withBorrowMutSpec_present_aborts
    (resource : Resource World Key Value) (key : Key) (initial : World)
    (body : Value → Spec World (Result × Value)) (value : Value) (code : Nat)
    (hlookup : resource.lookup initial key = some value) :
    (resource.withBorrowMutSpec key body).aborts initial code ↔
      (body value).aborts initial code := by
  simp [withBorrowMutSpec, hlookup]

@[simp] theorem withBorrowMutSpec_missing_not_ok
    (resource : Resource World Key Value) (key : Key) (initial finalWorld : World)
    (body : Value → Spec World (Result × Value)) (result : Result)
    (hlookup : resource.lookup initial key = none) :
    ¬(resource.withBorrowMutSpec key body).ok initial result finalWorld := by
  simp [withBorrowMutSpec, hlookup]

@[simp] theorem withBorrowMutSpec_missing_aborts
    (resource : Resource World Key Value) (key : Key) (initial : World)
    (body : Value → Spec World (Result × Value)) (code : Nat)
    (hlookup : resource.lookup initial key = none) :
    (resource.withBorrowMutSpec key body).aborts initial code ↔
      code = executionFailure := by
  simp [withBorrowMutSpec, hlookup]

end Resource

end Move.Semantics

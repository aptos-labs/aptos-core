-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Basic

/-!
# Move computations and compiler primitives

`Action` uses an opaque world token to retain native `do` notation and explicit
effect order.  The compiler erases the token while lowering recognized
primitives to Move stackless operations.
-/

namespace Move

/-- An unforgeable compile-time token sequencing Move effects. -/
structure World where
  private mk ::

/-- The source computation type.  This representation is a compiler device,
not an executable model of on-chain state. -/
abbrev Action (α : Type) := StateM World α

/-! Reference and storage primitives. -/

@[noinline] opaque borrowLocal {α : Type} (value : α) : Action (Ref α) :=
  fun world => (Ref.ofValue value, world)

@[noinline] opaque borrowLocalMut {α : Type} (value : α) : Action (MutRef α) :=
  fun world => (MutRef.ofValue value, world)

@[noinline] opaque borrowGlobal (α : Type) [Inhabited α] (_addr : Address) :
    Action (Ref α) := fun world => (Ref.default, world)

@[noinline] opaque borrowGlobalMut (α : Type) [Inhabited α] (_addr : Address) :
    Action (MutRef α) := fun world => (MutRef.default, world)

@[noinline] opaque borrowField {Owner FieldTy : Type} :
    Ref Owner → Field Owner FieldTy → Action (Ref FieldTy) := fun owner field world =>
  (Ref.ofValue (field.get owner.get), world)

@[noinline] opaque borrowFieldMut {Owner FieldTy : Type} :
    MutRef Owner → Field Owner FieldTy → Action (MutRef FieldTy) := fun owner field world =>
  (MutRef.ofValue (field.get owner.get), world)

@[noinline] opaque borrowElem {α : Type} [Inhabited α] :
    Ref (Move.Vector α) → U64 → Action (Ref α) := fun _ _ world =>
  (Ref.default, world)

@[noinline] opaque borrowElemMut {α : Type} [Inhabited α] :
    MutRef (Move.Vector α) → U64 → Action (MutRef α) := fun _ _ world =>
  (MutRef.default, world)

@[noinline] opaque freeze {α : Type} : MutRef α → Action (Ref α) := fun ref world =>
  (freezeRef ref, world)

@[noinline] opaque read {α : Type} : MutRef α → Action α := fun ref world =>
  (ref.get, world)

@[noinline] opaque readImm {α : Type} : Ref α → Action α := fun ref world =>
  (ref.get, world)

@[noinline] opaque write {α : Type} : MutRef α → α → Action Unit := fun _ref _value world =>
  ((), world)

@[noinline] opaque existsAt (_α : Type) (_addr : Address) : Action Bool := fun world =>
  (false, world)

@[noinline] opaque moveFrom (α : Type) [Inhabited α] (_addr : Address) : Action α := fun world =>
  ((Inhabited.default : α), world)

@[noinline] opaque moveTo {α : Type} : Ref Signer → α → Action Unit := fun _signer _value world =>
  ((), world)

@[noinline] opaque abort {α : Type} [Inhabited α] : U64 → Action α := fun _code world =>
  ((Inhabited.default : α), world)

/-- Move 2.4's well-known code for an abort carrying a diagnostic message. -/
def unspecifiedAbortCode : U64 := U64.ofNat 0xCA26CBD9BE0B0000

/-- Abort carrying a byte message. The backend's current outcome model records
the standard code; byte-message transport can be added without changing the
source or verification semantics. -/
@[always_inline] def abortMessage {Result : Type} [Inhabited Result]
    (_message : Move.Vector U8) : Action Result :=
  abort unspecifiedAbortCode

/-- Elaboration-only body used for a bodyless `native fun` declaration. -/
@[noinline] opaque nativeUnavailable {Result : Type} [Inhabited Result] : Result :=
  default

@[always_inline] def assert (condition : Bool) (code : U64) : Action Unit := do
  if condition then
    pure ()
  else
    abort (α := Unit) code

namespace Vector

/-- Insert an element at `index`, shifting the suffix to the right.  This is
the source-level shape of Move's `vector::insert`: it mutates a vector through
`&mut` and aborts when `index > length` (or the vector cannot grow). -/
@[noinline] opaque insert {α : Type} (self : MutRef (Move.Vector α))
    (index : U64) (value : α) : Action Unit := fun world => ((), world)

/-- Remove and return the element at `index`, shifting the suffix to the left.
This mirrors Move's `vector::remove` and aborts when `index >= length`. -/
@[noinline] opaque remove {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) (index : U64) : Action α :=
  fun world => (default, world)

/-- Remove and return the final element, aborting when the vector is empty. -/
@[noinline] opaque popBack {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) : Action α :=
  fun world => (default, world)

@[noinline] opaque swap {α : Type} (self : MutRef (Move.Vector α))
    (i j : U64) : Action Unit := fun world => ((), world)

@[noinline] opaque swapRemove {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) (i : U64) : Action α :=
  fun world => (default, world)

@[noinline] opaque append {α : Type} (self : MutRef (Move.Vector α))
    (other : Move.Vector α) : Action Unit := fun world => ((), world)

@[noinline] opaque reverse {α : Type} (self : MutRef (Move.Vector α)) :
    Action Unit := fun world => ((), world)

@[noinline] opaque reverseSlice {α : Type} (self : MutRef (Move.Vector α))
    (left right : U64) : Action Unit := fun world => ((), world)

@[noinline] opaque trim {α : Type} (self : MutRef (Move.Vector α))
    (newLen : U64) : Action (Move.Vector α) := fun world => (Move.Vector.empty, world)

@[noinline] opaque trimReverse {α : Type} (self : MutRef (Move.Vector α))
    (newLen : U64) : Action (Move.Vector α) := fun world => (Move.Vector.empty, world)

@[noinline] opaque rotate {α : Type} (self : MutRef (Move.Vector α))
    (rot : U64) : Action U64 := fun world => (0, world)

@[noinline] opaque rotateSlice {α : Type} (self : MutRef (Move.Vector α))
    (left rot right : U64) : Action U64 := fun world => (0, world)

@[noinline] opaque destroyEmpty {α : Type} (self : Move.Vector α) :
    Action Unit := fun world => ((), world)

@[noinline] opaque contains {α : Type} (self : Ref (Move.Vector α))
    (value : Ref α) : Bool := false

@[noinline] opaque indexOf {α : Type} (self : Ref (Move.Vector α))
    (value : Ref α) : Bool × U64 := (false, 0)

end Vector

namespace MutRef

/-- Receiver-style spelling of Move's `vector::insert`. Lean resolves
`valuesRef.insert index value` through the mutable-reference namespace. -/
@[noinline] def insert {α : Type} (self : MutRef (Move.Vector α))
    (index : U64) (value : α) : Action Unit :=
  Move.Vector.insert self index value

/-- Receiver-style spelling of Move's `vector::remove`. -/
@[noinline] def remove {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) (index : U64) : Action α :=
  Move.Vector.remove self index

/-- Receiver-style spelling of Move's `vector::pop_back`. -/
@[noinline] def popBack {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) : Action α :=
  Move.Vector.popBack self

@[noinline] def swap {α : Type} (self : MutRef (Move.Vector α)) (i j : U64) :
    Action Unit := Move.Vector.swap self i j

@[noinline] def swapRemove {α : Type} [Inhabited α]
    (self : MutRef (Move.Vector α)) (i : U64) : Action α :=
  Move.Vector.swapRemove self i

@[noinline] def append {α : Type} (self : MutRef (Move.Vector α))
    (other : Move.Vector α) : Action Unit := Move.Vector.append self other

@[noinline] def reverse {α : Type} (self : MutRef (Move.Vector α)) : Action Unit :=
  Move.Vector.reverse self

@[noinline] def reverseSlice {α : Type} (self : MutRef (Move.Vector α))
    (left right : U64) : Action Unit := Move.Vector.reverseSlice self left right

@[noinline] def trim {α : Type} (self : MutRef (Move.Vector α))
    (newLen : U64) : Action (Move.Vector α) := Move.Vector.trim self newLen

@[noinline] def trimReverse {α : Type} (self : MutRef (Move.Vector α))
    (newLen : U64) : Action (Move.Vector α) := Move.Vector.trimReverse self newLen

@[noinline] def rotate {α : Type} (self : MutRef (Move.Vector α))
    (rot : U64) : Action U64 := Move.Vector.rotate self rot

@[noinline] def rotateSlice {α : Type} (self : MutRef (Move.Vector α))
    (left rot right : U64) : Action U64 := Move.Vector.rotateSlice self left rot right

end MutRef

namespace Ref

/-- Length of a borrowed vector without reading the vector into an owned
value. The backend lowers this receiver form directly to `vector::length`. -/
@[noinline] def length {α : Type} (self : Ref (Move.Vector α)) : U64 :=
  Move.Vector.length self.get

end Ref

namespace MutRef

/-- Length of a mutably borrowed vector, observed without consuming or
copying it. -/
@[noinline] def length {α : Type} (self : MutRef (Move.Vector α)) : U64 :=
  Move.Vector.length self.get

end MutRef

end Move

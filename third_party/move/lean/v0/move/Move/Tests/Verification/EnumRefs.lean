-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! Enum payload references in automatic source specifications: `&r.f` and
`&mut r.f` on an enum referent (the variant-mismatch abort), `match` through
`&` and `&mut` binding payload references, and the derived specification
version of a function reading a payload field (`self.value` on an enum value
selects the field of the variants that have it). -/

namespace Tests.MovePrograms.EnumRefs

open Move
open scoped Move Move.Spec

module EnumRefs where

  enum Slot has Copy, Drop, Store where
    | Empty
    | Filled (value : U64)

  enum Shape has Copy, Drop, Store where
    | Circle (radius : U64)
    | Rectangle (width : U64) (height : U64)

  enum Triple has Copy, Drop, Store where
    | Values (first : U64) (second : U64) (third : U64)

  struct Holder has Copy, Drop, Store where
    slot : Slot

  struct SlotResource has Key where
    slot : Slot

  /-! ## `&r.f` on an enum referent -/

  fun peek (self : &Slot) : Action U64 := do
    let value ← &self.value
    let v ← *value
    pure v

  spec peek (self : Slot) where
    ensures result = self.value;
    aborts_if ¬(self is Slot.Filled) with Move.Semantics.variantMismatch

  fun peek_holder (holder : &Holder) : Action U64 := do
    let value ← &holder.slot.value
    let v ← *value
    pure v

  spec peek_holder (holder : Holder) where
    ensures result = holder.slot.value;
    aborts_if ¬(holder.slot is Slot.Filled) with Move.Semantics.variantMismatch

  fun fill (self : &mut Slot) (value : U64) : Action Unit := do
    let payload ← &mut self.value
    payload := value

  spec fill (self : &mut Slot) (value : U64) where
    ensures self = Slot.Filled value;
    aborts_if ¬(self is Slot.Filled) with Move.Semantics.variantMismatch

  /-- A global resource is restored only after its enum payload loan has
  reconciled. Variant selection contributes the usual mismatch abort. -/
  fun fill_global (address : Address) (value : U64) : Action Unit := do
    let payload ← &mut SlotResource[address].slot.value
    payload := value

  spec fill_global (address : Address) (value : U64) where
    requires existsAt<SlotResource>(address) ∧
      SlotResource[address].slot is Slot.Filled;
    modifies SlotResource[address];
    ensures SlotResource[address].slot = Slot.Filled value;
    aborts_if False

  /-- The guarded selection: no abort once the variant is known. -/
  fun peek_or (self : &Slot) (default : U64) : Action U64 := do
    let v ← *self
    if v is Slot.Filled then
      let value ← &self.value
      let w ← *value
      pure w
    else
      pure default

  spec peek_or (self : Slot) (default : U64) where
    ensures result = if self is Slot.Filled then self.value else default;
    aborts_if False

  /-! ## `match` through a reference -/

  fun value_or (slot : &Slot) (default : U64) : Action U64 := do
    match slot with
    | .Filled value =>
        let v ← *value
        pure v
    | _ => pure default

  spec value_or (slot : Slot) (default : U64) where
    ensures result = (match slot with | .Filled v => v | _ => default);
    aborts_if False

  fun scale (shape : &mut Shape) (factor : U64) : Action Unit := do
    match shape with
    | .Circle radius =>
        let r ← *radius
        radius := r * factor
    | .Rectangle width height =>
        let w ← *width
        width := w * factor
        let h ← *height
        height := h * factor

  spec scale (shape : &mut Shape) (factor : U64) where
    pragma aborts_if_is_partial;
    ensures shape = (match old(shape) with
      | .Circle r => Shape.Circle (r * factor)
      | .Rectangle w h => Shape.Rectangle (w * factor) (h * factor))

  fun replace (self : &mut Slot) (value : U64) : Action U64 := do
    match self with
    | .Filled old =>
        let previous ← *old
        old := value
        pure previous
    | .Empty => abort 7

  spec replace (self : &mut Slot) (value : U64) where
    ensures self = Slot.Filled value ∧ result = old(self).value;
    aborts_if ¬(self is Slot.Filled) with 7

  /-- All named payloads are independent loans. This crosses the former
  two-payload implementation limit for reference matches. -/
  fun overwrite_three (self : &mut Triple) : Action Unit := do
    match self with
    | .Values first second third =>
        first := 11
        second := 22
        third := 33

  spec overwrite_three (self : &mut Triple) where
    ensures self = Triple.Values 11 22 33;
    aborts_if False

  /-! ## The derived specification version of a payload read -/

  fun peek_twice (self : &Slot) : Action U64 := do
    let a ← peek self
    let b ← peek self
    pure (a + b)

  spec peek_twice (self : Slot) where
    pragma aborts_if_is_partial;
    ensures result = peek self + peek self

  /-! ## Proofs -/

  verify peek
  verify peek_holder
  verify fill
  verify fill_global
  verify peek_or
  verify value_or
  verify scale
  verify replace
  verify overwrite_three
  verify peek_twice

end Tests.MovePrograms.EnumRefs

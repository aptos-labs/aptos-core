-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Enum payload references

Port of v0's `Verification/EnumRefs.lean`.
-/

namespace LeanerLang.Tests.VerificationEnumRefs

leaner module 0x42::enum_refs where
  enum Slot has Copy, Drop, Store where
    | Empty
    | Filled (value : u64)

  enum Shape has Copy, Drop, Store where
    | Circle (radius : u64)
    | Rectangle (width : u64, height : u64)

  enum Triple has Copy, Drop, Store where
    | Values (first : u64, second : u64, third : u64)

  struct Holder has Copy, Drop, Store where
    slot : Slot

  struct SlotResource has Key where
    slot : Slot

  fun peek(self : &Slot) -> u64 := self.value

  spec peek where
    ensures result == self.value
    aborts_if !(self is Filled)

  fun peek_holder(holder : &Holder) -> u64 := holder.slot.value

  spec peek_holder where
    ensures result == holder.slot.value
    aborts_if !(holder.slot is Filled)

  fun fill(self : &mut Slot, value : u64) -> Unit :=
    self.value := value

  spec fill where
    ensures self == new Slot::Filled { value }
    aborts_if !(self is Filled)

  fun fill_global(address : Address, value : u64) -> Unit :=
    SlotResource[address].slot.value := value

  spec fill_global where
    requires exists<SlotResource>(address) &&
      global<SlotResource>(address).slot is Filled
    modifies global<SlotResource>(address)
    ensures global<SlotResource>(address).slot == new Slot::Filled { value }
    aborts_if false

  fun peek_or(self : &Slot, default : u64) -> u64 :=
    if self is Filled then self.value else default

  spec peek_or where
    ensures result == (if self is Filled then self.value else default)
    aborts_if false

  fun value_or(slot : &Slot, default : u64) -> u64 :=
    match slot with
      | Slot::Filled { value := value } => *value
      | Slot::Empty {} => default

  spec value_or where
    ensures result == (match slot with
      | Slot::Filled { value := value } => value
      | Slot::Empty {} => default)
    aborts_if false

  fun scale(shape : &mut Shape, factor : u64) -> Unit :=
    match shape with
      | Shape::Circle { radius := radius } =>
          *radius := *radius * factor
      | Shape::Rectangle { width := width, height := height } => do
          *width := *width * factor
          *height := *height * factor

  spec scale where
    pragma aborts_if_is_partial
    ensures shape == (match old(shape) with
      | Shape::Circle { radius := radius } =>
          new Shape::Circle { radius := radius * factor }
      | Shape::Rectangle { width := width, height := height } =>
          new Shape::Rectangle {
            width := width * factor, height := height * factor })

  fun replace(self : &mut Slot, value : u64) -> u64 :=
    match self with
      | Slot::Filled { value := payload } => do
          let previous := *payload
          *payload := value
          return previous
      | Slot::Empty {} => abort(7)

  spec replace where
    ensures self == new Slot::Filled { value } && result == old(self).value
    aborts_if !(self is Filled) with 7

  fun overwrite_three(self : &mut Triple) -> Unit :=
    match self with
      | Triple::Values { first := first, second := second, third := third } => do
          *first := 11
          *second := 22
          *third := 33

  spec overwrite_three where
    ensures self == new Triple::Values { first := 11, second := 22, third := 33 }
    aborts_if false

  fun peek_twice(self : &Slot) -> u64 := self.value + self.value

  spec peek_twice where
    pragma aborts_if_is_partial
    ensures result == peek(self) + peek(self)

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

end LeanerLang.Tests.VerificationEnumRefs

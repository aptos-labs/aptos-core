-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/EnumRefs: all sixteen functions and ten execution
assertions. The original file has no verification commands; contracts for
enum references are covered separately by Verification/EnumRefs. v0's
implicitly checked field projections use explicit variant guards here:
raw LIR field selection is partial, not a checked Move abort operation. -/

leaner module 0x42::language_enum_refs where
  enum Slot has Copy, Drop, Store where
    | Empty
    | Filled (value : u64)
  enum Shape has Copy, Drop, Store where
    | Circle (radius : u64)
    | Rectangle (width : u64, height : u64)
  struct Holder has Copy, Drop, Store where
    slot : Slot

  fun peek(self : &Slot) -> u64 := do
    if !(self is Filled) then abort()
    let value := &self.value
    return *value

  fun fill(self : &mut Slot, value : u64) -> Unit := do
    if !(self is Filled) then abort()
    let payload := &mut self.value
    *payload := value

  fun peek_holder(holder : &Holder) -> u64 := do
    let slot := &holder.slot
    if !(slot is Filled) then abort()
    let value := &slot.value
    return *value

  fun area(shape : &Shape) -> u64 :=
    match shape with
      | Shape::Circle { radius := radius } => do
          let r := *radius
          return (r * r) * 3
      | Shape::Rectangle { width := width, height := height } => do
          let w := *width
          let h := *height
          return w * h

  fun scale(shape : &mut Shape, factor : u64) -> Unit :=
    match shape with
      | Shape::Circle { radius := radius } => do
          let r := *radius
          *radius := r * factor
      | Shape::Rectangle { width := width, height := height } => do
          let w := *width
          *width := w * factor
          let h := *height
          *height := h * factor

  fun value_or(slot : &Slot, default : u64) -> u64 :=
    match slot with
      | Slot::Filled { value := value } => *value
      | _ => default

  fun peek_filled(value : u64) -> u64 := do
    let slot := new Slot::Filled { value }
    let slot_ref := &slot
    return peek(slot_ref)

  fun peek_empty() -> u64 := do
    let slot := new Slot::Empty {}
    let slot_ref := &slot
    return peek(slot_ref)

  fun fill_then_peek(value : u64) -> u64 := do
    let mut slot := new Slot::Filled { value := 0 }
    let slot_mut := &mut slot
    fill(slot_mut, value)
    let slot_ref := &slot
    return peek(slot_ref)

  fun fill_empty(value : u64) -> Unit := do
    let mut slot := new Slot::Empty {}
    let slot_mut := &mut slot
    fill(slot_mut, value)

  fun holder_value(value : u64) -> u64 := do
    let holder := new Holder { slot := new Slot::Filled { value } }
    let holder_ref := &holder
    return peek_holder(holder_ref)

  fun circle_area(radius : u64) -> u64 := do
    let shape := new Shape::Circle { radius }
    let shape_ref := &shape
    return area(shape_ref)

  fun scaled_rectangle_area(width : u64, height : u64, factor : u64) -> u64 := do
    let mut shape := new Shape::Rectangle { width, height }
    let shape_mut := &mut shape
    scale(shape_mut, factor)
    let shape_ref := &shape
    return area(shape_ref)

  fun scaled_circle_area(radius : u64, factor : u64) -> u64 := do
    let mut shape := new Shape::Circle { radius }
    let shape_mut := &mut shape
    scale(shape_mut, factor)
    let shape_ref := &shape
    return area(shape_ref)

  fun empty_value_or(default : u64) -> u64 := do
    let slot := new Slot::Empty {}
    let slot_ref := &slot
    return value_or(slot_ref, default)

  fun filled_value_or(value : u64, default : u64) -> u64 := do
    let slot := new Slot::Filled { value }
    let slot_ref := &slot
    return value_or(slot_ref, default)

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».language_enum_refs #[
    ⟨"peek_filled", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"peek_empty", #[], .threw .abort #[], {}⟩,
    ⟨"fill_then_peek", #[.integer 9], .returned #[.integer 9], {}⟩,
    ⟨"fill_empty", #[.integer 9], .threw .abort #[], {}⟩,
    ⟨"holder_value", #[.integer 11], .returned #[.integer 11], {}⟩,
    ⟨"circle_area", #[.integer 2], .returned #[.integer 12], {}⟩,
    ⟨"scaled_rectangle_area", #[.integer 2, .integer 3, .integer 2],
      .returned #[.integer 24], {}⟩,
    ⟨"scaled_circle_area", #[.integer 2, .integer 3], .returned #[.integer 108], {}⟩,
    ⟨"empty_value_or", #[.integer 5], .returned #[.integer 5], {}⟩,
    ⟨"filled_value_or", #[.integer 4, .integer 5], .returned #[.integer 4], {}⟩]

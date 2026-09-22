-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Enum payload references: `&r.field` on an enum referent, and `match`
through `&e` / `&mut e` binding payload fields by reference. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module EnumRefs where

  enum Slot has Copy, Drop, Store where
    | Empty
    | Filled (value : U64)

  enum Shape has Copy, Drop, Store where
    | Circle (radius : U64)
    | Rectangle (width : U64) (height : U64)

  struct Holder has Copy, Drop, Store where
    slot : Slot

  /-! ## Functions -/

  /-- `&self.value` on an enum referent: the variant field borrow, aborting
  when the referent is not `Filled`. -/
  fun peek (self : &Slot) : Action U64 := do
    let value ← &self.value
    let v ← *value
    pure v

  fun fill (self : &mut Slot) (value : U64) : Action Unit := do
    let payload ← &mut self.value
    payload := value

  fun peek_holder (holder : &Holder) : Action U64 := do
    let slot ← &holder.slot
    let value ← &slot.value
    let v ← *value
    pure v

  /-- `match` through an immutable reference: payload binders are references. -/
  fun area (shape : &Shape) : Action U64 := do
    match shape with
    | .Circle radius => do
        let r ← *radius
        pure ((r * r) * 3)
    | .Rectangle width height => do
        let w ← *width
        let h ← *height
        pure (w * h)

  /-- `match` through a mutable reference: writing through a payload binder. -/
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

  fun value_or (slot : &Slot) (default : U64) : Action U64 := do
    match slot with
    | .Filled value =>
        let v ← *value
        pure v
    | _ => pure default

  fun peek_filled (value : U64) : Action U64 := do
    let slot : Slot := .Filled value
    let slot_ref ← &slot
    peek slot_ref

  fun peek_empty : Action U64 := do
    let slot : Slot := .Empty
    let slot_ref ← &slot
    peek slot_ref

  fun fill_then_peek (value : U64) : Action U64 := do
    let slot : Slot := .Filled 0
    let slot_mut ← &mut slot
    fill slot_mut value
    let slot_ref ← &slot
    peek slot_ref

  fun fill_empty (value : U64) : Action Unit := do
    let slot : Slot := .Empty
    let slot_mut ← &mut slot
    fill slot_mut value

  fun holder_value (value : U64) : Action U64 := do
    let holder : Holder := { slot := .Filled value }
    let holder_ref ← &holder
    peek_holder holder_ref

  fun circle_area (radius : U64) : Action U64 := do
    let shape : Shape := .Circle radius
    let shape_ref ← &shape
    area shape_ref

  fun scaled_rectangle_area (width height factor : U64) : Action U64 := do
    let shape : Shape := .Rectangle width height
    let shape_mut ← &mut shape
    scale shape_mut factor
    let shape_ref ← &shape
    area shape_ref

  fun scaled_circle_area (radius factor : U64) : Action U64 := do
    let shape : Shape := .Circle radius
    let shape_mut ← &mut shape
    scale shape_mut factor
    let shape_ref ← &shape
    area shape_ref

  fun empty_value_or (default : U64) : Action U64 := do
    let slot : Slot := .Empty
    let slot_ref ← &slot
    value_or slot_ref default

  fun filled_value_or (value default : U64) : Action U64 := do
    let slot : Slot := .Filled value
    let slot_ref ← &slot
    value_or slot_ref default

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.EnumRefs

  private def run := Tests.run compiled

  #test run "peek_filled" [] [.u64 7] = Tests.okU64 7
  #test run "peek_empty" [] [] = Tests.aborted MoveModel.IR.runtimeAbortCode
  #test run "fill_then_peek" [] [.u64 9] = Tests.okU64 9
  #test run "fill_empty" [] [.u64 9] = Tests.aborted MoveModel.IR.runtimeAbortCode
  #test run "holder_value" [] [.u64 11] = Tests.okU64 11
  #test run "circle_area" [] [.u64 2] = Tests.okU64 12
  #test run "scaled_rectangle_area" [] [.u64 2, .u64 3, .u64 2] = Tests.okU64 24
  #test run "scaled_circle_area" [] [.u64 2, .u64 3] = Tests.okU64 108
  #test run "empty_value_or" [] [.u64 5] = Tests.okU64 5
  #test run "filled_value_or" [] [.u64 4, .u64 5] = Tests.okU64 4

end Tests.MovePrograms

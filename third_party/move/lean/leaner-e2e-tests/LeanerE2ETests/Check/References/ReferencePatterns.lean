-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Patterns through references

A pattern matched through a reference binds references to the fields it
names. A `let` aborts with the mismatch throw where a variant does not hold;
a `match` tries its arms in order, guards included; a field several
variants name at different types is reached through the variant.
-/

namespace LeanerLang.Tests.Check.References.ReferencePatterns

leaner module 0x42::reference_patterns where
  struct Pair has Copy, Drop where
    left : u64
    right : u64

  enum Shape has Copy, Drop where
    | Circle (radius : u64)
    | Rect (width : u64, height : u64)

  enum Outer has Copy, Drop where
    | Empty
    | Holds (inner : Shape)

  enum Mixed has Copy, Drop where
    | Wide (x : u64)
    | Narrow (x : u8)

  fun swap_in_place(p : Pair) -> Pair := do
    let mut q := p
    let Pair { left := l, right := r } := &mut q
    let t := *l
    *l := *r
    *r := t
    q
  spec swap_in_place where
    ensures result.left == p.right && result.right == p.left

  fun grow(s : Shape) -> Shape := do
    let mut t := s
    let Shape::Circle { radius := r } := &mut t
    *r := *r + 1
    t
  spec grow where
    pragma aborts_if_is_partial
    aborts_if !(s is Circle) with 14566554180833181697

  fun reset_incorrect(s : Shape) -> Shape := do
    let mut t := s
    let Shape::Circle { radius := r } := &mut t
    *r := 0
    t
  spec reset_incorrect where
    aborts_if false -- error: a rectangle aborts

  fun area(s : Shape) -> u64 :=
    match &s with
      | Shape::Circle { radius := r } => *r * *r
      | Shape::Rect { width := w, height := h } => *w * *h
  spec area where
    pragma aborts_if_is_partial
    ensures s is Rect ==> result == s.width * s.height

  fun capped_radius(s : Shape) -> u64 :=
    match &s with
      | Shape::Circle { radius := r } if *r > 10 => 10
      | Shape::Circle { radius := r } => *r
      | _ => 0
  spec capped_radius where
    ensures result <= 10

  fun inner_radius(o : Outer) -> u64 :=
    match &o with
      | Outer::Holds { inner := Shape::Circle { radius := r } } => *r
      | _ => 0
  spec inner_radius where
    ensures o is Empty ==> result == 0

  fun mixed_width(m : Mixed) -> u64 :=
    match &m with
      | Mixed::Wide { x := w } => *w
      | Mixed::Narrow { x := n } => (*n as u64)
  spec mixed_width where
    ensures m is Narrow ==> result < 256

  fun narrow_x(m : Mixed) -> u8 := do
    let mut n := m
    let r := &mut (downcast n as Mixed::Narrow).x
    *r
  spec narrow_x where
    aborts_if !(m is Narrow) with 14566554180833181697

-- The runtime makes the mismatch throw.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let shape (variant : String) (fields : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 1⟩ (some variant) fields
  assertRuns `«0x42».reference_patterns #[
    ⟨"grow", #[shape "Circle" #[.integer 2]], .returned #[shape "Circle" #[.integer 3]], {}⟩,
    ⟨"grow", #[shape "Rect" #[.integer 2, .integer 3]],
      .threw .abort #[.integer moveIncompleteMatchAbortCode], {}⟩,
    ⟨"area", #[shape "Rect" #[.integer 2, .integer 3]], .returned #[.integer 6], {}⟩,
    ⟨"capped_radius", #[shape "Circle" #[.integer 20]], .returned #[.integer 10], {}⟩]

end LeanerLang.Tests.Check.References.ReferencePatterns

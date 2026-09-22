-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value

/-!
# IR States

The bytecode state has two components:

* frame-indexed local stores, each a partial map from local indices to values;
* type-indexed global `memory`, where a resource type and account address
  identify a resource value.

Ordinary stackless-bytecode operands refer to locals in the active frame.
References also record a frame identity. Generic-aware global memory is
indexed by a resource declaration together with its structural type-argument
tags and then by address.

A `Footprint` is a predicate over global locations and represents a `modifies`
clause.  `agreesOutside` states that a memory transition changes only
locations in that footprint.
-/

namespace MoveModel.IR

/-- The local store of a function activation. `none` = uninitialized. -/
abbrev Locals := LocalIndex → Option Value

/-- Local stores of call frames, indexed by call depth.  Retired entries are
cleared.  Totality keeps frame lookup executable at the specification level;
an absent local still makes reference access stuck. -/
abbrev FrameStore := FrameId → Locals

/-- A frame store with every local uninitialized. -/
def emptyFrames : FrameStore := fun _ _ => none

/-- Replace one frame in a frame store. -/
def setFrame (frames : FrameStore) (frame : FrameId) (locals : Locals) :
    FrameStore :=
  fun frame' => if frame' = frame then locals else frames frame'

/-- Looking up the frame just written returns the new locals. -/
@[simp] theorem setFrame_same (frames : FrameStore) (frame : FrameId)
    (locals : Locals) : setFrame frames frame locals frame = locals := by
  simp [setFrame]

/-- Rewriting a frame with its current locals leaves the store unchanged. -/
@[simp] theorem setFrame_self (frames : FrameStore) (frame : FrameId) :
    setFrame frames frame (frames frame) = frames := by
  funext other
  by_cases h : other = frame
  · subst other
    simp
  · simp [setFrame, h]

/-- Writing one frame leaves every distinct frame unchanged. -/
theorem setFrame_of_ne (frames : FrameStore) {frame other : FrameId}
    (locals : Locals) (h : other ≠ frame) :
    setFrame frames frame locals other = frames other := by
  simp [setFrame, h]

/-- Type-indexed global memory: `memory r a` is the resource of type `r`
stored at address `a`, if present. -/
abbrev Memory := ResourceKey → Address → Option Value

/-- The initial local store of a function activation: locals
`0..args.length-1` hold the arguments, everything else is uninitialized. -/
def initLocals (args : List Value) : Locals :=
  fun x => args[x]?

/-- Store a resource of type `r` at address `a`. -/
def memWrite (m : Memory) (r : ResourceKey) (a : Address) (v : Value) : Memory :=
  fun r' a' => if r' = r ∧ a' = a then some v else m r' a'

/-- Remove the resource of type `r` at address `a`. -/
def memRemove (m : Memory) (r : ResourceKey) (a : Address) : Memory :=
  fun r' a' => if r' = r ∧ a' = a then none else m r' a'

/-- A global memory location: a (resource type, address) pair. -/
structure Location where
  rsrc : ResourceKey
  addr : Address

/-- A set of global memory locations, e.g. the footprint of a `modifies`
clause.  Kept as a predicate; no decidability is required because footprints
only occur in specifications and havoc relations. -/
abbrev Footprint := Location → Prop

/-- `agreesOutside Δ m m'`: the transition from memory `m` to `m'` did not
touch any location outside the footprint `Δ` (the frame condition of a
`modifies` clause). -/
def agreesOutside (Δ : Footprint) (m m' : Memory) : Prop :=
  ∀ (r : ResourceKey) (a : Address), ¬ Δ ⟨r, a⟩ → m' r a = m r a

/-- A full bytecode-level state.  Ordinary local operands address the
`current` frame; reference values contain an explicit frame identity and can
therefore address an ancestor frame during a call. -/
structure MoveState where
  current : FrameId
  frames : FrameStore
  memory : Memory

/-- Move states are equal when their current frame, frames, and memory agree. -/
@[ext] theorem MoveState.ext {s t : MoveState}
    (hcurrent : s.current = t.current)
    (hframes : s.frames = t.frames)
    (hmemory : s.memory = t.memory) : s = t := by
  cases s
  cases t
  simp_all

/-- State which crosses a normal function-return boundary.  The callee frame
has been retired; the caller chooses its own frame as current when resuming. -/
structure FrameWorld where
  frames : FrameStore
  memory : Memory

/-- Returned worlds are equal when their frames and memory agree. -/
@[ext] theorem FrameWorld.ext {s t : FrameWorld}
    (hframes : s.frames = t.frames)
    (hmemory : s.memory = t.memory) : s = t := by
  cases s
  cases t
  simp_all

namespace MoveState

/-- Two states agree on the frame identity and every frame below it.  This
is the compositional frame condition used at call boundaries: ordinary
instructions may update the active frame, but not its callers. -/
def SameBelow (before after : MoveState) : Prop :=
  after.current = before.current ∧
    ∀ frame, frame < before.current → after.frames frame = before.frames frame

/-- Every state agrees with itself below its current frame. -/
theorem SameBelow.refl (s : MoveState) : s.SameBelow s :=
  ⟨rfl, fun _ _ => rfl⟩

/-- Agreement below the same current frame is transitive. -/
theorem SameBelow.trans {s₁ s₂ s₃ : MoveState}
    (h₁₂ : s₁.SameBelow s₂) (h₂₃ : s₂.SameBelow s₃) :
    s₁.SameBelow s₃ := by
  refine ⟨h₂₃.1.trans h₁₂.1, ?_⟩
  intro frame hlt
  rw [h₂₃.2 frame (by simpa [h₁₂.1] using hlt), h₁₂.2 frame hlt]

/-- Locals of the currently executing frame. -/
def locals (s : MoveState) : Locals := s.frames s.current

/-- View an existing frame as the active frame without changing stores or
memory.  Stack-indexed simulations use this to apply one frame-local
relation uniformly to active and suspended activations. -/
def focusFrame (s : MoveState) (frame : FrameId) : MoveState :=
  { s with current := frame }

/-- Focusing a frame selects exactly that frame's locals. -/
@[simp] theorem focusFrame_locals (s : MoveState) (frame : FrameId) :
    (s.focusFrame frame).locals = s.frames frame := rfl

/-- Focusing a frame leaves its stores unchanged. -/
@[simp] theorem focusFrame_frames (s : MoveState) (frame other : FrameId) :
    (s.focusFrame frame).frames other = s.frames other := rfl

/-- Focusing a frame leaves global memory unchanged. -/
@[simp] theorem focusFrame_memory (s : MoveState) (frame : FrameId) :
    (s.focusFrame frame).memory = s.memory := rfl

/-- Focusing the active frame is the identity. -/
@[simp] theorem focusFrame_current (s : MoveState) :
    s.focusFrame s.current = s := by
  cases s
  rfl

/-- Active-local lookup reads the current frame's local store. -/
@[simp] theorem locals_apply (s : MoveState) (x : LocalIndex) :
    s.locals x = s.frames s.current x := rfl

/-- Initial state of an externally invoked function. -/
def initial (args : List Value) (memory : Memory) : MoveState :=
  { current := 0
    frames := setFrame emptyFrames 0 (initLocals args)
    memory }

/-- Enter a callee one level below the current frame. -/
def enterCall (s : MoveState) (args : List Value) : MoveState :=
  let child := s.current + 1
  { current := child
    frames := setFrame s.frames child (initLocals args)
    memory := s.memory }

/-- Entering a call increments the current frame identifier. -/
@[simp] theorem enterCall_current (s : MoveState) (args : List Value) :
    (s.enterCall args).current = s.current + 1 := rfl

/-- Entering a call preserves global memory. -/
@[simp] theorem enterCall_memory (s : MoveState) (args : List Value) :
    (s.enterCall args).memory = s.memory := rfl

/-- Entering a call preserves all existing frames at or below the caller. -/
theorem enterCall_frames_of_le (s : MoveState) (args : List Value)
    {frame : FrameId} (hle : frame ≤ s.current) :
    (s.enterCall args).frames frame = s.frames frame := by
  simp [enterCall, setFrame, Nat.ne_of_lt (Nat.lt_succ_of_le hle)]

/-- Retire the current frame on normal return. -/
def finishFrame (s : MoveState) : FrameWorld :=
  { frames := setFrame s.frames s.current (fun _ => none)
    memory := s.memory }

/-- Resume a caller frame from a callee's returned world. -/
def resumeFrame (world : FrameWorld) (caller : FrameId) : MoveState :=
  { current := caller, frames := world.frames, memory := world.memory }

/-- Update one local. -/
def writeLocal (s : MoveState) (x : LocalIndex) (v : Value) : MoveState :=
  { s with frames := setFrame s.frames s.current fun y =>
      if y = x then some v else s.locals y }

/-- Update a local in an explicitly named frame.  References use this form
when a callee writes through a reference rooted in a suspended caller. -/
def writeFrameLocal (s : MoveState) (frame : FrameId)
    (x : LocalIndex) (v : Value) : MoveState :=
  { s with frames := setFrame s.frames frame fun y =>
      if y = x then some v else s.frames frame y }

/-- Writing a frame local preserves the current frame identifier. -/
@[simp] theorem writeFrameLocal_current (s : MoveState) (frame : FrameId)
    (x : LocalIndex) (v : Value) :
    (s.writeFrameLocal frame x v).current = s.current := rfl

/-- Writing a frame local preserves global memory. -/
@[simp] theorem writeFrameLocal_memory (s : MoveState) (frame : FrameId)
    (x : LocalIndex) (v : Value) :
    (s.writeFrameLocal frame x v).memory = s.memory := rfl

/-- Characterize every frame after writing one explicitly named local. -/
@[simp] theorem writeFrameLocal_frames (s : MoveState) (frame : FrameId)
    (x : LocalIndex) (v : Value) (other : FrameId) :
    (s.writeFrameLocal frame x v).frames other =
      if other = frame then
        fun y => if y = x then some v else s.frames frame y
      else s.frames other := by
  simp [writeFrameLocal, setFrame]

/-- Writing an active local preserves all lower frames. -/
theorem sameBelow_writeLocal (s : MoveState) (x : LocalIndex) (v : Value) :
    s.SameBelow (s.writeLocal x v) := by
  refine ⟨rfl, ?_⟩
  intro frame hlt
  simp [writeLocal, setFrame, Nat.ne_of_lt hlt]

/-- Characterize every frame after writing an active local. -/
@[simp] theorem writeLocal_frames (s : MoveState) (x : LocalIndex)
    (v : Value) (frame : FrameId) :
    (s.writeLocal x v).frames frame =
      if frame = s.current then
        fun y => if y = x then some v else s.frames s.current y
      else s.frames frame := by
  simp [writeLocal, setFrame, locals]

/-- The active frame after a local write is the updated active frame. -/
@[simp] theorem writeLocal_activeFrame (s : MoveState) (x : LocalIndex)
    (v : Value) (y : LocalIndex) :
    (s.writeLocal x v).frames s.current y =
      if y = x then some v else s.frames s.current y := by
  simp [writeLocal, setFrame, locals]

/-- Characterize active-local lookup after writing one local. -/
@[simp] theorem writeLocal_locals (s : MoveState) (x : LocalIndex)
    (v : Value) (y : LocalIndex) :
    (s.writeLocal x v).locals y = if y = x then some v else s.locals y := by
  simp [writeLocal, locals]

/-- Writing a local preserves the current frame identifier. -/
@[simp] theorem writeLocal_current (s : MoveState) (x : LocalIndex)
    (v : Value) : (s.writeLocal x v).current = s.current := rfl

/-- Writing a local preserves global memory. -/
@[simp] theorem writeLocal_memory (s : MoveState) (x : LocalIndex)
    (v : Value) : (s.writeLocal x v).memory = s.memory := rfl

/-- Two states agree on active locals below a numeric boundary.  This is the
natural frame invariant for passes which allocate temporaries monotonically
above all existing locals. -/
def LocalsEqBelow (bound : Nat) (s s' : MoveState) : Prop :=
  ∀ x, x < bound → s'.locals x = s.locals x

/-- Local agreement below a boundary is reflexive. -/
theorem LocalsEqBelow.refl (bound : Nat) (s : MoveState) :
    LocalsEqBelow bound s s := fun _ _ => rfl

/-- Local agreement below a boundary is transitive. -/
theorem LocalsEqBelow.trans {bound : Nat} {s₁ s₂ s₃ : MoveState}
    (h₁₂ : LocalsEqBelow bound s₁ s₂)
    (h₂₃ : LocalsEqBelow bound s₂ s₃) :
    LocalsEqBelow bound s₁ s₃ := by
  intro x hx
  rw [h₂₃ x hx, h₁₂ x hx]

/-- Agreement below a larger boundary restricts to a smaller one. -/
theorem LocalsEqBelow.mono {small large : Nat} {s s' : MoveState}
    (h : LocalsEqBelow large s s') (hle : small ≤ large) :
    LocalsEqBelow small s s' :=
  fun x hx => h x (Nat.lt_of_lt_of_le hx hle)

/-- A write at or above the boundary preserves all locals below it. -/
theorem LocalsEqBelow.writeLocal {bound x : Nat} {s : MoveState} {v : Value}
    (hx : bound ≤ x) : LocalsEqBelow bound s (s.writeLocal x v) := by
  intro y hy
  simp [Nat.ne_of_lt (Nat.lt_of_lt_of_le hy hx)]

/-- Update several locals, pointwise (used for call returns). -/
def writeLocals : MoveState → List LocalIndex → List Value → MoveState
  | s, x :: xs, v :: vs => (s.writeLocal x v).writeLocals xs vs
  | s, _, _ => s

/-- Writing active locals preserves all lower frames. -/
theorem sameBelow_writeLocals (s : MoveState) :
    ∀ xs vs, s.SameBelow (s.writeLocals xs vs) := by
  intro xs
  induction xs generalizing s with
  | nil => intro vs; exact SameBelow.refl s
  | cons x xs ih =>
    intro vs
    cases vs with
    | nil => exact SameBelow.refl s
    | cons v vs =>
      exact (sameBelow_writeLocal s x v).trans (ih (s := s.writeLocal x v) vs)

/-- Equal active locals remain equal after identical parallel writes. -/
theorem writeLocals_locals_congr {s t : MoveState}
    (h : s.locals = t.locals) (xs : List LocalIndex) (vs : List Value) :
    (s.writeLocals xs vs).locals = (t.writeLocals xs vs).locals := by
  induction xs generalizing s t vs with
  | nil => cases vs <;> exact h
  | cons x xs ih =>
    cases vs with
    | nil => exact h
    | cons v vs =>
      apply ih
      funext y
      rw [writeLocal_locals, writeLocal_locals, h]

/-- Parallel local writes preserve the current frame identifier. -/
@[simp] theorem writeLocals_current (s : MoveState)
    (xs : List LocalIndex) (vs : List Value) :
    (s.writeLocals xs vs).current = s.current := by
  induction xs generalizing s vs with
  | nil => cases vs <;> rfl
  | cons x xs ih =>
    cases vs with
    | nil => rfl
      | cons v vs => exact ih (s := s.writeLocal x v) vs

/-- Parallel local writes preserve global memory. -/
@[simp] theorem writeLocals_memory (s : MoveState)
    (xs : List LocalIndex) (vs : List Value) :
    (s.writeLocals xs vs).memory = s.memory := by
  induction xs generalizing s vs with
  | nil => cases vs <;> rfl
  | cons x xs ih =>
      cases vs with
      | nil => rfl
      | cons v vs => exact ih (s := s.writeLocal x v) vs

/-- Parallel active-local writes preserve every non-current frame. -/
theorem writeLocals_frames_of_ne (s : MoveState) (xs : List LocalIndex)
    (vs : List Value) {frame : FrameId} (hne : frame ≠ s.current) :
    (s.writeLocals xs vs).frames frame = s.frames frame := by
  induction xs generalizing s vs with
  | nil => cases vs <;> rfl
  | cons x xs ih =>
      cases vs with
      | nil => rfl
      | cons v vs =>
        rw [MoveState.writeLocals, ih (s := s.writeLocal x v) (vs := vs)
          (by simpa using hne), MoveState.writeLocal_frames, if_neg hne]

/-- Outside written destinations lookup is unchanged; at a destination both
states receive the same reference-free result. -/
theorem writeLocals_free_lookup {s s' : MoveState}
    {dsts : List LocalIndex} {rets : List Value}
    (hlen : dsts.length = rets.length)
    (hfree : ∀ v ∈ rets, v.refFree) (x : LocalIndex) :
    (x ∉ dsts ∧ (s.writeLocals dsts rets).locals x = s.locals x ∧
      (s'.writeLocals dsts rets).locals x = s'.locals x) ∨
    ∃ v, v.refFree ∧ (s.writeLocals dsts rets).locals x = some v ∧
      (s'.writeLocals dsts rets).locals x = some v := by
  induction dsts generalizing s s' rets with
  | nil =>
      have hre : rets = [] :=
        List.length_eq_zero_iff.mp (by simpa using hlen.symm)
      subst rets
      exact Or.inl ⟨by simp, rfl, rfl⟩
  | cons dst dsts ih =>
      cases rets with
      | nil => simp at hlen
      | cons v rets =>
        have hlen' : dsts.length = rets.length := by simpa using hlen
        rcases ih hlen' (fun w hw => hfree w (by simp [hw]))
            (s := s.writeLocal dst v) (s' := s'.writeLocal dst v) with
          ⟨hx, hs, hs'⟩ | ⟨w, hwfree, hs, hs'⟩
        · by_cases heq : x = dst
          · subst x
            exact Or.inr ⟨v, hfree v (by simp), by
              simpa [MoveState.writeLocals] using hs, by
              simpa [MoveState.writeLocals] using hs'⟩
          · exact Or.inl ⟨by simp [heq, hx], by
              simpa [MoveState.writeLocals, heq] using hs, by
              simpa [MoveState.writeLocals, heq] using hs'⟩
        · exact Or.inr ⟨w, hwfree, by
            simpa [MoveState.writeLocals] using hs, by
            simpa [MoveState.writeLocals] using hs'⟩

/-- Replace the global memory (used at call boundaries). -/
def setMemory (s : MoveState) (m : Memory) : MoveState :=
  { s with memory := m }

/-- Replacing memory preserves the current frame identifier. -/
@[simp] theorem setMemory_current (s : MoveState) (m : Memory) :
    (s.setMemory m).current = s.current := rfl

/-- Replacing memory preserves the frame store. -/
@[simp] theorem setMemory_frames (s : MoveState) (m : Memory) :
    (s.setMemory m).frames = s.frames := rfl

/-- Replacing memory preserves active locals. -/
@[simp] theorem setMemory_locals (s : MoveState) (m : Memory) :
    (s.setMemory m).locals = s.locals := rfl

/-- Reading memory after replacement returns the replacement memory. -/
@[simp] theorem setMemory_memory (s : MoveState) (m : Memory) :
    (s.setMemory m).memory = m := rfl

/-- Replacing memory preserves all frames below the current frame. -/
theorem sameBelow_setMemory (s : MoveState) (m : Memory) :
    s.SameBelow (s.setMemory m) := SameBelow.refl s

/-- Store a resource of type `r` at address `a`. -/
def writeGlobal (s : MoveState) (r : ResourceKey) (a : Address) (v : Value) :
    MoveState :=
  { s with memory := memWrite s.memory r a v }

/-- Remove the resource of type `r` at address `a`. -/
def removeGlobal (s : MoveState) (r : ResourceKey) (a : Address) : MoveState :=
  { s with memory := memRemove s.memory r a }

/-- Read the value a reference designates (`none` if the root or the path
does not exist — e.g. a dangling reference after `move_from`). -/
def readTarget (s : MoveState) (t : RefTarget) : Option Value :=
  match t.root with
  | .loc frame x => (s.frames frame x).bind (·.getPath t.path)
  | .global r a => (s.memory r a).bind (·.getPath t.path)

/-- Reading a target with one appended path component performs one value lookup. -/
theorem readTarget_snoc {s : MoveState} {root : RefRoot}
    {p : List Nat} {i : Nat} :
    s.readTarget ⟨root, p ++ [i]⟩ =
      (s.readTarget ⟨root, p⟩).bind (·.getPath [i]) := by
  cases root <;>
    simp [readTarget, Value.getPath_append, Option.bind_assoc]

/-- Entering a call preserves reads rooted in existing frames. -/
theorem readTarget_enterCall_of_root_le (s : MoveState) (args : List Value)
    (t : RefTarget)
    (hroot : ∀ frame rootLocal, t.root = .loc frame rootLocal →
      frame ≤ s.current) :
    (s.enterCall args).readTarget t = s.readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => rfl
    | loc frame rootLocal =>
      simp [readTarget, enterCall_frames_of_le s args
        (hroot frame rootLocal rfl)]

/-- Writing an active-frame local does not affect a reference rooted in a
different frame, nor any global reference.  This is the basic preservation
fact used for suspended caller invariants. -/
theorem readTarget_writeLocal_of_rootFrame_ne (s : MoveState)
    (x : LocalIndex) (v : Value) (t : RefTarget)
    (hroot : ∀ frame rootLocal, t.root = .loc frame rootLocal →
      frame ≠ s.current) :
    (s.writeLocal x v).readTarget t = s.readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => rfl
    | loc frame rootLocal =>
      have hne := hroot frame rootLocal rfl
      simp [readTarget, writeLocal, setFrame, hne]

/-- The sharper active-frame form: a local write changes exactly the root
`(current, x)` and leaves every other reference target unchanged. -/
theorem readTarget_writeLocal_of_root_ne (s : MoveState)
    (x : LocalIndex) (v : Value) (t : RefTarget)
    (hroot : t.root ≠ .loc s.current x) :
    (s.writeLocal x v).readTarget t = s.readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => rfl
    | loc frame rootLocal =>
      by_cases hframe : frame = s.current
      · subst frame
        have hlocal : rootLocal ≠ x := by
          intro heq
          exact hroot (by simp [heq])
        simp [readTarget, writeLocal, setFrame, hlocal]
      · simp [readTarget, writeLocal, setFrame, hframe]

/-- Synchronized writes of the same value preserve agreement on every
reference target. -/
theorem readTarget_writeLocal_eq {s s' : MoveState}
    (hcurrent : s'.current = s.current) (x : LocalIndex) (v : Value)
    (t : RefTarget) (hread : s'.readTarget t = s.readTarget t) :
    (s'.writeLocal x v).readTarget t =
      (s.writeLocal x v).readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => exact hread
    | loc frame rootLocal =>
      by_cases hframe : frame = s.current
      · subst frame
        by_cases hlocal : rootLocal = x
        · subst rootLocal
          simp [readTarget, writeLocal, setFrame, hcurrent]
        · simpa [readTarget, writeLocal, setFrame, hcurrent, hlocal]
            using hread
      · have hframe' : frame ≠ s'.current := by
          simpa [hcurrent] using hframe
        simpa [readTarget, writeLocal, setFrame, hframe, hframe'] using hread

/-- Synchronized writes to an explicitly named frame preserve agreement on
every reference target. -/
theorem readTarget_writeFrameLocal_eq {s s' : MoveState}
    (frame : FrameId) (x : LocalIndex) (v : Value) (t : RefTarget)
    (hread : s'.readTarget t = s.readTarget t) :
    (s'.writeFrameLocal frame x v).readTarget t =
      (s.writeFrameLocal frame x v).readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => exact hread
    | loc rootFrame rootLocal =>
      by_cases hframe : rootFrame = frame
      · subst rootFrame
        by_cases hlocal : rootLocal = x
        · subst rootLocal
          simp [readTarget, writeFrameLocal, setFrame]
        · simpa [readTarget, writeFrameLocal, setFrame, hlocal] using hread
      · simpa [readTarget, writeFrameLocal, setFrame, hframe] using hread

/-- Synchronized multi-local writes preserve agreement on every reference
target. -/
theorem readTarget_writeLocals_eq {s s' : MoveState}
    (hcurrent : s'.current = s.current) (xs : List LocalIndex)
    (vs : List Value) (t : RefTarget)
    (hread : s'.readTarget t = s.readTarget t) :
    (s'.writeLocals xs vs).readTarget t =
      (s.writeLocals xs vs).readTarget t := by
  induction xs generalizing s s' vs with
  | nil => cases vs <;> exact hread
  | cons x xs ih =>
      cases vs with
      | nil => exact hread
      | cons v vs =>
        exact ih (by simpa using hcurrent) vs
          (readTarget_writeLocal_eq hcurrent x v t hread)

/-- Multi-local form of `readTarget_writeLocal_of_root_ne`. -/
theorem readTarget_writeLocals_of_root_ne (s : MoveState)
    (xs : List LocalIndex) (vs : List Value) (t : RefTarget)
    (hroot : ∀ x ∈ xs, t.root ≠ .loc s.current x) :
    (s.writeLocals xs vs).readTarget t = s.readTarget t := by
  induction xs generalizing s vs with
  | nil => cases vs <;> rfl
  | cons x xs ih =>
      cases vs with
      | nil => rfl
      | cons v vs =>
        rw [MoveState.writeLocals,
          ih (s := s.writeLocal x v) (vs := vs) (fun y hy => by
            simpa using hroot y (by simp [hy])),
          readTarget_writeLocal_of_root_ne s x v t (hroot x (by simp))]

/-- Write through a reference: read-modify-write of the root location. -/
def writeTarget (s : MoveState) (t : RefTarget) (v : Value) :
    Option MoveState :=
  match t.root with
  | .loc frame x => (s.frames frame x).bind fun root =>
      (root.setPath t.path v).map fun root' =>
        { s with frames := setFrame s.frames frame fun y =>
            if y = x then some root' else s.frames frame y }
  | .global r a => (s.memory r a).bind fun root =>
      (root.setPath t.path v).map fun root' => s.writeGlobal r a root'

/-- A successful reference write preserves the active frame identifier. -/
theorem writeTarget_current {s s' : MoveState} {t : RefTarget} {v : Value}
    (h : s.writeTarget t v = some s') : s'.current = s.current := by
  cases t with
  | mk root path =>
    cases root with
    | loc frame x =>
        cases hroot : s.frames frame x with
        | none => simp [writeTarget, hroot] at h
        | some root =>
            cases hset : root.setPath path v with
            | none => simp [writeTarget, hroot, hset] at h
            | some root' =>
                simp [writeTarget, hroot, hset] at h
                subst s'
                rfl
    | global r a =>
        cases hroot : s.memory r a with
        | none => simp [writeTarget, hroot] at h
        | some root =>
            cases hset : root.setPath path v with
            | none => simp [writeTarget, hroot, hset] at h
            | some root' =>
                simp [writeTarget, hroot, hset] at h
                subst s'
                rfl

/-- Writing a local-rooted target updates the corresponding frame-local root. -/
theorem writeTarget_loc {s : MoveState} {frame : FrameId}
    {x : LocalIndex} {path : List Nat} {v root root' : Value}
    (hroot : s.frames frame x = some root)
    (hset : root.setPath path v = some root') :
    s.writeTarget ⟨.loc frame x, path⟩ v =
      some (s.writeFrameLocal frame x root') := by
  simp [writeTarget, writeFrameLocal, hroot, hset]

/-- Writing a global-rooted target updates the corresponding resource value. -/
theorem writeTarget_global {s : MoveState} {r : ResourceKey} {a : Address}
    {path : List Nat} {v root root' : Value}
    (hroot : s.memory r a = some root)
    (hset : root.setPath path v = some root') :
    s.writeTarget ⟨.global r a, path⟩ v =
      some (s.writeGlobal r a root') := by
  simp [writeTarget, hroot, hset]

end MoveState

namespace FrameWorld

/-- Select a frame as current after returning from its child. -/
def resume (world : FrameWorld) (caller : FrameId) : MoveState :=
  MoveState.resumeFrame world caller

end FrameWorld

end MoveModel.IR

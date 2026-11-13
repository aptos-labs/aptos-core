-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Compile
import VerexIR.Proofs.Denote.Closures

/-!
# A computable reading of the denotation

`Term.denote` is a relation, so a test cannot run it. `Term.tree` computes the
same term as a `Tree`: its leaves are the outcomes, its inner nodes the two
places the denotation is not a function of its input, the choice of a prophecy
and the assumption that resolves one. `Term.tree_means` proves, case by case,
that a term's tree means exactly its denotation, so a case of `Term.denote` that
changes without its tree does not build. A node the tree does not cover is
`unknown`, which claims nothing.
-/

namespace VerexIR.Proofs.Denote

open VerexIR.Validation

section Tree
variable [Carriers]

/-- A computation as a tree of its outcomes. -/
inductive Tree (α : Type) : Type where
  | ok (value : α)
  | abort (failure : Failure)
  /-- No outcome, undefined (`Spec.stuck`). -/
  | stuck
  /-- No outcome (`Spec.bottom`). -/
  | bottom
  /-- A node the tree does not cover. -/
  | unknown (reason : String)
  /-- Any value of `σ`; `current` is what the borrowed place holds. -/
  | choose (σ : NTy) (current : σ.carrier) (next : σ.carrier → Tree α)
  /-- Continue only where a reference's current value is its prophecy. -/
  | assumeEq (σ : NTy) (current prophecy : σ.carrier) (next : Tree α)

def Tree.bind {α β : Type} : Tree α → (α → Tree β) → Tree β
  | .ok value, next => next value
  | .abort failure, _ => .abort failure
  | .stuck, _ => .stuck
  | .bottom, _ => .bottom
  | .unknown reason, _ => .unknown reason
  | .choose σ current rest, next => .choose σ current fun value => (rest value).bind next
  | .assumeEq σ current prophecy rest, next => .assumeEq σ current prophecy (rest.bind next)

/-- What a tree says of a computation: the leaves are exactly their `Spec`s, and
`unknown` says nothing. -/
def Tree.Means {σ₀ α : Type} : Tree α → Spec σ₀ Failure α → Prop
  | .ok value, spec => spec = Spec.pure value
  | .abort failure, spec => spec = Spec.abort failure
  | .stuck, spec => spec = Spec.stuck
  | .bottom, spec => spec = Spec.bottom
  | .unknown _, _ => True
  | .choose σ _ next, spec => ∃ rest : σ.carrier → Spec σ₀ Failure α,
      spec = Spec.bind Spec.choose rest ∧ ∀ value, (next value).Means (rest value)
  | .assumeEq _ current prophecy next, spec => ∃ rest : Spec σ₀ Failure α,
      spec = Spec.bind (Spec.assume (current = prophecy)) (fun _ => rest) ∧ next.Means rest

end Tree

/-! ## The term's outcome -/

section Run
variable [Carriers]

/-- The outcome of a tree under chosen prophecies. -/
inductive TreeOutcome (α : Type) where
  | ok (value : α)
  | abort (failure : Failure)
  | stuck
  | bottom
  | unknown (reason : String)
  /-- An assumption failed: no execution takes these prophecies. -/
  | vacuous (σ : NTy) (current prophecy : σ.carrier)

/-- A prophecy chosen on a path. -/
structure Chosen where
  σ : NTy
  value : σ.carrier

/-- The value a prophecy starts the search at: an integer differs by position,
so a failed assumption's prophecy names the one it read and a prophecy nothing
resolves shows in the result; any other value is the borrowed place's. -/
def NTy.searchStart : (σ : NTy) → σ.carrier → Nat → σ.carrier
  | .int width signed, current, position =>
      let offset : Int := (37 * (position + 1) : Nat) % 251 + 1
      if above : IntegerValueFits (.bits width) signed (current.val + offset) then
        ⟨current.val + offset, above⟩
      else if below : IntegerValueFits (.bits width) signed (current.val - offset) then
        ⟨current.val - offset, below⟩
      else current
  | _, current, _ => current

/-- The outcome of a tree under prophecies by position, starting where none is
given at the borrowed place's value or, if `distinct`, at `NTy.searchStart`, with
the prophecies of the path. -/
def Tree.walk {α : Type} (distinct : Bool) (oracle : Array (Option Chosen)) :
    Tree α → List Chosen → TreeOutcome α × List Chosen
  | .ok value, trail => (.ok value, trail)
  | .abort failure, trail => (.abort failure, trail)
  | .stuck, trail => (.stuck, trail)
  | .bottom, trail => (.bottom, trail)
  | .unknown reason, trail => (.unknown reason, trail)
  | .choose σ current next, trail =>
      let value : σ.carrier :=
        match oracle[trail.length]?.join with
        | some chosen => if same : chosen.σ = σ then same ▸ chosen.value else current
        | none => if distinct then σ.searchStart current trail.length else current
      Tree.walk distinct oracle (next value) (trail ++ [⟨σ, value⟩])
  | .assumeEq σ current prophecy next, trail =>
      match NTy.decEq σ current prophecy with
      | isTrue _ => Tree.walk distinct oracle next trail
      | isFalse _ => (.vacuous σ current prophecy, trail)

/-- The positions of the prophecies on the path with a value, last first. -/
def holding (σ : NTy) (value : σ.carrier) (trail : List Chosen) : List Nat :=
  ((trail.zipIdx.reverse.filter fun entry =>
    if same : entry.1.σ = σ then
      match NTy.decEq σ (same ▸ entry.1.value) value with
      | isTrue _ => true
      | isFalse _ => false
    else false).map (·.2))

/-- What identifies a set of written prophecies. -/
def oracleKey (oracle : Array (Option Chosen)) : List (Option RuntimeValue) :=
  oracle.toList.map (·.map fun chosen => chosen.σ.encode chosen.value)

/-- Search the prophecies under which a tree's assumptions hold, by writing back
what a failed assumption reads into a prophecy holding its value, trying each
such prophecy where several hold it, and then, for a reference holding a chosen
prophecy as its current value, the other way. A prophecy holding values itself may have to
be written again as what it holds gets written; prophecies seen before are not
tried again, so a search with no answer ends `vacuous`. Any outcome found is one
of the tree's (`Tree.walk_sound`). Returns the attempts left and the prophecies
seen. -/
def Tree.solveWith {α : Type} (tree : Tree α) (distinct : Bool) : Nat → Nat → Array (Option Chosen) →
    List (List (Option RuntimeValue)) → TreeOutcome α × Nat × List (List (Option RuntimeValue))
  | 0, attempts, _, seen => (.unknown "prophecy search depth", attempts, seen)
  | _ + 1, 0, _, seen => (.unknown "prophecy search attempts", 0, seen)
  | depth + 1, attempts + 1, oracle, seen =>
      match tree.walk distinct oracle [] with
      | (.vacuous σ current prophecy, trail) =>
          let writing (position : Nat) (value : σ.carrier) :=
            (oracle ++ Array.replicate (position + 1 - oracle.size) none).set! position
              (some ⟨σ, value⟩)
          -- Those not yet written come first.
          let (fresh, written) := (holding σ prophecy trail).partition fun position =>
            (oracle[position]?.join).isNone
          let attempts' := (fresh ++ written).map (·, current) ++
            (holding σ current trail).map (·, prophecy)
          attempts'.foldl (init := (.vacuous σ current prophecy, attempts, seen))
            fun found (position, value) =>
              match found with
              | (.vacuous _ _ _, attempts, seen) | (.unknown _, attempts, seen) =>
                  let candidate := writing position value
                  if seen.contains (oracleKey candidate) then found
                  else tree.solveWith distinct depth attempts candidate (oracleKey candidate :: seen)
              | other => other
      | (outcome, _) => (outcome, attempts, seen)

/-- A tree's outcome under the prophecies `solveWith` finds. A prophecy nothing
resolves is a returned reference's, the caller's to choose: `distinct` off starts it
at the value the reference holds, to compare the result with the runtime's. -/
def Tree.solve {α : Type} (tree : Tree α) (budget : Nat) (distinct : Bool := true) : TreeOutcome α :=
  (tree.solveWith distinct budget budget #[] []).1

end Run


/-! ## A tree means its computation -/

section SpecLaws
variable {σ₀ : Type}

theorem Spec.stuck_bind {α β : Type} (next : α → Spec σ₀ Failure β) :
    Spec.bind Spec.stuck next = Spec.stuck := by
  apply Spec.extensionality
  · funext initial value final
    simp [Spec.bind, Spec.stuck]
  · funext initial actual
    simp [Spec.bind, Spec.stuck]
  · funext initial
    simp [Spec.bind, Spec.stuck]

theorem Spec.bottom_bind {α β : Type} (next : α → Spec σ₀ Failure β) :
    Spec.bind Spec.bottom next = Spec.bottom := by
  apply Spec.extensionality
  · funext initial value final
    simp [Spec.bind, Spec.bottom]
  · funext initial actual
    simp [Spec.bind, Spec.bottom]
  · funext initial
    simp [Spec.bind, Spec.bottom]

theorem Spec.assume_bind {α : Type} {proposition : Prop} (holds : proposition)
    (rest : Spec σ₀ Failure α) :
    Spec.bind (Spec.assume proposition) (fun _ => rest) = rest := by
  apply Spec.extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨_, _, ⟨_, _, rfl⟩, held⟩
      exact held
    · exact fun held => ⟨(), initial, ⟨holds, rfl, rfl⟩, held⟩
  · funext initial error
    simp [Spec.bind, Spec.assume, holds]
  · funext initial
    simp [Spec.bind, Spec.assume, holds]

end SpecLaws

section Means
variable [Carriers] {σ₀ : Type}

theorem Tree.Means.bind {α β : Type} {next : α → Tree β} {after : α → Spec σ₀ Failure β}
    (following : ∀ value, (next value).Means (after value)) :
    (tree : Tree α) → (spec : Spec σ₀ Failure α) → tree.Means spec →
      (tree.bind next).Means (Spec.bind spec after)
  | .ok value, _, means => by
      cases (means : _ = Spec.pure value)
      show (next value).Means _
      rw [Spec.pure_bind]
      exact following value
  | .abort failure, _, means => by
      cases (means : _ = Spec.abort failure)
      show _ = _
      rw [Spec.abort_bind]
  | .stuck, _, means => by
      cases (means : _ = Spec.stuck)
      show _ = _
      rw [Spec.stuck_bind]
  | .bottom, _, means => by
      cases (means : _ = Spec.bottom)
      show _ = _
      rw [Spec.bottom_bind]
  | .unknown _, _, _ => True.intro
  | .choose _ _ rest, _, ⟨continued, same, means⟩ => by
      cases same
      exact ⟨fun value => Spec.bind (continued value) after, Spec.bind_assoc _ _ _,
        fun value => Tree.Means.bind following (rest value) _ (means value)⟩
  | .assumeEq _ _ _ rest, _, ⟨continued, same, means⟩ => by
      cases same
      exact ⟨Spec.bind continued after, Spec.bind_assoc _ _ _,
        Tree.Means.bind following rest _ means⟩

theorem Tree.means_ok {α : Type} (value : α) :
    (Tree.ok value).Means (Spec.pure value : Spec σ₀ Failure α) := rfl

theorem Tree.means_abort {α : Type} (failure : Failure) :
    (Tree.abort failure : Tree α).Means (Spec.abort failure : Spec σ₀ Failure α) := rfl

theorem Tree.means_stuck {α : Type} :
    (Tree.stuck : Tree α).Means (Spec.stuck : Spec σ₀ Failure α) := rfl

theorem Tree.means_bottom {α : Type} :
    (Tree.bottom : Tree α).Means (Spec.bottom : Spec σ₀ Failure α) := rfl

theorem Tree.means_bind {α β : Type} {tree : Tree α} {spec : Spec σ₀ Failure α}
    {next : α → Tree β} {after : α → Spec σ₀ Failure β} (means : tree.Means spec)
    (following : ∀ value, (next value).Means (after value)) :
    (tree.bind next).Means (Spec.bind spec after) :=
  Tree.Means.bind following tree spec means

theorem Tree.means_choose {α : Type} {σ : NTy} {current : σ.carrier} {next : σ.carrier → Tree α}
    {rest : σ.carrier → Spec σ₀ Failure α} (means : ∀ value, (next value).Means (rest value)) :
    (Tree.choose σ current next).Means (Spec.bind Spec.choose rest) :=
  ⟨rest, rfl, means⟩

theorem Tree.means_ite {α : Type} (condition : Prop) [Decidable condition]
    {thenTree elseTree : Tree α} {thenSpec elseSpec : Spec σ₀ Failure α}
    (holds : thenTree.Means thenSpec) (fails : elseTree.Means elseSpec) :
    Tree.Means (if condition then thenTree else elseTree)
      (if condition then thenSpec else elseSpec) := by
  by_cases held : condition
  · simpa only [if_pos held] using holds
  · simpa only [if_neg held] using fails

theorem Tree.means_dite {α : Type} (condition : Prop) [Decidable condition]
    {thenTree : condition → Tree α} {elseTree : ¬condition → Tree α}
    {thenSpec : condition → Spec σ₀ Failure α} {elseSpec : ¬condition → Spec σ₀ Failure α}
    (holds : ∀ held, (thenTree held).Means (thenSpec held))
    (fails : ∀ held, (elseTree held).Means (elseSpec held)) :
    Tree.Means (if held : condition then thenTree held else elseTree held)
      (if held : condition then thenSpec held else elseSpec held) := by
  by_cases held : condition
  · simpa only [dif_pos held] using holds held
  · simpa only [dif_neg held] using fails held

theorem Tree.means_assume {α : Type} {σ : NTy} {current prophecy : σ.carrier} {next : Tree α}
    {rest : Spec σ₀ Failure α} (means : next.Means rest) :
    (Tree.assumeEq σ current prophecy next).Means
      (Spec.bind (Spec.assume (current = prophecy)) fun _ => rest) :=
  ⟨rest, rfl, means⟩

/-- Every outcome a walk reaches is one of the computation's. -/
theorem Tree.walk_sound {α : Type} (distinct : Bool) (oracle : Array (Option Chosen)) (state : σ₀) :
    (tree : Tree α) → (spec : Spec σ₀ Failure α) → tree.Means spec → (trail : List Chosen) →
    match (tree.walk distinct oracle trail).1 with
    | .ok value => spec.ok state value state
    | .abort failure => spec.aborts state failure
    | .stuck => spec.undefined state
    | .bottom => True
    | .unknown _ => True
    | .vacuous _ _ _ => True
  | .ok value, _, means, _ => by
      cases (means : _ = Spec.pure value)
      exact ⟨rfl, rfl⟩
  | .abort failure, _, means, _ => by
      cases (means : _ = Spec.abort failure)
      exact rfl
  | .stuck, _, means, _ => by
      cases (means : _ = Spec.stuck)
      exact True.intro
  | .bottom, _, _, _ => True.intro
  | .unknown _, _, _, _ => True.intro
  | .choose σ current next, _, ⟨continued, same, means⟩, trail => by
      cases same
      simp only [Tree.walk]
      generalize (match oracle[trail.length]?.join with
        | some chosen => if same : chosen.σ = σ then same ▸ chosen.value else current
        | none => if distinct then σ.searchStart current trail.length else current) = value
      have rest := Tree.walk_sound distinct oracle state (next value) _ (means value) (trail ++ [⟨σ, value⟩])
      revert rest
      cases (Tree.walk distinct oracle (next value) (trail ++ [⟨σ, value⟩])).1 with
      | ok result => exact fun step => ⟨value, state, rfl, step⟩
      | abort failure => exact fun step => Or.inr ⟨value, state, rfl, step⟩
      | stuck => exact fun step => Or.inr ⟨value, state, rfl, step⟩
      | bottom => exact fun _ => True.intro
      | unknown _ => exact fun _ => True.intro
      | vacuous _ _ _ => exact fun _ => True.intro
  | .assumeEq σ current prophecy next, _, ⟨continued, same, means⟩, trail => by
      cases same
      simp only [Tree.walk]
      cases decision : NTy.decEq σ current prophecy with
      | isFalse _ => exact True.intro
      | isTrue holds =>
          have rest := Tree.walk_sound distinct oracle state next _ means trail
          revert rest
          cases (Tree.walk distinct oracle next trail).1 with
          | ok result => exact fun step => ⟨(), state, ⟨holds, rfl, rfl⟩, step⟩
          | abort failure => exact fun step => Or.inr ⟨(), state, ⟨holds, rfl, rfl⟩, step⟩
          | stuck => exact fun step => Or.inr ⟨(), state, ⟨holds, rfl, rfl⟩, step⟩
          | bottom => exact fun _ => True.intro
          | unknown _ => exact fun _ => True.intro
          | vacuous _ _ _ => exact fun _ => True.intro

end Means

section Term
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- What a call of a monomorphic callee computes, or of a callee at type arguments
holding a mutable reference, at its generic arguments and them. -/
abbrev DirectCallTree : Type :=
  FunctionHandle → Option (Array GenericArgument × TypeArgs) → (σs : NRow) →
    (shape : ResultShape) → HList σs → Tree shape.carrier

/-- Trees for direct calls, generic calls under their induced family, and
invocation of a function value. -/
structure CallTree [Θ : Skolems unit] where
  typeInstantiation : Array (TypeId × TypeId) := #[]
  direct : DirectCallTree (unit := unit)
  generic : FunctionHandle → Array GenericArgument → (θ : TypeArgs) →
    (σs : NRow) → (shape : ResultShape) →
    @HList (Skolems.instantiate θ Θ).toCarriers σs →
    Tree (@ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)
  invoke : (σs : NRow) → (shared : Sharing) → (shape : ResultShape) →
    (NTy.function σs shared shape.row).carrier → HList σs → Tree shape.carrier

instance : CoeFun (CallTree (unit := unit)) (fun _ => DirectCallTree (unit := unit)) :=
  ⟨CallTree.direct⟩

/-- What a call means, by the same reading. -/
def Meanings.callAt {executable : ExecutableUnit unit} (meanings : Meanings executable)
    (handle : FunctionHandle) :
    Option (Array GenericArgument × TypeArgs) → (σs : NRow) → (shape : ResultShape) → HList σs →
      Comp unit shape.carrier
  | none => meanings.call handle
  | some (typeArgs, θ) => meanings.atInstance handle typeArgs θ

/-- Each callback agrees with the corresponding denotation callback. -/
structure CallTree.Means {executable : ExecutableUnit unit}
    (calls : CallTree (unit := unit)) (meanings : Meanings executable) : Prop where
  frame : calls.typeInstantiation = meanings.typeInstantiation
  direct : ∀ handle substitution σs shape args,
    (calls handle substitution σs shape args).Means
      (meanings.callAt handle substitution σs shape args)
  generic : ∀ handle typeArgs θ σs shape args,
    (calls.generic handle typeArgs θ σs shape args).Means
      (meanings.generic handle typeArgs θ σs shape args)
  invoke : ∀ σs shared shape function args,
    (calls.invoke σs shared shape function args).Means
      (meanings.invoke σs shared shape function args)

def Tree.flowBind {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (action : Tree (Flow ρ Γ α)) (next : α → HEnv Γ → Tree (Flow ρ Γ β)) : Tree (Flow ρ Γ β) :=
  action.bind fun flow =>
    match flow with
    | .value v env => next v env
    | .return_ result env => .ok (.return_ result env)
    | .break_ nest env => .ok (.break_ nest env)
    | .continue_ nest env => .ok (.continue_ nest env)

def checkedIntTree (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int) :
    Tree (SpecInt (.bits width) signed) :=
  if fits : IntegerValueFits (.bits width) signed value then .ok ⟨value, fits⟩
  else match (Ty.integer (.bits width) signed).integerBounds? with
    | some _ => .abort (Failure.ofInts failure #[value])
    | none => .abort (Failure.ofInts failure #[])

def checkedOpTree (op : CheckedOp) (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) : Tree (SpecInt (.bits width) signed) :=
  match op with
  | .add => checkedIntTree failure width signed (left.val + right.val)
  | .subtract => checkedIntTree failure width signed (left.val - right.val)
  | .multiply => checkedIntTree failure width signed (left.val * right.val)
  | .divide =>
      if right.val = 0 then .abort (Failure.ofInts failure #[])
      else checkedIntTree failure width signed (left.val.tdiv right.val)
  | .modulo =>
      if right.val = 0 then .abort (Failure.ofInts failure #[])
      else (checkedIntTree failure width signed (left.val.tdiv right.val)).bind fun _ =>
        checkedIntTree failure width signed (left.val.tmod right.val)

def shiftLeftTree (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false) :
    Tree (SpecInt (.bits width) false) :=
  if distance.val < width then
    .ok (SpecInt.ofBounds value (Int.shiftLeft value.val distance.val.toNat % 2 ^ width)
      (shiftLeft_mod_bounds _ _ value.width_nonzero))
  else .abort (Failure.ofInts failure #[distance.val])

def shiftRightTree (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false) :
    Tree (SpecInt (.bits width) false) :=
  if distance.val < width then
    .ok (SpecInt.ofBounds value (Int.shiftRight value.val distance.val.toNat)
      (shiftRight_bounds value _))
  else .abort (Failure.ofInts failure #[distance.val])

def mismatchTree {α : Type} : Option Failure → Tree α
  | some failure => .abort failure
  | none => .stuck

/-- The iterations of a loop, up to `fuel`. -/
def loopTree {ρ : ResultShape} {Γ : NRow}
    (iteration : HEnv Γ → Tree (Flow ρ Γ Unit)) : Nat → HEnv Γ → Tree (Flow ρ Γ Unit)
  | 0, _ => .unknown "loop fuel"
  | fuel + 1, env =>
      (iteration env).bind fun flow =>
        match flow with
        | .value _ env => loopTree iteration fuel env
        | .continue_ 0 env => loopTree iteration fuel env
        | .continue_ (nest + 1) env => .ok (.continue_ nest env)
        | .break_ 0 env => .ok (.value () env)
        | .break_ (nest + 1) env => .ok (.break_ nest env)
        | .return_ result env => .ok (.return_ result env)

set_option maxHeartbeats 800000 in
mutual
/-- `Term.denote` as a tree, read case by case against it. -/
def Term.tree (calls : CallTree (unit := unit)) (fuel : Nat) {ρ : ResultShape} :
    {Γ : NRow} → {τ : NTy} → Term unit ρ Γ τ → HEnv Γ → Tree (Flow ρ Γ τ.carrier)
  | _, _, .lit value, env => .ok (.value (NTy.ofGround _ value) env)
  | _, _, .var x, env =>
      match x.get env with
      | some value => .ok (.value value env)
      | none => .bottom
  | _, _, .checked _ op failure left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          (checkedOpTree op failure l r).bind fun value => .ok (.value value env)
  | _, _, .modular op left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env => .ok (.value (op.run l r) env)
  | _, _, .compare op left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          .ok (.value (op.decide l.val r.val) env)
  | _, _, .equal negated left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          .ok (.value (if negated then !NTy.eqb _ l r else NTy.eqb _ l r) env)
  | _, _, .equalErased .., _ => .unknown "equalErased"
  | _, _, .not operand, env =>
      Tree.flowBind (operand.tree calls fuel env) fun b env => .ok (.value (!(b : Bool)) env)
  | _, _, .logical conjunction left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          .ok (.value (if conjunction then (l : Bool) && r else (l : Bool) || r) env)
  | _, _, .bitwise op left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env => .ok (.value (op.run l r) env)
  | _, _, .shift _ left failure value distance, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        Tree.flowBind (distance.tree calls fuel env) fun d env =>
          (if left then shiftLeftTree failure v d else shiftRightTree failure v d).bind fun result =>
            .ok (.value result env)
  | _, .int width signed, .shiftSigned _ left failure value distance, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        Tree.flowBind (distance.tree calls fuel env) fun d env =>
          if 0 ≤ d.val ∧ d.val < (width : Int) then
            .ok (.value ⟨wrapInt width signed (if left then Int.shiftLeft v.val d.val.toNat
              else Int.shiftRight v.val d.val.toNat), wrapInt_fits _ _ _ v.width_nonzero⟩ env)
          else .abort (Failure.ofInts failure #[d.val])
  | _, .int width signed, .shiftUnchecked _ left value distance, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        Tree.flowBind (distance.tree calls fuel env) fun d env =>
          if 0 ≤ d.val ∧ d.val < (width : Int) then
            .ok (.value ⟨wrapInt width signed (if left then Int.shiftLeft v.val d.val.toNat
              else Int.shiftRight v.val d.val.toNat), wrapInt_fits _ _ _ v.width_nonzero⟩ env)
          else .stuck
  | _, .int width signed, .divideUnchecked _ modulo left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          if r.val = 0 then .stuck
          else .ok (.value ⟨wrapInt width signed (if modulo then l.val.tmod r.val else l.val.tdiv r.val),
            wrapInt_fits _ _ _ l.width_nonzero⟩ env)
  | _, _, .overflowing op left right, env =>
      Tree.flowBind (left.tree calls fuel env) fun l env =>
        Tree.flowBind (right.tree calls fuel env) fun r env =>
          .ok (.value ((op.run l r, (op.overflows l r, ())) :
            HList (.cons (.int _ _) (.cons .bool .nil))) env)
  | _, .int width' signed', .wrap nonzero value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        .ok (.value ⟨wrapInt width' signed' v.val, wrapInt_fits _ _ _ nonzero⟩ env)
  | _, _, .charCode value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value (SpecChar.code v) env)
  | _, _, .charOf _ value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        if valid : 0 ≤ v.val ∧ isUnicodeScalar v.val.toNat = true then
          .ok (.value ⟨v.val.toNat, valid.2⟩ env)
        else .stuck
  | _, _, .charOfChecked _ failure value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        if valid : 0 ≤ v.val ∧ isUnicodeScalar v.val.toNat = true then
          .ok (.value ⟨v.val.toNat, valid.2⟩ env)
        else .abort (Failure.ofInts failure #[v.val])
  | _, _, .stringLength value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        .ok (.value ⟨wrapInt 64 false (v : String).utf8ByteSize,
          wrapInt_fits _ _ _ (by decide)⟩ env)
  | _, _, .unreachable, _ => .bottom
  | _, .int width' signed', .cast _ failure value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        (checkedIntTree failure width' signed' v.val).bind fun result => .ok (.value result env)
  | _, _, .ite _ condition thenBranch elseBranch, env =>
      Tree.flowBind (condition.tree calls fuel env) fun b env =>
        if (b : Bool) then thenBranch.tree calls fuel env else elseBranch.tree calls fuel env
  | _, _, .let_ x value body, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => body.tree calls fuel (x.set v env)
  | _, _, .drop value body, env =>
      Tree.flowBind (value.tree calls fuel env) fun _ env => body.tree calls fuel env
  | _, _, .assign x value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value () (x.set v env))
  | _, _, .const value, env =>
      (value.tree calls fuel ()).bind fun flow => .ok (flow.rebase env)
  | _, _, .throw0 _ kind, _ => .abort (Failure.ofInts kind #[])
  | _, _, .throw1 _ kind code, env =>
      Tree.flowBind (code.tree calls fuel env) fun value _ =>
        .abort (Failure.ofInts kind #[value.val])
  | _, _, .return_ value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.return_ (ResultShape.ofBody _ v) env)
  | _, _, .break_ nest, env => .ok (.break_ nest env)
  | _, _, .continue_ nest, env => .ok (.continue_ nest env)
  | _, _, .loop _ body, env => loopTree (fun env => body.tree calls fuel env) fuel env
  | _, _, .call _ handle shape arguments, env =>
      Tree.flowBind (arguments.tree calls fuel env) fun values env =>
        (calls handle none _ shape values).bind fun result =>
          .ok (.value (shape.toBody result) env)
  | _, _, .callGeneric _ handle typeArgs θ shape shape' resolved arguments, env =>
      Tree.flowBind (arguments.tree calls fuel env) fun values env =>
        (calls.generic handle typeArgs θ _ shape
          (HList.toSkolem θ _ (cast (congrArg HList (resolvedCall resolved).1.symm) values))).bind
          fun result => .ok (.value (shape'.toBody
            (cast (congrArg ResultShape.carrier (resolvedCall resolved).2)
              (ResultShape.ofSkolem θ shape result))) env)
  | _, _, .callInstance _ handle typeArgs θ shape arguments, env =>
      Tree.flowBind (arguments.tree calls fuel env) fun values env =>
        (calls handle (some (typeArgs, θ)) _ shape values).bind fun result =>
          .ok (.value (shape.toBody result) env)
  | _, _, .closure (supplied := supplied) handle weave results shared rows sharing closed faithful
      captures, env =>
      Tree.flowBind (captures.tree calls fuel env) fun values env =>
        .ok (.value (Skolems.construct supplied shared results handle weave.mask #[] values
          (SiteTyped.ofClosure rows sharing closed faithful)) env)
  | _, _, .closureGeneric handle weave typeArgs θ shape _ shared rows sharing below resolved captures, env =>
      Tree.flowBind (captures.tree calls fuel env) fun values env =>
        let instantiation := frameInstantiation unit handle calls.typeInstantiation typeArgs
        if coherent : @coherentCheck unit _ (Skolems.instantiate θ ‹_›) handle instantiation = true then
          if faithful : closureFaithful unit handle instantiation = true then
            .ok (.value (cast (congrArg NTy.carrier (resolvedClosure (shared := shared) resolved).2)
              (functionOfGeneric unit calls.typeInstantiation typeArgs θ weave shape shared handle
                (cast (congrArg HList (resolvedClosure (shared := shared) resolved).1.symm) values)
                (closureOfGeneric_typed calls.typeInstantiation typeArgs θ weave shape rows sharing below
                  ⟨@Coherent.ofCheck unit _ (Skolems.instantiate θ ‹_›) handle _ coherent, faithful⟩))) env)
          else .unknown "unfaithful generic closure"
        else .unknown "incoherent generic closure"
  | _, _, .invoke _ shape function arguments, env =>
      Tree.flowBind (function.tree calls fuel env) fun closure env =>
        Tree.flowBind (arguments.tree calls fuel env) fun values env =>
          (calls.invoke _ _ shape closure values).bind fun result =>
            .ok (.value (shape.toBody result) env)
  | _, _, .tuple elements, env =>
      Tree.flowBind (elements.tree calls fuel env) fun values env => .ok (.value values env)
  | _, _, .pack _ _ fields, env =>
      Tree.flowBind (fields.tree calls fuel env) fun values env => .ok (.value values env)
  | _, _, .variant _ _ _ choice fields, env =>
      Tree.flowBind (fields.tree calls fuel env) fun values env =>
        .ok (.value (choice.inject values) env)
  | _, _, .field x value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value (x.select v) env)
  | _, _, .isVariant tests value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        .ok (.value (namedIn (variantName _ _ v) tests) env)
  | _, _, .payload mismatch choices value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        match choices.select? v with
        | some field => .ok (.value field env)
        | none => mismatchTree mismatch
  | _, _, .letRow targets value body, env =>
      Tree.flowBind (value.tree calls fuel env) fun values env =>
        body.tree calls fuel (targets.set values env)
  | _, _, .letFields targets value body, env =>
      Tree.flowBind (value.tree calls fuel env) fun values env =>
        body.tree calls fuel (targets.set values env)
  | _, _, .caseOf scrutinee arms, env =>
      Tree.flowBind (scrutinee.tree calls fuel env) fun value env => arms.tree calls fuel value env
  | _, _, .deref value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value v.1 env)
  | _, _, .take x, env =>
      match x.get env with
      | some value => .ok (.value value (x.clear env))
      | none => .bottom
  | _, _, @Term.resolve _ _ _ τ x, env =>
      match x.get env with
      | some reference => .assumeEq τ reference.1 reference.2 (.ok (.value () env))
      | none => .ok (.value () env)
  | _, _, @Term.resolveAt _ _ _ _ σ x path, env =>
      match (x.get env).bind (path.get? env) with
      | some reference => .assumeEq σ reference.1 reference.2 (.ok (.value () env))
      | none => .ok (.value () env)
  | _, _, .mutate x value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        match x.get env with
        | some current => .ok (.value () (x.set (v, current.2) env))
        | none => .bottom
  | _, _, @Term.observe _ _ _ τ value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value (τ.observe v) env)
  | _, _, .readPlace mismatch x path, env =>
      match x.get env with
      | some value =>
          match path.get? env value with
          | some component => .ok (.value component env)
          | none => mismatchTree mismatch
      | none => .bottom
  | _, _, .takeAt mismatch x path, env =>
      match x.get env with
      | some value =>
          match path.get? env value with
          | some reference =>
              match path.set? env (reference.2, reference.2) value with
              | some cleared => .ok (.value reference (x.set cleared env))
              | none => mismatchTree mismatch
          | none => mismatchTree mismatch
      | none => .bottom
  | _, _, .writePlace mismatch x path value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        match x.get env with
        | some current =>
            match path.set? env v current with
            | some updated => .ok (.value () (x.set updated env))
            | none => mismatchTree mismatch
        | none => .bottom
  | _, _, @Term.borrowPlace _ _ _ _ σ mismatch x path, env =>
      match x.get env with
      | some value =>
          match path.get? env value with
          | some component =>
              .choose σ component fun prophecy =>
                match path.set? env prophecy value with
                | some lent => .ok (.value (component, prophecy) (x.set lent env))
                | none => .bottom
          | none => mismatchTree mismatch
      | none => .bottom
  | _, _, .sliceVector value, env | _, _, .vectorSlice value, env =>
      Tree.flowBind (value.tree calls fuel env) fun value env => .ok (.value value env)
  | _, _, .sliceCurrent value, env =>
      Tree.flowBind (value.tree calls fuel env) fun value env =>
        .ok (.value value.currentVector env)
  | _, _, @Term.borrowArraySlice _ _ _ _ σ length mismatch x path, env =>
      match x.get env with
      | some value => match path.get? env value with
        | some current =>
            .choose (.array σ length) current fun final =>
              match path.set? env final value with
              | some lent => .ok (.value (SpecSliceRef.ofArrays current final) (x.set lent env))
              | none => .bottom
        | none => mismatchTree mismatch
      | none => .bottom
  | _, _, @Term.borrowVectorSlice _ _ _ _ σ mismatch x path, env =>
      match x.get env with
      | some value => match path.get? env value with
        | some current =>
            .choose (.array σ current.values.size) ⟨current, rfl⟩ fun final =>
              match path.set? env final.vector value with
              | some lent => .ok (.value
                  (SpecSliceRef.ofArrays ⟨current, rfl⟩ final) (x.set lent env))
              | none => .bottom
        | none => mismatchTree mismatch
      | none => .bottom
  | _, _, @Term.reborrowSlice _ _ _ _ σ mismatch x path, env =>
      match x.get env with
      | some value => match path.get? env value with
        | some current =>
            .choose (.array σ current.extent) current.current fun final =>
              match path.set? env (current.withCurrent final) value with
              | some lent => .ok (.value
                  (SpecSliceRef.ofArrays current.current final) (x.set lent env))
              | none => .bottom
        | none => mismatchTree mismatch
      | none => .bottom
  | _, _, .takeSliceAt mismatch x path, env =>
      match x.get env with
      | some value => match path.get? env value with
        | some reference =>
            match path.set? env (SpecSliceRef.ofArrays reference.final reference.final) value with
            | some cleared => .ok (.value reference (x.set cleared env))
            | none => mismatchTree mismatch
        | none => mismatchTree mismatch
      | none => .bottom
  | _, _, @Term.resolveSliceAt _ _ _ _ σ x path, env =>
      match (x.get env).bind (path.get? env) with
      | some reference => .assumeEq (.array σ reference.extent) reference.current reference.final
          (.ok (.value () env))
      | none => .ok (.value () env)
  | _, _, .vectorLit count elements, env =>
      Tree.flowBind (elements.tree calls fuel env) fun values env =>
        match SpecVector.ofArray? (HList.toListOf count values).toArray with
        | some vector => .ok (.value vector env)
        | none => .bottom
  | _, _, .arrayVector array, env =>
      Tree.flowBind (array.tree calls fuel env) fun a env => .ok (.value a.vector env)
  | _, .array _ length, .arrayOf vector, env =>
      Tree.flowBind (vector.tree calls fuel env) fun values env =>
        if sized : values.values.size = length then .ok (.value ⟨values, sized⟩ env)
        else .stuck
  | _, _, .repeatVector count value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        match SpecVector.ofArray? (Array.replicate count v) with
        | some vector => .ok (.value vector env)
        | none => .stuck
  | _, _, .length vector, env =>
      Tree.flowBind (vector.tree calls fuel env) fun values env =>
        .ok (.value (vectorLength values) env)
  | _, _, .signerAddress .., _ => .unknown "signerAddress"
  | _, _, .index _ vector position, env =>
      Tree.flowBind (vector.tree calls fuel env) fun values env =>
        Tree.flowBind (position.tree calls fuel env) fun i env =>
          if i.val < 0 then .abort (Failure.ofInts .abort #[i.val])
          else match values.values[i.val.toNat]? with
            | some element => .ok (.value element env)
            | none => .abort (Failure.ofInts .abort #[i.val])
  | _, _, .checkIndex _ failure vector position, env =>
      Tree.flowBind (vector.tree calls fuel env) fun values env =>
        Tree.flowBind (position.tree calls fuel env) fun i env =>
          if 0 ≤ i.val ∧ i.val < values.values.size then .ok (.value () env)
          else .abort (Failure.ofInts failure #[1])
  | _, _, .push .., _ => .unknown "push"
  | _, _, .insert .., _ => .unknown "insert"
  | _, _, .remove .., _ => .unknown "remove"
  | _, _, .swap .., _ => .unknown "swap"
  | _, _, .concat .., _ => .unknown "concat"
  | _, _, .slice _ vector start stop, env =>
      Tree.flowBind (vector.tree calls fuel env) fun values env =>
        Tree.flowBind (start.tree calls fuel env) fun a env =>
          Tree.flowBind (stop.tree calls fuel env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              .abort (Failure.ofInts .abort #[a.val, b.val])
            else match SpecVector.ofArray? (values.values.extract a.val.toNat b.val.toNat) with
              | some vector => .ok (.value vector env)
              | none => .bottom
  | _, _, .reverseSlice .., _ => .unknown "reverseSlice"
  | _, _, .destroyEmpty .., _ => .unknown "destroyEmpty"
  | _, _, .contains .., _ => .unknown "contains"
  | _, _, .indexOf .., _ => .unknown "indexOf"
  | _, _, .order .., _ => .unknown "order"
  | _, _, .seqAfter value effect, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env =>
        Tree.flowBind (effect.tree calls fuel env) fun _ env => .ok (.value v env)
  | _, _, .assertion _, env => .ok (.value () env)
  | _, _, .anchor _, env => .ok (.value () env)
  | _, _, .assume .., _ => .unknown "assume"
  | _, _, .arbitrary, _ => .unknown "arbitrary"
  | _, _, .ghostValue .., _ => .unknown "ghostValue"
  | _, _, .ghostInit .., _ => .unknown "ghostInit"
  | _, _, .ghostSlot .., _ => .unknown "ghostSlot"
  | _, _, .ghostRead .., _ => .unknown "ghostRead"
  | _, _, .mutationEnd _, env => .ok (.value () env)
  | _, _, .memoryWritten _, env => .ok (.value () env)
  | _, _, .constructed _ value, env =>
      Tree.flowBind (value.tree calls fuel env) fun v env => .ok (.value v env)
  | _, _, .globalRead .., _ => .unknown "globalRead"
  | _, _, .globalContains .., _ => .unknown "globalContains"
  | _, _, .globalBorrow .., _ => .unknown "globalBorrow"
  | _, _, .globalPublish .., _ => .unknown "globalPublish"
  | _, _, .ghostStore .., _ => .unknown "ghostStore"
  | _, _, .globalTake .., _ => .unknown "globalTake"

def Args.tree (calls : CallTree (unit := unit)) (fuel : Nat) {ρ : ResultShape} :
    {Γ σs : NRow} → Args unit ρ Γ σs → HEnv Γ → Tree (Flow ρ Γ (HList σs))
  | _, _, .nil, env => .ok (.value () env)
  | _, _, .cons head tail, env =>
      Tree.flowBind (head.tree calls fuel env) fun value env =>
        Tree.flowBind (tail.tree calls fuel env) fun values env =>
          .ok (.value (value, values) env)

def Arms.tree (calls : CallTree (unit := unit)) (fuel : Nat) {ρ : ResultShape} :
    {Γ : NRow} → {σ τ : NTy} → Arms unit ρ Γ σ τ → σ.carrier → HEnv Γ → Tree (Flow ρ Γ τ.carrier)
  | _, _, _, .nil (some failure), _, _ => .abort failure
  | _, _, _, .nil none, _, _ => .bottom
  | _, _, _, .cons _ pattern body rest, value, env =>
      match pattern.bindValue value env with
      | some bound => body.tree calls fuel bound
      | none => rest.tree calls fuel value env
  | _, _, _, .guarded .., _, _ => .unknown "a guarded arm"
end

/-- `Mutables.resolve` as a tree. -/
def resolveTree {Γ : NRow} {α : Type} : Mutables Γ → HEnv Γ → Tree α → Tree α
  | .nil, _, next => next
  | @Mutables.cons _ τ x rest, env, next =>
      match x.get env with
      | some reference => .assumeEq τ reference.1 reference.2 (resolveTree rest env next)
      | none => resolveTree rest env next
  | @Mutables.at _ _ σ x path rest, env, next =>
      match (x.get env).bind (path.get? env) with
      | some reference => .assumeEq σ reference.1 reference.2 (resolveTree rest env next)
      | none => resolveTree rest env next
  | @Mutables.sliceAt _ _ σ x path rest, env, next =>
      match (x.get env).bind (path.get? env) with
      | some reference => .assumeEq (.array σ reference.extent) reference.current reference.final
          (resolveTree rest env next)
      | none => resolveTree rest env next

/-- `ResultShape.finish` as a tree. -/
def finishTree {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape) :
    Flow shape Γ shape.bodyType.carrier → Tree shape.carrier
  | .value value env => resolveTree mutables env (.ok (shape.ofBody value))
  | .return_ result env => resolveTree mutables env (.ok result)
  | .break_ _ _ => .bottom
  | .continue_ _ _ => .bottom

/-- `Function.denoteWith` as a tree. -/
def Function.tree (calls : CallTree (unit := unit)) (fuel : Nat) (f : Function unit)
    (args : HList f.params) : Tree f.result.carrier :=
  let body := (f.body.tree calls fuel (initialEnv f.params f.locals f.frame_eq args)).bind
    (finishTree f.mutables f.result)
  match f.scopedMethods with
  | [] => body
  | _ :: _ =>
      if scopedValues unit f.scopedMethods (HList.encode args).toArray then body else .stuck

end Term


/-! ## A term's tree means its denotation -/

section TermMeans
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

omit [Skolems unit] in
theorem branchIn_eq {α : Type} (site : Option Nat) (taken : Bool) (meaning : Comp unit α) :
    branchIn site taken meaning = meaning := by
  cases site <;> rfl

omit [Skolems unit] in
theorem armIn_eq {α : Type} (arm : Option (Nat × String)) (meaning : Comp unit α) :
    armIn arm meaning = meaning := by
  cases arm <;> rfl

theorem means_flowBind {ρ : ResultShape} {Γ : NRow} {α β : Type}
    {tree : Tree (Flow ρ Γ α)} {spec : Comp unit (Flow ρ Γ α)}
    {next : α → HEnv Γ → Tree (Flow ρ Γ β)} {after : α → HEnv Γ → Comp unit (Flow ρ Γ β)}
    (means : tree.Means spec) (following : ∀ value env, (next value env).Means (after value env)) :
    (Tree.flowBind tree next).Means (Flow.bind spec after) := by
  refine Tree.means_bind means fun flow => ?_
  cases flow with
  | value v env => exact following v env
  | return_ result env => exact rfl
  | break_ nest env => exact rfl
  | continue_ nest env => exact rfl

theorem checkedInt_means (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int) :
    (checkedIntTree failure width signed value).Means
      (checkedInt failure width signed value : Comp unit _) := by
  unfold checkedIntTree checkedInt
  refine Tree.means_dite _ (fun _ => rfl) fun _ => ?_
  cases (Ty.integer (.bits width) signed).integerBounds? <;> rfl

theorem checkedOp_means (op : CheckedOp) (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) :
    (checkedOpTree op failure left right).Means (op.run failure left right : Comp unit _) := by
  cases op with
  | add => exact checkedInt_means ..
  | subtract => exact checkedInt_means ..
  | multiply => exact checkedInt_means ..
  | divide => exact Tree.means_ite _ (Tree.means_abort _) (checkedInt_means ..)
  | modulo =>
      exact Tree.means_ite _ (Tree.means_abort _)
        (Tree.means_bind (checkedInt_means ..) fun _ => checkedInt_means ..)

theorem mismatch_means {α : Type} (mismatch : Option Failure) :
    (mismatchTree mismatch : Tree α).Means (mismatchOutcome mismatch : Comp unit α) := by
  cases mismatch <;> rfl

/-- A loop's fixed point unfolds once. -/
theorem loop_unfold {ρ : ResultShape} {Γ : NRow} (body : HEnv Γ → Comp unit (Flow ρ Γ Unit))
    (env : HEnv Γ) :
    Spec.fix (fun recurse env => Spec.bind (body env) (Flow.iterate recurse)) env =
      Spec.bind (body env) (Flow.iterate
        (Spec.fix fun recurse env => Spec.bind (body env) (Flow.iterate recurse))) := by
  let step : (HEnv Γ → Comp unit (Flow ρ Γ Unit)) → HEnv Γ → Comp unit (Flow ρ Γ Unit) :=
    fun recurse env => Spec.bind (body env) (Flow.iterate recurse)
  have okUp : ∀ (flow : Flow ρ Γ Unit) (fuel : Nat) initial result final,
      (Flow.iterate (Spec.fixApprox step fuel) flow).ok initial result final →
        (Flow.iterate (Spec.fix step) flow).ok initial result final := by
    intro flow fuel initial result final held
    match flow, held with
    | .value _ _, held => exact ⟨fuel, held⟩
    | .continue_ 0 _, held => exact ⟨fuel, held⟩
    | .continue_ (_ + 1) _, held => exact held
    | .break_ 0 _, held => exact held
    | .break_ (_ + 1) _, held => exact held
    | .return_ _ _, held => exact held
  have okDown : ∀ (flow : Flow ρ Γ Unit) initial result final,
      (Flow.iterate (Spec.fix step) flow).ok initial result final →
        ∃ fuel, (Flow.iterate (Spec.fixApprox step fuel) flow).ok initial result final := by
    intro flow initial result final held
    match flow, held with
    | .value _ _, held => exact held
    | .continue_ 0 _, held => exact held
    | .continue_ (_ + 1) _, held => exact ⟨0, held⟩
    | .break_ 0 _, held => exact ⟨0, held⟩
    | .break_ (_ + 1) _, held => exact ⟨0, held⟩
    | .return_ _ _, held => exact ⟨0, held⟩
  have abortsUp : ∀ (flow : Flow ρ Γ Unit) (fuel : Nat) initial error,
      (Flow.iterate (Spec.fixApprox step fuel) flow).aborts initial error →
        (Flow.iterate (Spec.fix step) flow).aborts initial error := by
    intro flow fuel initial error held
    match flow, held with
    | .value _ _, held => exact ⟨fuel, held⟩
    | .continue_ 0 _, held => exact ⟨fuel, held⟩
    | .continue_ (_ + 1) _, held => exact held
    | .break_ 0 _, held => exact held
    | .break_ (_ + 1) _, held => exact held
    | .return_ _ _, held => exact held
  have abortsDown : ∀ (flow : Flow ρ Γ Unit) initial error,
      (Flow.iterate (Spec.fix step) flow).aborts initial error →
        ∃ fuel, (Flow.iterate (Spec.fixApprox step fuel) flow).aborts initial error := by
    intro flow initial error held
    match flow, held with
    | .value _ _, held => exact held
    | .continue_ 0 _, held => exact held
    | .continue_ (_ + 1) _, held => exact ⟨0, held⟩
    | .break_ 0 _, held => exact ⟨0, held⟩
    | .break_ (_ + 1) _, held => exact ⟨0, held⟩
    | .return_ _ _, held => exact ⟨0, held⟩
  have undefinedUp : ∀ (flow : Flow ρ Γ Unit) (fuel : Nat) initial,
      (Flow.iterate (Spec.fixApprox step fuel) flow).undefined initial →
        (Flow.iterate (Spec.fix step) flow).undefined initial := by
    intro flow fuel initial held
    match flow, held with
    | .value _ _, held => exact ⟨fuel, held⟩
    | .continue_ 0 _, held => exact ⟨fuel, held⟩
    | .continue_ (_ + 1) _, held => exact held
    | .break_ 0 _, held => exact held
    | .break_ (_ + 1) _, held => exact held
    | .return_ _ _, held => exact held
  have undefinedDown : ∀ (flow : Flow ρ Γ Unit) initial,
      (Flow.iterate (Spec.fix step) flow).undefined initial →
        ∃ fuel, (Flow.iterate (Spec.fixApprox step fuel) flow).undefined initial := by
    intro flow initial held
    match flow, held with
    | .value _ _, held => exact held
    | .continue_ 0 _, held => exact held
    | .continue_ (_ + 1) _, held => exact ⟨0, held⟩
    | .break_ 0 _, held => exact ⟨0, held⟩
    | .break_ (_ + 1) _, held => exact ⟨0, held⟩
    | .return_ _ _, held => exact ⟨0, held⟩
  apply Spec.extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel =>
          obtain ⟨flow, middle, first, rest⟩ := execution
          exact ⟨flow, middle, first, okUp flow fuel _ _ _ rest⟩
    · rintro ⟨flow, middle, first, rest⟩
      obtain ⟨fuel, rest⟩ := okDown flow _ _ _ rest
      exact ⟨fuel + 1, flow, middle, first, rest⟩
  · funext initial error
    apply propext
    constructor
    · rintro ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel =>
          rcases execution with first | ⟨flow, middle, first, rest⟩
          · exact Or.inl first
          · exact Or.inr ⟨flow, middle, first, abortsUp flow fuel _ _ rest⟩
    · rintro (first | ⟨flow, middle, first, rest⟩)
      · exact ⟨1, Or.inl first⟩
      · obtain ⟨fuel, rest⟩ := abortsDown flow _ _ rest
        exact ⟨fuel + 1, Or.inr ⟨flow, middle, first, rest⟩⟩
  · funext initial
    apply propext
    constructor
    · rintro ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel =>
          rcases execution with first | ⟨flow, middle, first, rest⟩
          · exact Or.inl first
          · exact Or.inr ⟨flow, middle, first, undefinedUp flow fuel _ rest⟩
    · rintro (first | ⟨flow, middle, first, rest⟩)
      · exact ⟨1, Or.inl first⟩
      · obtain ⟨fuel, rest⟩ := undefinedDown flow _ rest
        exact ⟨fuel + 1, Or.inr ⟨flow, middle, first, rest⟩⟩

theorem loop_means {ρ : ResultShape} {Γ : NRow} {iteration : HEnv Γ → Tree (Flow ρ Γ Unit)}
    {body : HEnv Γ → Comp unit (Flow ρ Γ Unit)} (means : ∀ env, (iteration env).Means (body env)) :
    (fuel : Nat) → (env : HEnv Γ) → (loopTree iteration fuel env).Means
      (Spec.fix (fun recurse env => Spec.bind (body env) (Flow.iterate recurse)) env)
  | 0, _ => True.intro
  | fuel + 1, env => by
      rw [loop_unfold]
      refine Tree.means_bind (means env) fun flow => ?_
      match flow with
      | .value _ env => exact loop_means means fuel env
      | .continue_ 0 env => exact loop_means means fuel env
      | .continue_ (_ + 1) _ => exact rfl
      | .break_ 0 _ => exact rfl
      | .break_ (_ + 1) _ => exact rfl
      | .return_ _ _ => exact rfl

variable {executable : ExecutableUnit unit} (meanings : Meanings executable)
  (calls : CallTree (unit := unit)) (fuel : Nat)
  (callsMean : calls.Means meanings)

set_option linter.unusedSectionVars false in
include callsMean in
mutual
/-- A term's tree means its denotation, at every call meaning its tree means. -/
theorem Term.tree_means {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} → (term : Term unit ρ Γ τ) →
    (env : HEnv Γ) → (term.tree calls fuel env).Means (term.denote meanings env)
  | _, _, .lit _, _ => rfl
  | _, _, .var x, env => by
      show Tree.Means (match x.get env with
          | some value => .ok (Flow.value value env)
          | none => .bottom)
        (match x.get env with
          | some value => Spec.pure (Flow.value value env)
          | none => Spec.bottom)
      cases x.get env <;> rfl
  | _, _, .checked _ op failure left right, env =>
      means_flowBind (Term.tree_means left env) fun l env =>
        means_flowBind (Term.tree_means right env) fun r env =>
          Tree.means_bind (checkedOp_means op failure l r) fun _ => rfl
  | _, _, .modular _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, _, .compare _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, _, .equal _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, _, .equalErased .., _ => True.intro
  | _, _, .not operand, env => means_flowBind (Term.tree_means operand env) fun _ _ => rfl
  | _, _, .logical _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, _, .bitwise _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, _, .shift _ left failure value distance, env =>
      means_flowBind (Term.tree_means value env) fun v env =>
        means_flowBind (Term.tree_means distance env) fun d env =>
          Tree.means_bind
            (Tree.means_ite (left = true)
              (Tree.means_ite _ (Tree.means_ok _) (Tree.means_abort _))
              (Tree.means_ite _ (Tree.means_ok _) (Tree.means_abort _)))
            fun _ => rfl
  | _, .int _ _, .shiftSigned _ _ _ value distance, env =>
      means_flowBind (Term.tree_means value env) fun _ env =>
        means_flowBind (Term.tree_means distance env) fun _ _ =>
          Tree.means_ite _ (Tree.means_ok _) (Tree.means_abort _)
  | _, .int _ _, .shiftUnchecked _ _ value distance, env =>
      means_flowBind (Term.tree_means value env) fun _ env =>
        means_flowBind (Term.tree_means distance env) fun _ _ =>
          Tree.means_ite _ (Tree.means_ok _) Tree.means_stuck
  | _, .int _ _, .divideUnchecked _ _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ =>
          Tree.means_ite _ Tree.means_stuck (Tree.means_ok _)
  | _, _, .overflowing _ left right, env =>
      means_flowBind (Term.tree_means left env) fun _ env =>
        means_flowBind (Term.tree_means right env) fun _ _ => rfl
  | _, .int _ _, .wrap _ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .charCode value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .charOf _ value, env =>
      means_flowBind (Term.tree_means value env) fun _ _ =>
        Tree.means_dite _ (fun _ => Tree.means_ok _) fun _ => Tree.means_stuck
  | _, _, .charOfChecked _ _ value, env =>
      means_flowBind (Term.tree_means value env) fun _ _ =>
        Tree.means_dite _ (fun _ => Tree.means_ok _) fun _ => Tree.means_abort _
  | _, _, .stringLength value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .unreachable, _ => rfl
  | _, .int _ _, .cast _ failure value, env =>
      means_flowBind (Term.tree_means value env) fun v _ =>
        Tree.means_bind (checkedInt_means failure _ _ v.val) fun _ => rfl
  | _, _, .ite site condition thenBranch elseBranch, env =>
      means_flowBind (Term.tree_means condition env) fun held env => by
        cases held with
        | false =>
            show Tree.Means (elseBranch.tree calls fuel env)
              (branchIn site false (elseBranch.denote meanings env))
            rw [branchIn_eq]
            exact Term.tree_means elseBranch env
        | true =>
            show Tree.Means (thenBranch.tree calls fuel env)
              (branchIn site true (thenBranch.denote meanings env))
            rw [branchIn_eq]
            exact Term.tree_means thenBranch env
  | _, _, .let_ x value body, env =>
      means_flowBind (Term.tree_means value env) fun v env => Term.tree_means body (x.set v env)
  | _, _, .drop value body, env =>
      means_flowBind (Term.tree_means value env) fun _ env => Term.tree_means body env
  | _, _, .assign _ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .const value, _ => Tree.means_bind (Term.tree_means value ()) fun _ => rfl
  | _, _, .throw0 .., _ => rfl
  | _, _, .throw1 _ _ code, env => means_flowBind (Term.tree_means code env) fun _ _ => rfl
  | _, _, .return_ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .break_ _, _ => rfl
  | _, _, .continue_ _, _ => rfl
  | _, _, .loop _ body, env =>
      loop_means (body := fun env => body.denote meanings env)
        (fun env => Term.tree_means body env) fuel env
  | _, _, .call _ handle shape arguments, env =>
      means_flowBind (Args.tree_means arguments env) fun values _ =>
        Tree.means_bind (callsMean.direct handle none _ shape values) fun _ => rfl
  | _, _, .callGeneric _ handle typeArgs θ shape _ _ arguments, env =>
      means_flowBind (Args.tree_means arguments env) fun values _ =>
        Tree.means_bind (callsMean.generic handle typeArgs θ _ shape _) fun _ => rfl
  | _, _, .callInstance _ handle typeArgs θ shape arguments, env =>
      means_flowBind (Args.tree_means arguments env) fun values _ =>
        Tree.means_bind (callsMean.direct handle (some (typeArgs, θ)) _ shape values) fun _ => rfl
  | _, _, .closure _ _ _ _ _ _ _ _ captures, env =>
      means_flowBind (Args.tree_means captures env) fun _ _ => rfl
  | _, _, .closureGeneric handle weave typeArgs θ shape _ shared rows sharing below resolved captures, env =>
      means_flowBind (Args.tree_means captures env) fun values env => by
        dsimp only
        split
        · split
          · rename_i coherent faithful
            rw [Spec.given_of ⟨@Coherent.ofCheck unit _ (Skolems.instantiate θ ‹_›) handle _ (callsMean.frame ▸ coherent), callsMean.frame ▸ faithful⟩]
            simp only [callsMean.frame, Tree.Means]
          · exact True.intro
        · exact True.intro
  | _, _, .invoke _ shape function arguments, env =>
      means_flowBind (Term.tree_means function env) fun closure env =>
        means_flowBind (Args.tree_means arguments env) fun values _ =>
          Tree.means_bind (callsMean.invoke _ _ shape closure values) fun _ => rfl
  | _, _, .tuple elements, env => means_flowBind (Args.tree_means elements env) fun _ _ => rfl
  | _, _, .pack _ _ fields, env => means_flowBind (Args.tree_means fields env) fun _ _ => rfl
  | _, _, .variant _ _ _ _ fields, env =>
      means_flowBind (Args.tree_means fields env) fun _ _ => rfl
  | _, _, .field _ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .isVariant _ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .payload mismatch choices value, env =>
      means_flowBind (Term.tree_means value env) fun v env => by
        cases choices.select? v with
        | none => exact mismatch_means mismatch
        | some _ => rfl
  | _, _, .letRow targets value body, env =>
      means_flowBind (Term.tree_means value env) fun values env =>
        Term.tree_means body (targets.set values env)
  | _, _, .letFields targets value body, env =>
      means_flowBind (Term.tree_means value env) fun values env =>
        Term.tree_means body (targets.set values env)
  | _, _, .caseOf scrutinee arms, env =>
      means_flowBind (Term.tree_means scrutinee env) fun value env =>
        Arms.tree_means arms value env
  | _, _, .deref value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .take x, env => by
      show Tree.Means (match x.get env with
          | some value => .ok (Flow.value value (x.clear env))
          | none => .bottom)
        (match x.get env with
          | some value => Spec.pure (Flow.value value (x.clear env))
          | none => Spec.bottom)
      cases x.get env <;> rfl
  | _, _, .resolve x, env => by
      show Tree.Means (match x.get env with
          | some reference => .assumeEq _ reference.1 reference.2 (.ok (Flow.value () env))
          | none => .ok (Flow.value () env))
        (match x.get env with
          | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              Spec.pure (Flow.value () env)
          | none => Spec.pure (Flow.value () env))
      cases x.get env with
      | none => rfl
      | some _ => exact Tree.means_assume (Tree.means_ok _)
  | _, _, .resolveAt x path, env => by
      show Tree.Means (match (x.get env).bind (path.get? env) with
          | some reference => .assumeEq _ reference.1 reference.2 (.ok (Flow.value () env))
          | none => .ok (Flow.value () env))
        (match (x.get env).bind (path.get? env) with
          | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              Spec.pure (Flow.value () env)
          | none => Spec.pure (Flow.value () env))
      cases (x.get env).bind (path.get? env) with
      | none => rfl
      | some _ => exact Tree.means_assume (Tree.means_ok _)
  | _, _, .takeAt mismatch x path, env => by
      show Tree.Means (match x.get env with
          | some value =>
              match path.get? env value with
              | some reference =>
                  match path.set? env (reference.2, reference.2) value with
                  | some cleared => .ok (Flow.value reference (x.set cleared env))
                  | none => mismatchTree mismatch
              | none => mismatchTree mismatch
          | none => .bottom)
        (match x.get env with
          | some value =>
              match path.get? env value with
              | some reference =>
                  match path.set? env (reference.2, reference.2) value with
                  | some cleared => Spec.pure (Flow.value reference (x.set cleared env))
                  | none => mismatchOutcome mismatch
              | none => mismatchOutcome mismatch
          | none => Spec.bottom)
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some reference =>
              dsimp only
              cases path.set? env (reference.2, reference.2) value with
              | none => exact mismatch_means mismatch
              | some cleared => rfl
  | _, _, .mutate x value, env =>
      means_flowBind (Term.tree_means value env) fun _ env => by
        cases x.get env <;> rfl
  | _, _, .observe value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .readPlace mismatch x path, env => by
      show Tree.Means (match x.get env with
          | some value =>
              match path.get? env value with
              | some component => .ok (Flow.value component env)
              | none => mismatchTree mismatch
          | none => .bottom)
        (match x.get env with
          | some value =>
              match path.get? env value with
              | some component => Spec.pure (Flow.value component env)
              | none => mismatchOutcome mismatch
          | none => Spec.bottom)
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some _ => rfl
  | _, _, .writePlace mismatch x path value, env =>
      means_flowBind (Term.tree_means value env) fun v env => by
        cases x.get env with
        | none => rfl
        | some current =>
            dsimp only
            cases path.set? env v current with
            | none => exact mismatch_means mismatch
            | some _ => rfl
  | _, _, @Term.borrowPlace _ _ _ _ σ mismatch x path, env => by
      show Tree.Means (match x.get env with
          | some value =>
              match path.get? env value with
              | some component =>
                  .choose σ component fun prophecy =>
                    match path.set? env prophecy value with
                    | some lent => .ok (Flow.value (component, prophecy) (x.set lent env))
                    | none => .bottom
              | none => mismatchTree mismatch
          | none => .bottom)
        (match x.get env with
          | some value =>
              match path.get? env value with
              | some component =>
                  Spec.bind (prophecyOf x path) fun prophecy =>
                    match path.set? env prophecy value with
                    | some lent => Spec.pure (Flow.value (component, prophecy) (x.set lent env))
                    | none => Spec.bottom
              | none => mismatchOutcome mismatch
          | none => Spec.bottom)
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some component =>
              refine Tree.means_choose fun prophecy => ?_
              cases path.set? env prophecy value <;> rfl
  | _, _, .sliceVector value, env | _, _, .vectorSlice value, env =>
      means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .sliceCurrent value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .borrowArraySlice mismatch x path, env => by
      simp only [Term.tree, Term.denote, borrowArraySliceAt]
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some current =>
              refine Tree.means_choose fun prophecy => ?_
              cases path.set? env (prophecy) value <;> rfl
  | _, _, .borrowVectorSlice mismatch x path, env => by
      simp only [Term.tree, Term.denote, borrowVectorSliceAt]
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some current =>
              refine Tree.means_choose fun prophecy => ?_
              cases path.set? env (prophecy.vector) value <;> rfl
  | _, _, .reborrowSlice mismatch x path, env => by
      simp only [Term.tree, Term.denote, reborrowSliceAt]
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some current =>
              refine Tree.means_choose fun prophecy => ?_
              cases path.set? env (current.withCurrent prophecy) value <;> rfl
  | _, _, .takeSliceAt mismatch x path, env => by
      simp only [Term.tree, Term.denote, takeSliceAtValue]
      cases x.get env with
      | none => rfl
      | some value =>
          dsimp only
          cases path.get? env value with
          | none => exact mismatch_means mismatch
          | some reference =>
              dsimp only
              cases path.set? env (SpecSliceRef.ofArrays reference.final reference.final) value with
              | none => exact mismatch_means mismatch
              | some cleared => rfl
  | _, _, .resolveSliceAt x path, env => by
      simp only [Term.tree, Term.denote, resolveSliceAtValue]
      cases (x.get env).bind (path.get? env) with
      | none => rfl
      | some _ => exact Tree.means_assume (Tree.means_ok _)
  | _, _, .vectorLit count elements, env =>
      means_flowBind (Args.tree_means elements env) fun values _ => by
        cases SpecVector.ofArray? (HList.toListOf count values).toArray <;> rfl
  | _, _, @Term.arrayVector _ _ _ _ length array, env =>
      means_flowBind (Term.tree_means array env) fun a _ => by
        show Tree.Means _ (Spec.bind (Spec.assume (a.vector.values.size = length)) _)
        rw [Spec.assume_bind a.size_eq]
        exact Tree.means_ok _
  | _, .array _ _, .arrayOf vector, env =>
      means_flowBind (Term.tree_means vector env) fun _ _ =>
        Tree.means_dite _ (fun _ => Tree.means_ok _) fun _ => Tree.means_stuck
  | _, _, .repeatVector count value, env =>
      means_flowBind (Term.tree_means value env) fun v _ => by
        cases SpecVector.ofArray? (Array.replicate count v) <;> rfl
  | _, _, .length vector, env => means_flowBind (Term.tree_means vector env) fun _ _ => rfl
  | _, _, .signerAddress .., _ => True.intro
  | _, _, .index _ vector position, env =>
      means_flowBind (Term.tree_means vector env) fun values env =>
        means_flowBind (Term.tree_means position env) fun i env =>
          Tree.means_ite _ (Tree.means_abort _) (by
            cases values.values[i.val.toNat]? <;> rfl)
  | _, _, .checkIndex _ _ vector position, env =>
      means_flowBind (Term.tree_means vector env) fun _ env =>
        means_flowBind (Term.tree_means position env) fun _ _ =>
          Tree.means_ite _ (Tree.means_ok _) (Tree.means_abort _)
  | _, _, .push .., _ => True.intro
  | _, _, .insert .., _ => True.intro
  | _, _, .remove .., _ => True.intro
  | _, _, .swap .., _ => True.intro
  | _, _, .concat .., _ => True.intro
  | _, _, .slice _ vector start stop, env =>
      means_flowBind (Term.tree_means vector env) fun values env =>
        means_flowBind (Term.tree_means start env) fun a env =>
          means_flowBind (Term.tree_means stop env) fun b env =>
            Tree.means_ite _ (Tree.means_abort _) (by
              cases SpecVector.ofArray? (values.values.extract a.val.toNat b.val.toNat) <;> rfl)
  | _, _, .reverseSlice .., _ => True.intro
  | _, _, .destroyEmpty .., _ => True.intro
  | _, _, .contains .., _ => True.intro
  | _, _, .indexOf .., _ => True.intro
  | _, _, .order .., _ => True.intro
  | _, _, .seqAfter value effect, env =>
      means_flowBind (Term.tree_means value env) fun _ env =>
        means_flowBind (Term.tree_means effect env) fun _ _ => rfl
  | _, _, .assertion _, _ => rfl
  | _, _, .anchor _, _ => rfl
  | _, _, .assume .., _ => True.intro
  | _, _, .arbitrary, _ => True.intro
  | _, _, .ghostValue .., _ => True.intro
  | _, _, .ghostInit .., _ => True.intro
  | _, _, .ghostSlot .., _ => True.intro
  | _, _, .ghostRead .., _ => True.intro
  | _, _, .mutationEnd _, _ => rfl
  | _, _, .memoryWritten _, _ => rfl
  | _, _, .constructed _ value, env => means_flowBind (Term.tree_means value env) fun _ _ => rfl
  | _, _, .globalRead .., _ => True.intro
  | _, _, .globalContains .., _ => True.intro
  | _, _, .globalBorrow .., _ => True.intro
  | _, _, .globalPublish .., _ => True.intro
  | _, _, .ghostStore .., _ => True.intro
  | _, _, .globalTake .., _ => True.intro

theorem Args.tree_means {ρ : ResultShape} : {Γ σs : NRow} → (arguments : Args unit ρ Γ σs) →
    (env : HEnv Γ) → (arguments.tree calls fuel env).Means (arguments.denote meanings env)
  | _, _, .nil, _ => rfl
  | _, _, .cons head tail, env =>
      means_flowBind (Term.tree_means head env) fun _ env =>
        means_flowBind (Args.tree_means tail env) fun _ _ => rfl

theorem Arms.tree_means {ρ : ResultShape} : {Γ : NRow} → {σ τ : NTy} → (arms : Arms unit ρ Γ σ τ) →
    (value : σ.carrier) → (env : HEnv Γ) →
    (arms.tree calls fuel value env).Means (arms.denote meanings value env)
  | _, _, _, .nil (some _), _, _ => rfl
  | _, _, _, .nil none, _, _ => rfl
  | _, _, _, .cons arm pattern body rest, value, env =>
      by
        show Tree.Means (match pattern.bindValue value env with
            | some bound => body.tree calls fuel bound
            | none => rest.tree calls fuel value env)
          (match pattern.bindValue value env with
            | some bound => armIn arm (body.denote meanings bound)
            | none => rest.denote meanings value env)
        cases pattern.bindValue value env with
        | none => exact Arms.tree_means rest value env
        | some bound =>
            dsimp only
            rw [armIn_eq]
            exact Term.tree_means body bound
  | _, _, _, .guarded .., _, _ => True.intro
end

theorem resolve_means {Γ : NRow} {α : Type} {tree : Tree α} {spec : Comp unit α}
    (means : tree.Means spec) (env : HEnv Γ) :
    (mutables : Mutables Γ) → (resolveTree mutables env tree).Means (mutables.resolve env spec)
  | .nil => means
  | @Mutables.cons _ τ x rest => by
      show Tree.Means (match x.get env with
          | some reference => .assumeEq τ reference.1 reference.2 (resolveTree rest env tree)
          | none => resolveTree rest env tree)
        (match x.get env with
          | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              rest.resolve env spec
          | none => rest.resolve env spec)
      cases x.get env with
      | none => exact resolve_means means env rest
      | some _ => exact Tree.means_assume (resolve_means means env rest)
  | @Mutables.at _ _ σ x path rest => by
      show Tree.Means (match (x.get env).bind (path.get? env) with
          | some reference => .assumeEq σ reference.1 reference.2 (resolveTree rest env tree)
          | none => resolveTree rest env tree)
        (match (x.get env).bind (path.get? env) with
          | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              rest.resolve env spec
          | none => rest.resolve env spec)
      cases (x.get env).bind (path.get? env) with
      | none => exact resolve_means means env rest
      | some _ => exact Tree.means_assume (resolve_means means env rest)

  | .sliceAt x path rest => by
      simp only [resolveTree, Mutables.resolve]
      cases (x.get env).bind (path.get? env) with
      | none => exact resolve_means means env rest
      | some _ => exact Tree.means_assume (resolve_means means env rest)

include callsMean in
/-- A function's tree means its denotation, at every call meaning its tree means. -/
theorem Function.tree_means (f : Function unit) (args : HList f.params) :
    (Function.tree calls fuel f args).Means (f.denoteWith meanings args) := by
  have body :
      ((f.body.tree calls fuel (initialEnv f.params f.locals f.frame_eq args)).bind
        (finishTree f.mutables f.result)).Means
      (Spec.bind (f.body.denote meanings (initialEnv f.params f.locals f.frame_eq args))
        (f.result.finish f.mutables)) := by
    exact Tree.means_bind (Term.tree_means meanings calls fuel callsMean f.body _) fun flow =>
      match flow with
      | .value _ env => resolve_means (Tree.means_ok _) env f.mutables
      | .return_ _ env => resolve_means (Tree.means_ok _) env f.mutables
      | .break_ .. => rfl
      | .continue_ .. => rfl
  cases scopes : f.scopedMethods with
  | nil => simpa only [Function.tree, Function.denoteWith, scopes] using body
  | cons head tail =>
      simp only [Function.tree, Function.denoteWith, scopes]
      split
      · exact body
      · rfl

end TermMeans


/-! ## Calls -/

/-- The compiled functions of a unit by namespace and index. -/
abbrev CompiledUnit (unit : ValidatedUnit) := Array (Array (Except String (Function unit)))

/-- The compiled callee of a handle at a signature. -/
def calleeAt {unit : ValidatedUnit} (compiled : CompiledUnit unit) (handle : FunctionHandle) :
    Except String (Function unit) :=
  match (compiled[handle.namespaceId.index]?).bind fun row => row[handle.functionId.index]? with
  | some entry => entry
  | none => .error "a callee out of range"

section Calls
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- The callee a call runs: its compilation, or its compilation specialized at
the call's type arguments. -/
def substitutedCallee (compiled : CompiledUnit unit) (handle : FunctionHandle) :
    Option (Array GenericArgument × TypeArgs) → Except String (Function unit)
  | none => calleeAt compiled handle
  | some (_, θ) => compileFunctionAt unit handle (some θ)

/-- Evaluate a compiled callee at a checked signature. -/
def bodyTree (compiled : CompiledUnit unit) (calls : CallTree (unit := unit)) (loopFuel : Nat)
    (handle : FunctionHandle) (substitution : Option (Array GenericArgument × TypeArgs))
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Tree shape.carrier :=
  match substitutedCallee compiled handle substitution with
  | .ok f =>
      if same : f.params = σs ∧ f.result = shape then
        same.2 ▸ Function.tree calls loopFuel f (same.1.symm ▸ args)
      else .unknown "a callee at another signature"
  | .error reason => .unknown reason

/-- The same compiled body read relationally. -/
def bodyMeaning {executable : ExecutableUnit unit} (compiled : CompiledUnit unit)
    (meanings : Meanings executable) (handle : FunctionHandle)
    (substitution : Option (Array GenericArgument × TypeArgs))
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Comp unit shape.carrier :=
  match substitutedCallee compiled handle substitution with
  | .ok f =>
      if same : f.params = σs ∧ f.result = shape then
        same.2 ▸ f.denoteWith meanings (same.1.symm ▸ args)
      else Spec.bottom
  | .error _ => Spec.bottom

theorem bodyTree_means {executable : ExecutableUnit unit} (compiled : CompiledUnit unit)
    (calls : CallTree (unit := unit)) (meanings : Meanings executable) (loopFuel : Nat)
    (agrees : calls.Means meanings) (handle substitution σs shape args) :
    (bodyTree compiled calls loopFuel handle substitution σs shape args).Means
      (bodyMeaning compiled meanings handle substitution σs shape args) := by
  unfold bodyTree bodyMeaning
  cases substitutedCallee compiled handle substitution with
  | error reason => exact True.intro
  | ok f =>
      dsimp only
      by_cases same : f.params = σs ∧ f.result = shape
      · obtain ⟨rfl, rfl⟩ := same
        simp only [and_self, ↓reduceDIte]
        exact Function.tree_means _ _ _ agrees f args
      · simp only [same, ↓reduceDIte]
        exact True.intro

private theorem rowOfList : (row : NRow) → NRow.ofList row.toList = row
  | .nil => rfl
  | .cons τ rest => congrArg (NRow.cons τ) (rowOfList rest)

/-- Decode captures separately from the supplied arguments: encoding and
re-decoding the latter would lose a mutable argument's prophecy. -/
def closureCallee (closure : ClosureValue) (σs : NRow) (shape : ResultShape)
    (args : HList σs) : Except String (Σ f : Function unit, {_values : HList f.params // f.result = shape}) := do
  let row := NRow.ofList <| (List.range (typeArity unit closure.function)).map
    (closureArgument unit closure.function closure.typeInstantiation)
  if inhabited : row.inhabitable = true then
    let f ← compileFunctionAt unit closure.function (some ⟨row, inhabited⟩)
    let ⟨captured, supplied, weave⟩ := Weave.ofMask closure.mask f.params.toList
    if same : supplied = σs ∧ f.result = shape then
      let some values := (rowCodec captured).decode? closure.captures.toList
        | throw "cannot decode closure captures"
      let full := weave.compose values (same.1.symm ▸ args)
      return ⟨f, (rowOfList f.params ▸ full), same.2⟩
    else throw "closure invocation at another signature"
  else throw "uninhabitable closure type arguments"

/-- Calls as compiled bodies, retaining each generic call's runtime frame so
closures created in that body capture the same type instantiation as the IR. -/
def callTree (compiled : CompiledUnit unit) (loopFuel : Nat) :
    Nat → (outer : Array (TypeId × TypeId) := #[]) → CallTree (unit := unit)
  | 0, outer => {
      typeInstantiation := outer
      direct := fun _ _ _ _ _ => .unknown "call depth"
      generic := fun _ _ _ _ _ _ => .unknown "call depth"
      invoke := fun _ _ _ _ _ => .unknown "call depth" }
  | depth + 1, outer =>
      let body := fun handle substitution =>
        let nested := match substitution with
          | none => #[]
          | some (typeArgs, _) => frameInstantiation unit handle outer typeArgs
        bodyTree compiled (callTree compiled loopFuel depth nested) loopFuel handle substitution
      { typeInstantiation := outer
        direct := body
        generic := fun handle typeArgs θ σs shape args =>
          (body handle (some (typeArgs, θ)) (NRow.subst θ.1 σs) (shape.subst θ.1)
            (HList.ofSkolem θ σs args)).bind fun result =>
              .ok (ResultShape.toSkolem θ shape result)
        invoke := fun σs shared shape function args =>
          let closure := (Carriers.functions σs shared shape.row).closure function
          match closureCallee closure σs shape args with
          | .error reason => .unknown reason
          | .ok ⟨f, values, same⟩ =>
              same ▸ Function.tree (callTree compiled loopFuel depth closure.typeInstantiation) loopFuel f values }

/-- The relational counterpart of callTree. Dispatch checks compiled bodies;
this does not assume compiler correctness or runtime-dispatch agreement. -/
def inlined (executable : ExecutableUnit unit) (compiled : CompiledUnit unit) :
    Nat → (outer : Array (TypeId × TypeId) := #[]) → Meanings executable
  | 0, outer => { closedMeanings executable outer with
      call := fun _ _ _ _ => Spec.bottom
      atInstance := fun _ _ _ _ _ _ => Spec.bottom
      generic := fun _ _ _ _ _ _ => Spec.bottom
      invoke := fun _ _ _ _ _ => Spec.bottom }
  | depth + 1, outer =>
      let body := fun handle substitution =>
        let nested := match substitution with
          | none => #[]
          | some (typeArgs, _) => frameInstantiation unit handle outer typeArgs
        bodyMeaning compiled (inlined executable compiled depth nested) handle substitution
      { closedMeanings executable outer with
        call := fun handle σs shape args => body handle none σs shape args
        atInstance := fun handle typeArgs θ σs shape args => body handle (some (typeArgs, θ)) σs shape args
        generic := fun handle typeArgs θ σs shape args =>
          Spec.bind (body handle (some (typeArgs, θ)) (NRow.subst θ.1 σs) (shape.subst θ.1)
            (HList.ofSkolem θ σs args)) fun result => Spec.pure (ResultShape.toSkolem θ shape result)
        invoke := fun σs shared shape function args =>
          let closure := (Carriers.functions σs shared shape.row).closure function
          match closureCallee closure σs shape args with
          | .error _ => Spec.bottom
          | .ok ⟨f, values, same⟩ =>
              same ▸ f.denoteWith (inlined executable compiled depth closure.typeInstantiation) values }

theorem callTree_means (executable : ExecutableUnit unit) (compiled : CompiledUnit unit) (loopFuel : Nat) :
    (depth : Nat) → (outer : Array (TypeId × TypeId) := #[]) →
      (callTree compiled loopFuel depth outer).Means (inlined executable compiled depth outer)
  | 0, _ => by
      constructor
      · rfl
      · intro _ substitution; cases substitution <;> intros <;> exact True.intro
      · intros; exact True.intro
      · intros; exact True.intro
  | depth + 1, outer => by
      constructor
      · rfl
      · intro handle substitution σs shape args
        cases substitution with
        | none =>
            exact bodyTree_means compiled _ _ loopFuel
              (callTree_means executable compiled loopFuel depth #[]) handle none σs shape args
        | some entry =>
            exact bodyTree_means compiled _ _ loopFuel
              (callTree_means executable compiled loopFuel depth _) handle (some entry) σs shape args
      · intro handle typeArgs θ σs shape args
        exact Tree.means_bind (bodyTree_means compiled _ _ loopFuel
          (callTree_means executable compiled loopFuel depth _) handle (some (typeArgs, θ)) _ _ _) fun _ => rfl
      · intro σs shared shape function args
        dsimp only [callTree, inlined]
        split
        · exact True.intro
        · rename_i f values same found
          cases same
          exact Function.tree_means _ _ _
            (callTree_means executable compiled loopFuel depth _) f values

/-- What the check runs means the denotation of the function, each callee's
compiled body inlined: a walk's outcome is an outcome of it (`Tree.walk_sound`). -/
theorem checked_tree_means (executable : ExecutableUnit unit) (compiled : CompiledUnit unit)
    (loopFuel depth : Nat) (f : Function unit) (args : HList f.params) :
    (Function.tree (callTree compiled loopFuel depth) loopFuel f args).Means
      (f.denoteWith (inlined executable compiled depth) args) :=
  Function.tree_means _ _ _ (callTree_means executable compiled loopFuel depth) f args

end Calls

/-- info: 'VerexIR.Proofs.Denote.checked_tree_means' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms checked_tree_means

/-- info: 'VerexIR.Proofs.Denote.Tree.walk_sound' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms Tree.walk_sound

end VerexIR.Proofs.Denote

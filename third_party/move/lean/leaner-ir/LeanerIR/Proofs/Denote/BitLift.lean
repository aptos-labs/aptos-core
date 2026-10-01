-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.IntegerArithmetic
import LeanerIR.Proofs.Representation
import Std.Tactic.BVDecide
import Lean

/-!
# Lifting integer bit arithmetic to bit vectors

An unsigned integer below `2 ^ width` is the value of a bit vector of that
width, and the bitwise operations, shifts, and remainders by powers of two on
such integers are the bit vector operations on their vectors. Each lemma here
states one such correspondence over terms already known to be vector values,
so a proposition over them can be restated over bit vectors, where a bit
vector decision procedure takes over, as the Move Prover's `pragma bv` does.
-/

namespace LeanerIR.Proofs.Denote.BitLift

open LeanerIR.Proofs.IntegerArithmetic

variable {width : Nat}

/-- A certified unsigned integer is the value of a bit vector as wide as
its width or wider, below the bound of its width. -/
theorem atom {valueWidth : Nat} (value : SpecInt (.bits valueWidth) false) (bound : Nat)
    (isBound : 2 ^ valueWidth = bound) (fits : bound ≤ 2 ^ width) :
    ∃ vector : BitVec width, value.val = (vector.toNat : Int) ∧ vector.toNat < bound := by
  obtain ⟨lower, upper⟩ := value.unsigned_bounds
  have pow : ((2 ^ valueWidth : Nat) : Int) = (2 : Int) ^ valueWidth := Int.natCast_pow 2 valueWidth
  have below : value.val.toNat < bound := by have := Nat.two_pow_pos valueWidth; omega
  refine ⟨BitVec.ofNat width value.val.toNat, ?_, ?_⟩ <;>
    rw [BitVec.toNat_ofNat, Nat.mod_eq_of_lt (Nat.lt_of_lt_of_le below fits)]
  · omega
  · exact below

theorem literal (value : Nat) (fits : value < 2 ^ width) :
    (value : Int) = ((BitVec.ofNat width value).toNat : Int) := by
  rw [BitVec.toNat_ofNat, Nat.mod_eq_of_lt fits]

theorem natLiteral (value : Nat) (fits : value < 2 ^ width) :
    value = (BitVec.ofNat width value).toNat := by
  rw [BitVec.toNat_ofNat, Nat.mod_eq_of_lt fits]

theorem natCast {value : Nat} {vector : BitVec width} (lifted : value = vector.toNat) :
    (value : Int) = (vector.toNat : Int) := by rw [lifted]

theorem ofNat {value : Nat} {vector : BitVec width} (lifted : value = vector.toNat) :
    Int.ofNat value = (vector.toNat : Int) := by rw [lifted]; rfl

theorem toNat {value : Int} {vector : BitVec width} (lifted : value = vector.toNat) :
    value.toNat = vector.toNat := by rw [lifted]; rfl

theorem and {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    bitwiseAnd left right = ((leftVector &&& rightVector).toNat : Int) := by
  rw [leftLifted, rightLifted, BitVec.toNat_and]; rfl

theorem natAnd {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left &&& right = (leftVector &&& rightVector).toNat := by
  rw [leftLifted, rightLifted, BitVec.toNat_and]

theorem natOr {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left ||| right = (leftVector ||| rightVector).toNat := by
  rw [leftLifted, rightLifted, BitVec.toNat_or]

theorem natXor {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left ^^^ right = (leftVector ^^^ rightVector).toNat := by
  rw [leftLifted, rightLifted, BitVec.toNat_xor]

theorem natMod {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left % right = (leftVector % rightVector).toNat := by
  rw [leftLifted, rightLifted, BitVec.toNat_umod]

theorem natDiv {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left / right = (leftVector / rightVector).toNat := by
  rw [leftLifted, rightLifted, BitVec.toNat_udiv]

theorem mod {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left % right = ((leftVector % rightVector).toNat : Int) := by
  rw [leftLifted, rightLifted, BitVec.toNat_umod]; rfl

theorem div {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left / right = ((leftVector / rightVector).toNat : Int) := by
  rw [leftLifted, rightLifted, BitVec.toNat_udiv]; rfl

theorem tmod {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left.tmod right = ((leftVector % rightVector).toNat : Int) := by
  rw [Int.tmod_eq_emod_of_nonneg (by rw [leftLifted]; exact Int.natCast_nonneg _)]
  exact mod leftLifted rightLifted

theorem tdiv {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left.tdiv right = ((leftVector / rightVector).toNat : Int) := by
  rw [Int.tdiv_eq_ediv_of_nonneg (by rw [leftLifted]; exact Int.natCast_nonneg _)]
  exact div leftLifted rightLifted

theorem shiftRight {value : Int} {distance : Nat} {vector distanceVector : BitVec width}
    (lifted : value = vector.toNat) (distanceLifted : distance = distanceVector.toNat) :
    Int.shiftRight value distance = ((vector >>> distanceVector).toNat : Int) := by
  rw [lifted, distanceLifted, BitVec.ushiftRight_eq', BitVec.toNat_ushiftRight]
  show (vector.toNat : Int) >>> distanceVector.toNat = _
  rw [Int.shiftRight_eq_div_pow, Nat.shiftRight_eq_div_pow]
  simp

/-- A shift left read modulo a power of two within the width: the vector
shift, whose truncation at the width the remainder subsumes. -/
theorem shiftLeftMod {value modulus : Int} {distance : Nat}
    {vector distanceVector modulusVector : BitVec width}
    (lifted : value = vector.toNat) (distanceLifted : distance = distanceVector.toNat)
    (modulusLifted : modulus = modulusVector.toNat)
    (divides : modulusVector.toNat ∣ 2 ^ width) :
    Int.shiftLeft value distance % modulus =
      (((vector <<< distanceVector) % modulusVector).toNat : Int) := by
  rw [lifted, distanceLifted, modulusLifted, BitVec.toNat_umod, BitVec.shiftLeft_eq',
    BitVec.toNat_shiftLeft, Nat.mod_mod_of_dvd _ divides]
  show ((vector.toNat : Int) <<< distanceVector.toNat) % _ = _
  rw [Int.shiftLeft_eq, Nat.shiftLeft_eq]
  simp

theorem shiftLeftTmod {value modulus : Int} {distance : Nat}
    {vector distanceVector modulusVector : BitVec width}
    (lifted : value = vector.toNat) (distanceLifted : distance = distanceVector.toNat)
    (modulusLifted : modulus = modulusVector.toNat)
    (divides : modulusVector.toNat ∣ 2 ^ width) :
    (Int.shiftLeft value distance).tmod modulus =
      (((vector <<< distanceVector) % modulusVector).toNat : Int) := by
  rw [Int.tmod_eq_emod_of_nonneg]
  · exact shiftLeftMod lifted distanceLifted modulusLifted divides
  · show 0 ≤ value <<< distance
    rw [Int.shiftLeft_eq, lifted]
    exact Int.mul_nonneg (Int.natCast_nonneg _) (Int.pow_nonneg (by decide))

/-- A shift read modulo the width's power: the vector shift, which truncates
at the width. -/
theorem shiftLeftModWidth {value modulus : Int} {distance : Nat}
    {vector distanceVector : BitVec width}
    (lifted : value = vector.toNat) (distanceLifted : distance = distanceVector.toNat)
    (isWidth : modulus = ((2 ^ width : Nat) : Int)) :
    Int.shiftLeft value distance % modulus = ((vector <<< distanceVector).toNat : Int) := by
  rw [lifted, distanceLifted, isWidth, BitVec.shiftLeft_eq', BitVec.toNat_shiftLeft]
  show ((vector.toNat : Int) <<< distanceVector.toNat) % _ = _
  rw [Int.shiftLeft_eq, Nat.shiftLeft_eq]
  simp

theorem shiftLeftTmodWidth {value modulus : Int} {distance : Nat}
    {vector distanceVector : BitVec width}
    (lifted : value = vector.toNat) (distanceLifted : distance = distanceVector.toNat)
    (isWidth : modulus = ((2 ^ width : Nat) : Int)) :
    (Int.shiftLeft value distance).tmod modulus = ((vector <<< distanceVector).toNat : Int) := by
  rw [Int.tmod_eq_emod_of_nonneg]
  · exact shiftLeftModWidth lifted distanceLifted isWidth
  · show 0 ≤ value <<< distance
    rw [Int.shiftLeft_eq, lifted]
    exact Int.mul_nonneg (Int.natCast_nonneg _) (Int.pow_nonneg (by decide))

/-- A remainder by a modulus no vector value reaches leaves the value. -/
theorem modBeyond {value modulus : Int} {vector : BitVec width} (lifted : value = vector.toNat)
    (beyond : ((2 ^ width : Nat) : Int) ≤ modulus) :
    value % modulus = (vector.toNat : Int) := by
  rw [lifted]
  have := vector.isLt
  exact Int.emod_eq_of_lt (Int.natCast_nonneg _) (by omega)

theorem tmodBeyond {value modulus : Int} {vector : BitVec width} (lifted : value = vector.toNat)
    (beyond : ((2 ^ width : Nat) : Int) ≤ modulus) :
    value.tmod modulus = (vector.toNat : Int) := by
  rw [Int.tmod_eq_emod_of_nonneg (by rw [lifted]; exact Int.natCast_nonneg _)]
  exact modBeyond lifted beyond

theorem le {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left ≤ right ↔ leftVector ≤ rightVector := by
  rw [leftLifted, rightLifted, BitVec.le_def]; omega

theorem lt {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left < right ↔ leftVector < rightVector := by
  rw [leftLifted, rightLifted, BitVec.lt_def]; omega

theorem eq {left right : Int} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left = right ↔ leftVector = rightVector := by
  rw [leftLifted, rightLifted, ← BitVec.toNat_inj]; omega

theorem natLe {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left ≤ right ↔ leftVector ≤ rightVector := by
  rw [leftLifted, rightLifted, BitVec.le_def]

theorem natLt {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left < right ↔ leftVector < rightVector := by
  rw [leftLifted, rightLifted, BitVec.lt_def]

theorem natEq {left right : Nat} {leftVector rightVector : BitVec width}
    (leftLifted : left = leftVector.toNat) (rightLifted : right = rightVector.toNat) :
    left = right ↔ leftVector = rightVector := by
  rw [leftLifted, rightLifted, BitVec.toNat_inj]

/-! ## Lifting a leaf

Every certified unsigned integer of the leaf becomes a bit vector of the
widest width among them, and every proposition whose terms lift is restated
over those vectors, with its equivalence to the original as the proof. The
restated leaf goes to `bv_decide`, whose certificate the kernel checks. -/

open Lean Meta Elab Tactic

/-- The width of a certified unsigned integer's value, `x.val`. -/
private def unsignedWidth? (e : Lean.Expr) : MetaM (Option Nat) := do
  unless e.isAppOfArity ``LeanerIR.SpecInt.val 3 do return none
  let width ← whnfR (e.getArg! 0)
  unless width.isAppOfArity ``LeanerIR.IntWidth.bits 1 do return none
  unless (← whnfR (e.getArg! 1)).isConstOf ``Bool.false do return none
  return (← instantiateMVars (width.getArg! 0)).nat?

/-- The certified unsigned values a term reads, with their widths. -/
private def atomsOf (e : Lean.Expr) : MetaM (Array (Lean.Expr × Nat)) := do
  let collect : StateT (Array (Lean.Expr × Nat)) MetaM Unit :=
    Meta.forEachExpr' e fun sub => do
      if let some bits ← unsignedWidth? sub then
        unless (← get).any (·.1 == sub) do modify (·.push (sub, bits))
        return false
      return true
  let ((), found) ← collect.run #[]
  return found

/-- The vectors the atoms of a leaf became: the width, and for each atom,
its vector and the proof that the atom is that vector's value. -/
private structure Lifting where
  width : Nat
  atoms : Array (Lean.Expr × Lean.Expr × Lean.Expr)

private abbrev LiftM := ReaderT Lifting MetaM

/-- `e = ↑vector.toNat` stated of `e` itself, not of the form a lemma
derives it in. -/
private def lifted (e : Lean.Expr) (vector proof : Lean.Expr) (int : Bool) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
  let value ← mkAppM ``BitVec.toNat #[vector]
  let value ← if int then mkAppOptM ``Nat.cast #[mkConst ``Int, none, value] else pure value
  return some (vector, ← mkExpectedTypeHint proof (← mkEq e value))

/-- `2 ^ width`, as the lemmas state it. -/
private def powerOfTwo (width : Nat) : MetaM Lean.Expr :=
  mkAppM ``HPow.hPow #[mkNatLit 2, mkNatLit width]

private def literalVector (value : Nat) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
  let width := (← read).width
  unless value < 2 ^ width do return none
  let fits ← mkDecideProof (← mkAppM ``LT.lt #[mkNatLit value, ← powerOfTwo width])
  return some (mkApp2 (mkConst ``BitVec.ofNat) (mkNatLit width) (mkNatLit value), fits)

/-- The vector a lifting proof's right side reads. -/
private def vectorOf (proof : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType proof)
  let value := type.getArg! 2
  let value := if value.isAppOfArity ``Nat.cast 3 then value.appArg! else value
  return value.appArg!

private def shiftLeft? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``Int.shiftLeft 2 then some (e.getArg! 0, e.getArg! 1)
  else if e.isAppOfArity ``HShiftLeft.hShiftLeft 6 then some (e.getArg! 4, e.getArg! 5)
  else none

private def shiftRight? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``Int.shiftRight 2 then some (e.getArg! 0, e.getArg! 1)
  else if e.isAppOfArity ``HShiftRight.hShiftRight 6 then some (e.getArg! 4, e.getArg! 5)
  else none

mutual

/-- An integer term as a vector, with `e = ↑vector.toNat`. -/
private partial def liftInt (e : Lean.Expr) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
  let e ← instantiateMVars e
  if let some (_, vector, proof) := (← read).atoms.find? (·.1 == e) then
    return some (vector, proof)
  if let some value := e.int? then
    unless 0 ≤ value do return none
    let some (vector, fits) ← literalVector value.toNat | return none
    return ← lifted e vector (← mkAppOptM ``literal #[mkNatLit (← read).width, mkNatLit value.toNat, fits]) true
  let binary (lemma : Name) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
    let some (_, leftProof) ← liftInt (e.getArg! 4) | return none
    let some (_, rightProof) ← liftInt (e.getArg! 5) | return none
    let proof ← mkAppM lemma #[leftProof, rightProof]
    lifted e (← vectorOf proof) proof true
  if e.isAppOfArity ``Nat.cast 3 || e.isAppOfArity ``Int.ofNat 1 then
    let some (vector, proof) ← liftNat e.appArg! | return none
    let lemma := if e.isAppOfArity ``Int.ofNat 1 then ``ofNat else ``natCast
    return ← lifted e vector (← mkAppM lemma #[proof]) true
  if e.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    let some (_, leftProof) ← liftInt (e.getArg! 0) | return none
    let some (_, rightProof) ← liftInt (e.getArg! 1) | return none
    let proof ← mkAppM ``and #[leftProof, rightProof]
    return ← lifted e (← vectorOf proof) proof true
  if e.isAppOfArity ``HMod.hMod 6 || e.isAppOfArity ``Int.tmod 2 then
    return ← liftRemainder e
  if e.isAppOfArity ``HDiv.hDiv 6 then return ← binary ``div
  if e.isAppOfArity ``Int.tdiv 2 then
    let some (_, leftProof) ← liftInt (e.getArg! 0) | return none
    let some (_, rightProof) ← liftInt (e.getArg! 1) | return none
    let proof ← mkAppM ``tdiv #[leftProof, rightProof]
    return ← lifted e (← vectorOf proof) proof true
  if let some (value, distance) := shiftRight? e then
    let some (_, valueProof) ← liftInt value | return none
    let some (_, distanceProof) ← liftNat distance | return none
    let proof ← mkAppM ``shiftRight #[valueProof, distanceProof]
    return ← lifted e (← vectorOf proof) proof true
  return none

/-- A natural-number term as a vector, with `e = vector.toNat`. -/
private partial def liftNat (e : Lean.Expr) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
  let e ← instantiateMVars e
  if let some value := e.nat? then
    let some (vector, fits) ← literalVector value | return none
    return ← lifted e vector (← mkAppOptM ``natLiteral #[mkNatLit (← read).width, mkNatLit value, fits]) false
  -- A vector of the leaf's width, such as an atom's in its bound.
  if e.isAppOfArity ``BitVec.toNat 2 && (e.getArg! 0).nat? == some (← read).width then
    return some (e.appArg!, ← mkEqRefl e)
  if e.isAppOfArity ``Int.toNat 1 then
    let some (_, proof) ← liftInt e.appArg! | return none
    let proof ← mkAppM ``toNat #[proof]
    return ← lifted e (← vectorOf proof) proof false
  let lemma? := if e.isAppOfArity ``HAnd.hAnd 6 then some ``natAnd
    else if e.isAppOfArity ``HOr.hOr 6 then some ``natOr
    else if e.isAppOfArity ``HXor.hXor 6 then some ``natXor
    else if e.isAppOfArity ``HMod.hMod 6 then some ``natMod
    else if e.isAppOfArity ``HDiv.hDiv 6 then some ``natDiv
    else none
  let some lemma := lemma? | return none
  let some (_, leftProof) ← liftNat (e.getArg! 4) | return none
  let some (_, rightProof) ← liftNat (e.getArg! 5) | return none
  let proof ← mkAppM lemma #[leftProof, rightProof]
  lifted e (← vectorOf proof) proof false

/-- A remainder, `%` or `tmod`: of a shift, the truncated vector shift; by
a modulus beyond every vector value, the value; otherwise the vector
remainder. -/
private partial def liftRemainder (e : Lean.Expr) : LiftM (Option (Lean.Expr × Lean.Expr)) := do
  let truncating := e.isAppOfArity ``Int.tmod 2
  let (dividend, modulus) := if truncating then (e.getArg! 0, e.getArg! 1)
    else (e.getArg! 4, e.getArg! 5)
  let width := (← read).width
  let power := mkApp (mkConst ``Int.ofNat) (← powerOfTwo width)
  let literal := modulus.int?
  if let some (value, distance) := shiftLeft? dividend then
    let some (_, valueProof) ← liftInt value | return none
    let some (_, distanceProof) ← liftNat distance | return none
    if literal == some (2 ^ width : Nat) then
      let isWidth ← mkDecideProof (← mkEq modulus power)
      let proof ← mkAppM (if truncating then ``shiftLeftTmodWidth else ``shiftLeftModWidth)
        #[valueProof, distanceProof, isWidth]
      return ← lifted e (← vectorOf proof) proof true
    let some (modulusVector, modulusProof) ← liftInt modulus | return none
    let divides ← mkDecideProof (← mkAppM ``Dvd.dvd
      #[← mkAppM ``BitVec.toNat #[modulusVector], ← powerOfTwo width])
    let proof ← mkAppM (if truncating then ``shiftLeftTmod else ``shiftLeftMod)
      #[valueProof, distanceProof, modulusProof, divides]
    return ← lifted e (← vectorOf proof) proof true
  let some (_, dividendProof) ← liftInt dividend | return none
  if let some value := literal then
    if (2 ^ width : Nat) ≤ value then
      let beyond ← mkDecideProof (← mkAppM ``LE.le #[power, modulus])
      let proof ← mkAppM (if truncating then ``tmodBeyond else ``modBeyond) #[dividendProof, beyond]
      return ← lifted e (← vectorOf proof) proof true
  let some (_, modulusProof) ← liftInt modulus | return none
  let proof ← mkAppM (if truncating then ``tmod else ``mod) #[dividendProof, modulusProof]
  lifted e (← vectorOf proof) proof true

end

/-- A proposition over lifted terms as one over their vectors, with
`p ↔ lifted`, and whether any part of it lifted. A part that does not lift
becomes a boolean atom of the decision. -/
private partial def liftProp (p : Lean.Expr) : LiftM (Lean.Expr × Lean.Expr × Bool) := do
  let p ← instantiateMVars p
  let restated (proof : Lean.Expr) (lifted : Bool) : MetaM (Lean.Expr × Lean.Expr × Bool) := do
    return ((← whnfR (← inferType proof)).getArg! 1, proof, lifted)
  let connective (lemma : Name) (parts : Array Lean.Expr) : LiftM (Lean.Expr × Lean.Expr × Bool) := do
    let parts ← parts.mapM liftProp
    restated (← mkAppM lemma (parts.map (·.2.1))) (parts.any (·.2.2))
  if p.isAppOfArity ``And 2 || p.isAppOfArity ``Or 2 || p.isAppOfArity ``Iff 2 then
    let lemma := if p.isAppOfArity ``And 2 then ``and_congr
      else if p.isAppOfArity ``Or 2 then ``or_congr else ``iff_congr
    return ← connective lemma #[p.getArg! 0, p.getArg! 1]
  if p.isAppOfArity ``Not 1 then return ← connective ``not_congr #[p.getArg! 0]
  if p.isArrow then return ← connective ``imp_congr #[p.bindingDomain!, p.bindingBody!]
  let unchanged : LiftM (Lean.Expr × Lean.Expr × Bool) := do
    let boolean ← if p.isAppOfArity ``Eq 3 then
        pure ((← whnfR (p.getArg! 0)).isConstOf ``Bool)
      else pure false
    if p.isConstOf ``True || p.isConstOf ``False || boolean then
      return (p, ← mkAppM ``Iff.refl #[p], false)
    let instance_ ← try synthInstance (mkApp (mkConst ``Decidable) p)
      catch _ => pure (mkApp (mkConst ``Classical.propDecidable) p)
    let atom := mkApp2 (mkConst ``decide) p instance_
    let proof ← mkAppM ``Iff.symm #[mkApp2 (mkConst ``decide_eq_true_iff) p instance_]
    return (← mkEq atom (mkConst ``Bool.true), proof, false)
  let relation? : Option (Name × Name × Bool) :=
    if p.isAppOfArity ``LE.le 4 then some (``le, ``natLe, false)
    else if p.isAppOfArity ``LT.lt 4 then some (``lt, ``natLt, false)
    else if p.isAppOfArity ``GE.ge 4 then some (``le, ``natLe, true)
    else if p.isAppOfArity ``GT.gt 4 then some (``lt, ``natLt, true)
    else if p.isAppOfArity ``Eq 3 then some (``eq, ``natEq, false)
    else none
  let some (intLemma, natLemma, swapped) := relation? | unchanged
  let (left, right) := if p.isAppOfArity ``Eq 3 then (p.getArg! 1, p.getArg! 2)
    else (p.getArg! 2, p.getArg! 3)
  let (left, right) := if swapped then (right, left) else (left, right)
  let type ← whnfR (p.getArg! 0)
  unless type.isConstOf ``Int || type.isConstOf ``Nat do return ← unchanged
  let (lift, lemma) := if type.isConstOf ``Int then (liftInt, intLemma) else (liftNat, natLemma)
  let some (_, leftProof) ← lift left | unchanged
  let some (_, rightProof) ← lift right | unchanged
  let proof ← mkAppM lemma #[leftProof, rightProof]
  let lifted := (← whnfR (← inferType proof)).getArg! 1
  return (lifted, ← mkExpectedTypeHint proof (← mkAppM ``Iff #[p, lifted]), true)

/-- Restate a leaf over bit vectors: introduce a vector for each certified
unsigned value it reads, restate the goal and every hypothesis whose terms
lift, and leave the restated goal. -/
def liftGoal (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut atoms : Array (Lean.Expr × Nat) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    unless ← isProp type do continue
    for atom in ← atomsOf type do
      unless atoms.any (·.1 == atom.1) do atoms := atoms.push atom
  for atom in ← atomsOf (← instantiateMVars (← goal.getType)) do
    unless atoms.any (·.1 == atom.1) do atoms := atoms.push atom
  if atoms.isEmpty then throwError "the goal reads no certified unsigned integer"
  let width := atoms.foldl (fun width atom => max width atom.2) 0
  -- Each atom `x.val` is some vector's value, below its width's bound.
  let mut goal := goal
  let mut lifted := #[]
  for (atom, bits) in atoms do
    let bound := 2 ^ bits
    let isBound ← mkExpectedTypeHint (← mkEqRefl (mkNatLit bound))
      (← mkEq (← powerOfTwo bits) (mkNatLit bound))
    let fits ← mkDecideProof (← mkAppM ``LE.le #[mkNatLit bound, ← powerOfTwo width])
    let exists_ ← goal.withContext do
      mkAppOptM ``atom #[mkNatLit width, none, atom.appArg!, mkNatLit bound, isBound, fits]
    let (fact, next) ← (← goal.assert `lift (← goal.withContext (inferType exists_)) exists_).intro1P
    let #[case] ← next.cases fact | throwError "unexpected lifting case split"
    let vector := case.fields[0]!
    let #[case] ← case.mvarId.cases case.fields[1]!.fvarId! | throwError "unexpected lifting case split"
    lifted := lifted.push (atom, vector, case.fields[0]!)
    goal := case.mvarId
  goal.withContext do
    let lifting : Lifting := { width, atoms := lifted }
    let target ← instantiateMVars (← goal.getType)
    let (restated, equivalence, targetLifted) ← (liftProp target).run lifting
    let mut goal ← if restated == target then pure goal
      else goal.replaceTargetEq restated (← mkPropExt equivalence)
    let mut anyLifted := targetLifted
    let atomEquations := lifted.map (·.2.2)
    for decl in ← goal.withContext getLCtx do
      if decl.isImplementationDetail || atomEquations.contains decl.toExpr then continue
      let type ← instantiateMVars decl.type
      unless ← goal.withContext (isProp type) do continue
      -- A hypothesis whose atoms the goal may share is restated even
      -- where nothing in it lifts.
      let (restated, equivalence, hypothesisLifted) ←
        goal.withContext ((liftProp type).run lifting)
      if restated == type then continue
      anyLifted := anyLifted || hypothesisLifted
      let proof ← goal.withContext (mkAppM ``Iff.mp #[equivalence, decl.toExpr])
      let (_, next) ← (← goal.assert decl.userName restated proof).intro1P
      goal := next
    unless anyLifted do throwError "the leaf does not lift to bit vectors"
    return goal

register_option leaner.bitVectors : Bool := {
  defValue := false
  descr := "decide the leaves no other decision closes over bit vectors, as `pragma bv` selects"
}

/-- Decide a leaf over bit vectors, where `leaner.bitVectors` selects it. -/
elab "leaner_denote_bv" : tactic => do
  unless leaner.bitVectors.get (← getOptions) do throwError "bit-vector decisions are not selected"
  let goal ← liftGoal (← getMainGoal)
  replaceMainGoal [goal]
  evalTactic (← `(tactic| bv_decide))

end LeanerIR.Proofs.Denote.BitLift
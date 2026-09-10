-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Move.Attributes
import Move.Syntax
import Move.Semantics.Global
import Move.Semantics.Vector
import Move.Verify.BorrowChecker
import Move.Verify.Compare
import Move.Verify.Contract
import Move.Verify.Paths

/-- Set by the pragma-aware `spec` forms for `pragma opaque`: the function's
source semantics for callers is its contract's summary (`Contract.summary`),
not a translation of its body; the body, when translatable, is still what
`verify` checks against the contract. -/
register_option move.specOpaque : Bool := {
  defValue := false
  descr := "specify the function by its contract's summary (`pragma opaque`)"
}

/-- Report why a function's source semantics could not be derived at its
declaration (`#derive_move_source_spec`, emitted for every `fun` with a
body).  The derivation is best effort and silent by default: the `spec` or
the caller that needs the semantics reports the limitation. -/
register_option move.reportDerivation : Bool := {
  defValue := false
  descr := "report failures to derive a function's source semantics at its declaration"
}

-- The mutually recursive source translator is intentionally syntax-directed
-- and large; compiling its generated decision tree exceeds Lean's default
-- budget as the supported source surface grows.
set_option maxHeartbeats 400000

/-!
# Declarative source contracts

`spec f ...` attaches a source contract to `f` by defining `f.contract`.
`verify f ...` proves that contract as `f.verified`. Pure functions are
verified by reduction. For `Action` functions, the declaration macro records
the unexpanded source body and the effectful `spec` form translates its
accepted source constructs directly into `Semantics.Spec`.
-/

namespace Move.Spec

/-- Pre-state observation inside an effectful `ensures` clause. The
specification elaborator consumes this syntax before ordinary term
elaboration. -/
scoped syntax (name := oldResourceTerm) "old(" term ")" : term

/-- Test whether a typed resource exists at an address in the clause's
current state. -/
scoped syntax (name := resourceExistsTerm)
  "existsAt<" term ">(" term ")" : term

/-- Values accepted after `with` in an `aborts_if` clause. Move source abort
constants are `U64`, while the relational core stores codes as `Nat`. -/
class AbortCodeValue (Code : Type) where
  toNat : Code → Nat

instance : AbortCodeValue Nat := ⟨id⟩
instance : AbortCodeValue Move.U64 := ⟨Move.MoveInt.toNat⟩
instance {T : Type} : AbortCodeValue (Move.Vector T) :=
  ⟨fun _ => Move.unspecifiedAbortCode.toNat⟩

def abortCodeOf [AbortCodeValue Code] (code : Code) : Nat :=
  AbortCodeValue.toNat code

@[simp] theorem abortCodeOf_nat (code : Nat) : abortCodeOf code = code := rfl
@[simp] theorem abortCodeOf_u64 (code : Move.U64) :
    abortCodeOf code = code.toNat := rfl

/-! ## Mathematical integer expressions

Move integers remain bounded values in executable source. Specification
functions follow MSL and interpret their direct integers as unbounded
mathematical integers. The clause rewriter propagates that interpretation at
their call boundary and through clean mathematical contract/invariant
expressions, while explicit legacy representation views remain ordinary Lean.
Keeping this interpretation here, rather than as global coercions or
replacement operator instances, preserves Move's executable typing and
checked-arithmetic surface. -/

/-- A value which denotes a mathematical integer in a specification. -/
class IntValue (T : Type) where
  toInt : T → Int

instance : IntValue Int := ⟨id⟩
instance : IntValue Nat := ⟨Int.ofNat⟩
instance {S W : Type} [Move.Sign S] [Move.Width W] :
    IntValue (Move.MoveInt S W) := ⟨Move.MoveInt.toInt⟩

/-- Expected `Int` positions inside composite specification expressions
(`match` branches, conditionals, and opaque Lean applications) accept a
bounded leaf without exposing a projection in the authored term. Scoped so
executable source elaboration does not acquire a general widening rule. -/
scoped instance : Coe Move.U8 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.U16 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.U32 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.U64 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.U128 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.U256 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I8 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I16 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I32 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I64 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I128 Int := ⟨Move.MoveInt.toInt⟩
scoped instance : Coe Move.I256 Int := ⟨Move.MoveInt.toInt⟩

/-- The mathematical integer denoted by a specification value. -/
def int (value : T) [IntValue T] : Int := IntValue.toInt value

@[simp] theorem int_int (value : Int) : int value = value := rfl
@[simp] theorem int_nat (value : Nat) : int value = value := rfl
@[simp] theorem int_moveInt {S W : Type} [Move.Sign S] [Move.Width W]
    (value : Move.MoveInt S W) : int value = value.toInt := rfl

def intAdd (left : L) (right : R) [IntValue L] [IntValue R] : Int :=
  (int left) + (int right)
def intSub (left : L) (right : R) [IntValue L] [IntValue R] : Int :=
  (int left) - (int right)
def intMul (left : L) (right : R) [IntValue L] [IntValue R] : Int :=
  (int left) * (int right)
def intDiv (left : L) (right : R) [IntValue L] [IntValue R] : Int :=
  (int left) / (int right)
def intMod (left : L) (right : R) [IntValue L] [IntValue R] : Int :=
  (int left) % (int right)
def intNeg (value : T) [IntValue T] : Int :=
  -int value
def intShiftLeft (value : T) (amount : I) [IntValue T] [IntValue I] : Int :=
  (int value) <<< (int amount).toNat
def intShiftRight (value : T) (amount : I) [IntValue T] [IntValue I] : Int :=
  (int value) >>> (int amount).toNat

/-- Equality in clauses is heterogeneous for mathematical integers and
ordinary homogeneous equality for every other source type. -/
class LogicalEq (L R : Type) where
  equal : L → R → Prop

instance (priority := high) [IntValue L] [IntValue R] : LogicalEq L R where
  equal left right := int left = int right

instance : LogicalEq T T where
  equal := Eq

/-- Lean already permits a Boolean in proposition position.  Preserve that
reading for the common `result = predicate` contract idiom. -/
instance : LogicalEq Bool Prop where
  equal value proposition := (value = true) ↔ proposition
instance : LogicalEq Prop Bool where
  equal proposition value := proposition ↔ value = true

/-- A vector and its list-shaped specification value denote the same
sequence.  These two instances support vector update/append expressions,
whose mathematical results are lists. -/
instance : LogicalEq (Move.Vector T) (List T) where
  equal values list := values.toList = list
instance : LogicalEq (List T) (Move.Vector T) where
  equal list values := list = values.toList

def logicalEq (left : L) (right : R) [LogicalEq L R] : Prop :=
  LogicalEq.equal left right

instance (priority := high) [IntValue L] [IntValue R] (left : L) (right : R) :
    Decidable (logicalEq left right) := by
  change Decidable (int left = int right)
  infer_instance

instance [DecidableEq T] (left right : T) : Decidable (logicalEq left right) := by
  change Decidable (left = right)
  infer_instance

/-- Ordering is heterogeneous for mathematical integers.  The low-priority
homogeneous cases retain Lean's existing order for non-integer values. -/
class LogicalLT (L R : Type) where
  less : L → R → Prop

instance (priority := high) [IntValue L] [IntValue R] : LogicalLT L R where
  less left right := int left < int right

instance [LT T] : LogicalLT T T where
  less := LT.lt

def logicalLT (left : L) (right : R) [LogicalLT L R] : Prop :=
  LogicalLT.less left right

instance (priority := high) [IntValue L] [IntValue R] (left : L) (right : R) :
    Decidable (logicalLT left right) := by
  change Decidable (int left < int right)
  infer_instance

instance [LT T] [DecidableRel (@LT.lt T inferInstance)] (left right : T) :
    Decidable (logicalLT left right) := by
  change Decidable (left < right)
  infer_instance

class LogicalLE (L R : Type) where
  lessEq : L → R → Prop

instance (priority := high) [IntValue L] [IntValue R] : LogicalLE L R where
  lessEq left right := int left ≤ int right

instance [LE T] : LogicalLE T T where
  lessEq := LE.le

def logicalLE (left : L) (right : R) [LogicalLE L R] : Prop :=
  LogicalLE.lessEq left right

instance (priority := high) [IntValue L] [IntValue R] (left : L) (right : R) :
    Decidable (logicalLE left right) := by
  change Decidable (int left ≤ int right)
  infer_instance

instance [LE T] [DecidableRel (@LE.le T inferInstance)] (left right : T) :
    Decidable (logicalLE left right) := by
  change Decidable (left ≤ right)
  infer_instance

/-! Derived Move functions retain `Bool` comparison results.  These Boolean
companions use mathematical comparison whenever either side is an integer,
and Move's sealed structural comparison for homogeneous non-integer values. -/

class LogicalBoolEq (L R : Type) where
  equal : L → R → Bool

instance (priority := high) [IntValue L] [IntValue R] : LogicalBoolEq L R where
  equal left right := decide (int left = int right)

instance : LogicalBoolEq T T where
  equal := Move.Compare.equal

def logicalBoolEq (left : L) (right : R) [LogicalBoolEq L R] : Bool :=
  LogicalBoolEq.equal left right

class LogicalBoolLT (L R : Type) where
  less : L → R → Bool

instance (priority := high) [IntValue L] [IntValue R] : LogicalBoolLT L R where
  less left right := decide (int left < int right)

instance : LogicalBoolLT T T where
  less := Move.Compare.less

def logicalBoolLT (left : L) (right : R) [LogicalBoolLT L R] : Bool :=
  LogicalBoolLT.less left right

class LogicalBoolLE (L R : Type) where
  lessEq : L → R → Bool

instance (priority := high) [IntValue L] [IntValue R] : LogicalBoolLE L R where
  lessEq left right := decide (int left ≤ int right)

instance : LogicalBoolLE T T where
  lessEq left right := !Move.Compare.less right left

def logicalBoolLE (left : L) (right : R) [LogicalBoolLE L R] : Bool :=
  LogicalBoolLE.lessEq left right

def logicalVectorContains (values : Move.Vector T) (value : V)
    [LogicalBoolEq T V] : Bool :=
  values.toList.any (fun element => logicalBoolEq element value)

/-- Logical indexing of a Move vector.  This is a scoped specification
instance, so executable source still uses Move's `U64` vector primitive. -/
scoped instance logicalVectorGetElem [IntValue I] :
    GetElem? (Move.Vector T) I T
      (fun values index => 0 ≤ int index ∧ (int index).toNat < values.toList.length) where
  getElem values index inBounds :=
    values.toList[(int index).toNat]'inBounds.2
  getElem? values index := values.toList[(int index).toNat]?
  getElem! values index := values.toList[(int index).toNat]!

/-- Logical membership in a Move vector. -/
scoped instance logicalVectorMembership : Membership T (Move.Vector T) where
  mem values value := value ∈ values.toList

/-- Mathematical vector update and concatenation used by the transpiler for
MSL's value-level vector operations. -/
def vectorSet (values : Move.Vector T) (index : I) (value : T) [IntValue I] : List T :=
  values.toList.set (int index).toNat value

def vectorAppend (left right : Move.Vector T) : List T :=
  left.toList ++ right.toList

end Move.Spec

namespace Move.Verify.Source

open Lean Elab Command
open Lean.Parser.Term
open scoped Move Move.Spec

/-- Logical interpretation of authored `<` for the comparison operation that
the Move compiler lowers: Move's sealed structural comparison marker,
uniformly for every source type. Direct use of the marker, rather than
typeclass dispatch, ensures source verification cannot select semantics
different from the generated Move instruction. -/
def logicalLT {T : Type} (left right : T) : Prop :=
  Move.Compare.Less left right

instance (left right : T) : Decidable (logicalLT left right) :=
  inferInstanceAs (Decidable (Move.Compare.less left right = true))

/-- The comparison instruction on Move's integer types is numeric: the
sealed marker denotes the mathematical order at every width. Part of the
explicit trust base, like `logicalLT_move`. -/
axiom logicalLT_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalLT left right ↔ left.toNat < right.toNat

attribute [simp] logicalLT_uint

/-- The compiler's generic comparison marker denotes the fixed structural
ordering at a generic instantiation. The marker is opaque in executable
source, so this is the verification interface for that compiler semantic
fact. The comparator is fixed to `Move.Compare.genericLT`; it never uses a
caller-selected `LT` instance. -/
theorem logicalLT_move [Move.Compare.Total T] (left right : T) :
    logicalLT left right ↔
      @LT.lt T (Move.Compare.genericLT (T := T)) left right := Iff.rfl

attribute [simp] logicalLT_move

/-- Logical interpretation of authored `≤`, sealed to the same numeric
semantics used by the generated Move `lessEq` instruction. -/
def logicalLE {W : Type} [Move.Width W] (left right : Move.UInt W) : Prop :=
  left.toNat ≤ right.toNat

instance {W : Type} [Move.Width W] (left right : Move.UInt W) :
    Decidable (logicalLE left right) :=
  by unfold logicalLE; infer_instance

@[simp] theorem logicalLE_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalLE left right ↔ left.toNat ≤ right.toNat := Iff.rfl

/-- Sealed logical equality for authored `==`: Move's fixed structural
equality marker, uniformly for every source type, without consulting a
caller-provided `BEq` instance when a source contract is generated. -/
def logicalBEq {T : Type} (left right : T) : Bool :=
  Move.Compare.equal left right

/-- The equality instruction on Move's integer types is numeric: the sealed
marker denotes mathematical equality at every width. Part of the explicit
trust base, like `logicalBEq_move`. -/
axiom logicalBEq_uint {W : Type} [Move.Width W] (left right : Move.UInt W) :
    logicalBEq left right = true ↔ left.toNat = right.toNat

attribute [simp] logicalBEq_uint

@[simp] theorem logicalBoolEq_uint_toInt {W : Type} [Move.Width W]
    (left right : Move.UInt W) :
    Move.Spec.logicalBoolEq left right.toInt = logicalBEq left right := by
  change decide (left.toInt = right.toInt) = logicalBEq left right
  apply Bool.eq_iff_iff.mpr
  simp only [decide_eq_true_eq, logicalBEq_uint, Move.UInt.toInt_eq_toNat]
  exact Int.ofNat_inj

/-- The fixed generic equality marker is the source-level representation used
by Move's compiler for a type parameter constrained by `Compare.Total`. -/
theorem logicalBEq_move [Move.Compare.Total T] (left right : T) :
    logicalBEq left right = Move.Compare.equal left right := rfl

attribute [simp] logicalBEq_move

/-- Logical interpretation of `vector::contains`, using Move's sealed
structural equality rather than a caller-selected Lean `BEq` instance. -/
def vectorContains (values : Move.Vector T) (value : T) : Bool :=
  values.toList.any (fun element => logicalBEq element value)

@[simp] theorem logicalVectorContains_uint_int {W : Type} [Move.Width W]
    (values : Move.Vector (Move.UInt W)) (value : Move.UInt W) :
    Move.Spec.logicalVectorContains values value.toInt =
      vectorContains values value := by
  simp [Move.Spec.logicalVectorContains, vectorContains]

/-- First structural-equality match, or the list length when absent. The
propositional branch makes the sealed equality laws available to proof
simplification while retaining `List.findIdx` behavior. -/
def vectorFindIndex (value : T) : List T → Nat
  | [] => 0
  | head :: tail =>
      if logicalBEq head value = true then 0 else 1 + vectorFindIndex value tail

/-- Logical interpretation of `vector::index_of`. Move returns index zero
when no element matches, alongside a false presence flag. -/
def vectorIndexOf (values : Move.Vector T) (value : T) : Bool × Move.U64 :=
  let index := vectorFindIndex value values.toList
  let found := decide (index < values.toList.length)
  (found, Move.U64.ofNat (if found then index else 0))

private def lastString? : Name → Option String
  | .str _ suffix => some suffix
  | _ => none

private def sameLastName (left right : Name) : Bool :=
  left == right || match lastString? left, lastString? right with
    | some left, some right => left == right
    | _, _ => false

/-- Generated contracts package a function's parameters as one tuple, so the
operands a certified fact is about are projections of a local rather than
locals; expand a product-typed local into its components. -/
private def components (fuel : Nat) (e : Lean.Expr) (ty : Lean.Expr) :
    Lean.MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let ty ← Lean.Meta.whnfR ty
  match fuel with
  | 0 => return #[(e, ty)]
  | fuel + 1 =>
    if ty.isAppOfArity ``Prod 2 then
      let args := ty.getAppArgs
      let fst ← Lean.Meta.mkAppM ``Prod.fst #[e]
      let snd ← Lean.Meta.mkAppM ``Prod.snd #[e]
      return (← components fuel fst args[0]!) ++ (← components fuel snd args[1]!)
    return #[(e, ty)]

/-- Add the certified range facts for every integer- and vector-typed
hypothesis (`x.toNat < 2^n`, `v.toList.length < 2^64`), making the
representation bounds visible to `omega` and `grind`. -/
elab "uint_bounds" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut goal ← Lean.Elab.Tactic.getMainGoal
    let ctx ← Lean.getLCtx
    for decl in ctx do
      if decl.isImplementationDetail then continue
      let declType ← Lean.Meta.whnfR (← Lean.instantiateMVars decl.type)
      for (target, targetType) in ← components 8 decl.toExpr declType do
       let decl : Lean.Expr := target
       let type := targetType
       if type.isAppOf ``Move.Vector then
         let bound ← Lean.Meta.mkAppM ``Move.Vector.toList_length_lt
           #[decl]
         let lengthExpr ← Lean.Meta.mkAppM ``List.length
           #[← Lean.Meta.mkAppM ``Move.Vector.toList #[decl]]
         let boundType ← Lean.Meta.mkAppM ``LT.lt
           #[lengthExpr, Lean.mkNatLit (2 ^ 64)]
         goal ← (← goal.assert (Lean.Name.mkSimple "vectorBound") boundType
           bound).intro1P <&> (·.2)
         continue
       if type.isAppOf ``Move.MoveInt then
         -- `MoveInt S W`: the sign tag is the first type argument, the width
         -- tag the second.  Unsigned locals get the natural-number bound the
         -- specification language uses; signed locals get both `Int` bounds.
         let args := type.getAppArgs
         let bits? : Option Nat :=
           match args[1]? with
           | some (Lean.Expr.const tag _) =>
               if tag == ``Move.W8 then some 8
               else if tag == ``Move.W16 then some 16
               else if tag == ``Move.W32 then some 32
               else if tag == ``Move.W64 then some 64
               else if tag == ``Move.W128 then some 128
               else if tag == ``Move.W256 then some 256
               else none
           | _ => none
         let signed? : Option Bool :=
           match args[0]? with
           | some (Lean.Expr.const tag _) =>
               if tag == ``Move.Unsigned then some false
               else if tag == ``Move.Signed then some true
               else none
           | _ => none
         match signed? with
         | some false =>
             let bound ← Lean.Meta.mkAppM ``Move.UInt.toNat_lt #[decl]
             let boundType ← match bits? with
               | some bits =>
                   let toNatExpr ← Lean.Meta.mkAppM ``Move.MoveInt.toNat #[decl]
                   Lean.Meta.mkAppM ``LT.lt #[toNatExpr, Lean.mkNatLit (2 ^ bits)]
               | none => Lean.Meta.inferType bound
             goal ← (← goal.assert (Lean.Name.mkSimple "uintBound") boundType bound).intro1P
               <&> (·.2)
             -- Only the natural-number bound.  The `Int` view of the same
             -- fact was asserted here while the checked rules still spoke the
             -- neutral `Int` form; now that they are per-view, adding it puts
             -- `Int` atoms into otherwise pure-`Nat` goals and makes the
             -- decision procedures reason over a mixed domain for nothing.
         | some true =>
             let lower ← Lean.Meta.mkAppM ``Move.SInt.neg_halfSize_le_toInt #[decl]
             goal ← (← goal.assert (Lean.Name.mkSimple "sintLower")
               (← Lean.Meta.inferType lower) lower).intro1P <&> (·.2)
             let upper ← Lean.Meta.mkAppM ``Move.SInt.toInt_lt_halfSize #[decl]
             goal ← (← goal.assert (Lean.Name.mkSimple "sintUpper")
               (← Lean.Meta.inferType upper) upper).intro1P <&> (·.2)
         | none => pure ()
    Lean.Elab.Tactic.replaceMainGoal [goal]

/-- Add the data invariant of every certified-typed hypothesis, which is what
makes the invariant "available wherever the value is" without naming the type
or its generated condition.

Deliberately *not* folded into `uint_bounds`.  A width bound is one cheap
atomic fact; a data invariant can be an arbitrarily large predicate — the
ordered map's is a sortedness condition over the whole entry list — and
asserting one into every context the automatic cascade normalizes costs far
more than the proofs that want it save.  It is a tactic a proof asks for. -/
elab "data_invariants" : tactic => do
  Lean.Elab.Tactic.withMainContext do
    let mut goal ← Lean.Elab.Tactic.getMainGoal
    let ctx ← Lean.getLCtx
    for decl in ctx do
      if decl.isImplementationDetail then continue
      let declType ← Lean.Meta.whnfR (← Lean.instantiateMVars decl.type)
      for (target, targetType) in ← components 8 decl.toExpr declType do
        let some typeName := targetType.getAppFn.constName? | continue
        unless (Move.dataInvariant? (← Lean.getEnv) typeName).isSome do continue
        try
          let dataInvariant ← Lean.Meta.mkAppM (typeName ++ `dataInvariant) #[target]
          -- Assert the condition with its generated name unfolded: this runs
          -- after the cascade's normalization, so `move_invariant_norm` would
          -- no longer see it.
          let condition ← Lean.Meta.inferType dataInvariant
          let condition := (← Lean.Meta.unfoldDefinition? condition).getD condition
          goal ← (← goal.assert (Lean.Name.mkSimple "dataInvariant")
            condition dataInvariant).intro1P <&> (·.2)
        catch _ => pure ()
    Lean.Elab.Tactic.replaceMainGoal [goal]

/-- Discharge a `Nat`/`Int` (or `UInt` (in)equality) goal that mixes source
integer and vector-length views.  It brings in the data invariant of every
certified value (`data_invariants`) and the width/length bounds
(`uint_bounds`), folds vectors to their `.toList`, integers to `Nat`
(`toInt`/`ofNat`/`mod`, and `UInt` (in)equalities to their `toNat` view),
normalizes Boolean loop guards (`!decide (a < b) = true` and friends) to the
underlying `Nat` comparison, and finishes with `omega`.  This is the numeric
closer for shifted-index vector reasoning, where a raw-field length atom and a
`.toList` length atom would otherwise stay distinct. -/
syntax "uint_arith" : tactic
macro_rules
  | `(tactic| uint_arith) =>
    `(tactic|
      (data_invariants
       uint_bounds
       (try simp only [Move.Vector.elems_eq_toList, Move.UInt.toInt_eq_toNat,
          Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le,
          Move.UInt.eq_iff_toNat_eq, Move.UInt.toNat_zero, Move.UInt.toNat_one,
          Bool.not_eq_true, Bool.not_eq_true', Bool.not_eq_false,
          decide_eq_true_eq, decide_eq_false_iff_not,
          Nat.not_lt, Nat.not_le, Classical.not_not] at *)
       (try simp (disch := omega) only [Move.UInt.toNat_ofNat_u64,
          Move.UInt.toNat_ofNat_numeral, Nat.mod_eq_of_lt] at *)
       omega))

/-- Source retained by the `fun` command for later specification generation.
This is deliberately syntax, rather than LIR or Move IR: verification is
defined over the authored source constructs. -/
private structure Declaration where
  resultType : Syntax
  value : Syntax
  deriving Inhabited

/-- Retained declarations, persisted so that an imported module's functions
keep their source for callers in other modules. -/
private initialize declarations :
    SimplePersistentEnvExtension (Name × Declaration) (NameMap Declaration) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, declaration) => map.insert name declaration
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun map (name, declaration) => map.insert name declaration) {} entries
  }

syntax (name := move_source)
  "move_source" "(" term ", " num ", " str ")" : attr

/-- Replace the source preceding a retained body with byte-for-byte whitespace.
Parsing the padded body then recreates its original raw positions without
allowing preceding declarations to become part of the retained term. -/
private def retainedSourcePrefix (source : String) (length : Nat) : String :=
  let bytes := source.toUTF8.extract 0 length
  let whitespace := bytes.data.map fun byte =>
    if byte == 9 || byte == 10 || byte == 13 then byte else 32
  String.fromUTF8! ⟨whitespace⟩

initialize moveSourceAttr : Unit ← Lean.registerBuiltinAttribute {
  name := Name.mkSimple "move_source"
  descr := "retained Move source for relational specification generation"
  add := fun declarationName stx _ => do
    match stx.getHeadInfo with
    | .synthetic .. => pure ()
    | _ => throwErrorAt stx "`move_source` is compiler-internal; use `fun` to retain a source body"
    let `(attr| move_source ($resultType:term, $offset:num, $encoded:str)) := stx
      | throwErrorAt stx "invalid retained Move source"
    let some offset := offset.raw.isNatLit?
      | throwErrorAt offset "expected retained Move source offset"
    let some source := encoded.raw.isStrLit?
      | throwErrorAt encoded "expected encoded Move source"
    let fileMap ← getFileMap
    let input := retainedSourcePrefix fileMap.source offset ++ source
    let value ← match Lean.Parser.runParserCategory (← getEnv) `term input (← getFileName) with
      | .ok value => pure value
      | .error message => throwErrorAt encoded
          "failed to restore retained Move source: {message}\n{source}"
    let declaration := { resultType := resultType.raw, value }
    modifyEnv fun env => declarations.addEntry env (declarationName, declaration)
}

private def declarationFor (function : Syntax) : CommandElabM Declaration := do
  let name := (← getCurrNamespace) ++ function.getId
  let some declaration := declarations.getState (← getEnv) |>.find? name
    | throwErrorAt function
        "no retained Move source declaration; use `fun`, not `def`, for an effectful Move function"
  pure declaration

private partial def findTypeApplication? (name : Name) (stx : Syntax) : Option Syntax :=
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 &&
      stx[0].isIdent && sameLastName stx[0].getId name then
    if stx[1].isOfKind `null && stx[1].getNumArgs == 1 then some stx[1][0]
    else some stx[1]
  else
    stx.getArgs.findSome? (findTypeApplication? name)

/-- One flattened Move result component. Products are transient multiple
returns, so their reference components are represented independently at the
mutation boundary and rebuilt in the same right-nested product order. -/
private inductive ResultLeaf where
  | value (type : TSyntax `term)
  | immutable (referent : TSyntax `term)
  | mutable (referent : TSyntax `term)
  deriving Inhabited

private partial def resultLeaves (type : TSyntax `term) : Array ResultLeaf :=
  match type with
  | `(($inner:term)) => resultLeaves inner
  | `($left:term × $right:term) => resultLeaves left ++ resultLeaves right
  | `(Prod $left:term $right:term) => resultLeaves left ++ resultLeaves right
  | `(& $referent:term) => #[.immutable referent]
  | `(&mut $referent:term) => #[.mutable referent]
  | `(Move.ImmRef $referent:term) => #[.immutable referent]
  | `(Move.MutRef $referent:term) => #[.mutable referent]
  | _ => #[.value type]

private def ResultLeaf.logicalType : ResultLeaf → TSyntax `term
  | .value type | .immutable type | .mutable type => type

private def ResultLeaf.isMutable : ResultLeaf → Bool
  | .mutable _ => true
  | _ => false

/-- References are erased recursively in specification results. -/
private def erasedResultTypes (type : TSyntax `term) : Array (TSyntax `term) :=
  (resultLeaves type).map (·.logicalType)

private def packResultTypes (types : Array (TSyntax `term)) : MacroM (TSyntax `term) := do
  match types.size with
  | 0 => `(Unit)
  | 1 => pure types[0]!
  | _ =>
      let reversed := types.toList.reverse
      reversed.tail.foldlM (init := reversed.head!) fun result type =>
        `($type × $result)

private def declaredResultType (declaration : Declaration) : TSyntax `term :=
  match findTypeApplication? ``Move.Action declaration.resultType with
  | some result => ⟨result⟩
  | none => ⟨declaration.resultType⟩

private def returnsMutableReferences (declaration : Declaration) : Bool :=
  (resultLeaves (declaredResultType declaration)).any (·.isMutable)

private def actionResultType (declaration : Declaration) : CommandElabM (TSyntax `term) := do
  let result := declaredResultType declaration
  liftMacroM <| packResultTypes (erasedResultTypes result)

/-- Result type retained for an effectful Move source declaration.  This is
also used when a recursive function supplies its relational source semantics
as an ordinary, predeclared `f.sourceSpec` helper. -/
def resultTypeOf (function : Syntax) : CommandElabM (TSyntax `term) := do
  actionResultType (← declarationFor function)

/-- Whether a Move source function is effectful — its result is an `Action`, so
its contract observes global state rather than being a pure value predicate. -/
def isEffectfulFunction (function : Syntax) : CommandElabM Bool := do
  return (findTypeApplication? ``Move.Action (← declarationFor function).resultType).isSome

/-- The components of a dotted identifier, `x.f.g` ↦ `["x", "f", "g"]`.  A
hygienic name — a loop-freshened local, or a dotted use of one — is read
without its macro scopes, which `sameScopes` compares. -/
private def fieldParts (name : Name) : List String :=
  go (extractMacroScopes name).name
where
  go : Name → List String
    | .anonymous => []
    | .str base part => go base ++ [part]
    | .num _ _ => []

/-- Whether two identifiers carry the same macro scopes: a dotted use of a
loop-freshened local shares the scopes of its root. -/
private def sameScopes (a b : Name) : Bool :=
  let av := extractMacroScopes a
  let bv := extractMacroScopes b
  av.scopes == bv.scopes && av.ctx == bv.ctx && av.imported == bv.imported

/-- The root `root` of the dotted identifier `name`, as an identifier with
`name`'s macro scopes. -/
private def rootIdent (ref : Syntax) (name : Name) (root : String) : TSyntax `ident :=
  let view := extractMacroScopes name
  mkIdentFrom ref { view with name := Name.mkSimple root }.review

/-- Split a dotted identifier `receiver.field` at its last component; the
receiver keeps the macro scopes. -/
private def splitLastField? (name : Name) : Option (Name × Name) :=
  let view := extractMacroScopes name
  match view.name with
  | .str base field =>
      if base.isAnonymous then none
      else some ({ view with name := base }.review, Name.mkSimple field)
  | _ => none

private partial def splitFieldPath (place : TSyntax `term) :
    (TSyntax `term × Array (TSyntax `ident)) :=
  if place.raw.isOfKind ``Lean.Parser.Term.proj && place.raw.getNumArgs >= 3 &&
      place.raw[2].isIdent then
    let base : TSyntax `term := ⟨place.raw[0]⟩
    let fields := fieldParts place.raw[2].getId |>.toArray.map
      (fun field => mkIdentFrom place (Name.mkSimple field))
    let result := splitFieldPath base
    (result.1, result.2 ++ fields)
  else
    (place, #[])

def canonicalResourceName (resource : TSyntax `ident) : CommandElabM Name := do
  try
    resolveGlobalConstNoOverload resource.raw
  catch _ =>
    pure ((← getCurrNamespace) ++ resource.getId)

private def isResourceIdentifier (resource : TSyntax `ident) : CommandElabM Bool := do
  let env ← getEnv
  let current := (← getCurrNamespace) ++ resource.getId
  pure <| Move.moveKeyAttr.hasTag env current ||
    Move.moveKeyAttr.hasTag env resource.getId

/-- A resource family as source names it: a `Key` type applied to its type
arguments (`Vault T`), or a bare resource type (`Counter`).  Two mentions
denote the same family when the head type and the arguments, as written,
agree.  Families with the same head may still coincide at different
arguments (`Vault T` and `Vault U` when `T = U`), so only families with
distinct heads are ever assumed independent. -/
structure Family where
  /-- The family's type, as a term: the descriptor's `Value`. -/
  term : TSyntax `term
  /-- The canonical name of the head resource type. -/
  head : Name
  /-- The identity: head and arguments as written. -/
  key : String
  /-- Whether `term` names the family in the current scope.  A generic family
  of a callee (`Vault T` at the callee's own `T`) is known to the caller only
  by its head: the head's store is in scope, but no concrete instantiation is
  named, so frames do not mention it. -/
  concrete : Bool := true
  deriving Inhabited

/-- The distinct heads of some families, in order of first mention. -/
def distinctHeads (families : Array Family) : Array Name :=
  families.foldl (init := #[]) fun heads family =>
    if heads.contains family.head then heads else heads.push family.head

/-- Fresh names for the type parameters of a resource head, one per
parameter; a resource's parameters are types. -/
def headParameters (head : Name) : CommandElabM (Array (TSyntax `ident)) := do
  let info ← getConstInfo head
  liftTermElabM <| Lean.Meta.forallTelescope info.type fun parameters _ => do
    let mut names := #[]
    for parameter in parameters do
      let type ← Lean.Meta.whnf (← Lean.Meta.inferType parameter)
      unless type == Lean.mkSort Lean.Level.one do
        throwError "resource `{head}` has a parameter that is not a `Type`; automatic source specifications support only type parameters"
      names := names.push (mkIdent (Name.mkSimple s!"_moveSpecT{names.size}"))
    return names

/-- A resource head applied to parameters. -/
def appliedHead (head : Name) (parameters : Array (TSyntax `ident)) :
    CommandElabM (TSyntax `term) :=
  if parameters.isEmpty then pure ⟨mkIdent head⟩
  else `($(mkIdent head) $parameters*)

/-- Quantify a proposition over type parameters, `∀ {T₀ … : Type}, body`. -/
def forallTypes (parameters : Array (TSyntax `ident)) (body : TSyntax `term) :
    CommandElabM (TSyntax `term) :=
  parameters.foldrM (init := body) fun parameter body => `(∀ {$parameter : Type}, $body)

/-- The store instance of a head: for a generic head, one store for every
instantiation — each instantiation is its own family, as in Move. -/
def storeType (world : TSyntax `term) (head : Name) : CommandElabM (TSyntax `term) := do
  let parameters ← headParameters head
  forallTypes parameters
    (← `(Move.Semantics.ResourceStore $world $(← appliedHead head parameters)))

/-- Independence of two heads' stores, at every instantiation of each. -/
def independenceType (world : TSyntax `term) (left right : Name) :
    CommandElabM (TSyntax `term) := do
  let leftParameters ← headParameters left
  let rightParameters ← (← headParameters right).mapM fun parameter =>
    pure (mkIdent (parameter.getId.appendAfter "'"))
  forallTypes (leftParameters ++ rightParameters)
    (← `(Move.Semantics.IndependentResourceStores $world
      $(← appliedHead left leftParameters) $(← appliedHead right rightParameters)))

/-- Whether a type term names only global constants, so it denotes the same
family in every scope. -/
private partial def closedType (stx : Syntax) : CommandElabM Bool := do
  if stx.isIdent then
    return ← try
        let names ← resolveGlobalConst stx
        pure !names.isEmpty
      catch _ => pure false
  stx.getArgs.allM closedType

/-- The family named by a resource type term, if its head is a resource. -/
partial def familyOfTerm? (type : Syntax) : CommandElabM (Option Family) := do
  let type := if type.isOfKind ``Lean.Parser.Term.paren then type[1] else type
  if type.isOfKind ``Lean.Parser.Term.paren then return ← familyOfTerm? type
  let (headIdent, arguments) ←
    if type.isIdent then
      pure (type, #[])
    else if type.isOfKind ``Lean.Parser.Term.app && type.getNumArgs == 2 && type[0].isIdent then
      pure (type[0], type[1].getArgs)
    else
      return none
  let headIdent : TSyntax `ident := ⟨headIdent⟩
  unless ← isResourceIdentifier headIdent do return none
  let head ← canonicalResourceName headIdent
  let key := arguments.foldl (init := head.toString) fun key argument =>
    key ++ " " ++ toString argument.prettyPrint
  return some { term := ⟨type⟩, head, key }

/-- The family of a bare resource type named canonically. -/
def familyOfName (ref : Syntax) (name : Name) : Family :=
  { term := ⟨mkIdentFrom ref name⟩, head := name, key := name.toString }

private def pushResource (resources : Array Family) (family : Family) : Array Family :=
  if resources.any (·.key == family.key) then resources else resources.push family

/-- The leading type constructor of a (possibly parenthesized, applied, or
ascribed) type expression: `Vault` from `(Vault T)`, `Counter` from
`({ … } : Counter)`. -/
private partial def typeHead? (stx : Syntax) : Option (TSyntax `ident) :=
  if stx.isIdent then some ⟨stx⟩
  else if stx.isOfKind ``Lean.Parser.Term.paren then
    stx.getArgs[1]? |>.bind typeHead?
  else if stx.isOfKind ``Lean.Parser.Term.app then
    stx.getArgs[0]? |>.bind typeHead?
  else if stx.isOfKind ``Lean.Parser.Term.typeAscription then
    -- `(expr : type)` — the type is the fourth child, wrapped in a `null` node.
    (stx.getArgs[3]?.bind (·.getArgs[0]?)) |>.bind typeHead?
  else if stx.isOfKind nullKind then
    stx.getArgs[0]? |>.bind typeHead?
  else none

/-- The type term an argument names: itself, or the ascribed type of
`({ … } : T)`. -/
private def namedType? (stx : Syntax) : Option Syntax :=
  if stx.isOfKind ``Lean.Parser.Term.typeAscription then
    stx.getArgs[3]?.bind (·.getArgs[0]?)
  else
    some stx

/-- The resource family named by a global-storage primitive application
(`existsAt`/`moveFrom` name it directly; `moveTo` names it through the published
value's ascription), if `stx` is such an application. -/
private def globalPrimitiveResource? (stx : Syntax) :
    CommandElabM (Option Family) := do
  unless stx.isOfKind ``Lean.Parser.Term.app do return none
  let head := stx[0]
  unless head.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload head))
      catch _ => pure none) | return none
  let arguments := stx[1].getArgs
  if name == ``Move.existsAt || name == ``Move.moveFrom then
    let some type := arguments[0]? | return none
    familyOfTerm? type
  else if name == ``Move.moveTo then
    let some value := arguments[1]? | return none
    let some type := namedType? value | return none
    familyOfTerm? type
  else
    return none

/-- The family and key of a global place root `R[key]` / `(R T)[key]`. -/
private def rootFamily? (root : TSyntax `term) :
    CommandElabM (Option (Family × TSyntax `term)) := do
  let (typeStx, key) ← match root with
    | `($resource:ident[$key:term]) => pure (resource.raw, key)
    | `(getElem $resource:ident $key:term $_:term) => pure (resource.raw, key)
    | _ =>
        if root.raw.getKind == `«term__[_]» && root.raw.getNumArgs == 4 then
          pure (root.raw[0], (⟨root.raw[2]⟩ : TSyntax `term))
        else
          return none
  let some family ← familyOfTerm? typeStx | return none
  return some (family, key)

private partial def collectResources (stx : Syntax)
    (resources : Array Family := #[]) : CommandElabM (Array Family) := do
  let mut resources := resources
  if let some family ← globalPrimitiveResource? stx then
    resources := pushResource resources family
  if stx.isOfKind ``Move.borrowTerm || stx.isOfKind ``Move.borrowMutTerm then
    if let some place := stx[1]? then
      let (root, _) := splitFieldPath ⟨place⟩
      if let some (family, _) ← rootFamily? root then
        resources := pushResource resources family
  else if stx.isOfKind ``Move.borrowIndexTerm ||
      stx.isOfKind ``Move.borrowMutIndexTerm then
    if let some candidate := stx[1]? then
      if let some family ← familyOfTerm? candidate then
        resources := pushResource resources family
  for child in stx.getArgs do
    resources ← collectResources child resources
  pure resources

private def globalPlace? (place : TSyntax `term) :
    CommandElabM (Option (Family × TSyntax `term × Array (TSyntax `ident))) := do
  let (root, fields) := splitFieldPath place
  let some (family, key) ← rootFamily? root | return none
  return some (family, key, fields)

private def globalPlace (place : TSyntax `term) :
    CommandElabM (Family × TSyntax `term × Array (TSyntax `ident)) := do
  let some result ← globalPlace? place
    | throwErrorAt place
        "automatic source specifications currently expect a global place `Resource[key]`"
  pure result

private def projectPath (owner : TSyntax `term)
    (fields : Array (TSyntax `ident)) : CommandElabM (TSyntax `term) := do
  if fields.isEmpty then return owner
  `(selectPath% $owner [$fields,*])

private partial def replaceIdentifier (name : Name) (replacement stx : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then replacement
  else if stx.isIdent && stx.getId.getRoot == name && replacement.isIdent then
    -- A dotted use of the variable (`x.push v`): the root is renamed.
    mkIdentFrom stx (replacement.getId ++ stx.getId.replacePrefix name .anonymous)
  else if stx.isOfKind ``Move.anchoredOldTerm then
    -- An anchored old-value observation is a snapshot from before the loop,
    -- not another spelling of its threaded state variable.
    stx
  else if stx.isOfKind ``Lean.Parser.Term.structInstField then
    -- A structure-instance field label is not a variable.
    stx.setArgs (stx.getArgs.mapIdx fun i arg =>
      if i == 0 then arg else replaceIdentifier name replacement arg)
  else stx.setArgs (stx.getArgs.map (replaceIdentifier name replacement))

/-- `owner` with `newValue` written at `fields` (decided per step from the
owner's type when elaborated: a structure update, or a variant rebuild for an
enum payload field). -/
private def updatePath (owner newValue : TSyntax `term)
    (fields : List (TSyntax `ident)) : CommandElabM (TSyntax `term) := do
  match fields with
  | [] => pure newValue
  | _ => `(updatePath% $owner [$(fields.toArray),*] $newValue)

/-- The type a mutable parameter refers to, if source translation can name
it. -/
private def referentTypeName? (type : TSyntax `term) : CommandElabM (Option Name) := do
  let head := if type.raw.isIdent then type.raw else
    (Lean.Syntax.getArgs type.raw)[0]? |>.getD type.raw
  unless head.isIdent do return none
  try
    return some (← resolveGlobalConstNoOverload head)
  catch _ =>
    return none

/-- Rebuild the owner of a mutated field.  A certified owner is re-created
through `Spec.certified`, which is where its data invariant is owed; an
ordinary owner is a plain structure update. -/
private def rebuildOwner (owner newValue : TSyntax `term)
    (fields : List (TSyntax `ident)) (certified? : Option (Name × Name)) :
    CommandElabM (Option (TSyntax `term)) := do
  let some (typeName, invariantName) := certified? | return none
  -- Replacing the whole value installs a value that already carries its
  -- certificate, so only a field write-back re-creates the owner.
  let field :: rest := fields | return none
  let env ← getEnv
  let some info := getStructureInfo? env typeName | return none
  let dataFields := info.fieldNames.filter (· != `dataInvariant)
  let arguments ← dataFields.mapM fun fieldName =>
    if fieldName == field.getId then
      if rest.isEmpty then pure newValue
      else do updatePath (← `($owner.$field)) newValue rest
    else
      `($owner.$(mkIdent fieldName))
  let rawValue ← `($(mkIdent (typeName ++ `Raw ++ `mk)) $arguments*)
  let holds := mkIdentFrom field `_moveSpecInvariant
  let built ← `(fun $holds =>
    $(mkIdent (typeName ++ `mk)) $arguments* $holds)
  return some (← `(Move.Semantics.Spec.certified
    (Invariant := $(mkIdent invariantName) $rawValue) $built))

private structure ResourceBinding where
  head : Name
  /-- The descriptor of an instantiation of the head. -/
  descriptorFor : Family → CommandElabM (TSyntax `term)

private def resourceFor (resources : Array ResourceBinding)
    (family : Family) : CommandElabM (TSyntax `term) := do
  let some binding := resources.find? fun binding => binding.head == family.head
    | throwErrorAt family.term
        "no resource descriptor was supplied for `{family.term}`"
  binding.descriptorFor family

/-- Whether an identifier names a resource family in scope (of any
instantiation). -/
private def hasResource (resources : Array ResourceBinding)
    (candidate : TSyntax `ident) : CommandElabM Bool := do
  let candidate ← canonicalResourceName candidate
  return resources.any fun binding => binding.head == candidate

private def localVectorPlace? (contextResources : Array ResourceBinding)
    (place : TSyntax `term) :
    CommandElabM (Option (TSyntax `ident × TSyntax `term × Array (TSyntax `ident))) := do
  let (root, fields) := splitFieldPath place
  match root with
  | `($owner:ident[$index:term]) =>
      if ← hasResource contextResources owner then pure none
      else pure (some (owner, index, fields))
  | `(getElem $owner:ident $index:term $_:term) =>
      if ← hasResource contextResources owner then pure none
      else pure (some (owner, index, fields))
  | _ => pure none

private def localPlace? (contextResources : Array ResourceBinding)
    (place : TSyntax `term) : CommandElabM (Option (TSyntax `ident × Array (TSyntax `ident))) := do
  if place.raw.isIdent then
    let parts := fieldParts place.raw.getId
    if let ownerName :: fields := parts then
      let owner := rootIdent place place.raw.getId ownerName
      if !(← hasResource contextResources owner) then
        return some (owner, fields.toArray.map fun field =>
          mkIdentFrom place (Name.mkSimple field))
  let (root, fields) := splitFieldPath place
  unless root.raw.isIdent do return none
  let owner : TSyntax `ident := ⟨root.raw⟩
  if ← hasResource contextResources owner then return none
  return some (owner, fields)

private structure VerificationLoopFrame where
  sourceLabel? : Option Name
  recursive : TSyntax `term
  after : TSyntax `term
  assigned : List (TSyntax `ident)
  state : List (TSyntax `ident)

/-- A control-flow exit selected while a mutable loan is still open.  The
loan body returns a continuation function instead of executing the exit;
each `roots` binder is supplied with its revived mutation after the enclosing
loan combinator has reconciled. `normal` is the ordinary post-loan path and
may mention `value`, which is bound to the loan body's result. -/
private structure LoanExitFrame where
  roots : Array (TSyntax `ident)
  value : TSyntax `ident
  normal : TSyntax `term
  /-- Special mutation-returning loans do not have an ordinary continuation:
  an enclosing control exit resolves this transferred mutation directly. -/
  resolve? : Option (TSyntax `ident) := none
  /-- Fixed points already active when the loan opened. Exits targeting loops
  created inside the loan stay inside it; only these enclosing targets unwind
  the loan. -/
  enclosingLoops : List Name
  resumeMutation? : Option (TSyntax `ident)
  resumeMutationType? : Option Name
  resumeMutationAncestors : List (TSyntax `ident × Option Name)
  resumeMutationOwnerAliases : List (Name × TSyntax `ident × Option Name)
  resumeMutationRefs : List (TSyntax `ident)
  resumeRootMutations : Array (TSyntax `ident)
  resumeTransferredReturns : Array (TSyntax `ident)
  resumeResolveBeforeReturn : Array (TSyntax `ident)
  resumeMutationLoans : List (Name × List Name)

/-- One value captured by the compiler before an inlined fold.  The Move
prover represents this as `with_state_anchor(label, old(value))`; generated
Leaner keeps the label in syntax until the source translator replaces it with
this lexical snapshot. -/
private structure StateAnchorCapture where
  label : Nat
  observed : Syntax
  snapshot : TSyntax `ident

private structure TranslationContext where
  world : TSyntax `term
  resources : Array ResourceBinding
  functionName : Name
  /-- The function's parameters: `old(p)` in a loop invariant is `p` at entry. -/
  parameters : List Name := []
  /-- Value snapshots installed by preceding compiler-generated fold capture
  markers.  They remain lexical across the loop fixed point. -/
  stateAnchors : List StateAnchorCapture := []
  recursiveSpec? : Option (TSyntax `term) := none
  /-- Recursive entry points for every member of the current mutual SCC. -/
  recursiveSpecs : Array (Name × TSyntax `term) := #[]
  mutation? : Option (TSyntax `ident) := none
  /-- The type of the active mutation's referent, when source translation can
  name it: the resource of a global borrow, the declared referent of the
  mutable parameter, or the field reached from either.  A certified referent is
  re-created when a nested loan dies, which is a creation site of its data
  invariant. -/
  mutationType? : Option Name := none
  /-- Outstanding owner mutations below the active focused mutation. They let
  a nested loan focus a disjoint sibling field of an ancestor owner. -/
  mutationAncestors : List (TSyntax `ident × Option Name) := []
  /-- Source owners whose current value is held by a live mutation handle.
  This also permits a checked same-place handle to focus the current mutation
  without opening an unrelated owner prophecy. -/
  mutationOwnerAliases : List
    (Name × TSyntax `ident × Option Name) := []
  /-- Other live focused mutations surrounding the active one. Their values
  remain readable while a disjoint sibling loan is active. -/
  mutationRefs : List (TSyntax `ident) := []
  /-- The mutable parameters opened at the function boundary, in source
  order.  A nested field loan temporarily replaces `mutation?`, but the
  function result must still return every root mutation. -/
  rootMutations : Array (TSyntax `ident) := #[]
  /-- This body is the mutation-level relation of a function returning
  one or more `&mut` components. -/
  returnsMutation : Bool := false
  /-- Flattened declared result layout at a mutation boundary. -/
  resultLeaves : Array ResultLeaf := #[]
  /-- Mutations already transferred into this function by returned-reference
  calls. Returning these handles forwards their existing prophecies instead of
  allocating another final reborrow. -/
  transferredReturns : Array (TSyntax `ident) := #[]
  /-- Reborrowed handles used only inside a returned-reference computation.
  Their prophecies are installed in the rebuilt owner, so they must resolve
  before the function returns. -/
  resolveBeforeReturn : Array (TSyntax `ident) := #[]
  /-- Field paths currently checked out from each owner. Only paths with
  distinct first fields can be borrowed as siblings. -/
  mutationLoans : List (Name × List Name) := []
  /-- The loan whose body is being translated. Special mutation-returning
  paths still use this marker for source-positioned loop-exit diagnostics. -/
  loanScope? : Option (TSyntax `ident) := none
  /-- An ordinary lexical loan returns its post-loan computation as a closure.
  This both delays control exits until reconciliation and transports writes to
  disjoint outer mutations captured by that closure. -/
  loanExits : List LoanExitFrame := []
  loops : List VerificationLoopFrame := []

private def mkLoanExitFrame (context : TranslationContext)
    (roots : Array (TSyntax `ident)) (value : TSyntax `ident)
    (normal : TSyntax `term) : LoanExitFrame := {
  roots
  value
  normal
  enclosingLoops := context.loops.map (·.recursive.raw.getId)
  resumeMutation? := context.mutation?
  resumeMutationType? := context.mutationType?
  resumeMutationAncestors := context.mutationAncestors
  resumeMutationOwnerAliases := context.mutationOwnerAliases
  resumeMutationRefs := context.mutationRefs
  resumeRootMutations := context.rootMutations
  resumeTransferredReturns := context.transferredReturns
  resumeResolveBeforeReturn := context.resolveBeforeReturn
  resumeMutationLoans := context.mutationLoans }

private def mkTransferredLoanExitFrame (context : TranslationContext)
    (mutation : TSyntax `ident) (normal : TSyntax `term) : LoanExitFrame :=
  { mkLoanExitFrame context #[] mutation normal with
      resolve? := some mutation }

private def resumeAfterLoan (context : TranslationContext)
    (frame : LoanExitFrame) : TranslationContext := {
  context with
    mutation? := frame.resumeMutation?
    mutationType? := frame.resumeMutationType?
    mutationAncestors := frame.resumeMutationAncestors
    mutationOwnerAliases := frame.resumeMutationOwnerAliases
    mutationRefs := frame.resumeMutationRefs
    rootMutations := frame.resumeRootMutations
    transferredReturns := frame.resumeTransferredReturns
    resolveBeforeReturn := frame.resumeResolveBeforeReturn
    mutationLoans := frame.resumeMutationLoans
    loanScope? := none
    loanExits := context.loanExits.tail }

private partial def resumeAfterAllLoans
    (context : TranslationContext) : TranslationContext :=
  match context.loanExits.head? with
  | none => context
  | some frame => resumeAfterAllLoans (resumeAfterLoan context frame)

/-- The data invariant certified by the active mutation's referent, if its
type is known and declares one. -/
private def certifiedMutation? (context : TranslationContext) :
    CommandElabM (Option (Name × Name)) := do
  let some typeName := context.mutationType? | return none
  return (Move.dataInvariant? (← getEnv) typeName).map (typeName, ·)

/-- The type of a structure field, named by its projection's codomain, when the
owner type and field are known. -/
private def fieldTypeName? (typeName : Name) (field : Name) :
    CommandElabM (Option Name) := do
  let projection := typeName ++ field
  unless (← getEnv).contains projection do return none
  liftTermElabM do
    let projectionFn ← Lean.Meta.mkConstWithFreshMVarLevels projection
    Lean.Meta.forallTelescopeReducing (← Lean.Meta.inferType projectionFn)
      fun _ body => do
        return (← Lean.Meta.whnfR body).getAppFn.constName?

/-- The type reached from `typeName` along a field path, when every step is
known. -/
private def pathTypeName? (typeName? : Option Name) (fields : List Name) :
    CommandElabM (Option Name) := do
  let mut current := typeName?
  for field in fields do
    let some typeName := current | return none
    current ← fieldTypeName? typeName field
  return current

/-- Whether a field path selects through an enum payload at any step. Plain
structure-only paths keep the smaller lens-based global-borrow term used by
existing proofs; enum paths need guarded selection and reconstruction. -/
private def pathCrossesEnum (typeName? : Option Name) (fields : List Name) :
    CommandElabM Bool := do
  let env ← getEnv
  let mut current := typeName?
  for field in fields do
    let some typeName := current | return false
    if Move.moveEnumAttr.hasTag env typeName then return true
    current ← fieldTypeName? typeName field
  return false

private def mutationValue (context : TranslationContext)
    (owner : TSyntax `ident) : CommandElabM (TSyntax `term) := do
  if let some mutation := context.mutation? then
    if mutation.getId == owner.getId then
      return ← `(Move.Semantics.Mutation.read $owner)
  if context.mutationRefs.any (·.getId == owner.getId) then
    return ← `(Move.Semantics.Mutation.read $owner)
  pure ⟨owner.raw⟩

private def liveMutations (context : TranslationContext) : List (TSyntax `ident) :=
  context.mutation?.toList ++ context.mutationRefs

private def liveMutation? (context : TranslationContext)
    (term : TSyntax `term) : Option (TSyntax `ident) :=
  if term.raw.isIdent then
    liveMutations context |>.find? (·.getId == term.raw.getId)
  else
    none

private def dereferenceValue (context : TranslationContext)
    (reference : TSyntax `term) : CommandElabM (TSyntax `term) := do
  if reference.raw.isIdent then
    let name := reference.raw.getId
    if context.mutation?.any (·.getId == name) ||
        context.mutationRefs.any (·.getId == name) then
      return ← `(Move.Semantics.Mutation.read $reference)
  pure reference

private def application? (term : TSyntax `term) :
    Option (TSyntax `term × Array (TSyntax `term)) :=
  if term.raw.isOfKind ``Lean.Parser.Term.app && term.raw.getNumArgs == 2 then
    let arguments := term.raw[1].getArgs.map (⟨·⟩)
    some (⟨term.raw[0]⟩, arguments)
  else
    none

/-- The constants an application's head identifier may resolve to, with the
arguments.  A primitive such as `read` shares its short name with unrelated
declarations, so overload resolution is left to the elaboration that follows
the desugaring; the candidates are enough to recognize the primitive. -/
private def primitiveApplication? (term : Syntax) :
    CommandElabM (Option (List Name × Array Syntax)) := do
  unless term.isOfKind ``Lean.Parser.Term.app && term.getNumArgs == 2 do return none
  let head := term[0]
  unless head.isIdent do return none
  let candidates ← try resolveGlobalConst head catch _ => pure []
  if candidates.isEmpty then return none
  return some (candidates, term[1].getArgs)

/-- The field named by a `fieldOfProjection` argument: `fun owner => owner.f`
or the projection `T.f` itself. -/
private def projectedField? (descriptor : Syntax) : Option Name := do
  let descriptor := if descriptor.isOfKind ``Lean.Parser.Term.paren then descriptor[1] else descriptor
  guard (descriptor.isOfKind ``Lean.Parser.Term.app && descriptor.getNumArgs == 2)
  guard (descriptor[0].isIdent && descriptor[0].getId.getString! == "fieldOfProjection")
  let some projection := descriptor[1].getArgs[0]? | none
  let projection := if projection.isOfKind ``Lean.Parser.Term.paren then projection[1] else projection
  if projection.isOfKind ``Lean.Parser.Term.fun then
    -- `fun owner => owner.f`: the body is a projection of the bound owner.
    let body := projection[1][3]
    if body.isOfKind ``Lean.Parser.Term.proj && body[2].isIdent then
      return body[2].getId
    if body.isIdent then
      let (_, field) ← splitLastField? body.getId
      return field
    none
  else if projection.isIdent then
    let (_, field) ← splitLastField? projection.getId
    return field
  else none

/-- A place `owner[index]` as the surface borrow parsers produce it. -/
private def indexPlace (owner index : Syntax) : Syntax :=
  mkNode `«term__[_]» #[owner, mkAtom "[", index, mkAtom "]"]

/-- A borrow term `&place` / `&mut place` as the surface parser produces it. -/
private def borrowSyntax (mutable : Bool) (place : Syntax) : Syntax :=
  if mutable then mkNode ``Move.borrowMutTerm #[mkAtom "&mut ", place]
  else mkNode ``Move.borrowTerm #[mkAtom "&", place]

/-- The surface borrow for an explicitly spelled borrow primitive, if the
application is one: `borrowLocal x` is `&x`, `borrowGlobalMut R a` is
`&mut R[a]`, `borrowField r (fieldOfProjection (fun o => o.f))` is `&r.f`,
`borrowElemMut r i` is `&mut r[i]`. -/
private def desugarBorrowPrimitive? (term : Syntax) : CommandElabM (Option Syntax) := do
  let some (candidates, arguments) ← primitiveApplication? term | return none
  let candidateHas (name : Name) : Bool := candidates.contains name
  let mutable := candidateHas ``Move.borrowLocalMut || candidateHas ``Move.borrowGlobalMut ||
    candidateHas ``Move.borrowFieldMut || candidateHas ``Move.borrowElemMut
  if candidateHas ``Move.borrowLocal || candidateHas ``Move.borrowLocalMut then
    let some place := arguments[0]? | return none
    return some (borrowSyntax mutable place)
  if candidateHas ``Move.borrowGlobal || candidateHas ``Move.borrowGlobalMut then
    let some resource := arguments[0]? | return none
    let some address := arguments[1]? | return none
    return some (borrowSyntax mutable (indexPlace resource address))
  if candidateHas ``Move.borrowField || candidateHas ``Move.borrowFieldMut then
    let some reference := arguments[0]? | return none
    let some descriptor := arguments[1]? | return none
    unless reference.isIdent do return none
    let some field := projectedField? descriptor | return none
    return some (borrowSyntax mutable (mkIdentFrom reference (reference.getId ++ field)))
  if candidateHas ``Move.borrowElem || candidateHas ``Move.borrowElemMut then
    let some reference := arguments[0]? | return none
    let some index := arguments[1]? | return none
    return some (borrowSyntax mutable (indexPlace reference index))
  return none

/-- Rewrite explicitly spelled core primitives to the surface forms the
translator models: `read r` / `readImm r` / `freeze r` read the reference
(`*r`), `write r v` assigns through it (`r := v`), the borrow primitives are
their `&` / `&mut` places, and `Move.abort c` is `abort c`.  The surface
forms and the primitives lower to the same Move operations, so their
semantics is the same; this keeps one translation for both spellings. -/
private partial def desugarPrimitives (stx : Syntax)
    (preserveFreeze : Bool := false) : CommandElabM Syntax := do
  -- `write r v` as a statement
  if stx.isOfKind ``Lean.Parser.Term.doExpr then
    if let some (candidates, arguments) ← primitiveApplication? stx[0] then
      if candidates.contains ``Move.write then
        if let (some reference, some value) := (arguments[0]?, arguments[1]?) then
          if reference.isIdent then
            let value ← desugarPrimitives value preserveFreeze
            let reassign ← `(doElem| $(⟨reference⟩):ident := $(⟨value⟩))
            return reassign.raw
  -- `assert!(c, e)` (and `assert_eq!`/`assert_ne!`): the surface macros over
  -- `Move.assert`, desugared like the primitive itself.
  if stx.isOfKind ``Move.moveAssert then
    let condition : TSyntax `term := ⟨← desugarPrimitives stx[1]⟩
    let code : TSyntax `term := ⟨← desugarPrimitives stx[3]⟩
    return (← `(do if $condition then pure () else abort $code)).raw
  if stx.isOfKind ``Move.moveAssertEq || stx.isOfKind ``Move.moveAssertNe then
    let left : TSyntax `term := ⟨← desugarPrimitives stx[1]⟩
    let right : TSyntax `term := ⟨← desugarPrimitives stx[3]⟩
    let code : TSyntax `term := ⟨← desugarPrimitives stx[5]⟩
    let condition : TSyntax `term ← if stx.isOfKind ``Move.moveAssertEq then
        `(Move.Compare.equal $left $right)
      else `(!Move.Compare.equal $left $right)
    return (← `(do if $condition then pure () else abort $code)).raw
  -- borrows, reads, and `Move.abort c`, wherever they appear
  if let some borrow ← desugarBorrowPrimitive? stx then
    return borrow
  if let some (candidates, arguments) ← primitiveApplication? stx then
    if candidates.contains ``Move.read || candidates.contains ``Move.readImm ||
        (candidates.contains ``Move.freeze && !preserveFreeze) then
      if let some reference := arguments[0]? then
        return mkNode ``Move.derefTerm #[mkAtom "*", reference]
    if candidates.contains ``Move.abort then
      if let some code := arguments[0]? then
        let code ← desugarPrimitives code preserveFreeze
        return mkNode ``Move.abortTerm #[mkAtom "abort ", code]
    if candidates.contains ``Move.assert then
      if let (some condition, some code) := (arguments[0]?, arguments[1]?) then
        let condition : TSyntax `term := ⟨← desugarPrimitives condition preserveFreeze⟩
        let code : TSyntax `term := ⟨← desugarPrimitives code preserveFreeze⟩
        return (← `(do if $condition then pure () else abort $code)).raw
  return stx.setArgs (← stx.getArgs.mapM fun child =>
    desugarPrimitives child preserveFreeze)

private def sourceBody (declaration : Declaration) : CommandElabM (TSyntax `term) := do
  let body := if declaration.value.isOfKind ``Lean.Parser.Term.paren then
      declaration.value[1]
    else
      declaration.value
  let body : TSyntax `term := ⟨← desugarPrimitives body⟩
  match body with
  | `(do $sequence:doSeq) =>
      let sequence ← Lean.Elab.Command.liftCoreM <|
        Move.freshenShadowedLocals sequence
      `(do $sequence)
  | _ => pure body

/-- Core primitives whose executable Move behavior is not yet represented by
the automatically generated source semantics. -/
private def unsupportedSourceOperation (name : Name) : Bool :=
  name == ``Move.borrowLocal || name == ``Move.borrowLocalMut ||
  name == ``Move.borrowGlobal || name == ``Move.borrowGlobalMut ||
  name == ``Move.borrowField || name == ``Move.borrowFieldMut ||
  name == ``Move.borrowElem || name == ``Move.borrowElemMut ||
  name == ``Move.freeze || name == ``Move.read || name == ``Move.readImm ||
  name == ``Move.write || name == ``Move.assert || name == ``Move.abort ||
  name == ``Move.borrowVariantField || name == ``Move.borrowVariantFieldMut ||
  name == ``Move.testVariantRef || name == ``Move.testVariantMutRef ||
  name == ``Move.Vector.get || name == ``Move.Vector.set

private def unsupportedSourceOperation? (term : TSyntax `term) :
    CommandElabM (Option Name) := do
  let some (head, _) := application? term | return none
  unless head.raw.isIdent do return none
  let name ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  return name.filter unsupportedSourceOperation

/-- Receiver notation for a checked vector operation (`values.get i`,
`r.insert i e`): the raw source does not retain what it resolves to, so it
is not assumed to be the native operation — `Spec.pure` would give it Lean's
total semantics instead of Move's abort. -/
private def receiverStyleVectorOperation? (term : Syntax) : CommandElabM Bool := do
  unless term.isOfKind ``Lean.Parser.Term.app && term.getNumArgs == 2 do return false
  let head := term[0]
  let field? : Option Name :=
    if head.isOfKind ``Lean.Parser.Term.proj && head.getNumArgs == 3 && head[2].isIdent then
      some head[2].getId
    else if head.isIdent then
      (splitLastField? head.getId).map (·.2)
    else none
  let some field := field? | return false
  unless field == `get || field == `set || field == `insert || field == `remove do
    return false
  -- A globally resolvable head (`Move.Vector.get`) is not receiver notation.
  if head.isIdent then
    let resolved ← try resolveGlobalConst head catch _ => pure []
    if !resolved.isEmpty then return false
  return true

/-- Split field notation before elaboration has resolved it.  Lean retains
`values.get i` either as a projection node or as the dotted identifier
`values.get`; in both cases the receiver is still recoverable from syntax. -/
private def receiverApplication? (term : TSyntax `term) :
    Option (TSyntax `term × Name × Array (TSyntax `term)) := do
  let (head, arguments) := (application? term).getD (term, #[])
  if head.raw.isOfKind ``Lean.Parser.Term.proj && head.raw.getNumArgs == 3 &&
      head.raw[2].isIdent then
    return (⟨head.raw[0]⟩, head.raw[2].getId, arguments)
  if head.raw.isIdent then
    if let some (receiver, field) := splitLastField? head.raw.getId then
      return (⟨mkIdentFrom head.raw receiver⟩, field, arguments)
  none

/-- Refuse source fragments for which `Spec.pure` would erase an executable
Move effect or abort. -/
private partial def ensureSupportedSourceTerm (term : TSyntax `term) :
    CommandElabM Unit := do
  if let some operation ← unsupportedSourceOperation? term then
    throwErrorAt term
      "automatic source specifications do not yet model `{operation}`; provide an explicit `sourceSpec` or omit `verify`"
  if ← receiverStyleVectorOperation? term.raw then
    throwErrorAt term
      "automatic source specifications require fully qualified `Move.Vector.get`, `Move.Vector.set`, `Move.Vector.insert`, or `Move.Vector.remove`"
  for child in term.raw.getArgs do
    ensureSupportedSourceTerm ⟨child⟩

/-- Arithmetic must be sequenced through `Checked.*Spec` so its VM abort
behavior remains visible. `rewritePure` is used only in source contexts which
cannot currently sequence a `Spec`, such as vector indices. -/
private partial def containsArithmetic (term : Syntax) : Bool :=
  (term.getNumArgs == 3 && term[1].isAtom &&
    (term[1].getAtomVal == "+" || term[1].getAtomVal == "-" ||
      term[1].getAtomVal == "*" || term[1].getAtomVal == "/" ||
      term[1].getAtomVal == "%" || term[1].getAtomVal == "<<<" ||
      term[1].getAtomVal == ">>>")) ||
  (term.isIdent && (match term.getId with
    | .str _ "cast" => true
    | _ => false)) ||
  term.getArgs.any containsArithmetic

/-- The checked relational operation behind an explicitly spelled integer
operation: the shared `MoveInt` marker or its `UInt`/`SInt` view
abbreviation — one map, since the markers are shared by both signednesses. -/
private def checkedOperationSpec? (functionName : Name) : Option Name :=
  match functionName with
  | .str prefix_ operation =>
      if prefix_ == ``Move.MoveInt || prefix_ == ``Move.UInt || prefix_ == ``Move.SInt then
        match operation with
        | "add" => some ``Move.Semantics.Checked.addSpec
        | "sub" => some ``Move.Semantics.Checked.subSpec
        | "mul" => some ``Move.Semantics.Checked.mulSpec
        | "div" => some ``Move.Semantics.Checked.divSpec
        | "mod" => some ``Move.Semantics.Checked.modSpec
        | "shl" => some ``Move.Semantics.Checked.shlSpec
        | "shr" => some ``Move.Semantics.Checked.shrSpec
        | "cast" => some ``Move.Semantics.Checked.castSpec
        | _ => none
      else none
  | _ => none

private def checkedArithmeticCall? (term : TSyntax `term) :
    CommandElabM (Option (Name × TSyntax `term × TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent && arguments.size == 2 do return none
  let functionName ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  let operation? := functionName.bind checkedOperationSpec?
  return operation?.map (·, arguments[0]!, arguments[1]!)

private partial def containsCheckedArithmeticCall (term : Syntax) :
    CommandElabM Bool := do
  if (← checkedArithmeticCall? ⟨term⟩).isSome then return true
  for child in term.getArgs do
    if ← containsCheckedArithmeticCall child then return true
  return false

private def resolvePureMoveFunction? (identifier : TSyntax `ident) :
    CommandElabM (Option Name) := do
  let env ← getEnv
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload identifier.raw))
    catch _ => pure none
  let some functionName := functionName? | return none
  unless Move.isMoveFunction env functionName do
    return none
  let some declaration := declarations.getState env |>.find? functionName
    | return none
  if (findTypeApplication? ``Move.Action declaration.resultType).isSome then
    return none
  return some functionName

private def pureMoveCallAtRoot? (term : TSyntax `term) :
    CommandElabM (Option Name) := do
  let some (head, _) := application? term | return none
  unless head.raw.isIdent do return none
  resolvePureMoveFunction? ⟨head.raw⟩

private partial def nestedPureMoveCall? (term : Syntax) :
    CommandElabM (Option Name) := do
  if let some functionName ← pureMoveCallAtRoot? ⟨term⟩ then
    return some functionName
  for child in term.getArgs do
    if let some functionName ← nestedPureMoveCall? child then
      return some functionName
  return none

private inductive VectorSearchCall where
  | contains (values value : TSyntax `term)
  | indexOf (values value : TSyntax `term)

private def vectorSearchCall? (term : TSyntax `term) :
    CommandElabM (Option VectorSearchCall) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent && arguments.size == 2 do return none
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  if functionName? == some ``Move.Vector.contains then
    return some (.contains arguments[0]! arguments[1]!)
  if functionName? == some ``Move.Vector.indexOf then
    return some (.indexOf arguments[0]! arguments[1]!)
  return none

private partial def rewritePure (mutations : List (TSyntax `ident))
    (term : TSyntax `term) : CommandElabM (TSyntax `term) := do
  ensureSupportedSourceTerm term
  if containsArithmetic term.raw || (← containsCheckedArithmeticCall term.raw) then
    throwErrorAt term
      "automatic source specifications do not yet support arithmetic in this context; bind it to a local first"
  if let some search ← vectorSearchCall? term then
    match search with
    | .contains values value =>
        return ← `(vectorContains $(← rewritePure mutations values)
          $(← rewritePure mutations value))
    | .indexOf values value =>
        return ← `(vectorIndexOf $(← rewritePure mutations values)
          $(← rewritePure mutations value))
  if let some functionName ← nestedPureMoveCall? term.raw then
    throwErrorAt term
      "automatic source specifications do not yet model pure Move callee `{functionName}`; inline it or omit `verify`"
  match term with
  | `($value:ident) =>
      let parts := fieldParts value.getId
      -- A dotted name that resolves to a global (an enum constructor, a
      -- constant) is not a field path.
      let isGlobal ← try
          let _ ← resolveGlobalConstNoOverload value
          pure true
        catch _ => pure false
      if parts.length > 1 && !isGlobal then
        let owner := rootIdent value value.getId parts.head!
        let fields := parts.tail.toArray.map fun field =>
          mkIdentFrom value (Name.mkSimple field)
        let ownerTerm ← if mutations.any (·.getId == owner.getId) then
          `(Move.Semantics.Mutation.read $owner)
        else pure ⟨owner.raw⟩
        return ← projectPath ownerTerm fields
      if mutations.any (·.getId == value.getId) then
        return ← `(Move.Semantics.Mutation.read $value)
      pure term
  | `(* $reference:term) =>
      if reference.raw.isIdent &&
          mutations.any (·.getId == reference.raw.getId) then
        return ← `(Move.Semantics.Mutation.read $reference)
      rewritePure mutations reference
  | `(($value:term)) => `(($(← rewritePure mutations value)))
  | `($lhs:term < $rhs:term) =>
      `(logicalLT $(← rewritePure mutations lhs) $(← rewritePure mutations rhs))
  | `($lhs:term <= $rhs:term) =>
      `(logicalLE $(← rewritePure mutations lhs) $(← rewritePure mutations rhs))
  -- `>`, `>=`, `!=` are the flipped and negated comparisons: the same sealed
  -- markers, so no instance of the host's is consulted.
  | `($lhs:term > $rhs:term) =>
      `(logicalLT $(← rewritePure mutations rhs) $(← rewritePure mutations lhs))
  | `($lhs:term >= $rhs:term) =>
      `(logicalLE $(← rewritePure mutations rhs) $(← rewritePure mutations lhs))
  | `($lhs:term == $rhs:term) =>
      `(logicalBEq $(← rewritePure mutations lhs) $(← rewritePure mutations rhs))
  | `($lhs:term != $rhs:term) =>
      `(!logicalBEq $(← rewritePure mutations lhs) $(← rewritePure mutations rhs))
  | `(! $value:term) => `(! $(← rewritePure mutations value))
  | _ => pure term

/-- Rewrite a specification clause's local reads through every live mutable
reference.  `rewritePure` already knows how to turn a direct or dotted local
read into `Mutation.read`; this traversal applies that fact below logical
connectives, quantifiers, and other specification-only syntax as well. -/
private partial def rewriteClauseMutations (mutations : List (TSyntax `ident))
    (stx : Syntax) : CommandElabM Syntax := do
  if stx.isOfKind ``Move.anchoredOldTerm then return stx
  if stx.isIdent || stx.isOfKind ``Move.derefTerm then
    return (← rewritePure mutations ⟨stx⟩).raw
  if stx.isOfKind ``Lean.Parser.Term.structInstField then
    return stx.setArgs (← stx.getArgs.mapIdxM fun index arg =>
      if index == 0 then pure arg else rewriteClauseMutations mutations arg)
  return stx.setArgs (← stx.getArgs.mapM (rewriteClauseMutations mutations))

private inductive VectorMutationCall where
  | insert (reference index value : TSyntax `term)
  | remove (reference index : TSyntax `term)
  | popBack (reference : TSyntax `term)
  | swap (reference i j : TSyntax `term)
  | swapRemove (reference i : TSyntax `term)
  | append (reference other : TSyntax `term)
  | reverse (reference : TSyntax `term)
  | reverseSlice (reference left right : TSyntax `term)
  | trim (reference newLen : TSyntax `term)
  | trimReverse (reference newLen : TSyntax `term)
  | rotate (reference rot : TSyntax `term)
  | rotateSlice (reference left rot right : TSyntax `term)

private def nativeVectorMutationCall? (functionName : Name)
    (reference : TSyntax `term) (arguments : Array (TSyntax `term)) :
    Option VectorMutationCall :=
  if functionName == ``Move.Vector.insert ||
      functionName == ``Move.MutRef.insert then
    if arguments.size == 2 then
      some (.insert reference arguments[0]! arguments[1]!)
    else
      none
  else if functionName == ``Move.Vector.remove ||
      functionName == ``Move.MutRef.remove then
    if arguments.size == 1 then
      some (.remove reference arguments[0]!)
    else
      none
  else if functionName == ``Move.Vector.popBack ||
      functionName == ``Move.MutRef.popBack then
    if arguments.isEmpty then some (.popBack reference) else none
  else if functionName == ``Move.Vector.swap || functionName == ``Move.MutRef.swap then
    if arguments.size == 2 then some (.swap reference arguments[0]! arguments[1]!) else none
  else if functionName == ``Move.Vector.swapRemove || functionName == ``Move.MutRef.swapRemove then
    if arguments.size == 1 then some (.swapRemove reference arguments[0]!) else none
  else if functionName == ``Move.Vector.append || functionName == ``Move.MutRef.append then
    if arguments.size == 1 then some (.append reference arguments[0]!) else none
  else if functionName == ``Move.Vector.reverse || functionName == ``Move.MutRef.reverse then
    if arguments.isEmpty then some (.reverse reference) else none
  else if functionName == ``Move.Vector.reverseSlice || functionName == ``Move.MutRef.reverseSlice then
    if arguments.size == 2 then some (.reverseSlice reference arguments[0]! arguments[1]!) else none
  else if functionName == ``Move.Vector.trim || functionName == ``Move.MutRef.trim then
    if arguments.size == 1 then some (.trim reference arguments[0]!) else none
  else if functionName == ``Move.Vector.trimReverse || functionName == ``Move.MutRef.trimReverse then
    if arguments.size == 1 then some (.trimReverse reference arguments[0]!) else none
  else if functionName == ``Move.Vector.rotate || functionName == ``Move.MutRef.rotate then
    if arguments.size == 1 then some (.rotate reference arguments[0]!) else none
  else if functionName == ``Move.Vector.rotateSlice || functionName == ``Move.MutRef.rotateSlice then
    if arguments.size == 3 then
      some (.rotateSlice reference arguments[0]! arguments[1]! arguments[2]!) else none
  else
    none

/-- Receiver notation does not retain which declaration it resolves to in the
raw source syntax used for automatic specifications. Reject it rather than
assuming that a field named `insert`, `remove`, `get`, or `set` is a native
vector operation. Use the fully qualified `Move.Vector` operation instead. -/
private def receiverStyleVectorMutation? (term : TSyntax `term) : Bool :=
  match application? term with
  | none => false
  | some (head, _) =>
      if head.raw.isOfKind ``Lean.Parser.Term.proj then
        let projection := head.raw.getArgs
        match projection[2]? with
        | some field => field.isIdent &&
            (field.getId == `insert || field.getId == `remove || field.getId == `popBack ||
              field.getId == `get || field.getId == `set)
        | none => false
      else if head.raw.isIdent then
        match head.raw.getId with
        | Name.str _ field => field == "insert" || field == "remove" || field == "popBack" ||
            field == "get" || field == "set"
        | _ => false
      else
        false

private def vectorMutationCall? (term : TSyntax `term) :
    CommandElabM (Option VectorMutationCall) :=
  do
    if let some (head, arguments) := application? term then
      if head.raw.isIdent then
        let resolved? ← try
            pure (some (← resolveGlobalConstNoOverload head.raw))
          catch _ => pure none
        if let some functionName := resolved? then
          if (← getEnv).contains functionName then
            if let some reference := arguments[0]? then
              if let some call := nativeVectorMutationCall? functionName reference
                  (arguments.extract 1 arguments.size) then
                return some call
    if let some (reference, field, arguments) := receiverApplication? term then
      if field == `insert && arguments.size == 2 then
        return some (.insert reference arguments[0]! arguments[1]!)
      if field == `remove && arguments.size == 1 then
        return some (.remove reference arguments[0]!)
      if field == `popBack && arguments.isEmpty then
        return some (.popBack reference)
      if field == `swap && arguments.size == 2 then
        return some (.swap reference arguments[0]! arguments[1]!)
      if field == `swapRemove && arguments.size == 1 then
        return some (.swapRemove reference arguments[0]!)
      if field == `append && arguments.size == 1 then
        return some (.append reference arguments[0]!)
      if field == `reverse && arguments.isEmpty then return some (.reverse reference)
      if field == `reverseSlice && arguments.size == 2 then
        return some (.reverseSlice reference arguments[0]! arguments[1]!)
      if field == `trim && arguments.size == 1 then return some (.trim reference arguments[0]!)
      if field == `trimReverse && arguments.size == 1 then
        return some (.trimReverse reference arguments[0]!)
      if field == `rotate && arguments.size == 1 then return some (.rotate reference arguments[0]!)
      if field == `rotateSlice && arguments.size == 3 then
        return some (.rotateSlice reference arguments[0]! arguments[1]! arguments[2]!)
    return none

/-- `destroy_empty` consumes a vector value rather than a mutable reference,
so it participates in ordinary expression sequencing instead of the mutable
vector-call path above. -/
private def vectorDestroyArgument? (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent && arguments.size == 1 do return none
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none
  return if functionName? == some ``Move.Vector.destroyEmpty then
    arguments[0]?
  else
    none

private def packCallArguments (_anchor : Syntax)
    (arguments : Array (TSyntax `term)) : CommandElabM (TSyntax `term) := do
  match arguments.size with
  | 0 => `(())
  | 1 => pure arguments[0]!
  | _ =>
      let reversed := arguments.toList.reverse
      reversed.tail.foldlM (init := reversed.head!) fun result argument =>
        `(($argument, $result))

private def resolveMoveFunction? (identifier : TSyntax `ident) :
    CommandElabM (Option Name) := do
  let env ← getEnv
  let functionName? ← try
      pure (some (← resolveGlobalConstNoOverload identifier.raw))
    catch _ => pure none
  let some functionName := functionName? | return none
  if Move.isMoveFunction env functionName then
    return some functionName
  return none

/-- The positions, among a Move function's explicit parameters, that take a
mutable reference. -/
private def mutableParameterPositions (functionName : Name) :
    CommandElabM (Array Nat) :=
  liftTermElabM do
    let function ← Lean.Meta.mkConstWithFreshMVarLevels functionName
    let (parameters, binderInfos, _) ←
      Lean.Meta.forallMetaTelescope (← Lean.Meta.inferType function)
    let mut positions := #[]
    let mut position := 0
    for (parameter, binderInfo) in parameters.zip binderInfos do
      if binderInfo.isExplicit then
        let parameterType ← Lean.Meta.whnf (← Lean.Meta.inferType parameter)
        if parameterType.isAppOfArity ``Move.MutRef 1 then
          positions := positions.push position
        position := position + 1
    return positions

/-- Source semantics for the built-in global-storage primitives.  `existsAt`
becomes `containsSpec`; `moveFrom`/`moveTo` become `moveFromSpec`/`moveToSpec`,
and — since they change the resource state — re-certify any global invariant on
the family immediately afterward, exactly as a `&mut` write does. -/
private def globalPrimitiveSpec?
    (translateArgument : TSyntax `term → CommandElabM (TSyntax `term))
    (context : TranslationContext) (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload head.raw))
      catch _ => pure none) | return none
  let some resource ← globalPrimitiveResource? term
    | return none
  let descriptor ← resourceFor context.resources resource
  let resourceName := resource.head
  let invariants := Move.globalInvariants (← getEnv) resourceName
  -- Re-establish the family's global invariants at this state change: an
  -- `update` invariant wraps the op (relating pre/post); a regular invariant
  -- is asserted after it, then the op's own result is returned.
  let recertify (result : TSyntax `term) (core : TSyntax `term) :
      CommandElabM (TSyntax `term) := do
    let mut wrapped := core
    for (isUpdate, body, _) in invariants do
      if isUpdate then
        let bodyId := mkIdentFrom head body
        wrapped ← `(Move.Semantics.Spec.certifyUpdate $bodyId $wrapped)
    let regulars := invariants.filterMap fun (u, b, _) => if u then none else some b
    if regulars.isEmpty then
      return wrapped
    let mut tail ← `(Move.Semantics.Spec.pure $result)
    for body in regulars.reverse do
      let bodyId := mkIdentFrom head body
      tail ← `(Move.Semantics.Spec.bind
        (Move.Semantics.Spec.certifyState $bodyId)
        (fun _moveSpecCertify => $tail))
    `(Move.Semantics.Spec.bind $wrapped (fun $result => $tail))
  if name == ``Move.existsAt then
    let some addr := arguments[1]? | return none
    let addrSpec ← translateArgument addr
    let key := mkIdentFrom addr `_moveSpecKey
    return some (← `(Move.Semantics.Spec.bind $addrSpec (fun $key =>
      Move.Semantics.Resource.containsSpec $descriptor $key)))
  else if name == ``Move.moveFrom then
    let some addr := arguments[1]? | return none
    let addrSpec ← translateArgument addr
    let key := mkIdentFrom addr `_moveSpecKey
    let removed := mkIdentFrom addr `_moveSpecRemoved
    let core ← `(Move.Semantics.Resource.moveFromSpec $descriptor $key)
    let body ← recertify removed core
    return some (← `(Move.Semantics.Spec.bind $addrSpec (fun $key => $body)))
  else if name == ``Move.moveTo then
    let some signer := arguments[0]? | return none
    let some value := arguments[1]? | return none
    let signerSpec ← translateArgument signer
    let valueSpec ← translateArgument value
    let signerName := mkIdentFrom signer `_moveSpecSigner
    let valueName := mkIdentFrom value `_moveSpecPublished
    let unitName := mkIdentFrom term `_moveSpecMoveToResult
    let core ← `(Move.Semantics.Resource.moveToSpec $descriptor
      (Move.Ref.address $signerName) $valueName)
    let body ← recertify unitName core
    return some (← `(Move.Semantics.Spec.bind $signerSpec (fun $signerName =>
      Move.Semantics.Spec.bind $valueSpec (fun $valueName => $body))))
  else
    return none

private def finishCore (context : TranslationContext) (valueSpec : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  if context.returnsMutation then
    throwError "internal error: a mutable-reference result must move a live mutation"
  match context.mutation? with
  | none => pure valueSpec
  | some mutation =>
      let finalMutations ←
        if context.rootMutations.any (·.getId == mutation.getId) then
          packCallArguments mutation.raw <|
            context.rootMutations.map fun root =>
              (⟨root.raw⟩ : TSyntax `term)
        else
          pure ⟨mutation.raw⟩
      `(Move.Semantics.Spec.bind $valueSpec fun _moveSpecValue =>
          Move.Semantics.Spec.pure (_moveSpecValue, $finalMutations))

/-- Abstract the mutations a control-flow exit cannot use until its enclosing
loan has reconciled. Curried binders avoid committing the generated term to a
particular packed-tuple arity. -/
private def abstractLoanRoots (roots : Array (TSyntax `ident))
    (action : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let mut deferred := action
  for root in roots.reverse do
    deferred ← `(fun $root => $deferred)
  pure deferred

/-- Finish a path that transfers control while a loan is live. The
selected continuation becomes the loan's value and is executed only after all
of this frame's roots have been revived. -/
private def finishOneLoanExit (context : TranslationContext)
    (action : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let some frame := context.loanExits.head?
    | throwError "internal error: no live-loan exit frame"
  if let some mutation := frame.resolve? then
    let ignored := mkIdentFrom mutation `_moveSpecResolvedLoanExit
    return ← `(Move.Semantics.Spec.bind
      (Move.Semantics.resolveMutation $mutation)
      (fun $ignored => $action))
  let deferred ← abstractLoanRoots frame.roots action
  finishCore { context with loanExits := context.loanExits.tail }
    (← `(Move.Semantics.Spec.pure $deferred))

/-- Delay `action` through every consecutive active loan selected by
`unwind`. Recursing in the resumed context builds the outer reconciliation
first; the current loan then returns that outer action as its deferred value. -/
private partial def finishLoanExits (context : TranslationContext)
    (unwind : LoanExitFrame → Bool) (action : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let some frame := context.loanExits.head? | return action
  unless unwind frame do return action
  let outer ← finishLoanExits (resumeAfterLoan context frame) unwind action
  finishOneLoanExit context outer

private def finish (context : TranslationContext) (valueSpec : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  match context.loanExits.head? with
  | none => finishCore context valueSpec
  | some frame@{ resolve? := some mutation, .. } =>
      let ignored := mkIdentFrom mutation `_moveSpecLoanBodyValue
      `(Move.Semantics.Spec.bind $valueSpec (fun $ignored => $(frame.normal)))
  | some frame =>
      let deferred ← abstractLoanRoots frame.roots frame.normal
      finishCore { context with loanExits := context.loanExits.tail }
        (← `(Move.Semantics.Spec.bind $valueSpec (fun $(frame.value) =>
          Move.Semantics.Spec.pure $deferred)))

private def packUpdatedRootMutations (context : TranslationContext)
    (root : TSyntax `ident) (updated : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  unless context.rootMutations.any (·.getId == root.getId) do
    throwErrorAt root
      "a returned mutable reference must reconcile into a mutable parameter"
  let finalRoot (candidate : TSyntax `ident) : TSyntax `term :=
    if candidate.getId == root.getId then updated else ⟨candidate.raw⟩
  packCallArguments root.raw (context.rootMutations.map finalRoot)

private def resolveMutationReturns (context : TranslationContext)
    (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let mut result := body
  for mutation in context.resolveBeforeReturn.reverse do
    let ignored := mkIdentFrom mutation `_moveSpecResolvedBeforeReturn
    result ← `(Move.Semantics.Spec.bind
      (Move.Semantics.resolveMutation $mutation)
      (fun $ignored => $result))
  pure result

/-- Finish a mutation-level return.  The returned loan receives a fresh
prophecy and the selected root is suspended at that future value while
retaining its enclosing prophecy.  Every root is returned because the caller
cannot know which mutable parameter the result selected. -/
private def finishMutationReturn (context : TranslationContext)
    (returned : TSyntax `ident) : CommandElabM (TSyntax `term) := do
  if context.transferredReturns.any (·.getId == returned.getId) then
    let finalMutations ← packCallArguments returned.raw <|
      context.rootMutations.map fun root => (⟨root.raw⟩ : TSyntax `term)
    return ← resolveMutationReturns context
      (← `(Move.Semantics.Spec.pure ($returned, $finalMutations)))
  unless context.rootMutations.any (·.getId == returned.getId) do
    throwErrorAt returned
      "automatic mutation-level returns currently support only a direct mutable-reference parameter"
  let transferred := mkIdentFrom returned `_moveSpecTransferredMutation
  let finalMutations ← packUpdatedRootMutations context returned
    (← `($transferred.2))
  resolveMutationReturns context (← `(Move.Semantics.Spec.bind
      (Move.Semantics.transferMutation $returned)
      (fun $transferred =>
        Move.Semantics.Spec.pure ($transferred.1, $finalMutations))))

private partial def packLoopState (ids : List (TSyntax `ident)) :
    CommandElabM (TSyntax `term) := do
  match ids with
  | [] => `(())
  | [id] => `($id)
  | id :: rest =>
      let tail ← packLoopState rest
      `(($id, $tail))

private def identifierExtends (root candidate : Name) : Bool :=
  let rootParts := fieldParts root
  let candidateParts := fieldParts candidate
  !rootParts.isEmpty && candidateParts.take rootParts.length == rootParts &&
    sameScopes root candidate

private partial def containsIdentifier (name : Name) (stx : Syntax) : Bool :=
  (stx.isIdent && identifierExtends name stx.getId) ||
    stx.getArgs.any (containsIdentifier name)

/-- Whether syntax projects a field from a local rather than merely using the
local itself. Compound field names need to remain in the borrow's generated
scope so Lean can resolve the projection against the bound value. -/
private partial def containsProjectionOfIdentifier (name : Name)
    (stx : Syntax) : Bool :=
  (stx.isIdent &&
    let rootParts := fieldParts name
    let candidateParts := fieldParts stx.getId
    candidateParts.length > rootParts.length &&
      candidateParts.take rootParts.length == rootParts &&
      sameScopes name stx.getId) ||
    stx.getArgs.any (containsProjectionOfIdentifier name)

/-- Whether a lexical loan body contains a control exit that may need to
unwind the loan before transferring control. Inner-loop exits remain inside
the loan; their target frame is distinguished during translation. -/
private partial def containsLoanControlExit (stx : Syntax) : Bool :=
  stx.isOfKind ``Lean.Parser.Term.doReturn ||
  stx.isOfKind ``Lean.Parser.Term.doBreak ||
  stx.isOfKind ``Lean.Parser.Term.doContinue ||
  stx.isOfKind ``Move.moveBreakLabeledDo ||
  stx.isOfKind ``Move.moveContinueLabeledDo ||
  stx.isOfKind ``Move.moveBreakInternal ||
  stx.isOfKind ``Move.moveContinueInternal ||
  stx.getArgs.any containsLoanControlExit

private def freshLoopStateIdents (ref : Syntax)
    (assigned : List (TSyntax `ident)) : List (TSyntax `ident) :=
  assigned.zipIdx.map fun (_, index) =>
    mkIdentFrom ref (Name.mkSimple s!"_moveSpecLoopVar{index}")

private def replaceLoopState (assigned state : List (TSyntax `ident))
    (stx : Syntax) : Syntax :=
  (assigned.zip state).foldl (init := stx) fun stx (source, replacement) =>
    replaceIdentifier source.getId replacement.raw stx

private def findVerificationLoop? (loops : List VerificationLoopFrame)
    (sourceLabel? : Option Name) :
    Option (List VerificationLoopFrame × VerificationLoopFrame) :=
  match sourceLabel? with
  | none => loops.head?.map ([], ·)
  | some sourceLabel =>
      let rec find (inner : List VerificationLoopFrame)
          : List VerificationLoopFrame →
            Option (List VerificationLoopFrame × VerificationLoopFrame)
        | [] => none
        | frame :: rest =>
            if frame.sourceLabel? == some sourceLabel then
              some (inner, frame)
            else
              find (frame :: inner) rest
      find [] loops

private def resolvedLoopState (inner : List VerificationLoopFrame)
    (target : VerificationLoopFrame) : List (TSyntax `ident) :=
  inner.foldl (init := target.state) fun current frame =>
    current.map fun id =>
      match (frame.assigned.zip frame.state).find? (·.1.getId == id.getId) with
      | some (_, replacement) => replacement
      | none => id

private def loopContinueSpec (context : TranslationContext)
    (sourceLabel? : Option Name) (ref : Syntax) :
    CommandElabM (TSyntax `term) := do
  let some (inner, frame) := findVerificationLoop? context.loops sourceLabel?
    | if let some loan := context.loanScope? then
        throwErrorAt ref "`continue` inside the scope of the mutable borrow `{loan.getId}` is not supported by automatic source specifications; end the loan first"
      match sourceLabel? with
      | none => throwErrorAt ref "`continue` requires an enclosing `loop` or `while`"
      | some sourceLabel => throwErrorAt ref "unknown loop label `{sourceLabel}`"
  let pack ← packLoopState (resolvedLoopState inner frame)
  let action ← `($(frame.recursive) $pack)
  finishLoanExits context
    (fun exit => exit.enclosingLoops.contains frame.recursive.raw.getId) action

private def loopBreakSpec (context : TranslationContext)
    (sourceLabel? : Option Name) (ref : Syntax) :
    CommandElabM (TSyntax `term) := do
  let some (inner, frame) := findVerificationLoop? context.loops sourceLabel?
    | if let some loan := context.loanScope? then
        throwErrorAt ref "`break` inside the scope of the mutable borrow `{loan.getId}` is not supported by automatic source specifications; end the loan first"
      match sourceLabel? with
      | none => throwErrorAt ref "`break` requires an enclosing `loop` or `while`"
      | some sourceLabel => throwErrorAt ref "unknown loop label `{sourceLabel}`"
  let current := resolvedLoopState inner frame
  let action : TSyntax `term :=
    ⟨replaceLoopState frame.state current frame.after.raw⟩
  finishLoanExits context
    (fun exit => exit.enclosingLoops.contains frame.recursive.raw.getId) action

private def emptyFinish (context : TranslationContext) : CommandElabM (TSyntax `term) := do
  if !context.loops.isEmpty then
    loopContinueSpec context none Syntax.missing
  else
    finish context (← `(Move.Semantics.Spec.pure ()))

/-- The implicit fallthrough from a special mutation-returning loan can only
occur in a loop body. Resolve every structural child inside-out, then invoke
the loop continuation (which also unwinds any loans surrounding this one). -/
private def transferredLoanNormal (context : TranslationContext)
    (mutations : Array (TSyntax `ident)) : CommandElabM (TSyntax `term) := do
  if context.loops.isEmpty then
    -- A well-typed mutation-returning function must transfer a reference on
    -- every non-aborting terminal path, so this placeholder is never emitted.
    return ⟨mutations[0]!.raw⟩
  let mut result ← emptyFinish context
  for mutation in mutations do
    let ignored := mkIdentFrom mutation `_moveSpecResolvedLoanFallthrough
    result ← `(Move.Semantics.Spec.bind
      (Move.Semantics.resolveMutation $mutation)
      (fun $ignored => $result))
  pure result

/-- Bind the loop state's components from the packed state, by projection
(the shape of a function's arguments, and what the automatic prover
normalizes). -/
private partial def unpackLoopState (ids : List (TSyntax `ident)) (packed : TSyntax `term)
    (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
  match ids with
  | [] => `(let _ := $packed; $body)
  | [id] => `(let $id := $packed; $body)
  | id :: rest =>
      let tail := mkIdentFrom id `_moveSpecLoopTail
      let nested ← unpackLoopState rest ⟨tail.raw⟩ body
      `(let $id := Prod.fst $packed; let $tail := Prod.snd $packed; $nested)

/-- The `invariant` statements at the head of a loop body, and the body
without them. -/
private def splitLoopInvariants (elements : Array Lean.DoElem) :
    Array (TSyntax `term) × Array Lean.DoElem :=
  let count := elements.size - (elements.toList.dropWhile fun element =>
    element.raw.isOfKind ``Move.moveInvariant).length
  let invariants := (elements.extract 0 count).map fun element =>
    (⟨element.raw[1]⟩ : TSyntax `term)
  (invariants, elements.extract count elements.size)

/-- The local a statement binds, and whether every use of it — rather than only
a projection out of it — extends the loan it was bound in. Generated loan
bodies are lexical expressions, so every later use must remain inside them;
this is conservative relative to Move's NLL but preserves owned values bound
while a loan is live. -/
private def boundIdentifier? (element : Lean.DoElem) : Option (Name × Bool) :=
  match element with
  | `(doElem| let mut $name:ident ← $_value:term) => some (name.getId, true)
  | `(doElem| let mut $name:ident := $_value:term) => some (name.getId, true)
  | `(doElem| let $name:ident ← $_value:term) => some (name.getId, true)
  | `(doElem| let $name:ident := $_value:term) => some (name.getId, true)
  | `(doElem| let $name:ident : $_type:term := $_value:term) =>
      some (name.getId, true)
  | _ => none

private partial def closeBorrowScope (elements : Array Lean.DoElem)
    (size : Nat) : Nat :=
  let extended := (elements.extract 0 size).foldl (init := size) fun result element =>
    match boundIdentifier? element with
    | none => result
    | some (name, includePlainUses) =>
        elements.zipIdx.foldl (init := result) fun result (candidate, index) =>
          if (if includePlainUses then containsIdentifier
              else containsProjectionOfIdentifier) name candidate.raw then
            max result (index + 1)
          else
            result
  if extended = size then size else closeBorrowScope elements extended

/-- Split the statements following a mutable borrow at the last use of its
reference local. The prefix is the loan body; the suffix executes after the
loan has been reconciled. -/
private def mutableBorrowScope (name : Name) (elements : Array Lean.DoElem) :
    CommandElabM (Array Lean.DoElem × Array Lean.DoElem) := do
  let lastUse := elements.zipIdx.foldl (init := none) fun result (element, index) =>
    if containsIdentifier name element.raw then some index else result
  let size := closeBorrowScope elements (lastUse.map (· + 1) |>.getD 0)
  pure (elements.extract 0 size, elements.extract size elements.size)

private def isDirectMutationReturn (name : Name)
    (elements : Array Lean.DoElem) : Bool :=
  if elements.size != 1 then false
  else
    match elements[0]! with
    | `(doElem| pure $value:term) =>
        value.raw.isIdent && value.raw.getId == name
    | `(doElem| $value:term) =>
        value.raw.isIdent && value.raw.getId == name
    | _ => false

private partial def flattenResultTerms (term : TSyntax `term) : Array (TSyntax `term) :=
  if term.raw.isOfKind `null then
    let components := term.raw.getSepArgs
    if components.size == 1 then
      flattenResultTerms ⟨components[0]!⟩
    else
      components.flatMap fun component => flattenResultTerms ⟨component⟩
  else
    match term with
    | `(($inner:term)) => flattenResultTerms inner
    | _ =>
        if term.raw.isOfKind ``Lean.Parser.Term.tuple && term.raw.getNumArgs > 1 then
          flattenResultTerms ⟨term.raw[1]⟩
        else
          #[term]

private def mutationResultContains (context : TranslationContext)
    (name : Name) (value : TSyntax `term) : Bool :=
  let values := flattenResultTerms value
  values.zipIdx.any fun (component, index) =>
    context.resultLeaves[index]?.any (·.isMutable) &&
      component.raw.isIdent && component.raw.getId == name

private def forwardsTransferredMutationResult (context : TranslationContext)
    (value : TSyntax `term) : Bool :=
  context.transferredReturns.any fun mutation =>
    mutationResultContains context mutation.getId value

private partial def containsExplicitMutationReturn
    (context : TranslationContext) (name : Name) (stx : Syntax) : Bool :=
  let returned := if stx.isOfKind ``Lean.Parser.Term.doReturn then
    let element : Lean.DoElem := ⟨stx⟩
    match element with
    | `(doElem| return $value:term) => mutationResultContains context name value
    | _ => false
  else
    false
  returned || stx.getArgs.any (containsExplicitMutationReturn context name)

/-- Whether any path returns `name` in a mutable-reference result component.
This includes an explicit early `return`, not only the final expression. -/
private def isMutationResultComponent (context : TranslationContext)
    (name : Name) (elements : Array Lean.DoElem) : Bool :=
  elements.any (containsExplicitMutationReturn context name ·.raw) ||
  elements.back?.any fun last =>
    let value? : Option (TSyntax `term) := match last with
      | `(doElem| pure $value:term) => some value
      | `(doElem| $value:term) => some value
      | _ => none
    value?.any (mutationResultContains context name)

private def mutableResultScope (names : Array (TSyntax `ident))
    (elements : Array Lean.DoElem) : CommandElabM (Array Lean.DoElem × Array Lean.DoElem) := do
  let size := names.foldl (init := 0) fun size name =>
    elements.zipIdx.foldl (init := size) fun size (element, index) =>
      if containsIdentifier name.getId element.raw then max size (index + 1) else size
  let size := closeBorrowScope elements size
  pure (elements.extract 0 size, elements.extract size elements.size)

/-- NLL for the borrow checker follows reference derivations, but not copied
values.  The prophecy translator's `mutableBorrowScope` above intentionally
keeps ordinary bound values in lexical scope; using it for safety analysis
would incorrectly keep `let value ← *reference` alive until every use of
`value`. -/
private def sourceReferenceBoundIdentifier? (element : Lean.DoElem) : Option Name :=
  match element with
  | `(doElem| let $name:ident ← $value:term) =>
      if value.raw.isOfKind ``Move.borrowTerm ||
          value.raw.isOfKind ``Move.borrowMutTerm ||
          value.raw.isOfKind ``Move.borrowIndexTerm ||
          value.raw.isOfKind ``Move.borrowMutIndexTerm then
        some name.getId
      else none
  | _ => none

private partial def closeSourceBorrowScope (elements : Array Lean.DoElem)
    (size : Nat) : Nat :=
  let extended := (elements.extract 0 size).foldl (init := size) fun result element =>
    match sourceReferenceBoundIdentifier? element with
    | none => result
    | some name =>
        elements.zipIdx.foldl (init := result) fun result (candidate, index) =>
          if containsIdentifier name candidate.raw then max result (index + 1)
          else result
  if extended = size then size else closeSourceBorrowScope elements extended

private def sourceBorrowScope (name : Name) (elements : Array Lean.DoElem) :
    Array Lean.DoElem × Array Lean.DoElem :=
  let lastUse := elements.zipIdx.foldl (init := none) fun result (element, index) =>
    if containsIdentifier name element.raw then some index else result
  let size := closeSourceBorrowScope elements (lastUse.map (· + 1) |>.getD 0)
  (elements.extract 0 size, elements.extract size elements.size)

private def sourceBorrowScopes (names : Array Name) (elements : Array Lean.DoElem) :
    Array Lean.DoElem × Array Lean.DoElem :=
  let size := names.foldl (init := 0) fun size name =>
    elements.zipIdx.foldl (init := size) fun size (element, index) =>
      if containsIdentifier name element.raw then max size (index + 1) else size
  let size := closeSourceBorrowScope elements size
  (elements.extract 0 size, elements.extract size elements.size)

/-- `Move.Vector.get` / `Move.Vector.set`: checked element access whose abort
the relational semantics sequences. -/
private inductive VectorAccessCall where
  | get (values index : TSyntax `term)
  | set (values index value : TSyntax `term)

private def vectorAccessCall? (term : TSyntax `term) :
    CommandElabM (Option VectorAccessCall) := do
  let some (head, arguments) := application? term | return none
  if head.raw.isIdent then
    if let some name ← (try pure (some (← resolveGlobalConstNoOverload head.raw))
        catch _ => pure none) then
      if name == ``Move.Vector.get then
        if h : arguments.size = 2 then
          return some (.get arguments[0] arguments[1])
      else if name == ``Move.Vector.set then
        if h : arguments.size = 3 then
          return some (.set arguments[0] arguments[1] arguments[2])
  if let some (values, field, arguments) := receiverApplication? term then
    if field == `get && arguments.size == 1 then
      return some (.get values arguments[0]!)
    if field == `set && arguments.size == 2 then
      return some (.set values arguments[0]! arguments[1]!)
  return none

/-- A call to a Move function with retained source, including a
`continue`-marked self-call. -/
private def moveFunctionCall? (term : Syntax) : CommandElabM Bool := do
  let head := if term.isOfKind ``Lean.Parser.Term.app && term.getNumArgs == 2 then some term[0]
    else if term.isOfKind ``Move.continueCallTerm && term.getNumArgs > 1 then some term[1]
    else none
  let some head := head | return false
  unless head.isIdent do return false
  let some functionName ← resolveMoveFunction? ⟨head⟩ | return false
  return (declarations.getState (← getEnv)).contains functionName

/-- Whether a term is an operation the relational semantics sequences — a
checked arithmetic operation, cast, or vector access, or a Move call — and
so cannot stay inside a pure position. -/
private def effectfulNode? (term : Syntax) : CommandElabM Bool := do
  if term.isOfKind `choice then
    -- `lhs * rhs` is parsed as a choice between multiplication and an
    -- application to a dereference; the infix alternative decides.
    return term.getArgs.any fun alternative =>
      alternative.getNumArgs == 3 && alternative[1].isAtom &&
        alternative[1].getAtomVal == "*"
  if term.getNumArgs == 3 && term[1].isAtom &&
      (term[1].getAtomVal == "+" || term[1].getAtomVal == "-" ||
        term[1].getAtomVal == "*" || term[1].getAtomVal == "/" ||
        term[1].getAtomVal == "%" || term[1].getAtomVal == "<<<" ||
        term[1].getAtomVal == ">>>") then
    return true
  if (← checkedArithmeticCall? ⟨term⟩).isSome then return true
  if term.isOfKind ``Lean.Parser.Term.typeAscription && term.getNumArgs > 1 then
    -- `(x.cast : T)`, Move's `as`
    let value := term[1]
    if value.isIdent then
      if let .str _ "cast" := value.getId then return true
  if (← vectorAccessCall? ⟨term⟩).isSome then return true
  if ← moveFunctionCall? term then return true
  return false

/-- Hoist the sequenced operations out of an eager pure position: each maximal
effectful subterm is replaced by a fresh local, and the bindings are returned
in Move's left-to-right evaluation order. Conditional forms are hoisted as a
whole and translated compositionally by `expressionSpec`, so effects in a
branch remain conditional rather than being moved in front of the branch. -/
private partial def hoistEffects (term : Syntax) (start : Nat) :
    CommandElabM (Array (TSyntax `ident × TSyntax `term) × Syntax) := do
  let (bindings, residual) ← go term #[] false
  return (bindings, residual)
where
  go (stx : Syntax) (bindings : Array (TSyntax `ident × TSyntax `term))
      (conditional : Bool) :
      CommandElabM (Array (TSyntax `ident × TSyntax `term) × Syntax) := do
    if ← effectfulNode? stx then
      let hoisted := mkIdentFrom stx (Name.mkSimple s!"_moveSpecHoisted{start + bindings.size}")
      return (bindings.push (hoisted, ⟨stx⟩), hoisted.raw)
    let kind := stx.getKind
    -- Binder bodies and conditional positions: descend only to reject.
    if kind == ``Lean.Parser.Term.fun then
      let (bindings, _) ← go stx[1] bindings true
      return (bindings, stx)
    if (kind == `«term_&&_» || kind == `«term_||_») && stx.getNumArgs == 3 then
      let hoisted := mkIdentFrom stx (Name.mkSimple s!"_moveSpecHoisted{start + bindings.size}")
      return (bindings.push (hoisted, ⟨stx⟩), hoisted.raw)
    if kind == ``Lean.Parser.Term.match then
      let hoisted := mkIdentFrom stx (Name.mkSimple s!"_moveSpecHoisted{start + bindings.size}")
      return (bindings.push (hoisted, ⟨stx⟩), hoisted.raw)
    if kind == ``Lean.Parser.Term.matchAlts || kind == ``Lean.Parser.Term.matchAlt then
      return (bindings, stx)
    if kind == `termIfThenElse || kind == `termDepIfThenElse then
      let hoisted := mkIdentFrom stx (Name.mkSimple s!"_moveSpecHoisted{start + bindings.size}")
      return (bindings.push (hoisted, ⟨stx⟩), hoisted.raw)
    descend stx bindings conditional
  descend (stx : Syntax) (bindings : Array (TSyntax `ident × TSyntax `term))
      (conditional : Bool) :
      CommandElabM (Array (TSyntax `ident × TSyntax `term) × Syntax) := do
    let mut bindings := bindings
    let mut args := stx.getArgs
    for i in [0:args.size] do
      let (more, child) ← go args[i]! bindings conditional
      bindings := more
      args := args.set! i child
    return (bindings, stx.setArgs args)

/-- The pieces of a function's signature that shape its relational
semantics: generic context, the explicit parameters with their logical
types (a reference parameter contributes its referent), and the
mutable-reference parameter. -/
structure SourceSignature where
  context : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  arguments : Array (TSyntax `ident) := #[]
  types : Array (TSyntax `term) := #[]
  mutableParameters : Array (TSyntax `ident × TSyntax `term) := #[]
  /-- Reference parameters before reference erasure, with their explicit
  argument positions.  The source borrow checker consumes this view. -/
  referenceParameters : Array
    (TSyntax `ident × Move.Verify.Borrow.RefKind × Nat) := #[]

/-- Checked, parameter-relative summaries exported with retained declarations.
Call-site borrow extraction instantiates these facts instead of inlining. -/
private initialize borrowSummaries :
    SimplePersistentEnvExtension
      (Name × Move.Verify.Borrow.Summary)
      (NameMap Move.Verify.Borrow.Summary) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun summaries (name, summary) => summaries.insert name summary
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun summaries (name, summary) => summaries.insert name summary) {} entries
  }

/-- The source signature of a Move function, from its elaborated type: what a
`spec` command's binders state, derived when a callee has no `spec` yet. -/
private def signatureOf (functionName : Name) : CommandElabM SourceSignature :=
  liftTermElabM do
    let info ← getConstInfo functionName
    Lean.Meta.forallTelescope info.type fun parameters _resultType => do
      let delab (type : Lean.Expr) : Lean.Elab.TermElabM (TSyntax `term) :=
        withOptions (fun options => options.setBool `pp.fullNames true) do
          Lean.PrettyPrinter.delab type
      let mut signature : SourceSignature := {}
      let mut explicitPosition := 0
      for parameter in parameters do
        let declaration ← parameter.fvarId!.getDecl
        let name := declaration.userName
        let type ← instantiateMVars declaration.type
        let ident := mkIdent name
        match declaration.binderInfo with
        | .default =>
            if type.isAppOfArity ``Move.MutRef 1 then
              let referent ← delab type.appArg!
              signature := { signature with
                arguments := signature.arguments.push ident
                types := signature.types.push referent
                mutableParameters := signature.mutableParameters.push (ident, referent)
                referenceParameters := signature.referenceParameters.push
                  (ident, .mutable, explicitPosition) }
            else if type.isAppOfArity ``Move.Ref 1 &&
                !type.appArg!.isConstOf ``Move.Signer then
              -- An immutable reference is the observed value; a signer
              -- reference stays one, as `moveTo` addresses it.
              signature := { signature with
                arguments := signature.arguments.push ident
                types := signature.types.push (← delab type.appArg!)
                referenceParameters := signature.referenceParameters.push
                  (ident, .immutable, explicitPosition) }
            else
              signature := { signature with
                arguments := signature.arguments.push ident
                types := signature.types.push (← delab type) }
            explicitPosition := explicitPosition + 1
        | .instImplicit =>
            let typeStx ← delab type
            let binder ← `(bracketedBinder| [$typeStx])
            signature := { signature with context := signature.context.push binder }
        | _ =>
            let typeStx ← delab type
            let binder ← `(bracketedBinder| {$ident : $typeStx})
            signature := { signature with context := signature.context.push binder }
      return signature

private structure BorrowRefBinding where
  name : Name
  place : Move.Verify.Borrow.Place
  kind : Move.Verify.Borrow.RefKind
  isParameter : Bool := false

private structure BorrowExtractionContext where
  resources : Array ResourceBinding
  references : List BorrowRefBinding := []

private def BorrowExtractionContext.find? (context : BorrowExtractionContext)
    (name : Name) : Option BorrowRefBinding :=
  context.references.find? (·.name == name)

private def borrowFieldSteps (fields : Array (TSyntax `ident)) :
    Array Move.Verify.Borrow.Step :=
  fields.map fun field => .field field.getId.toString

private def borrowPlaceFromOwner (context : BorrowExtractionContext)
    (owner : TSyntax `ident) (suffix : Array Move.Verify.Borrow.Step) :
    Move.Verify.Borrow.Place × Option String :=
  match context.find? owner.getId with
  | some parent =>
      ({ parent.place with path := parent.place.path ++ suffix },
        some parent.name.toString)
  | none =>
      ({ root := .local owner.getId.toString, path := suffix }, none)

/-- Resolve a retained-source place without compiler temporaries.  Global
roots are resource families and vector indices use the conservative shared
`anyIndex` step. -/
private def sourceBorrowPlace (context : BorrowExtractionContext)
    (place : TSyntax `term) :
    CommandElabM (Move.Verify.Borrow.Place × Option String) := do
  if let some (owner, _, fields) ← localVectorPlace? context.resources place then
    return borrowPlaceFromOwner context owner
      (#[.anyIndex] ++ borrowFieldSteps fields)
  if let some (owner, fields) ← localPlace? context.resources place then
    return borrowPlaceFromOwner context owner (borrowFieldSteps fields)
  let (family, _, fields) ← globalPlace place
  return ({ root := .global family.key, path := borrowFieldSteps fields }, none)

private def vectorMutationReference : VectorMutationCall → TSyntax `term
  | .insert reference _ _ | .remove reference _ | .popBack reference |
    .swap reference _ _ | .swapRemove reference _ | .append reference _ |
    .reverse reference | .reverseSlice reference _ _ | .trim reference _ |
    .trimReverse reference _ | .rotate reference _ | .rotateSlice reference _ _ _ =>
      reference

private def sourceFreezeArgument? (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `ident)) := do
  let some (candidates, arguments) ← primitiveApplication? term.raw | return none
  unless candidates.contains ``Move.freeze do return none
  let some reference := arguments[0]? | return none
  if reference.isIdent then pure (some ⟨reference⟩) else pure none

private partial def collectDerefReferences (context : BorrowExtractionContext)
    (stx : Syntax) (found : Array String := #[]) : Array String :=
  let found :=
    if stx.isOfKind ``Move.derefTerm && stx.getNumArgs > 1 && stx[1].isIdent then
      let name := stx[1].getId
      if (context.find? name).isSome && !found.contains name.toString then
        found.push name.toString
      else found
    else found
  stx.getArgs.foldl (fun found child =>
    collectDerefReferences context child found) found

/-- Borrow effects of one expression.  Ordinary callees use their reference
signature; mutable parameters conservatively write until SCC summary
inference refines them below. -/
private def sourceTermBorrowEvents (context : BorrowExtractionContext)
    (term : TSyntax `term)
    (destinations : Array (Nat × Name) := #[]) :
    CommandElabM (Array Move.Verify.Borrow.Event) := do
  if let some family ← globalPrimitiveResource? term.raw then
    if let some (head, _) := application? term then
      if head.raw.isIdent then
        let name? ← try
          pure (some (← resolveGlobalConstNoOverload head.raw))
        catch _ => pure none
        if name? == some ``Move.moveFrom || name? == some ``Move.moveTo then
          return #[.ownerWrite { root := .global family.key }]
  if let some mutation ← vectorMutationCall? term then
    let reference := vectorMutationReference mutation
    if reference.raw.isIdent && (context.find? reference.raw.getId).isSome then
      return #[.write reference.raw.getId.toString]
  if let some (head, rawArguments) := application? term then
    if head.raw.isIdent then
      let identifier : TSyntax `ident := ⟨head.raw⟩
      if let some functionName ← resolveMoveFunction? identifier then
        if (declarations.getState (← getEnv)).contains functionName then
          let signature ← signatureOf functionName
          let arguments := rawArguments.filter fun argument =>
            !argument.raw.isOfKind ``Lean.Parser.Term.namedArgument
          let mut callArguments : Array Move.Verify.Borrow.CallArgument := #[]
          let summary? := borrowSummaries.getState (← getEnv) |>.find? functionName
          for ((_, kind, position), referenceIndex) in
              signature.referenceParameters.zipIdx do
            if let some argument := arguments[position]? then
              if argument.raw.isIdent &&
                  (context.find? argument.raw.getId).isSome then
                let effect := summary?.bind (·.parameterEffects[referenceIndex]?) |>.getD
                  (if kind == .mutable then .write else .read)
                callArguments := callArguments.push {
                  reference := argument.raw.getId.toString
                  parameter := referenceIndex
                  effect }
          if !callArguments.isEmpty then
            let results := match summary? with
              | some summary => summary.returns.filterMap fun derivation =>
                  destinations.find? (·.1 == derivation.result) |>.map
                    fun (_, destination) => {
                      destination := destination.toString, derivation }
              | none => #[]
            return #[.call functionName.toString callArguments
              (summary?.map (·.requiredSeparations) |>.getD #[]) results]
  return (collectDerefReferences context term.raw).map .read

/-- The caller-side identity and instantiated place of a single reference
returned by an ordinary Move call.  The call event itself creates the slot;
this binding lets extraction recognize later source uses of it. -/
private def sourceCallResultBinding? (context : BorrowExtractionContext)
    (destination : Name) (term : TSyntax `term) :
    CommandElabM (Option BorrowRefBinding) := do
  let events ← sourceTermBorrowEvents context term #[(0, destination)]
  let some event := events[0]? | return none
  let Move.Verify.Borrow.Event.call _ arguments _ results := event | return none
  let some result := results.find? (·.destination == destination.toString)
    | return none
  let some argument := arguments.find? (·.parameter == result.derivation.parameter)
    | return none
  let some parent := context.references.find? (·.name.toString == argument.reference)
    | return none
  return some {
    name := destination
    place := { parent.place with path := parent.place.path ++ result.derivation.path }
    kind := result.derivation.kind }

private def sourceCallResultBindings? (context : BorrowExtractionContext)
    (pattern : TSyntax `term) (term : TSyntax `term) :
    CommandElabM (Array BorrowRefBinding) := do
  let components := flattenResultTerms pattern
  let destinations := components.zipIdx.filterMap fun (component, index) =>
    if component.raw.isIdent && !component.raw.getId.isAnonymous then
      some (index, component.raw.getId)
    else none
  let events ← sourceTermBorrowEvents context term destinations
  let some event := events[0]? | return #[]
  let Move.Verify.Borrow.Event.call _ arguments _ results := event | return #[]
  let mut bindings := #[]
  for result in results do
    if bindings.any (·.name.toString == result.destination) then continue
    let some argument := arguments.find? (·.parameter == result.derivation.parameter)
      | continue
    let some parent := context.references.find? (·.name.toString == argument.reference)
      | continue
    let some (_, destination) := destinations.find? (·.2.toString == result.destination)
      | continue
    bindings := bindings.push {
      name := destination
      place := { parent.place with path := parent.place.path ++ result.derivation.path }
      kind := result.derivation.kind }
  return bindings

private def sourcePureReference? (context : BorrowExtractionContext)
    (term : TSyntax `term) : Option BorrowRefBinding := do
  let (head, arguments) ← application? term
  guard (head.raw.isIdent && head.raw.getId == `pure)
  let argument ← arguments[0]?
  guard argument.raw.isIdent
  context.find? argument.raw.getId

private def sourceReturnedReferenceEvents (context : BorrowExtractionContext)
    (term : TSyntax `term) : Array Move.Verify.Borrow.Event :=
  let value := match term with
    | `(pure $value:term) => value
    | _ => term
  (flattenResultTerms value).zipIdx.filterMap fun (component, index) =>
    if component.raw.isIdent then
      (context.find? component.raw.getId).map fun reference =>
        .returnRef reference.name.toString index
    else none

private def sourcePoint (sites : IO.Ref (Array Syntax)) (ref : Syntax) :
    CommandElabM Nat := do
  let current ← sites.get
  sites.set (current.push ref)
  pure current.size

private def prependBorrowEvents (sites : IO.Ref (Array Syntax)) (ref : Syntax)
    (events : Array Move.Verify.Borrow.Event)
    (tail : Move.Verify.Borrow.Block) : CommandElabM Move.Verify.Borrow.Block := do
  events.foldrM (init := tail) fun event tail => do
    pure (.event (← sourcePoint sites ref) event tail)

private def sourceReferenceUsed (name : Name) (elements : Array Lean.DoElem) : Bool :=
  elements.any fun element => containsIdentifier name element.raw

private partial def sourceBorrowBlock (sites : IO.Ref (Array Syntax))
    (context : BorrowExtractionContext) (elements : Array Lean.DoElem)
    (tail : Move.Verify.Borrow.Block := .done) :
    CommandElabM Move.Verify.Borrow.Block := do
  if elements.isEmpty then return tail
  let first := elements[0]!
  let rest := elements.extract 1 elements.size
  let continuationBlock ← sourceBorrowBlock sites context rest tail
  -- Source-edge liveness: a reference used only by an `if` condition dies on
  -- both outgoing edges before either branch body.
  let sourceBorrowBranch (branch following : Array Lean.DoElem) (ref : Syntax) := do
    let live := context.references.filter fun binding =>
      binding.isParameter || binding.kind == .mutable ||
      sourceReferenceUsed binding.name branch ||
        sourceReferenceUsed binding.name following
    let branchContext := { context with references := live }
    let mut block ← sourceBorrowBlock sites branchContext branch
    for dead in context.references do
      unless live.any (·.name == dead.name) do
        block := .event (← sourcePoint sites ref) (.drop dead.name.toString) block
    pure block
  let kind := first.raw.getKind
  if kind == ``Move.moveAddAssign || kind == ``Move.moveSubAssign ||
      kind == ``Move.moveMulAssign || kind == ``Move.moveDivAssign ||
      kind == ``Move.moveModAssign then
    let name : TSyntax `ident := ⟨first.raw[0]⟩
    if (context.find? name.getId).isSome then
      return .event (← sourcePoint sites first.raw) (.write name.getId.toString) continuationBlock
    return continuationBlock
  if first.raw.isOfKind ``Lean.Parser.Term.doReassign then
    let assignment : TSyntax ``Lean.Parser.Term.doReassign := ⟨first.raw⟩
    match assignment with
    | `(doReassign| $name:ident $[: $_]? :=%$_ $rhs:term) =>
        let rhsEvents ← sourceTermBorrowEvents context rhs
        let continuationBlock ← prependBorrowEvents sites rhs.raw rhsEvents continuationBlock
        if (context.find? name.getId).isSome then
          return .event (← sourcePoint sites first.raw) (.write name.getId.toString) continuationBlock
        if context.references.any fun reference =>
            reference.place.root == .local name.getId.toString then
          return .event (← sourcePoint sites first.raw)
            (.ownerWrite { root := .local name.getId.toString }) continuationBlock
        return continuationBlock
    | _ => return continuationBlock
  if first.raw.isOfKind ``Lean.Parser.Term.doMatch then
    let alternatives := first.raw[6][0].getArgs
    let mut branches : Array Move.Verify.Borrow.Block := #[]
    for alternative in alternatives do
      let `(Lean.Parser.Term.matchAltExpr| | $_patterns,* => $body) := alternative
        | continue
      let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨body.raw⟩
      branches := branches.push (← sourceBorrowBranch
        (Lean.Parser.Term.getDoElems body) rest alternative)
    let combined := branches.foldr (init := (.done : Move.Verify.Borrow.Block))
      fun branch alternative => .branch 0 branch alternative .done
    return .branch (← sourcePoint sites first.raw) combined .done continuationBlock
  if kind == ``Move.moveLoopDo || kind == ``Move.moveLoopLabeledDo then
    let label? := if kind == ``Move.moveLoopLabeledDo then
      some first.raw[1]!.getId.toString
    else
      none
    let bodyIndex := if kind == ``Move.moveLoopLabeledDo then 2 else 1
    let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨first.raw[bodyIndex]!⟩
    let bodyBlock ← sourceBorrowBlock sites context
      (Lean.Parser.Term.getDoElems body)
    return .loop (← sourcePoint sites first.raw) label? bodyBlock continuationBlock
  match first with
  | `(doElem| let $name:ident ← &mut $owner:ident[$index:term]) =>
      let (loanBody, continuation) := sourceBorrowScope name.getId rest
      let after ← sourceBorrowBlock sites context continuation tail
      let drop := .event (← sourcePoint sites first.raw)
        (.drop name.getId.toString) after
      let placeTerm ← `($owner[$index])
      let (place, parent?) ← sourceBorrowPlace context placeTerm
      let nestedContext := { context with references := {
        name := name.getId, place, kind := .mutable } :: context.references }
      let body ← sourceBorrowBlock sites nestedContext loanBody drop
      return .event (← sourcePoint sites first.raw)
        (.borrowMut name.getId.toString place parent?) body
  | `(doElem| let $name:ident ← & $owner:ident[$index:term]) =>
      let (loanBody, continuation) := sourceBorrowScope name.getId rest
      let after ← sourceBorrowBlock sites context continuation tail
      let drop := .event (← sourcePoint sites first.raw)
        (.drop name.getId.toString) after
      let placeTerm ← `($owner[$index])
      let (place, parent?) ← sourceBorrowPlace context placeTerm
      let nestedContext := { context with references := {
        name := name.getId, place, kind := .immutable } :: context.references }
      let body ← sourceBorrowBlock sites nestedContext loanBody drop
      return .event (← sourcePoint sites first.raw)
        (.borrowImm name.getId.toString place parent?) body
  | `(doElem| let $name:ident ← &mut $place:term) =>
      let (loanBody, continuation) := sourceBorrowScope name.getId rest
      let after ← sourceBorrowBlock sites context continuation tail
      let drop := .event (← sourcePoint sites first.raw)
        (.drop name.getId.toString) after
      let (place, parent?) ← sourceBorrowPlace context place
      let nestedContext := { context with references := {
        name := name.getId, place, kind := .mutable } :: context.references }
      let body ← sourceBorrowBlock sites nestedContext loanBody drop
      return .event (← sourcePoint sites first.raw)
        (.borrowMut name.getId.toString place parent?) body
  | `(doElem| let $name:ident ← & $place:term) =>
      let (loanBody, continuation) := sourceBorrowScope name.getId rest
      let after ← sourceBorrowBlock sites context continuation tail
      let drop := .event (← sourcePoint sites first.raw)
        (.drop name.getId.toString) after
      let (place, parent?) ← sourceBorrowPlace context place
      let nestedContext := { context with references := {
        name := name.getId, place, kind := .immutable } :: context.references }
      let body ← sourceBorrowBlock sites nestedContext loanBody drop
      return .event (← sourcePoint sites first.raw)
        (.borrowImm name.getId.toString place parent?) body
  | `(doElem| let $_name:ident ← * $reference:term) =>
      if reference.raw.isIdent && (context.find? reference.raw.getId).isSome then
        return .event (← sourcePoint sites first.raw)
          (.read reference.raw.getId.toString) continuationBlock
      return continuationBlock
  | `(doElem| let $name:ident ← $value:term) =>
      if let some source ← sourceFreezeArgument? value then
        if let some sourceBinding := context.find? source.getId then
          let (loanBody, continuation) := sourceBorrowScope name.getId rest
          let withoutSource := context.references.filter (·.name != source.getId)
          let afterContext := { context with references := withoutSource }
          let after ← sourceBorrowBlock sites afterContext continuation tail
          let drop := .event (← sourcePoint sites first.raw)
            (.drop name.getId.toString) after
          let nestedContext := { context with references := {
            name := name.getId
            place := sourceBinding.place
            kind := .immutable } :: withoutSource }
          let body ← sourceBorrowBlock sites nestedContext loanBody drop
          return .event (← sourcePoint sites first.raw)
            (.freeze source.getId.toString name.getId.toString) body
      if let some returned ← sourceCallResultBinding? context name.getId value then
        let (loanBody, continuation) := sourceBorrowScope name.getId rest
        let after ← sourceBorrowBlock sites context continuation tail
        let drop := .event (← sourcePoint sites first.raw)
          (.drop name.getId.toString) after
        let nestedContext := {
          context with references := returned :: context.references }
        let body ← sourceBorrowBlock sites nestedContext loanBody drop
        return ← prependBorrowEvents sites value.raw
          (← sourceTermBorrowEvents context value #[(0, name.getId)]) body
      prependBorrowEvents sites value.raw (← sourceTermBorrowEvents context value)
        continuationBlock
  | `(doElem| let $pattern:term ← $value:term) =>
      let returned ← sourceCallResultBindings? context pattern value
      if returned.isEmpty then
        return ← prependBorrowEvents sites value.raw
          (← sourceTermBorrowEvents context value) continuationBlock
      let names := returned.map (·.name)
      let (loanBody, continuation) := sourceBorrowScopes names rest
      let after ← sourceBorrowBlock sites context continuation tail
      let mut dropped := after
      for returnedReference in returned.reverse do
        dropped := .event (← sourcePoint sites first.raw)
          (.drop returnedReference.name.toString) dropped
      let nestedContext := {
        context with references := returned.toList ++ context.references }
      let body ← sourceBorrowBlock sites nestedContext loanBody dropped
      let components := flattenResultTerms pattern
      let destinations := components.zipIdx.filterMap fun (component, index) =>
        if component.raw.isIdent && !component.raw.getId.isAnonymous then
          some (index, component.raw.getId)
        else none
      return ← prependBorrowEvents sites value.raw
        (← sourceTermBorrowEvents context value destinations) body
  | `(doElem| let mut $_name:ident ← * $reference:term) =>
      if reference.raw.isIdent && (context.find? reference.raw.getId).isSome then
        return .event (← sourcePoint sites first.raw)
          (.read reference.raw.getId.toString) continuationBlock
      return continuationBlock
  | `(doElem| while $_condition:doIfCond do $body:doSeq) =>
      let bodyBlock ← sourceBorrowBlock sites context
        (Lean.Parser.Term.getDoElems body)
      return .loop (← sourcePoint sites first.raw) none bodyBlock continuationBlock
  | `(doElem| break@$label:ident) =>
      return .break (some label.getId.toString)
  | `(doElem| continue@$label:ident) =>
      return .continue (some label.getId.toString)
  | `(doElem| break) => return .break none
  | `(doElem| continue) => return .continue none
  | `(doElem| if $_condition:doIfCond then $thenBranch:doSeq) =>
      let thenBlock ← sourceBorrowBranch
        (Lean.Parser.Term.getDoElems thenBranch) rest first.raw
      let elseBlock ← sourceBorrowBranch #[] rest first.raw
      return .branch (← sourcePoint sites first.raw) thenBlock elseBlock continuationBlock
  | `(doElem| if $_condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
      let thenBlock ← sourceBorrowBranch
        (Lean.Parser.Term.getDoElems thenBranch) rest first.raw
      let elseBlock ← sourceBorrowBranch
        (Lean.Parser.Term.getDoElems elseBranch) rest first.raw
      return .branch (← sourcePoint sites first.raw) thenBlock elseBlock continuationBlock
  | `(doElem| return $value:term) =>
      let returned := sourceReturnedReferenceEvents context value
      if !returned.isEmpty then
        return ← prependBorrowEvents sites value.raw returned .stop
      if let some reference := sourcePureReference? context value then
        return .event (← sourcePoint sites value.raw)
          (.returnRef reference.name.toString) .stop
      if value.raw.isIdent then
        if let some reference := context.find? value.raw.getId then
          return .event (← sourcePoint sites value.raw)
            (.returnRef reference.name.toString) .stop
      match value with
      | `(& $place:term) =>
          let point ← sourcePoint sites value.raw
          let name := s!"_moveBorrowReturn{point}"
          let (place, parent?) ← sourceBorrowPlace context place
          return .event point (.borrowImm name place parent?) <|
            .event (← sourcePoint sites value.raw) (.returnRef name) .stop
      | `(&mut $place:term) =>
          let point ← sourcePoint sites value.raw
          let name := s!"_moveBorrowReturn{point}"
          let (place, parent?) ← sourceBorrowPlace context place
          return .event point (.borrowMut name place parent?) <|
            .event (← sourcePoint sites value.raw) (.returnRef name) .stop
      | _ => pure ()
      prependBorrowEvents sites value.raw (← sourceTermBorrowEvents context value) .stop
  | `(doElem| $value:term) =>
      if value.raw.isOfKind ``Move.abortTerm then return .abort
      let returned := sourceReturnedReferenceEvents context value
      if !returned.isEmpty then
        return ← prependBorrowEvents sites value.raw returned .stop
      if let some reference := sourcePureReference? context value then
        return .event (← sourcePoint sites value.raw)
          (.returnRef reference.name.toString) .stop
      if value.raw.isIdent then
        if let some reference := context.find? value.raw.getId then
          return .event (← sourcePoint sites value.raw)
            (.returnRef reference.name.toString) .stop
      match value with
      | `(& $place:term) =>
          let point ← sourcePoint sites value.raw
          let name := s!"_moveBorrowReturn{point}"
          let (place, parent?) ← sourceBorrowPlace context place
          return .event point (.borrowImm name place parent?) <|
            .event (← sourcePoint sites value.raw) (.returnRef name) .stop
      | `(&mut $place:term) =>
          let point ← sourcePoint sites value.raw
          let name := s!"_moveBorrowReturn{point}"
          let (place, parent?) ← sourceBorrowPlace context place
          return .event point (.borrowMut name place parent?) <|
            .event (← sourcePoint sites value.raw) (.returnRef name) .stop
      | _ => pure ()
      prependBorrowEvents sites value.raw (← sourceTermBorrowEvents context value) continuationBlock
  | `(doElem| let $_name:ident := $value:term)
  | `(doElem| let mut $_name:ident ← $value:term)
  | `(doElem| let mut $_name:ident := $value:term) =>
      prependBorrowEvents sites value.raw (← sourceTermBorrowEvents context value) continuationBlock
  | _ => pure continuationBlock

private structure BuiltBorrowProgram where
  program : Move.Verify.Borrow.Program
  sites : Array Syntax

private def buildBorrowProgram (function : TSyntax `ident)
    (signature : SourceSignature) : CommandElabM BuiltBorrowProgram := do
  let declaration ← declarationFor function
  let sourceBodySyntax := if declaration.value.isOfKind ``Lean.Parser.Term.paren then
      declaration.value[1] else declaration.value
  let source : TSyntax `term := ⟨← desugarPrimitives sourceBodySyntax true⟩
  let source ← match source with
    | `(do $sequence:doSeq) =>
        let sequence ← Lean.Elab.Command.liftCoreM <|
          Move.freshenShadowedLocals sequence
        `(do $sequence)
    | _ => pure source
  let resources ← collectResources source.raw
  let resourceBindings := (distinctHeads resources).map fun head => {
    head
    descriptorFor := fun _ => throwError "borrow extraction does not use store descriptors" }
  let references := signature.referenceParameters.toList.mapIdx fun index (name, kind, _) => {
    name := name.getId
    place := { root := .parameter index }
    kind
    isParameter := true }
  let context : BorrowExtractionContext := { resources := resourceBindings, references }
  let sites ← IO.mkRef #[]
  let body ← match source with
    | `(do $sequence:doSeq) =>
        sourceBorrowBlock sites context (Lean.Parser.Term.getDoElems sequence)
    | _ => pure .done
  let parameters := signature.referenceParameters.map fun (name, kind, _) => {
    name := name.getId.toString, kind }
  let parameterEffects := signature.referenceParameters.map fun (_, kind, _) =>
    if kind == .immutable then .read else .ignore
  pure {
    program := {
      declaration := ((← getCurrNamespace) ++ function.getId).toString
      parameters
      body
      summary := { parameterEffects } }
    sites := ← sites.get }

private def borrowErrorMessage (error : Move.Verify.Borrow.BorrowError) : MessageData :=
  let reference := error.reference?.map (s!" `{·}`") |>.getD ""
  let conflict := error.conflicting?.map (s!" (conflicts with `{·}`)") |>.getD ""
  m!"borrow safety error{reference}: {error.kind.message}{conflict}"

private def emitBorrowCertificate (function : TSyntax `ident)
    (built : BuiltBorrowProgram) : CommandElabM Unit := do
  let programName := mkIdentFrom function (function.getId ++ `borrowProgram)
  if (← getEnv).contains ((← getCurrNamespace) ++ programName.getId) then return
  let firstAnalysis ← match Move.Verify.Borrow.analyze built.program with
    | .ok analysis => pure analysis
    | .error error =>
        let site := built.sites[error.point]?.getD function.raw
        throwErrorAt site (borrowErrorMessage error)
  let summary : Move.Verify.Borrow.Summary := {
    parameterEffects := firstAnalysis.finalState.parameterEffects
    requiredSeparations := firstAnalysis.finalState.requiredSeparations
    returns := firstAnalysis.finalState.returns }
  let program := { built.program with summary }
  let certificate ← match Move.Verify.Borrow.makeCertificate program with
    | .ok certificate => pure certificate
    | .error error =>
        let site := built.sites[error.point]?.getD function.raw
        throwErrorAt site (borrowErrorMessage error)
  let programTerm ← liftTermElabM <|
    Lean.PrettyPrinter.delab (Lean.toExpr program)
  let certificateTerm ← liftTermElabM <|
    Lean.PrettyPrinter.delab (Lean.toExpr certificate)
  let certificateName := mkIdentFrom function (function.getId ++ `borrowCertificate)
  let theoremName := mkIdentFrom function (function.getId ++ `wellBorrowed)
  elabCommand (← `(def $programName : Move.Verify.Borrow.Program := $(⟨programTerm⟩)))
  elabCommand (← `(def $certificateName : Move.Verify.Borrow.Certificate :=
    $(⟨certificateTerm⟩)))
  elabCommand (← `(theorem $theoremName :
      Move.Verify.Borrow.WellBorrowed $programName := by
    exact Move.Verify.Borrow.soundChecked (certificate := $certificateName) (by native_decide)))
  let functionName := (← getCurrNamespace) ++ function.getId
  modifyEnv fun env => borrowSummaries.addEntry env (functionName, summary)

private def sourceEffectJoin : Move.Verify.Borrow.CallEffect →
    Move.Verify.Borrow.CallEffect → Move.Verify.Borrow.CallEffect
  | .write, _ | _, .write => .write
  | .consume, _ | _, .consume => .consume
  | .read, _ | _, .read => .read
  | _, _ => .ignore

private def sourceSummaryJoin (left right : Move.Verify.Borrow.Summary) :
    Move.Verify.Borrow.Summary :=
  let count := max left.parameterEffects.size right.parameterEffects.size
  let effects := (List.range count).toArray.map fun index =>
    sourceEffectJoin (left.parameterEffects[index]?.getD .ignore)
      (right.parameterEffects[index]?.getD .ignore)
  let separations := right.requiredSeparations.foldl
    (fun accumulated requirement => if accumulated.contains requirement then
      accumulated else accumulated.push requirement)
    left.requiredSeparations
  { parameterEffects := effects
    requiredSeparations := separations
    returns := right.returns.foldl
      (fun accumulated returned => if accumulated.contains returned then
        accumulated else accumulated.push returned)
      left.returns }

/-- Monotone call-summary iteration for a recursive component.  The final
certificates replay calls using these summaries; reaching equality is checked
by one final iteration rather than trusting an iteration counter. -/
private def stabilizeBorrowSummaries
    (functions : Array (TSyntax `ident × SourceSignature)) (ref : Syntax) :
    CommandElabM Unit := do
  for (function, signature) in functions do
    let initial : Move.Verify.Borrow.Summary := {
      parameterEffects := signature.referenceParameters.map fun (_, kind, _) =>
        if kind == .immutable then .read else .ignore }
    let name := (← getCurrNamespace) ++ function.getId
    unless (borrowSummaries.getState (← getEnv)).contains name do
      modifyEnv fun env => borrowSummaries.addEntry env (name, initial)
  let fuel := Nat.mul (Nat.mul functions.size functions.size) 4 + 16
  let mut stable := false
  for _ in [0:fuel] do
    let mut changed := false
    for (function, signature) in functions do
      let built ← buildBorrowProgram function signature
      let analysis ← match Move.Verify.Borrow.analyze built.program with
        | .ok analysis => pure analysis
        | .error error =>
            throwErrorAt (built.sites[error.point]?.getD ref)
              (borrowErrorMessage error)
      let inferred : Move.Verify.Borrow.Summary := {
        parameterEffects := analysis.finalState.parameterEffects
        requiredSeparations := analysis.finalState.requiredSeparations
        returns := analysis.finalState.returns }
      let name := (← getCurrNamespace) ++ function.getId
      let old := borrowSummaries.getState (← getEnv) |>.find? name |>.getD {}
      let joined := sourceSummaryJoin old inferred
      if joined != old then
        changed := true
        modifyEnv fun env => borrowSummaries.addEntry env (name, joined)
    if !changed then
      stable := true
      break
  unless stable do
    throwErrorAt ref "recursive borrow summaries did not reach a post-fixpoint"

/-- Functions whose relational semantics is being generated, to cut mutual
recursion between on-demand generations. -/
private initialize generationInProgress : IO.Ref NameSet ← IO.mkRef {}

private partial def containsFunctionCall
    (fullName shortName : Name) (stx : Syntax) : Bool :=
  let direct := stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs > 0 &&
    stx[0].isIdent &&
      (stx[0].getId == fullName || stx[0].getId == shortName)
  let marked := stx.isOfKind ``Move.continueCallTerm && stx.getNumArgs > 1 &&
    stx[1].isIdent &&
      (stx[1].getId == fullName || stx[1].getId == shortName)
  direct || marked || stx.getArgs.any (containsFunctionCall fullName shortName)

/-- Whether the retained body directly refers to its own Move declaration.
Move has no first-class functions, so such an occurrence is a recursive call. -/
def isRecursive (function : Syntax) : CommandElabM Bool := do
  let declaration ← declarationFor function
  let body ← sourceBody declaration
  let fullName := (← getCurrNamespace) ++ function.getId
  pure (containsFunctionCall fullName function.getId body.raw)

/-- The Move callees a body names: functions with retained source. -/
private partial def collectCallees (stx : Syntax) (callees : Array Name := #[]) :
    CommandElabM (Array Name) := do
  let mut callees := callees
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs > 0 && stx[0].isIdent then
    if let some callee ← resolveMoveFunction? ⟨stx[0]⟩ then
      if (declarations.getState (← getEnv)).contains callee && !callees.contains callee then
        callees := callees.push callee
  if stx.isOfKind ``Move.continueCallTerm && stx.getNumArgs > 1 && stx[1].isIdent then
    if let some callee ← resolveMoveFunction? ⟨stx[1]⟩ then
      if (declarations.getState (← getEnv)).contains callee && !callees.contains callee then
        callees := callees.push callee
  for child in stx.getArgs do
    callees ← collectCallees child callees
  pure callees

/-- The resource families a body touches, including — transitively — those
its Move callees touch: a callee's `sourceSpec` needs its families' stores,
and the caller's semantics applies it. -/
private partial def collectResourcesTransitively (body : Syntax)
    (visited : Array Name := #[]) (resources : Array Family := #[]) :
    CommandElabM (Array Family) := do
  let mut resources ← collectResources body resources
  let mut visited := visited
  for callee in ← collectCallees body do
    if visited.contains callee then continue
    visited := visited.push callee
    let some calleeDeclaration := declarations.getState (← getEnv) |>.find? callee
      | continue
    let calleeBody ← sourceBody calleeDeclaration
    -- A callee family named by global constants alone is the same family
    -- here; one at the callee's own type parameters (`Vault T`) is known
    -- only by its head.
    let calleeResources ← collectResourcesTransitively calleeBody.raw visited #[]
    for family in calleeResources do
      if family.concrete && (← closedType family.term) then
        resources := pushResource resources family
      else
        resources := pushResource resources {
          term := ⟨mkIdentFrom body family.head⟩
          head := family.head
          key := family.head.toString ++ " …"
          concrete := false }
  pure resources

/-- Resource families an effectful source function must bring into scope: those
it borrows or publishes/removes, closed under global invariants.  A write to a
family re-checks every invariant naming it, and that obligation refers to every
family the invariant mentions — so those families' stores must be in scope even
when the function never touches them directly. -/
def inferredResources (function : Syntax) : CommandElabM (Array Family) := do
  let declaration ← declarationFor function
  let body ← sourceBody declaration
  let touched ← collectResourcesTransitively body.raw
  let env ← getEnv
  -- Close the touched families under global-invariant mentions, at the level
  -- of canonical head names.  Bounded fixed point, deduplicating by string to
  -- avoid `Name` representation pitfalls; the mention lists are already
  -- complete per invariant, so a handful of passes reach closure.
  let touchedNames := touched.map (·.head)
  let has (arr : Array Name) (n : Name) : Bool := arr.any (·.toString == n.toString)
  let mut all := touchedNames
  for _ in [0:8] do
    let previous := all
    for family in previous do
      for (_, _, mentioned) in Move.globalInvariants env family do
        for m in mentioned do
          unless has all m do all := all.push m
    if all.size == previous.size then break
  let mut families := touched
  for m in all do
    unless has touchedNames m do
      families := pushResource families (familyOfName function m)
  return families

private def argumentType (types : Array (TSyntax `term)) : MacroM (TSyntax `term) := do
  match types.size with
  | 0 => `(Unit)
  | 1 => pure types[0]!
  | _ =>
      let reversed := types.toList.reverse
      let result := reversed.head!
      reversed.tail.foldlM (init := result) fun result type => `($type × $result)

private def sourceResultType (resultType : TSyntax `term)
    (mutableParameters : Array (TSyntax `ident × TSyntax `term)) :
    MacroM (TSyntax `term) := do
  if mutableParameters.isEmpty then return resultType
  let referents ← argumentType (mutableParameters.map (·.2))
  `($resultType × $referents)

private def mutationArgumentTypes (signature : SourceSignature) :
    MacroM (Array (TSyntax `term)) := do
  signature.arguments.zip signature.types |>.mapM fun (argument, type) =>
    if signature.mutableParameters.any (·.1.getId == argument.getId) then
      `(Move.Semantics.Mutation $type)
    else
      pure type

private def mutationResultType (declaredResult logicalResult : TSyntax `term)
    (mutableParameters : Array (TSyntax `ident × TSyntax `term)) :
    MacroM (TSyntax `term) := do
  let logicalTypes := (resultLeaves logicalResult).map (·.logicalType)
  let resultTypes ← (resultLeaves declaredResult).zipIdx.mapM fun (leaf, index) =>
    let logicalType := logicalTypes[index]!
    match leaf with
    | .mutable _ => `(Move.Semantics.Mutation $logicalType)
    | _ => pure logicalType
  let resultType ← argumentType resultTypes
  let rootTypes ← mutableParameters.mapM fun (_, referent) =>
    `(Move.Semantics.Mutation $referent)
  let roots ← argumentType rootTypes
  `($resultType × $roots)

private def argumentProjection (base : TSyntax `term) (index count : Nat) :
    MacroM (TSyntax `term) := do
  let mut projection := base
  for _ in [:index] do
    projection ← `($projection.2)
  if index + 1 < count then `($projection.1) else pure projection

/-- Open an arbitrary heterogeneous set of mutable parameters through the
generic tuple combinator, rebinding the source parameter names to their
mutation components for `body`. -/
private def withMutableParameters
    (parameters : Array (TSyntax `ident × TSyntax `term))
    (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
  unless !parameters.isEmpty do
    throwError "internal error: an empty mutable-parameter scope"
  match parameters.size with
  | 1 =>
      let parameter := parameters[0]!.1
      return ← `(Move.Semantics.withMutation $parameter
        (fun $parameter => $body))
  | 2 =>
      let first := parameters[0]!.1
      let second := parameters[1]!.1
      return ← `(Move.Semantics.withMutations2 $first $second
        (fun $first $second => $body))
  | 3 =>
      let first := parameters[0]!.1
      let second := parameters[1]!.1
      let third := parameters[2]!.1
      return ← `(Move.Semantics.withMutations3 $first $second $third
        (fun $first $second $third => $body))
  | _ => pure ()
  let owners ← packCallArguments parameters[0]!.1.raw
    (parameters.map fun (parameter, _) => (⟨parameter.raw⟩ : TSyntax `term))
  let packed := mkIdentFrom parameters[0]!.1 `_moveSpecMutations
  let packedTerm : TSyntax `term := ⟨packed.raw⟩
  let mut unpacked := body
  for ((parameter, _), index) in parameters.zipIdx.reverse do
    let projection ← liftMacroM <|
      argumentProjection packedTerm index parameters.size
    unpacked ← `(let $parameter := $projection; $unpacked)
  let types : TSyntaxArray `term := parameters.map (·.2)
  `(Move.Semantics.withMutations (types := [$types,*]) (fun value => value)
      (fun value => value) $owners
      (fun $packed => $unpacked))

/-- The same arbitrary-arity scope when the mutation types come from pattern
binders and are therefore known to Lean's elaborator but not retained in the
surface syntax. The holes are fixed by the packed owner and body types. -/
private def withInferredMutableValues (values : Array (TSyntax `ident))
    (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
  unless !values.isEmpty do
    throwError "internal error: an empty inferred mutable-value scope"
  match values.size with
  | 1 => return ← `(Move.Semantics.withMutation $(values[0]!)
      (fun $(values[0]!) => $body))
  | 2 => return ← `(Move.Semantics.withMutations2 $(values[0]!) $(values[1]!)
      (fun $(values[0]!) $(values[1]!) => $body))
  | 3 => return ← `(Move.Semantics.withMutations3 $(values[0]!) $(values[1]!)
      $(values[2]!) (fun $(values[0]!) $(values[1]!) $(values[2]!) => $body))
  | _ => pure ()
  let owners ← packCallArguments values[0]!.raw
    (values.map fun value => (⟨value.raw⟩ : TSyntax `term))
  let packed := mkIdentFrom values[0]! `_moveSpecMutations
  let packedTerm : TSyntax `term := ⟨packed.raw⟩
  let mut unpacked := body
  for (value, index) in values.zipIdx.reverse do
    let projection ← liftMacroM <| argumentProjection packedTerm index values.size
    unpacked ← `(let $value := $projection; $unpacked)
  let mut inferredTypes : Array (TSyntax `term) := #[]
  for _ in values do inferredTypes := inferredTypes.push (← `(_))
  `(Move.Semantics.withMutations (types := [$inferredTypes,*])
      (fun value => value) (fun value => value) $owners
      (fun $packed => $unpacked))

/-- The name holding a parameter's value at function entry, for `old(p)` in
a loop invariant. -/
def entryValueName (parameter : Name) : Name :=
  Name.mkSimple s!"_moveSpecOld_{parameter}"

private def unpackArguments (arguments : Array (TSyntax `ident))
    (body : TSyntax `term) : MacroM (TSyntax `term) := do
  -- Every parameter's entry value stays available under its `old` name.
  let body ← arguments.foldrM (init := body)
      fun (argument : TSyntax `ident) (body : TSyntax `term) => do
    let entry := mkIdentFrom argument (entryValueName argument.getId)
    let argumentTerm : TSyntax `term := ⟨argument.raw⟩
    `(let $entry:ident := $argumentTerm; $body)
  match arguments.size with
  | 0 => `(fun _moveSpecArgs => $body)
  | 1 => `(fun $(arguments[0]!) => $body)
  | _ =>
      let args := mkIdentFrom arguments[0]! `_moveSpecArgs
      let argsTerm : TSyntax `term := ⟨args.raw⟩
      let mut result := body
      for index in (List.range arguments.size).reverse do
        let argument := arguments[index]!
        let projection ← argumentProjection argsTerm index arguments.size
        result ← `(let $argument := $projection; $result)
      `(fun $args => $result)

private structure MutualFamilyInfo where
  anchor : Name
  indexType : Name
  argsFamily : Name
  resultFamily : Name
  body : Name
  source : Name
  members : Array Name
  constructors : Array Name
  hasMutationMembers : Bool := false
  deriving Inhabited

private initialize mutualFamilies :
    SimplePersistentEnvExtension (Name × MutualFamilyInfo) (NameMap MutualFamilyInfo) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, info) => map.insert name info
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun map (name, info) => map.insert name info) {} entries
  }

/-- Whether a family's head is in scope: every instantiation of a head the
function touches has its store in scope. -/
private def knownResource (resources : Array Family) (candidate : Family) : Bool :=
  resources.any (·.head == candidate.head)

/-- A specification function, as specification clauses apply it: `decl` is
its Lean definition.  A *stateful* one reads global memory — `R[a]`,
`existsAt<R>(a)`, or another stateful specification function — and its
definition takes the store instances of the resource heads in `families` and
then, as its first explicit argument, the state to read, which the clause
that applies it supplies (the clause's current state; the pre-state under
`old(…)`). -/
structure SpecFunctionInfo where
  decl : Name
  stateful : Bool
  families : Array Name
  /-- Whether each parameter is a mathematical integer.  Every such parameter
  has type `Int`; applications project bounded Move arguments at the contract
  boundary. -/
  integerArguments : Array Bool := #[]
  /-- Whether the direct result is a mathematical integer. -/
  integerResult : Bool := false
  deriving Inhabited

/-- Specification functions by the name a specification writes: a Move
function `f` with a `spec fun f` (the definition is `f.specFun`), or a
standalone specification function (the definition is the name itself).
Persisted, so an imported module's specification functions serve the
specifications of importing modules. -/
private initialize specFunctions :
    SimplePersistentEnvExtension (Name × SpecFunctionInfo) (NameMap SpecFunctionInfo) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (name, info) => map.insert name info
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun map (name, info) => map.insert name info) {} entries
  }

private def moveIntegerTypeNames : Array Name :=
  #[``Move.U8, ``Move.U16, ``Move.U32, ``Move.U64, ``Move.U128, ``Move.U256,
    ``Move.I8, ``Move.I16, ``Move.I32, ``Move.I64, ``Move.I128, ``Move.I256]

/-- The resolved direct integer name of a surface type.  Nested Move integers
remain part of their enclosing data type; only a spec function's direct
integer domain and codomain are mathematical `Int`. -/
def integerSpecTypeName? (type : TSyntax `term) : CommandElabM (Option Name) := do
  unless type.raw.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload type.raw))
      catch _ => pure none) | return none
  return if name == ``Int || moveIntegerTypeNames.contains name then some name else none

private def logicalSpecType (type : TSyntax `term) : CommandElabM (TSyntax `term) := do
  if (← integerSpecTypeName? type).isSome then `(Int) else pure type


/-- Record a specification function. -/
def registerSpecFunction (name : Name) (info : SpecFunctionInfo) : CommandElabM Unit :=
  modifyEnv fun env => specFunctions.addEntry env (name, info)

/-- Whether `name` is a registered specification function (by the name
specifications write). -/
def isSpecFunction (env : Environment) (name : Name) : Bool :=
  (specFunctions.getState env).contains name

/-- The error for a Move function without a specification version: what in
its body has no pure reading, and the way out. -/
private def noSpecificationVersion {α : Type} (function : Name) (ref : Syntax)
    (reason : MessageData) : CommandElabM α :=
  throwErrorAt ref (m!"Move function `{function}` has no specification version: {reason}; " ++
    m!"`spec fun {Name.mkSimple function.getString!} …` can declare one")

private def isUnitStatement (element : Lean.DoElem) : Bool :=
  match element with
  | `(doElem| pure ()) | `(doElem| ()) => true
  | _ => false

private def isAbortStatement (element : Lean.DoElem) : Bool :=
  match element with
  | `(doElem| abort $_:term) => true
  | _ => false

/-- `assert!(c, code)` as the primitive desugaring leaves it: the statement
`do if c then pure () else abort code`, or that conditional itself.  A pure
reading drops it — the reading is partial, as the specification language's
is: the book's rule for a Move function called in a specification. -/
private def isAssertShape (element : Lean.DoElem) : Bool :=
  let conditional? : Option Lean.DoElem :=
    match element with
    | `(doElem| $value:term) =>
        match value with
        | `(do $sequence:doSeq) =>
            match Lean.Parser.Term.getDoElems sequence with
            | #[single] => some single
            | _ => none
        | _ => none
    | _ => some element
  match conditional? with
  | some conditional =>
      match conditional with
      | `(doElem| if $_:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
          (match Lean.Parser.Term.getDoElems thenBranch with
            | #[unit] => isUnitStatement unit
            | _ => false) &&
          (match Lean.Parser.Term.getDoElems elseBranch with
            | #[aborting] => isAbortStatement aborting
            | _ => false)
      | _ => false
  | none => false

/-- The vector operations that mutate their vector, by function name and by
receiver-style field. -/
private def vectorMutationNames : List Name :=
  [``Move.Vector.insert, ``Move.Vector.remove, ``Move.Vector.popBack, ``Move.Vector.swap,
   ``Move.Vector.swapRemove, ``Move.Vector.append, ``Move.Vector.reverse,
   ``Move.Vector.reverseSlice, ``Move.Vector.trim, ``Move.Vector.trimReverse,
   ``Move.Vector.rotate, ``Move.Vector.rotateSlice,
   ``Move.MutRef.insert, ``Move.MutRef.remove, ``Move.MutRef.popBack, ``Move.MutRef.swap,
   ``Move.MutRef.swapRemove, ``Move.MutRef.append, ``Move.MutRef.reverse,
   ``Move.MutRef.reverseSlice, ``Move.MutRef.trim, ``Move.MutRef.trimReverse,
   ``Move.MutRef.rotate, ``Move.MutRef.rotateSlice]

private def vectorMutationFields : List Name :=
  [`insert, `remove, `popBack, `swap, `swapRemove, `append, `reverse, `reverseSlice,
   `trim, `trimReverse, `rotate, `rotateSlice]

/-- A plain or dependent condition of a source `if`. -/
private def pureCondition (function : Name) (condition : TSyntax ``Lean.Parser.Term.doIfCond) :
    CommandElabM (Option (TSyntax `ident) × TSyntax `term) := do
  match condition with
  | `(doIfCond| $term:term) => pure (none, term)
  | `(doIfCond| $binder:ident : $term:term) => pure (some binder, term)
  | _ => noSpecificationVersion function condition "uses an `if let` condition"

mutual
/-- The *pure reading* of a Move function's retained body: the value it
computes, read over values — the specification version the Move Prover
derives for a pure Move function called in a specification (the rules of
the book's *Function calls*).  References are erased (a borrow is its place,
a read its reference), `assert!`s are dropped, a global read is the
specification place `R[a]` and `existsAt R a` the test `existsAt<R>(a)` (the
clause that applies the function supplies the state), `vector::contains` and
`index_of` are the translator's value-level readings, and a call stays a call
— to the callee's specification version, resolved by the clause rewriting.
A body that writes, reassigns, loops, returns early, creates or moves
resources, or aborts in a value position has no pure reading; the failure
says why. -/
private partial def pureReadingTerm (function : Name) (term : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let read := pureReadingTerm function
  -- `lhs * rhs` and `*r` share a token: the retained syntax keeps both parses
  -- as a `choice`; multiplication is the three-child infix alternative.
  if term.raw.isOfKind `choice then
    if let some multiplication := term.raw.getArgs.find? fun alternative =>
        alternative.getNumArgs == 3 && alternative[1].isAtom &&
          alternative[1].getAtomVal == "*" then
      let lhs ← read ⟨multiplication[0]⟩
      let rhs ← read ⟨multiplication[2]⟩
      return ← `(Move.Spec.intMul $lhs $rhs)
    if term.raw.getNumArgs > 0 then return ← read ⟨term.raw[0]⟩
  match term with
  | `(do $sequence:doSeq) => pureReadingSeq function (Lean.Parser.Term.getDoElems sequence)
  | `(($inner:term)) => do let inner ← read inner; `(($inner))
  | `(* $reference:term) => read reference
  -- `&v[i]` / `&mut v[i]` are element-borrow nodes of their own.
  | `(& $base:ident[$index:term]) => pureReadingPlace function (← `($base[$index]))
  | `(&mut $base:ident[$index:term]) => pureReadingPlace function (← `($base[$index]))
  | `(& $place:term) => pureReadingPlace function place
  | `(&mut $place:term) => pureReadingPlace function place
  | `(pure $value:term) => read value
  -- An abort in value position: the specification language's partial reading
  -- gives it an unspecified value (the Move Prover's arbitrary value of the
  -- site), as the compiler's own conversion keeps the abort expression.
  | `(abort $_:term) =>
      let site := Syntax.mkNumLit (toString ((term.raw.getPos?).map (·.byteIdx) |>.getD 0))
      `(Move.Spec.arbitrary _ $site)
  | `(if $condition:term then $thenBranch:term else $elseBranch:term) => do
      let condition ← read condition
      let thenBranch ← read thenBranch
      let elseBranch ← read elseBranch
      `(if $condition then $thenBranch else $elseBranch)
  | `(if $binder:ident : $condition:term then $thenBranch:term else $elseBranch:term) => do
      let condition ← read condition
      let thenBranch ← read thenBranch
      let elseBranch ← read elseBranch
      `(if $binder:ident : $condition then $thenBranch else $elseBranch)
  | `(match $discriminant:term with $alternatives:matchAlt*) => do
      let discriminant ← read discriminant
      let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
      for alternative in alternatives do
        match alternative with
        | `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $rhs:term) =>
            let rhs ← read rhs
            arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $rhs))
        | _ => noSpecificationVersion function alternative "uses a `match` alternative form without a pure reading"
      `(match $discriminant:term with $arms:matchAlt*)
  | `(($value:term : $type:term)) => do
      let value ← read value
      let type ← logicalSpecType type
      `(($value : $type))
  -- A derived specification version reads source integer operators in the
  -- mathematical domain. Comparisons retain the source function's `Bool`
  -- result, while arithmetic produces `Int`.
  | `($left:term + $right:term) =>
      `(Move.Spec.intAdd $(← read left) $(← read right))
  | `($left:term - $right:term) =>
      `(Move.Spec.intSub $(← read left) $(← read right))
  | `($left:term * $right:term) =>
      `(Move.Spec.intMul $(← read left) $(← read right))
  | `($left:term / $right:term) =>
      `(Move.Spec.intDiv $(← read left) $(← read right))
  | `($left:term % $right:term) =>
      `(Move.Spec.intMod $(← read left) $(← read right))
  | `(-$value:term) => `(Move.Spec.intNeg $(← read value))
  | `($left:term <<< $right:term) =>
      `(Move.Spec.intShiftLeft $(← read left) $(← read right))
  | `($left:term >>> $right:term) =>
      `(Move.Spec.intShiftRight $(← read left) $(← read right))
  | `($left:term < $right:term) =>
      `(Move.Spec.logicalBoolLT $(← read left) $(← read right))
  | `($left:term <= $right:term) =>
      `(Move.Spec.logicalBoolLE $(← read left) $(← read right))
  | `($left:term > $right:term) =>
      `(Move.Spec.logicalBoolLT $(← read right) $(← read left))
  | `($left:term >= $right:term) =>
      `(Move.Spec.logicalBoolLE $(← read right) $(← read left))
  | `($left:term == $right:term) =>
      `(Move.Spec.logicalBoolEq $(← read left) $(← read right))
  | `($left:term != $right:term) =>
      `(!Move.Spec.logicalBoolEq $(← read left) $(← read right))
  | _ =>
      if let some (head, arguments) := application? term then
        if head.raw.isIdent then
          let name? ← try pure (some (← resolveGlobalConstNoOverload head.raw)) catch _ => pure none
          if let some name := name? then
            if name == ``Move.existsAt then
              if let (some resource, some address) := (arguments[0]?, arguments[1]?) then
                let address ← read address
                return ← `(existsAt<$resource>($address))
            if name == ``Move.moveFrom then
              noSpecificationVersion function term "removes a resource (`moveFrom`)"
            if name == ``Move.moveTo then
              noSpecificationVersion function term "publishes a resource (`moveTo`)"
            if name == ``Move.Vector.contains then
              if let (some values, some value) := (arguments[0]?, arguments[1]?) then
                let values ← read values
                let value ← read value
                return ← `(Move.Spec.logicalVectorContains $values $value)
            if name == ``Move.Vector.indexOf then
              if let (some values, some value) := (arguments[0]?, arguments[1]?) then
                let values ← read values
                let value ← read value
                return ← `(vectorIndexOf $values $value)
            if vectorMutationNames.contains name then
              noSpecificationVersion function term m!"mutates a vector (`{name}`)"
            if let .str typeName "certify" := name then
              if (Move.dataInvariant? (← getEnv) typeName).isSome then
                noSpecificationVersion function term
                  m!"creates a certified `{typeName}` (`certify`), whose data invariant is owed by verification"
        if let some (_, field, _) := receiverApplication? term then
          if vectorMutationFields.contains field then
            noSpecificationVersion function term m!"mutates a vector (`{field}`)"
      let children ← term.raw.getArgs.mapM fun child => do
        pure (← read ⟨child⟩).raw
      pure ⟨term.raw.setArgs children⟩

/-- The place of a borrow, read as a value: a global place stays a place
(`R[key].f`, read by the clause's state), a vector element is the element of
the list view, a local or field is itself. -/
private partial def pureReadingPlace (function : Name) (place : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let (root, fields) := splitFieldPath place
  if root.raw.getKind == `«term__[_]» && root.raw.getNumArgs == 4 then
    let collection : TSyntax `term := ⟨root.raw[0]⟩
    let index : TSyntax `term := ⟨root.raw[2]⟩
    let index ← pureReadingTerm function index
    if (← familyOfTerm? collection.raw).isSome then
      return ← projectPath ⟨root.raw.setArg 2 index.raw⟩ fields
    let values ← pureReadingTerm function collection
    -- Keep the derived definition independent of a `GetElem` instance's
    -- proof motive. The list view is internal; authored/transpiled specs keep
    -- the clean `values[index]!` surface handled by `logicalVectorGetElem`.
    let element ← `(($values).toList[(Move.Spec.int $index).toNat]!)
    return ← projectPath element fields
  pureReadingTerm function place

/-- The pure reading of a statement sequence: the value of its last statement
under the `let`s before it.  `assert!`s, units, and aborts before the value
are dropped (the specification language's partial reading); anything else
before the value — a reassignment, a loop, an early `return`, a conditional
or a call as a statement — is an effect the reading cannot express. -/
private partial def pureReadingSeq (function : Name) (elements : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  if elements.isEmpty then return ← `(())
  let first := elements[0]!
  let rest := elements.extract 1 elements.size
  let readRest : CommandElabM (TSyntax `term) := pureReadingSeq function rest
  let read := pureReadingTerm function
  -- The right-hand side of `let x ← …`: a conditional with statement branches
  -- binds the value of each branch; anything else is read as a term.
  let readBound (value : Syntax) : CommandElabM (TSyntax `term) := do
    let element : Lean.DoElem := ⟨value⟩
    match element with
    | `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
        let (binder?, condition) ← pureCondition function condition
        let condition ← read condition
        let thenBranch ← pureReadingSeq function (Lean.Parser.Term.getDoElems thenBranch)
        let elseBranch ← pureReadingSeq function (Lean.Parser.Term.getDoElems elseBranch)
        match binder? with
        | some binder => `(if $binder:ident : $condition then $thenBranch else $elseBranch)
        | none => `(if $condition then $thenBranch else $elseBranch)
    | `(doElem| $term:term) => read term
    | _ => noSpecificationVersion function value "binds a statement form without a pure reading"
  if isAssertShape first then return ← readRest
  let kind := first.raw.getKind
  if kind == ``Move.moveNamedStructLet || kind == ``Move.movePositionalStructLet then
    let fields : TSyntaxArray `term := first.raw[3].getSepArgs.map (⟨·⟩)
    let value ← read ⟨first.raw[6]⟩
    let body ← readRest
    return ← `(let ⟨$fields:term,*⟩ := $value; $body)
  if kind == ``Move.moveAddAssign || kind == ``Move.moveSubAssign ||
      kind == ``Move.moveMulAssign || kind == ``Move.moveDivAssign ||
      kind == ``Move.moveModAssign || first.raw.isOfKind ``Lean.Parser.Term.doReassign ||
      first.raw.isOfKind ``Lean.Parser.Term.doReassignArrow then
    noSpecificationVersion function first.raw "reassigns a local (or writes through a reference)"
  if kind == ``Move.moveForRange || kind == ``Move.moveLoopDo ||
      kind == ``Move.moveLoopLabeledDo then
    noSpecificationVersion function first.raw "loops"
  if first.raw.isOfKind ``Lean.Parser.Term.doMatch then
    unless rest.isEmpty do
      noSpecificationVersion function first.raw "has a `match` statement before its value"
    let discriminants ← first.raw[4].getSepArgs.mapM fun discriminant =>
      read ⟨discriminant[1]⟩
    let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
    for alternative in first.raw[6][0].getArgs do
      let `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $body) := alternative
        | noSpecificationVersion function alternative "uses a `match` alternative form without a pure reading"
      let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨body.raw⟩
      let armValue ← pureReadingSeq function (Lean.Parser.Term.getDoElems body)
      arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $armValue))
    if discriminants.size == 1 then
      return ← `(match $(discriminants[0]!):term with $arms:matchAlt*)
    if discriminants.size == 2 then
      return ← `(match $(discriminants[0]!):term, $(discriminants[1]!):term with $arms:matchAlt*)
    noSpecificationVersion function first.raw "matches on more than two discriminants"
  match first with
  | `(doElem| let $name:ident := $value:term) => do
      let value ← read value; let body ← readRest
      `(let $name := $value; $body)
  | `(doElem| let $name:ident : $type:term := $value:term) => do
      let value ← read value; let body ← readRest; let type ← logicalSpecType type
      `(let $name : $type := $value; $body)
  | `(doElem| let mut $name:ident := $value:term) => do
      let value ← read value; let body ← readRest
      `(let $name := $value; $body)
  | `(doElem| let mut $name:ident : $type:term := $value:term) => do
      let value ← read value; let body ← readRest; let type ← logicalSpecType type
      `(let $name : $type := $value; $body)
  | `(doElem| let $pattern:term := $value:term) => do
      let value ← read value; let body ← readRest
      `(let $pattern:term := $value; $body)
  | `(doElem| let $name:ident ← if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) => do
      let conditional ← `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq)
      let value ← readBound conditional.raw; let body ← readRest
      `(let $name := $value; $body)
  | `(doElem| let $name:ident ← $value:term) => do
      let value ← read value; let body ← readRest
      `(let $name := $value; $body)
  | `(doElem| let $name:ident : $type:term ← $value:term) => do
      let value ← read value; let body ← readRest; let type ← logicalSpecType type
      `(let $name : $type := $value; $body)
  | `(doElem| let mut $name:ident ← $value:term) => do
      let value ← read value; let body ← readRest
      `(let $name := $value; $body)
  | `(doElem| let mut $name:ident : $type:term ← $value:term) => do
      let value ← read value; let body ← readRest; let type ← logicalSpecType type
      `(let $name : $type := $value; $body)
  | `(doElem| let $pattern:term ← $value:term) => do
      let value ← read value; let body ← readRest
      `(let $pattern:term := $value; $body)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) => do
      unless rest.isEmpty do
        noSpecificationVersion function first.raw "has a conditional statement before its value"
      let (binder?, condition) ← pureCondition function condition
      let condition ← read condition
      let thenBranch ← pureReadingSeq function (Lean.Parser.Term.getDoElems thenBranch)
      let elseBranch ← pureReadingSeq function (Lean.Parser.Term.getDoElems elseBranch)
      match binder? with
      | some binder => `(if $binder:ident : $condition then $thenBranch else $elseBranch)
      | none => `(if $condition then $thenBranch else $elseBranch)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq $[else if $conditions:doIfCond then $branches:doSeq]* else $elseBranch:doSeq) => do
      -- An `else if` chain is nested conditionals.
      let mut elseSeq := elseBranch
      for (c, b) in (conditions.zip branches).reverse do
        let nested ← `(doElem| if $c:doIfCond then $b:doSeq else $elseSeq:doSeq)
        elseSeq := ⟨mkNode ``Lean.Parser.Term.doSeqIndent #[mkNullNode #[mkNullNode #[nested.raw, mkNullNode]]]⟩
      let rebuilt ← `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseSeq:doSeq)
      pureReadingSeq function (#[rebuilt] ++ rest)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq) => do
      unless rest.isEmpty do
        noSpecificationVersion function first.raw "has a conditional statement before its value"
      let (binder?, condition) ← pureCondition function condition
      let condition ← read condition
      let thenBranch ← pureReadingSeq function (Lean.Parser.Term.getDoElems thenBranch)
      match binder? with
      | some binder => `(if $binder:ident : $condition then $thenBranch else ())
      | none => `(if $condition then $thenBranch else ())
  | `(doElem| return $_:term) => noSpecificationVersion function first.raw "returns early"
  | `(doElem| while $_:doIfCond do $_:doSeq) => noSpecificationVersion function first.raw "loops"
  | `(doElem| break) | `(doElem| continue) => noSpecificationVersion function first.raw "loops"
  | `(doElem| $value:term) =>
      if rest.isEmpty then read value
      else if isUnitStatement first || isAbortStatement first then readRest
      else noSpecificationVersion function first.raw "has an effect statement before its value"
  | _ => noSpecificationVersion function first.raw "uses a statement form without a pure reading"
end

/-- The pure reading of a Move function's body, for its specification
version; `none` when the function has no retained source. -/
private def pureReadingOf (functionName : Name) (ref : Syntax) : CommandElabM (Option (TSyntax `term)) := do
  let some declaration := declarations.getState (← getEnv) |>.find? functionName | return none
  let body ← sourceBody declaration
  let _ := ref
  return some (← pureReadingTerm functionName body)

/-- Whether a term observes a pre-state (`old(…)`). -/
private partial def observesOld (stx : Syntax) : Bool :=
  stx.isOfKind ``Move.Spec.oldResourceTerm || stx.getArgs.any observesOld

/-- Specification versions being derived: a recursive function's body applies
the function itself, whose specification version is then not available. -/
private initialize specFunctionDerivationInProgress : IO.Ref NameSet ← IO.mkRef {}

/-- The first component of a (possibly dotted) identifier: `coin.value` is
a field access on `coin`, which is what a binder shadows. -/
private def rootName (name : Name) : Name :=
  (name.components.head?).getD name

private partial def identsOf (stx : Syntax) : Array Name :=
  if stx.isIdent then #[stx.getId]
  else stx.getArgs.foldl (fun acc child => acc ++ identsOf child) #[]

/-- The names a syntax node binds for the rest of itself: the binders of
`∀`, `∃`, `fun`, `let`, `have`, and the patterns of a `match` alternative
(over-approximated to every identifier of the binder group).  A bound name is
never read as a specification function. -/
private def binderIdents (stx : Syntax) : Array Name :=
  if stx.isOfKind ``Lean.Parser.Term.forall || stx.getKind == `«term∃_,_» then
    if stx.getNumArgs > 1 then identsOf stx[1] else #[]
  else if stx.isOfKind ``Lean.Parser.Term.fun then
    if stx.getNumArgs > 1 && stx[1].isOfKind ``Lean.Parser.Term.basicFun &&
        stx[1].getNumArgs > 0 then
      identsOf stx[1][0]
    else #[]
  else if stx.isOfKind ``Lean.Parser.Term.let || stx.isOfKind ``Lean.Parser.Term.let_fun ||
      stx.isOfKind ``Lean.Parser.Term.have then
    if stx.getNumArgs > 1 && stx[1].getNumArgs > 0 then
      let declaration := stx[1][0]
      if declaration.getNumArgs > 1 then identsOf declaration[0] ++ identsOf declaration[1]
      else if declaration.getNumArgs > 0 then identsOf declaration[0]
      else #[]
    else #[]
  else if stx.isOfKind ``Lean.Parser.Term.matchAlt then
    if stx.getNumArgs > 1 then identsOf stx[1] else #[]
  else #[]

/-- Whether child `index` of a syntax node is in term position: the field
name of a projection `e.f` or of a structure-instance field `f := e` is not,
and is never read as an application of a specification function. -/
private def childIsTerm (stx : Syntax) (index : Nat) : Bool :=
  if stx.isOfKind ``Lean.Parser.Term.proj then index == 0
  else if stx.isOfKind ``Lean.Parser.Term.structInstField then index != 0
  else true

/-- The names a clause binds implicitly: its result and states. -/
def implicitClauseBinders : Array Name :=
  #[`result, `initial, `final, `abortCode, `_moveSpecOutput, `this]

/-- Add the concrete families a specification clause names (`existsAt<R>(…)`,
global places `R[…]`, `modifies` targets) whose heads are in scope, so that
frames mention them: a caller reaching a generic family only through a callee
names the instantiation in its clauses. -/
partial def addMentionedFamilies (clause : Syntax) (resources : Array Family) :
    CommandElabM (Array Family) := do
  let mut resources := resources
  let candidate? ←
    if clause.isOfKind ``Move.Spec.resourceExistsTerm then
      familyOfTerm? clause[1]
    else if clause.getKind == `«term__[_]» then
      pure ((← rootFamily? ⟨clause⟩).map (·.1))
    else if clause.isOfKind `Move.Spec.modifiesAddress ||
        clause.isOfKind `Move.Spec.modifiesFamily then
      familyOfTerm? clause[0]
    else if clause.isOfKind `Move.Spec.modifiesGenericAddress ||
        clause.isOfKind `Move.Spec.modifiesGenericFamily then
      familyOfTerm? clause[1]
    else
      pure none
  if let some candidate := candidate? then
    if knownResource resources candidate then
      resources := pushResource resources candidate
  for child in clause.getArgs do
    resources ← addMentionedFamilies child resources
  return resources

private def rewriteGlobalPlace (resources : Array Family)
    (state place : TSyntax `term) : CommandElabM (Option (TSyntax `term)) := do
  let (root, fields) := splitFieldPath place
  let some (family, key) ← rootFamily? root | return none
  unless knownResource resources family do return none
  let owner ← `(Move.Semantics.ResourceStore.get
    (Value := $(family.term)) $state $key)
  return some (← projectPath owner fields)


mutual
/-- The specification function an identifier names: a registered one, or —
for a Move function — its specification version, derived on demand from its
retained source when it is not registered yet.  A native (no source) and a
function without a pure reading have none; the error says which and why, and
that `spec fun` can declare one. -/
private partial def specFunctionOfIdent? (identifier : Syntax) :
    CommandElabM (Option (Name × SpecFunctionInfo)) := do
  unless identifier.isIdent do return none
  let env ← getEnv
  let candidates ← try resolveGlobalConst identifier catch _ => pure []
  for name in candidates do
    if let some info := (specFunctions.getState env).find? name then
      return some (name, info)
  for name in candidates do
    if Move.isMoveFunction env name then
      return some (name, ← ensureSpecFunction name identifier)
  return none

/-- Whether a clause observes global state: a global place, `existsAt`,
`old`, or a stateful specification function. -/
partial def mentionsState (stx : Syntax) (bound : Array Name := #[]) : CommandElabM Bool := do
  if stx.isOfKind ``Move.Spec.oldResourceTerm || stx.isOfKind ``Move.Spec.resourceExistsTerm then
    return true
  if stx.getKind == `«term__[_]» then
    if (← rootFamily? ⟨stx⟩).isSome then return true
  let head? := if stx.isIdent then some stx
    else if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 && stx[0].isIdent then
      some stx[0]
    else none
  if let some head := head? then
    unless bound.contains (rootName head.getId) do
      if let some (_, info) ← specFunctionOfIdent? head then
        if info.stateful then return true
  let bound := bound ++ binderIdents stx
  (stx.getArgs.zipIdx.filter fun (_, index) => childIsTerm stx index).anyM
    fun (child, _) => mentionsState child bound

/-- Every family a term reads: its global places and `existsAt` tests, and
the families of the stateful specification functions it applies. -/
partial def mentionedFamilies (stx : Syntax) (bound : Array Name := #[]) :
    CommandElabM (Array Family) := do
  let mut families : Array Family := #[]
  if stx.isOfKind ``Move.Spec.resourceExistsTerm then
    if let some family ← familyOfTerm? stx[1] then families := families.push family
  else if stx.getKind == `«term__[_]» then
    if let some (family, _) ← rootFamily? ⟨stx⟩ then families := families.push family
  let head? := if stx.isIdent then some stx
    else if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 && stx[0].isIdent then
      some stx[0]
    else none
  if let some head := head? then
    unless bound.contains (rootName head.getId) do
      if let some (_, info) ← specFunctionOfIdent? head then
        for family in info.families do
          families := families.push (familyOfName head family)
  let bound := bound ++ binderIdents stx
  for (child, index) in stx.getArgs.zipIdx do
    if childIsTerm stx index then
      families := families ++ (← mentionedFamilies child bound)
  return families

/-- Whether a term applies a registered specification function whose direct
result is a mathematical integer. -/
private partial def usesIntegerSpecFunction (stx : Syntax)
    (bound : Array Name := #[]) : CommandElabM Bool := do
  let head? := if stx.isIdent then some stx
    else if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 && stx[0].isIdent then
      some stx[0]
    else none
  if let some head := head? then
    unless bound.contains (rootName head.getId) do
      if let some (_, info) ← specFunctionOfIdent? head then
        if info.integerResult then return true
  let bound := bound ++ binderIdents stx
  (stx.getArgs.zipIdx.filter fun (_, index) => childIsTerm stx index).anyM
    fun (child, _) => usesIntegerSpecFunction child bound

/-- Rewrite an application of a specification function — an identifier
applied to arguments, or a bare identifier — in a clause: the head becomes the
function's definition and, for a stateful function, `state` is passed first.
The families a stateful function reads must be among `resources`. -/
private partial def rewriteSpecFunctionApplication (resources : Array Family)
    (state : TSyntax `term) (bound : Array Name) (clause : Syntax)
    (rewriteArgument : Bool → Syntax → CommandElabM Syntax) :
    CommandElabM (Option Syntax) := do
  let (head, arguments) :=
    if clause.isOfKind ``Lean.Parser.Term.app && clause.getNumArgs == 2 then
      (clause[0], clause[1].getArgs)
    else if clause.isIdent then (clause, #[])
    else (Syntax.missing, #[])
  unless head.isIdent do return none
  if bound.contains (rootName head.getId) then return none
  let some (name, info) ← specFunctionOfIdent? head | return none
  for family in info.families do
    unless knownResource resources (familyOfName head family) do
      throwErrorAt head (m!"specification function `{name}` reads resource `{family}`, " ++
        m!"which the specified function does not use")
  let arguments ← arguments.zipIdx.mapM fun (argument, index) => do
    let integerArgument := info.integerArguments[index]?.getD false
    let argument ← rewriteArgument integerArgument argument
    if integerArgument then
      let argumentTerm : TSyntax `term := ⟨argument⟩
      pure (← `(Move.Spec.int $argumentTerm)).raw
    else pure argument
  let headIdent := mkCIdentFrom head info.decl
  let arguments := if info.stateful then #[state.raw] ++ arguments else arguments
  if arguments.isEmpty then return some headIdent.raw
  return some (mkNode ``Lean.Parser.Term.app #[headIdent.raw, mkNullNode arguments])

private partial def containsIntegerArithmeticSyntax (term : Syntax) : Bool :=
  (term.isAtom && ["+", "-", "*", "/", "%", "<<<", ">>>"].contains term.getAtomVal) ||
  term.getArgs.any containsIntegerArithmeticSyntax

private partial def isConditionalOrMatch (term : TSyntax `term) : Bool :=
  match term with
  | `(($inner:term)) => isConditionalOrMatch inner
  | `(if $_:term then $_:term else $_:term) => true
  | `(if $_:ident : $_:term then $_:term else $_:term) => true
  | `(match $_:term with $_:matchAlt*) => true
  | _ => false

/-- Some ordinary Lean terms rely on the relation's first operand to supply
their type: leading-dot constructors, non-numeric conditionals/matches, and
polymorphic integer packers are the important cases in existing contracts.
Keep those relations homogeneous; there is no mathematical expression at
their top level to relate heterogeneously. -/
private partial def relationOperandNeedsExpectedType (term : TSyntax `term) : Bool :=
  match term with
  | `(($inner:term)) => relationOperandNeedsExpectedType inner
  | _ =>
      if isConditionalOrMatch term && !containsIntegerArithmeticSyntax term.raw then
        true
      else
        let text := term.raw.reprint.getD term.raw.prettyPrint.pretty |>.trimAscii |>.toString
        if text.startsWith "." then true
        else
          let head :=
            if term.raw.isOfKind ``Lean.Parser.Term.app && term.raw.getNumArgs == 2 then
              term.raw[0]
            else term.raw
          head.isIdent && match lastString? head.getId with
            | some name => ["ofInt", "ofNat"].contains name
            | none => false

/-- Surface views which make a surrounding relation mathematical even when
its other operand is still a bounded Move value. -/
private partial def relationUsesMathematicalSurface (term : TSyntax `term) : Bool :=
  let text := term.raw.reprint.getD term.raw.prettyPrint.pretty
  text.contains ".length" || text.contains ".size"

private partial def relationUsesExplicitRepresentation (term : TSyntax `term) : Bool :=
  let text := term.raw.reprint.getD term.raw.prettyPrint.pretty
  text.contains ".toList" || text.contains ".toNat" || text.contains ".toInt"

private partial def isNumericLiteral (term : TSyntax `term) : Bool :=
  match term with
  | `(($inner:term)) => isNumericLiteral inner
  | _ => term.raw.isNatLit?.isSome

/-- Whether this node immediately introduces an explicitly mathematical
integer binder. The body below such a binder is an integer-specification
context even when a relation's operands are otherwise bare identifiers. -/
private partial def introducesIntBinder (stx : Syntax) : Bool :=
  if (binderIdents stx).isEmpty then false
  else
    let text := stx.reprint.getD stx.prettyPrint.pretty
    text.contains "Int"

/-- A numeric conditional or match passed to a heterogeneous relation needs
an explicit result type so its bounded leaf branches use the scoped
specification coercion. -/
private partial def annotateLogicalComposite (mathematical : Bool)
    (original rewritten : TSyntax `term) :
    CommandElabM (TSyntax `term) :=
  if mathematical && isConditionalOrMatch original then
    `(($rewritten : Int))
  else pure rewritten

/-- Rewrite global-place observations in a contract clause. Bare places refer
to `current`; `old(place)` refers to `previous`.  Applications of
specification functions are rewritten to their definitions, a stateful one
reading the state of its position.  `bound` are the names the clause's
binders introduce, which no rewrite touches. -/
partial def rewriteClause (resources : Array Family)
    (current previous : TSyntax `term) (clause : TSyntax `term)
    (bound : Array Name := #[]) (mathematical : Bool := false) :
    CommandElabM (TSyntax `term) := do
  match clause with
  | `(old($place:term)) =>
      if let some rewritten ← rewriteGlobalPlace resources previous place then
        return rewritten
      -- Any other pre-state observation is the term read over the pre-state:
      -- a stateful specification function, or a place inside a larger term.
      -- A term that observes no state has no pre-state to read.
      let rewritten ← rewriteClause resources previous previous place bound mathematical
      if rewritten.raw == place.raw then
        throwErrorAt place "`old` expects a global resource place"
      pure rewritten
  | `(existsAt<$resourceType:term>($address:term)) =>
      let some family ← familyOfTerm? resourceType.raw
        | throwErrorAt resourceType "`existsAt<…>` expects a resource type"
      unless knownResource resources family do
        throwErrorAt resourceType
          "resource `{resourceType}` is not used by the specified function"
      let address ← rewriteClause resources current previous address bound mathematical
      `(Move.Semantics.ResourceStore.contains
        (Value := $(family.term)) $current $address)
  | `($left:term + $right:term) =>
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if mathematical then `(Move.Spec.intAdd $left $right) else `($left + $right)
  | `($left:term - $right:term) =>
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if mathematical then `(Move.Spec.intSub $left $right) else `($left - $right)
  | `($left:term * $right:term) =>
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if mathematical then `(Move.Spec.intMul $left $right) else `($left * $right)
  | `($left:term / $right:term) =>
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if mathematical then `(Move.Spec.intDiv $left $right) else `($left / $right)
  | `($left:term % $right:term) =>
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if mathematical then `(Move.Spec.intMod $left $right) else `($left % $right)
  | `(-$value:term) =>
      let value ← rewriteClause resources current previous value bound mathematical
      if mathematical then `(Move.Spec.intNeg $value) else `(-$value)
  | `($value:term <<< $amount:term) =>
      let mathematical := mathematical || (!relationUsesExplicitRepresentation value &&
        !relationUsesExplicitRepresentation amount)
      let value ← rewriteClause resources current previous value bound mathematical
      let amount ← rewriteClause resources current previous amount bound mathematical
      if mathematical then `(Move.Spec.intShiftLeft $value $amount) else `($value <<< $amount)
  | `($value:term >>> $amount:term) =>
      let mathematical := mathematical || (!relationUsesExplicitRepresentation value &&
        !relationUsesExplicitRepresentation amount)
      let value ← rewriteClause resources current previous value bound mathematical
      let amount ← rewriteClause resources current previous amount bound mathematical
      if mathematical then `(Move.Spec.intShiftRight $value $amount) else `($value >>> $amount)
  | `($left:term = $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left = $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(Move.Spec.logicalEq $left $right)
  | `($left:term ≠ $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left ≠ $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(¬Move.Spec.logicalEq $left $right)
  | `($left:term < $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (isNumericLiteral left &&
        containsIntegerArithmeticSyntax right.raw && !relationUsesExplicitRepresentation right) ||
        (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left < $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(Move.Spec.logicalLT $left $right)
  | `($left:term > $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (containsIntegerArithmeticSyntax left.raw &&
        isNumericLiteral right && !relationUsesExplicitRepresentation left) ||
        (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left > $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(Move.Spec.logicalLT $right $left)
  | `($left:term ≤ $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (containsIntegerArithmeticSyntax left.raw &&
        isNumericLiteral right && !relationUsesExplicitRepresentation left) ||
        (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left ≤ $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(Move.Spec.logicalLE $left $right)
  | `($left:term ≥ $right:term) =>
      let mathematical := mathematical || ((relationUsesMathematicalSurface left ||
        relationUsesMathematicalSurface right) && !relationUsesExplicitRepresentation left &&
        !relationUsesExplicitRepresentation right) || (isNumericLiteral left &&
        containsIntegerArithmeticSyntax right.raw && !relationUsesExplicitRepresentation right) ||
        (← usesIntegerSpecFunction left.raw bound) ||
        (← usesIntegerSpecFunction right.raw bound)
      let homogeneous :=
        !mathematical || relationOperandNeedsExpectedType left ||
          relationOperandNeedsExpectedType right
      let leftOriginal := left
      let rightOriginal := right
      let left ← rewriteClause resources current previous left bound mathematical
      let right ← rewriteClause resources current previous right bound mathematical
      if homogeneous then `($left ≥ $right)
      else
        let left ← annotateLogicalComposite mathematical leftOriginal left
        let right ← annotateLogicalComposite mathematical rightOriginal right
        `(Move.Spec.logicalLE $right $left)
  | _ =>
      if let some rewritten ← rewriteGlobalPlace resources current clause then
        return rewritten
      if let some rewritten ← rewriteSpecFunctionApplication resources current bound clause.raw
          (fun integerArgument child => do
            pure (← rewriteClause resources current previous ⟨child⟩ bound
              (mathematical || integerArgument)).raw) then
        return ⟨rewritten⟩
      let mathematical := mathematical || introducesIntBinder clause.raw
      let bound := bound ++ binderIdents clause.raw
      let args ← clause.raw.getArgs.zipIdx.mapM fun (child, index) => do
        if childIsTerm clause.raw index then
          let rewritten ← rewriteClause resources current previous ⟨child⟩ bound mathematical
          pure rewritten.raw
        else pure child
      pure ⟨clause.raw.setArgs args⟩

/-- Declare a specification function — the definition `f.specFun` of the
specification version of the Move function `f` when `attached`, the
definition `f` otherwise — from its binders and body, and register it.  The
body is a specification term: its global places and `existsAt` tests, and
the stateful specification functions it applies, make the function stateful,
whose definition then takes the store instances of the families it reads and
the state to read as its first explicit argument.  Applications of other
specification functions (and of Move functions, through their specification
versions) are resolved here. -/
partial def declareSpecFunction (function : TSyntax `ident) (fullName : Name) (attached : Bool)
    (context : Array (TSyntax ``Lean.Parser.Term.bracketedBinder))
    (arguments : Array (TSyntax `ident)) (types : Array (TSyntax `term))
    (resultType? : Option (TSyntax `term)) (body : TSyntax `term)
    (doc? : Option (TSyntax ``Lean.Parser.Command.docComment) := none)
    (derived : Bool := false) : CommandElabM Unit := do
  if observesOld body.raw then
    throwErrorAt body (m!"`old` cannot be used in a specification function; the specification " ++
      m!"that applies the function chooses the state it reads (`old(f …)` reads the pre-state)")
  let bound := arguments.map (·.getId)
  let mut resources : Array Family := #[]
  for family in ← mentionedFamilies body.raw bound do
    unless resources.any (·.key == family.key) do resources := resources.push family
  let stateful := !resources.isEmpty
  let world := mkIdentFrom function `_moveSpecWorld
  let state := mkIdentFrom function `_moveSpecState
  let worldTerm : TSyntax `term := ⟨world.raw⟩
  let stateTerm : TSyntax `term := ⟨state.raw⟩
  let rewritten ← rewriteClause resources stateTerm stateTerm body bound true
  let integerTypes ← types.mapM integerSpecTypeName?
  let integerResultType? ← match resultType? with
    | some resultType => integerSpecTypeName? resultType
    | none => pure none
  unless derived do
    for (type, integerType?) in types.zip integerTypes do
      if let some integerType := integerType? then
        if integerType != ``Int then
          throwErrorAt type
            "specification functions use mathematical `Int`, not Move integer type `{integerType}`"
    if let some integerType := integerResultType? then
      if integerType != ``Int then
        throwErrorAt resultType?.get!
          "specification functions return mathematical `Int`, not Move integer type `{integerType}`"
  let logicalTypes ← types.mapM logicalSpecType
  let logicalResultType? ← resultType?.mapM logicalSpecType
  let mut integerResult := integerResultType?.isSome
  let rewritten ← if integerResult then `(($rewritten : Int)) else pure rewritten
  let declIdent := mkIdentFrom function
    (if attached then function.getId ++ `specFun else function.getId)
  let declName := if attached then fullName ++ `specFun else fullName
  let mut declBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := context
  let heads := distinctHeads resources
  if stateful then
    declBinders := declBinders.push (← `(bracketedBinder| {$world : Type}))
    for (head, index) in heads.zipIdx do
      let storeName := mkIdentFrom function (Name.mkSimple s!"_moveSpecStore{index}")
      declBinders := declBinders.push (← `(bracketedBinder|
        [$storeName : $(← storeType worldTerm head)]))
    declBinders := declBinders.push (← `(bracketedBinder| ($state : $worldTerm)))
  for (argument, type) in arguments.zip logicalTypes do
    declBinders := declBinders.push (← `(bracketedBinder| ($argument : $type)))
  -- A Lean-only helper: kept out of line, as `module` keeps every `def`, so
  -- a `fun` that wrongly calls it is rejected instead of absorbing it.
  let command ← match logicalResultType? with
    | some resultType =>
        `(@[noinline] def $declIdent $declBinders* : $resultType := $rewritten)
    | none => `(@[noinline] def $declIdent $declBinders* := $rewritten)
  -- The doc comment goes into the declaration's modifiers.
  let command := match doc? with
    | some doc => command.raw.setArg 0 (command.raw[0].setArg 0 (mkNullNode #[doc.raw]))
    | none => command.raw
  elabCommand command
  unless (← getEnv).contains declName do
    throwErrorAt function m!"specification function `{declName}` could not be declared"
  unless derived || resultType?.isSome do
    if let some info := (← getEnv).find? declName then
      let rec resultType (type : Lean.Expr) : Lean.Expr :=
        match type with
        | .forallE _ _ body _ => resultType body
        | result => result
      let inferredResult ← liftTermElabM <| Lean.Meta.whnfR (resultType info.type)
      if inferredResult.isAppOf ``Move.MoveInt then
        throwErrorAt body
          "specification function result inferred as a Move integer; declare and return `Int`"
      if inferredResult.isConstOf ``Int then integerResult := true
  registerSpecFunction fullName {
    decl := declName
    stateful
    families := heads
    integerArguments := integerTypes.map (·.isSome)
    integerResult }

/-- The specification version of the Move function `functionName`, registered
or derived now from its retained source: its pure reading, over the
function's signature with references erased (`signatureOf`), declared in the
function's namespace.  A native has no source, an imported function's version
is derived in its own module, a recursive function cannot read itself, and a
body without a pure reading says why: each is an error, pointing at
`spec fun` as the way to declare one. -/
partial def ensureSpecFunction (functionName : Name) (ref : Syntax) :
    CommandElabM SpecFunctionInfo := do
  let env ← getEnv
  if let some info := (specFunctions.getState env).find? functionName then return info
  let short := Name.mkSimple functionName.getString!
  -- A native retains a placeholder body only.
  if Move.moveNativeAttr.hasTag env functionName then
    throwErrorAt ref (m!"Move function `{functionName}` has no specification version: it has no " ++
      m!"body (a native); `spec fun {short} …` can declare one")
  let some declaration := declarations.getState env |>.find? functionName
    | throwErrorAt ref (m!"Move function `{functionName}` has no specification version: it has no " ++
        m!"retained source; `spec fun {short} …` can declare one")
  if (env.getModuleIdxFor? functionName).isSome then
    -- Imported: the version is derived in its own module; say why it was not.
    let body ← sourceBody declaration
    let _ ← pureReadingTerm functionName body
    throwErrorAt ref (m!"imported Move function `{functionName}` has no specification version " ++
      m!"(it is derived at the function's declaration, in its module)")
  if (← specFunctionDerivationInProgress.get).contains functionName then
    throwErrorAt ref (m!"Move function `{functionName}` has no specification version: it is " ++
      m!"recursive; `spec fun {short} …` can declare one")
  specFunctionDerivationInProgress.modify (·.insert functionName)
  try
    let signature ← signatureOf functionName
    unless signature.mutableParameters.isEmpty do
      noSpecificationVersion functionName ref "it takes a mutable reference"
    let body ← sourceBody declaration
    let reading ← pureReadingTerm functionName body
    let resultType ← actionResultType declaration
    let .str namespace_ shortName := functionName
      | throwErrorAt ref "cannot derive a specification version for `{functionName}`"
    let function := mkIdentFrom ref (Name.mkSimple shortName)
    withScope (fun scope => { scope with currNamespace := namespace_ }) do
      declareSpecFunction function functionName (attached := true) signature.context
        signature.arguments signature.types (some resultType) reading (derived := true)
  finally
    specFunctionDerivationInProgress.modify (·.erase functionName)
  match (specFunctions.getState (← getEnv)).find? functionName with
  | some info => return info
  | none => throwErrorAt ref "specification version of `{functionName}` was not registered"
end

/-- The definitions of the specification functions a declaration's value
reaches, transitively through their own bodies. -/
partial def specFunctionDependencies (env : Environment) (declName : Name) : Array Name :=
  let definitions : NameSet :=
    (specFunctions.getState env).foldl (fun acc _ info => acc.insert info.decl) {}
  let rec go (todo : List Name) (seen : NameSet) (acc : Array Name) : Array Name :=
    match todo with
    | [] => acc
    | name :: rest =>
      if seen.contains name then go rest seen acc
      else
        let seen := seen.insert name
        match env.find? name >>= (·.value? (allowOpaque := true)) with
        | some value =>
          let found := value.getUsedConstants.filter fun constant =>
            definitions.contains constant && !seen.contains constant
          go (found.toList ++ rest) seen (acc ++ found.filter fun c => !acc.contains c)
        | none => go rest seen acc
  go [declName] {} #[]

/-- `old(p)` in a loop invariant: the parameter's value at function entry. -/
private partial def replaceEntryValues (parameters : List Name) (stx : Syntax) :
    CommandElabM Syntax := do
  if stx.isOfKind ``Move.Spec.oldResourceTerm && stx.getNumArgs == 3 then
    let inner := stx[1]
    unless inner.isIdent do
      throwErrorAt stx "`old` in a loop invariant takes a parameter"
    unless parameters.contains inner.getId do
      throwErrorAt stx "`old({inner.getId})` in a loop invariant: `{inner.getId}` is not a parameter"
    return mkIdentFrom stx (entryValueName inner.getId)
  return stx.setArgs (← stx.getArgs.mapM (replaceEntryValues parameters))

/-- The label and observed term in an internal `oldAt[label](term)`. -/
private def anchoredOld? (stx : Syntax) : Option (Nat × Syntax) :=
  match stx with
  | `(oldAt[$label:num]($value:term)) =>
      label.raw.isNatLit?.map fun label => (label, value.raw)
  | _ => none

/-- Every distinct value read through `oldAt[label]` in a syntax tree. -/
private partial def anchoredObservations (label : Nat) (stx : Syntax)
    (seen : Array Syntax := #[]) : Array Syntax :=
  let seen := match anchoredOld? stx with
    | some (found, value) =>
      if found == label && !seen.any (fun previous => previous == value) then
        seen.push value
      else
        seen
    | _ => seen
  stx.getArgs.foldl (fun seen child => anchoredObservations label child seen) seen

/-- Replace every anchored old-value syntax node with the lexical value saved
at its corresponding compiler-generated capture point. -/
private partial def replaceAnchoredValues (captures : List StateAnchorCapture)
    (stx : Syntax) : CommandElabM Syntax := do
  if let some (label, observed) := anchoredOld? stx then
    let some savedCapture := captures.find? fun savedCapture =>
      savedCapture.label == label && savedCapture.observed == observed
      | throwErrorAt stx s!"no state-anchor capture {label} value for this anchored `old` observation"
    return savedCapture.snapshot.raw
  return stx.setArgs (← stx.getArgs.mapM (replaceAnchoredValues captures))

/-- A loop's fixed point: `Spec.fix` of its body over the packed state, or,
with stated invariants, `Spec.withInvariant` with one proposition over the
packed state (the store binder is unused — source invariants range over the
loop's locals and the current referents of live mutable references, which
travel with the state), for the automatic prover. -/
private def loopFixpoint (context : TranslationContext)
    (invariants : Array (TSyntax `term)) (state : List (TSyntax `ident))
    (recName stateName : TSyntax `ident) (body pack : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  if invariants.isEmpty then
    return ← `(Move.Semantics.Spec.fix (fun $recName $stateName => $body) $pack)
  let invariants ← invariants.mapM fun clause => do
    let clause : TSyntax `term := ⟨← replaceEntryValues context.parameters clause.raw⟩
    let clause : TSyntax `term := ⟨← replaceAnchoredValues context.stateAnchors clause.raw⟩
    -- Specification functions apply in an invariant as in a clause.  An
    -- invariant ranges over the loop's locals, not over global state.
    if ← mentionsState clause.raw then
      throwErrorAt clause ("a loop invariant ranges over the loop's locals; global state " ++
        "(`R[a]`, `existsAt<R>(a)`, a stateful specification function) is not " ++
        "supported in a loop invariant")
    let bound := context.parameters.toArray ++ state.map (·.getId) ++ implicitClauseBinders
    let placeholder ← `(_moveSpecStore)
    rewriteClause #[] placeholder placeholder clause bound
  let mut conjunction := invariants[0]!
  for clause in invariants[1:] do
    conjunction ← `($conjunction ∧ $clause)
  -- An invariant speaks about values: each live mutable reference, a
  -- component of the state, is read.
  let valued ← (liveMutations context).foldrM (init := conjunction) fun handle acc => do
    let handleTerm : TSyntax `term := ⟨handle.raw⟩
    `(let $handle:ident := Move.Semantics.Mutation.read $handleTerm; $acc)
  let unpacked ← unpackLoopState state ⟨stateName.raw⟩ valued
  -- The state (`pack`) precedes the invariant, so the invariant's binders
  -- have their types when it is elaborated.
  `(Move.Semantics.Spec.withInvariant
      (fun $recName $stateName => $body) $pack
      (fun $stateName _moveSpecStore => $unpacked))

/-- Turn an in-body assertion or assumption into a predicate over the exact
current store.  As in contracts, ordinary resource reads select that store;
`old(parameter)` selects the value retained at function entry and an anchored
old value selects its preceding capture. -/
private def inlineSpecPredicate (context : TranslationContext)
    (source : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let source : TSyntax `term :=
    ⟨← replaceEntryValues context.parameters source.raw⟩
  let source : TSyntax `term :=
    ⟨← replaceAnchoredValues context.stateAnchors source.raw⟩
  let state := mkIdentFrom source `_moveSpecInlineState
  let stateTerm : TSyntax `term := ⟨state.raw⟩
  let loopBound := context.loops.flatMap fun frame => frame.state.map (·.getId)
  let bound := context.parameters.toArray ++ loopBound.toArray ++ implicitClauseBinders
  let families ← mentionedFamilies source.raw bound
  let rewritten ← rewriteClause families stateTerm stateTerm source bound
  let rewritten : TSyntax `term :=
    ⟨← rewriteClauseMutations (liveMutations context) rewritten.raw⟩
  `(fun $state => $rewritten)

/-- Whether a local is a live mutable reference, a retained ancestor, or the
owner of a live loan: another mutable borrow of it must chain through that
loan, as a nested borrow. -/
private def isLoanedOwner (context : TranslationContext) (id : Name) : Bool :=
  (liveMutations context).any (·.getId == id) ||
  context.mutationAncestors.any (fun (ancestor, _) => ancestor.getId == id) ||
  context.mutationOwnerAliases.any (fun (owner, _, _) => owner == id)

/-- Whether retained source directly writes one of `mutations`. Generated
owner-reconciliation writes must not count: those remain inside the loan and
would otherwise spuriously turn sibling structural loans into nested closure
boundaries. -/
private partial def sourceWritesMutation (mutations : List Name)
    (stx : Syntax) : Bool :=
  let assignment := if stx.isOfKind ``Lean.Parser.Term.doReassign then
    let assignment : TSyntax ``Lean.Parser.Term.doReassign := ⟨stx⟩
    match assignment with
    | `(doReassign| $name:ident $[: $_]? :=%$_ $_rhs:term) =>
        mutations.contains name.getId
    | _ => false
  else if stx.isOfKind ``Move.moveAddAssign ||
      stx.isOfKind ``Move.moveSubAssign ||
      stx.isOfKind ``Move.moveMulAssign ||
      stx.isOfKind ``Move.moveDivAssign ||
      stx.isOfKind ``Move.moveModAssign then
    stx[0]?.any fun target => target.isIdent && mutations.contains target.getId
  else
    false
  assignment || stx.getArgs.any (sourceWritesMutation mutations)

mutual
/-- Translate pure positions that may embed sequenced operations.  The
effectful subterms of `terms` are hoisted into bindings, in evaluation order,
and `build` receives the residual pure terms (rewritten for the active
mutation); the bindings are sequenced in front of what it builds. -/
private partial def withHoisted (context : TranslationContext)
    (terms : Array (TSyntax `term))
    (build : Array (TSyntax `term) → CommandElabM (TSyntax `term)) :
    CommandElabM (TSyntax `term) := do
  let mut bindings : Array (TSyntax `ident × TSyntax `term) := #[]
  let mut residuals : Array (TSyntax `term) := #[]
  for term in terms do
    let (more, residual) ← hoistEffects term.raw bindings.size
    bindings := bindings ++ more
    residuals := residuals.push (← rewritePure (liveMutations context) ⟨residual⟩)
  let mut result ← build residuals
  for (hoisted, operation) in bindings.reverse do
    result ← `(Move.Semantics.Spec.bind $(← expressionSpec context operation)
        (fun $hoisted => $result))
  pure result

/-- The relational semantics of a call to a Move function, and whether the
call passes a mutable reference.  The callee's semantics is its `sourceSpec`
— generated on demand from its retained source when it has no `spec` yet — or
the fixed point's recursive argument for a self-call.  A mutable-reference
argument must be the live reference of an enclosing borrow or the mutable
parameter; the callee receives its current value and — its `sourceSpec`
returning the pair of its result and the final referent — the statement
translating the call writes that final value back into the reference.  This
is the prophecy-passing summary of `verification-design.md`: the caller
suspends its owner with the callee's final value and resumes with it. -/
private partial def moveCallSpec? (context : TranslationContext)
    (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `term × Array (TSyntax `ident) × Bool)) := do
  let (head, arguments, markedContinue) ← match term with
    | `(continue $head:term $arguments:term*) =>
        pure (head, arguments, true)
    | _ =>
        let some (head, arguments) := application? term | return none
        pure (head, arguments, false)
  unless head.raw.isIdent do return none
  let identifier : TSyntax `ident := ⟨head.raw⟩
  let some functionName ← resolveMoveFunction? identifier | return none
  let some callee := (declarations.getState (← getEnv)).find? functionName
    | throwErrorAt term
        "Move callee `{functionName}` has no retained source; declare it with `fun` so its semantics can be generated"
  let returnsMutation := returnsMutableReferences callee
  -- Named arguments instantiate type parameters (`has_generic (T := U64) a`);
  -- the callee's semantics takes them under the same names.
  let isNamed (argument : TSyntax `term) :=
    argument.raw.isOfKind ``Lean.Parser.Term.namedArgument
  let namedArguments := arguments.filter isNamed
  let arguments := arguments.filter (!isNamed ·)
  let mutablePositions ← mutableParameterPositions functionName
  let mut valueNames : Array (TSyntax `term) := #[]
  let mut argumentSpecs : Array (TSyntax `term × TSyntax `ident) := #[]
  let mut passedMutations : Array (TSyntax `ident) := #[]
  for (argument, index) in arguments.zipIdx do
    let valueName := mkIdentFrom argument (Name.mkSimple s!"_moveSpecCallArg{index}")
    valueNames := valueNames.push ⟨valueName.raw⟩
    if mutablePositions.contains index then
      let some mutation := liveMutation? context argument
        | throwErrorAt argument
            "a mutable-reference argument must be a live mutable reference: bind the place with `let r ← &mut …` first"
      if passedMutations.any (·.getId == mutation.getId) then
        throwErrorAt argument
          "a call may pass the live mutable reference `{mutation.getId}` only once"
      passedMutations := passedMutations.push mutation
      let argumentSpec ← if returnsMutation then
        `(Move.Semantics.Spec.pure $mutation)
      else
        `(Move.Semantics.Spec.pure (Move.Semantics.Mutation.read $mutation))
      argumentSpecs := argumentSpecs.push (argumentSpec, valueName)
    else
      argumentSpecs := argumentSpecs.push (← expressionSpec context argument, valueName)
  let packed ← packCallArguments term.raw valueNames
  let recursiveSpec? := match context.recursiveSpecs.find? (·.1 == functionName) with
    | some (_, recursiveSpec) => some recursiveSpec
    | none => if functionName == context.functionName then context.recursiveSpec? else none
  let mut call ← if returnsMutation then do
    if let some recursiveSpec := recursiveSpec? then
      `($recursiveSpec $packed)
    else do
      if markedContinue then
        throwErrorAt term "`continue` must target the current recursive Move function"
      let env ← getEnv
      let mutationSpecName := functionName ++ `mutationSpec
      if Move.moveNativeAttr.hasTag env functionName || Move.moveOpaqueAttr.hasTag env functionName then
        unless env.contains mutationSpecName do
          throwErrorAt term
            "a native or body-less opaque mutable-reference result requires an explicit mutation-level summary `{mutationSpecName}`"
      else
        ensureSourceSpec functionName term
      unless (← getEnv).contains mutationSpecName do
        throwErrorAt term
          "Move callee `{functionName}` has no mutation-level source specification"
      let mutationSpec := mkIdentFrom head mutationSpecName
      `($mutationSpec $namedArguments* $packed)
  else if let some recursiveSpec := recursiveSpec? then
    `($recursiveSpec $packed)
  else do
    if markedContinue then
      throwErrorAt term "`continue` must target the current recursive Move function"
    let env ← getEnv
    if Move.moveNativeAttr.hasTag env functionName || Move.moveOpaqueAttr.hasTag env functionName then
      -- A summarized callee (native, or `pragma opaque`): its contract's
      -- summary, declared by its `spec`.
      let summarySpecName := functionName ++ `summarySpec
      unless env.contains summarySpecName do
        throwErrorAt term
          "Move callee `{functionName}` is summarized by its contract but has no `spec`; declare one in its module"
      let summarySpec := mkIdentFrom head summarySpecName
      `($summarySpec $namedArguments* $packed)
    else
      ensureSourceSpec functionName term
      let sourceSpec := mkIdentFrom head (functionName ++ `sourceSpec)
      `($sourceSpec $namedArguments* $packed)
  for (argumentSpec, valueName) in argumentSpecs.reverse do
    call ← `(Move.Semantics.Spec.bind $argumentSpec fun $valueName => $call)
  return some (call, passedMutations, returnsMutation)

/-- Whether retained source calls a Move function with one of `mutations` in
a mutable-reference parameter position. This is deliberately source-based:
generated owner-reconciliation writes are implementation details of a loan,
whereas a source call can update a disjoint outer prophecy that the loan's
continuation must transport. -/
private partial def sourceCallsMutation (mutations : List Name)
    (stx : Syntax) : CommandElabM Bool := do
  if stx.isOfKind ``Lean.Parser.Term.app && stx.getNumArgs == 2 then
    let term : TSyntax `term := ⟨stx⟩
    if let some (head, rawArguments) := application? term then
      if head.raw.isIdent then
        if let some functionName ← resolveMoveFunction? ⟨head.raw⟩ then
          let arguments := rawArguments.filter fun argument =>
            !argument.raw.isOfKind ``Lean.Parser.Term.namedArgument
          let mutablePositions ← mutableParameterPositions functionName
          for (argument, index) in arguments.zipIdx do
            if mutablePositions.contains index && argument.raw.isIdent &&
                mutations.contains argument.raw.getId then
              return true
  for child in stx.getArgs do
    if ← sourceCallsMutation mutations child then return true
  return false

private partial def sourceMutatesAny (mutations : List Name)
    (elements : Array Lean.DoElem) : CommandElabM Bool := do
  for element in elements do
    if sourceWritesMutation mutations element.raw ||
        (← sourceCallsMutation mutations element.raw) then
      return true
  return false

/-- `T.certify a b …`, the creation of a certified value: `Spec.certified`
builds the value from the proof of its data invariant, owed here. -/
private partial def certifyCall? (context : TranslationContext)
    (term : TSyntax `term) : CommandElabM (Option (TSyntax `term)) := do
  let some (head, arguments) := application? term | return none
  unless head.raw.isIdent do return none
  let some name ← (try pure (some (← resolveGlobalConstNoOverload head.raw))
    catch _ => pure none) | return none
  let .str typeName "certify" := name | return none
  let some invariantName := Move.dataInvariant? (← getEnv) typeName | return none
  return some (← withHoisted context arguments fun residuals => do
    let rawValue ← `($(mkIdent (typeName ++ `Raw ++ `mk)) $residuals*)
    let holds := mkIdentFrom term `_moveSpecInvariant
    let built ← `(fun $holds => $(mkIdent (typeName ++ `mk)) $residuals* $holds)
    `(Move.Semantics.Spec.certified
      (Invariant := $(mkIdent invariantName) $rawValue) $built))

/-- Translate an expression in value position. Arithmetic is sequenced
relationally so overflow, underflow, and division by zero remain observable;
every other sequenced operation embedded in the expression — a cast, a
checked vector access, a Move call — is hoisted in front of it in evaluation
order.  A source conditional or `match` in value position translates branch
by branch. -/
private partial def expressionSpec (context : TranslationContext)
    (term : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  if term.raw.isOfKind ``Move.movePrimitiveMatch then
    let expanded ← liftMacroM <| Move.expandPrimitiveMatchSyntax term.raw
      ⟨term.raw[1]⟩
      (term.raw[3].getArgs.map fun alternative => ⟨alternative⟩)
    let `(let $value:ident := $discriminant:term; $body:term) := expanded
      | throwErrorAt term "failed to expand primitive Move match"
    let discriminantSpec ← expressionSpec context discriminant
    let rec translateBody (body : TSyntax `term) : CommandElabM (TSyntax `term) := do
      match body with
      | `(if $condition:term then $thenBranch:term else $elseBranch:term) =>
          let condition ← rewritePure (liveMutations context) condition
          let thenSpec ← expressionSpec context thenBranch
          let elseSpec ← translateBody elseBranch
          `(if $condition then $thenSpec else $elseSpec)
      | _ => expressionSpec context body
    let bodySpec ← translateBody body
    return ← `(Move.Semantics.Spec.bind $discriminantSpec fun $value => $bodySpec)
  let binary (operation : Name) (lhs rhs : TSyntax `term) := do
    let lhsSpec ← expressionSpec context lhs
    let rhsSpec ← expressionSpec context rhs
    let op := mkIdentFrom term operation
    `(Move.Semantics.Spec.bind $lhsSpec fun _moveSpecLhs =>
        Move.Semantics.Spec.bind $rhsSpec fun _moveSpecRhs =>
          $op _moveSpecLhs _moveSpecRhs)
  -- `*` is both Lean's multiplication token and Move's prefix dereference
  -- token. Before term elaboration, `lhs * rhs` is therefore represented as
  -- a `choice` between the infix parse and application to `*rhs`. Source
  -- verification works on retained pre-elaboration syntax, so select the
  -- ordinary three-child infix alternative explicitly.
  if term.raw.isOfKind `choice then
    if let some multiplication := term.raw.getArgs.find? fun alternative =>
        alternative.getNumArgs == 3 && alternative[1].isAtom &&
          alternative[1].getAtomVal == "*" then
      let lhs : TSyntax `term := ⟨multiplication[0]⟩
      let rhs : TSyntax `term := ⟨multiplication[2]⟩
      return ← binary ``Move.Semantics.Checked.mulSpec lhs rhs
  if let some (operation, lhs, rhs) ← checkedArithmeticCall? term then
    return ← binary operation lhs rhs
  match term with
  | `(($value:term)) => expressionSpec context value
  | `($lhs:term + $rhs:term) => binary ``Move.Semantics.Checked.addSpec lhs rhs
  | `($lhs:term - $rhs:term) => binary ``Move.Semantics.Checked.subSpec lhs rhs
  | `($lhs:term * $rhs:term) => binary ``Move.Semantics.Checked.mulSpec lhs rhs
  | `($lhs:term / $rhs:term) => binary ``Move.Semantics.Checked.divSpec lhs rhs
  | `($lhs:term % $rhs:term) => binary ``Move.Semantics.Checked.modSpec lhs rhs
  | `($lhs:term <<< $rhs:term) =>
      binary ``Move.Semantics.Checked.shlSpec lhs rhs
  | `($lhs:term >>> $rhs:term) =>
      binary ``Move.Semantics.Checked.shrSpec lhs rhs
  | `($lhs:term && $rhs:term) =>
      let lhsSpec ← expressionSpec context lhs
      let rhsSpec ← expressionSpec context rhs
      `(Move.Semantics.Spec.bind ($lhsSpec : Move.Semantics.Spec _ Bool)
          fun (_moveSpecLhs : Bool) =>
          if _moveSpecLhs then $rhsSpec else Move.Semantics.Spec.pure false)
  | `($lhs:term || $rhs:term) =>
      let lhsSpec ← expressionSpec context lhs
      let rhsSpec ← expressionSpec context rhs
      `(Move.Semantics.Spec.bind ($lhsSpec : Move.Semantics.Spec _ Bool)
          fun (_moveSpecLhs : Bool) =>
          if _moveSpecLhs then Move.Semantics.Spec.pure true else $rhsSpec)
  -- A comparison in value position (`let ok := a < b`) is the `Bool` of the
  -- logical comparison the pure rewriting produces.
  | `($_:term < $_:term) | `($_:term <= $_:term) | `($_:term > $_:term)
  | `($_:term >= $_:term) =>
      withHoisted context #[term] fun residuals =>
        ``(Move.Semantics.Spec.pure (decide $(residuals[0]!)))
  | `(($value:term : $type:term)) =>
      -- An ascribed integer cast, Move's `(x as T)`. The ascription
      -- supplies the target width of the checked cast.
      if value.raw.isIdent then
        match value.raw.getId with
        | .str base "cast" =>
            let operand : TSyntax `term :=
              ⟨mkIdentFrom value.raw base⟩
            let operand ← rewritePure (liveMutations context) operand
            ``((Move.Semantics.Checked.castSpec $operand :
                Move.Semantics.Spec _ $type))
        | _ =>
            withHoisted context #[term] fun residuals =>
              ``(Move.Semantics.Spec.pure $(residuals[0]!))
      else
        withHoisted context #[term] fun residuals =>
          ``(Move.Semantics.Spec.pure $(residuals[0]!))
  | `(if $condition:term then $thenBranch:term else $elseBranch:term) =>
      let conditionSpec ← expressionSpec context condition
      let thenSpec ← expressionSpec context thenBranch
      let elseSpec ← expressionSpec context elseBranch
      `(Move.Semantics.Spec.bind $conditionSpec fun _moveSpecCondition =>
          if _moveSpecCondition then $thenSpec else $elseSpec)
  | `(if $binder:ident : $condition:term then $thenBranch:term else $elseBranch:term) =>
      let conditionSpec ← expressionSpec context condition
      let thenSpec ← expressionSpec context thenBranch
      let elseSpec ← expressionSpec context elseBranch
      `(Move.Semantics.Spec.bind $conditionSpec fun _moveSpecCondition =>
          if $binder:ident : _moveSpecCondition then $thenSpec else $elseSpec)
  | `(match $discriminant:term with $alternatives:matchAlt*) =>
      let discriminantSpec ← expressionSpec context discriminant
      let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
      for alternative in alternatives do
        match alternative with
        | `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $rhs:term) =>
            let armSpec ← expressionSpec context rhs
            arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $armSpec))
        | _ => throwErrorAt alternative "unsupported `match` alternative in automatic source specification"
      `(Move.Semantics.Spec.bind $discriminantSpec fun _moveSpecDiscriminant =>
          match _moveSpecDiscriminant with $arms:matchAlt*)
  | _ =>
      if let some call ← globalPrimitiveSpec? (expressionSpec context) context term then
        pure call
      else if let some access ← vectorAccessCall? term then
        match access with
        | .get values index =>
            let valuesSpec ← expressionSpec context values
            let indexSpec ← expressionSpec context index
            `(Move.Semantics.Spec.bind $valuesSpec fun _moveSpecValues =>
                Move.Semantics.Spec.bind $indexSpec fun _moveSpecIndex =>
                  Move.Semantics.Vector.borrowElemSpec _moveSpecValues _moveSpecIndex)
        | .set values index value =>
            let valuesSpec ← expressionSpec context values
            let indexSpec ← expressionSpec context index
            let valueSpec ← expressionSpec context value
            `(Move.Semantics.Spec.bind $valuesSpec fun _moveSpecValues =>
                Move.Semantics.Spec.bind $indexSpec fun _moveSpecIndex =>
                  Move.Semantics.Spec.bind $valueSpec fun _moveSpecElement =>
                  Move.Semantics.Vector.setSpec _moveSpecValues _moveSpecIndex
                      _moveSpecElement)
      else if let some values ← vectorDestroyArgument? term then
        let valuesSpec ← expressionSpec context values
        `(Move.Semantics.Spec.bind $valuesSpec fun _moveSpecValues =>
            Move.Semantics.Vector.destroyEmptySpec _moveSpecValues)
      else if let some creation ← certifyCall? context term then
        pure creation
      else if let some (call, passedMutations, returnsMutation) ← moveCallSpec? context term then
        if returnsMutation then
          throwErrorAt term
            "a call returning a mutable reference must be bound with `let`"
        if !passedMutations.isEmpty then
          throwErrorAt term
            "a call passing a mutable reference must be a `do` statement or bound with `let`"
        pure call
      else
        withHoisted context #[term] fun residuals =>
          `(Move.Semantics.Spec.pure $(residuals[0]!))

/-- Resolve every mutable component of a returned multiple-value and erase its
reference representation for the value-level source relation. -/
private partial def resolveMutationResult (anchor : Syntax)
    (leaves : Array ResultLeaf) (packed : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let rec go (index : Nat) (values : Array (TSyntax `term)) :
      CommandElabM (TSyntax `term) := do
    if index == leaves.size then
      return ← `(Move.Semantics.Spec.pure $(← packCallArguments anchor values))
    let component ← liftMacroM <| argumentProjection packed index leaves.size
    match leaves[index]! with
    | .mutable _ =>
        let returned := mkIdentFrom anchor
          (Name.mkSimple s!"_moveSpecReturnedMutation{index}")
        let returnedValue := mkIdentFrom anchor
          (Name.mkSimple s!"_moveSpecReturnedValue{index}")
        let transferred ← `(Move.Semantics.withTransferredMutation $component
          (fun $returned => Move.Semantics.Spec.pure
            (Move.Semantics.Mutation.read $returned, $returned)))
        let tail ← go (index + 1) (values.push ⟨returnedValue.raw⟩)
        `(Move.Semantics.Spec.bind $transferred (fun $returnedValue => $tail))
    | .value _ | .immutable _ =>
        go (index + 1) (values.push component)
  go 0 #[]

/-- Define `f.sourceSpec` — and `f.bodySpec` for a recursive `f` — from the
retained body of `function`, a short name in the current namespace, with
the source signature stating its generic context and logical parameter types.
The semantics is state-polymorphic: it quantifies over an abstract state and
one typed store per resource family the body (transitively) touches. -/
private partial def generateSourceSpec (function : TSyntax `ident)
    (signature : SourceSignature) : CommandElabM Unit := do
  let declaration ← declarationFor function.raw
  let recursive ← isRecursive function.raw
  if recursive then
    stabilizeBorrowSummaries #[(function, signature)] function.raw
  let borrowProgram ← buildBorrowProgram function signature
  emitBorrowCertificate function borrowProgram
  let world := mkIdentFrom function `_moveSpecState
  let resourceTypes ← inferredResources function.raw
  let sourceSpecName := mkIdentFrom function (function.getId ++ `sourceSpec)
  let bodySpecName := mkIdentFrom function (function.getId ++ `bodySpec)
  let argsType ← liftMacroM <| argumentType signature.types
  let resultType ← resultTypeOf function.raw
  let sourceResultType ← liftMacroM <|
    sourceResultType resultType signature.mutableParameters
  let recursiveName := mkIdentFrom function `_moveSpecRecursive
  let recursiveTerm : TSyntax `term := ⟨recursiveName.raw⟩
  let worldBinder ← `(bracketedBinder| {$world : Type})
  let mut storeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  for (head, index) in (distinctHeads resourceTypes).zipIdx do
    let storeName := mkIdentFrom function
      (Name.mkSimple s!"_moveSpecStore{index}")
    let storeBinder ← `(bracketedBinder|
      [$storeName : $(← storeType ⟨world⟩ head)])
    storeBinders := storeBinders.push storeBinder

  if returnsMutableReferences declaration then
    if signature.mutableParameters.isEmpty then
      throwErrorAt function
        "a mutable-reference result requires a mutable-reference parameter"
    let mutationSpecName := mkIdentFrom function (function.getId ++ `mutationSpec)
    let mutationTypes ← liftMacroM <| mutationArgumentTypes signature
    let mutationArgsType ← liftMacroM <| argumentType mutationTypes
    let (mutationBody, mutationResultType) ← translateWithStores function.raw world
      resourceTypes (if recursive then some recursiveTerm else none)
      signature.mutableParameters #[] true
    let mutationLambda ← liftMacroM <|
      unpackArguments signature.arguments mutationBody
    if recursive then
      let recursiveBinder ← `(bracketedBinder|
        ($recursiveName : $mutationArgsType →
          Move.Semantics.Spec $world $mutationResultType))
      elabCommand (← `(noncomputable def $bodySpecName $signature.context* $worldBinder
        $storeBinders* $recursiveBinder : $mutationArgsType →
          Move.Semantics.Spec $world $mutationResultType := $mutationLambda))
      elabCommand (← `(noncomputable def $mutationSpecName $signature.context* $worldBinder
        $storeBinders* : $mutationArgsType →
          Move.Semantics.Spec $world $mutationResultType :=
          Move.Semantics.Spec.fix $bodySpecName))
    else
      elabCommand (← `(noncomputable def $mutationSpecName $signature.context* $worldBinder
        $storeBinders* : $mutationArgsType →
          Move.Semantics.Spec $world $mutationResultType := $mutationLambda))

    let packed ← packCallArguments function.raw
      (signature.arguments.map fun argument => (⟨argument.raw⟩ : TSyntax `term))
    let mutationCall ← `($mutationSpecName $packed)
    let mutationOutput := mkIdentFrom function `_moveSpecMutationOutput
    let returnedValue := mkIdentFrom function `_moveSpecReturnedValue
    let transferred ← resolveMutationResult function.raw
      (resultLeaves (declaredResultType declaration)) (← `($mutationOutput.1))
    let opened ← `(Move.Semantics.Spec.bind $mutationCall
      (fun $mutationOutput =>
        Move.Semantics.Spec.bind $transferred (fun $returnedValue =>
          Move.Semantics.Spec.pure ($returnedValue, $mutationOutput.2))))
    let wrapper ← withMutableParameters signature.mutableParameters opened
    let sourceLambda ← liftMacroM <| unpackArguments signature.arguments wrapper
    elabCommand (← `(noncomputable def $sourceSpecName $signature.context* $worldBinder
      $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
      $sourceLambda))
    return

  let (body, _) ← translateWithStores function.raw world resourceTypes
    (if recursive then some recursiveTerm else none)
    signature.mutableParameters
  let sourceLambda ← liftMacroM <| unpackArguments signature.arguments body
  if recursive then
    let recursiveBinder ← `(bracketedBinder|
      ($recursiveName : $argsType → Move.Semantics.Spec $world $sourceResultType))
    let bodyCommand ← `(noncomputable def $bodySpecName $signature.context* $worldBinder
        $storeBinders* $recursiveBinder :
        $argsType → Move.Semantics.Spec $world $sourceResultType := $sourceLambda)
    elabCommand bodyCommand
    let sourceCommand ← `(noncomputable def $sourceSpecName $signature.context* $worldBinder
        $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
        Move.Semantics.Spec.fix $bodySpecName)
    elabCommand sourceCommand
  else
    let sourceCommand ← `(noncomputable def $sourceSpecName $signature.context* $worldBinder
        $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
        $sourceLambda)
    elabCommand sourceCommand

private partial def calleesOf (functionName : Name) : CommandElabM (Array Name) := do
  let some declaration := declarations.getState (← getEnv) |>.find? functionName
    | return #[]
  let body ← sourceBody declaration
  let namespace_ := functionName.getPrefix
  withScope (fun scope => { scope with currNamespace := namespace_ }) do
    collectCallees body.raw

private partial def reachableFrom (start : Name) : CommandElabM (Array Name) := do
  let rec visit (pending visited : Array Name) : CommandElabM (Array Name) := do
    let some current := pending[0]? | return visited
    let pending := pending.extract 1 pending.size
    if visited.contains current then return ← visit pending visited
    let visited := visited.push current
    let callees ← calleesOf current
    visit (pending ++ callees) visited
  visit #[start] #[]

/-- The strongly connected component containing `functionName`, computed
from retained Move call syntax. -/
private partial def recursiveComponent (functionName : Name) : CommandElabM (Array Name) := do
  let forward ← reachableFrom functionName
  let mut component := #[]
  for candidate in forward do
    if (← reachableFrom candidate).contains functionName then
      component := component.push candidate
  pure component

/-- Generate one dependent `Spec.fixFamily` for a mutually recursive SCC.
Reference-returning members occupy mutation-level entries in the family and
receive both a raw `f.mutationSpec` projection and a value-level `f.sourceSpec`
wrapper. Ordinary members remain direct `f.sourceSpec` projections. -/
private partial def generateMutualSourceSpecs (members : Array Name)
    (ref : Syntax) : CommandElabM Unit := do
  let some anchor := members[0]? | return
  let namespace_ := anchor.getPrefix
  unless members.all (·.getPrefix == namespace_) do
    throwErrorAt ref "mutually recursive Move functions must belong to one module"
  withScope (fun scope => { scope with currNamespace := namespace_ }) do
    let mut memberDeclarations : Array Declaration := #[]
    for member in members do
      let some declaration := (declarations.getState (← getEnv)).find? member
        | throwErrorAt ref "no retained Move source declaration for `{member}`"
      memberDeclarations := memberDeclarations.push declaration
    let mutationMembers := memberDeclarations.map returnsMutableReferences
    let mut signatures : Array SourceSignature := #[]
    for member in members do signatures := signatures.push (← signatureOf member)
    let summaryMembers := (members.zip signatures).map fun (member, signature) =>
      (mkIdentFrom ref (Name.mkSimple member.getString!), signature)
    stabilizeBorrowSummaries summaryMembers ref
    for (member, signature) in members.zip signatures do
      let short := mkIdentFrom ref (Name.mkSimple member.getString!)
      emitBorrowCertificate short (← buildBorrowProgram short signature)
    let some firstSignature := signatures[0]? | return
    let commonContext := firstSignature.context
    unless signatures.all (·.context.map (·.raw) == commonContext.map (·.raw)) do
      throwErrorAt ref
        "mutually recursive generic functions must use the same type context"
    let anchorShort := anchor.getString!
    let indexIdent := mkIdentFrom ref (Name.mkSimple s!"{anchorShort}MutualIndex")
    let argsIdent := mkIdentFrom ref (Name.mkSimple s!"{anchorShort}MutualArgs")
    let resultIdent := mkIdentFrom ref (Name.mkSimple s!"{anchorShort}MutualResult")
    let bodyIdent := mkIdentFrom ref (Name.mkSimple s!"{anchorShort}MutualBody")
    let sourceIdent := mkIdentFrom ref (Name.mkSimple s!"{anchorShort}MutualSourceSpec")
    let constructors := members.mapIdx fun index _ =>
      mkIdentFrom ref (Name.mkSimple s!"member{index}")
    let ctorDecls ← constructors.mapM fun constructor =>
      `(Parser.Command.ctor| | $constructor:ident)
    elabCommand (← `(inductive $indexIdent where $ctorDecls*))
    let constructorTerms := constructors.map fun constructor =>
      mkIdentFrom ref (indexIdent.getId ++ constructor.getId)
    let mut argumentTypes : Array (TSyntax `term) := #[]
    let mut resultTypes : Array (TSyntax `term) := #[]
    for (((signature, member), declaration), mutationMember) in
        signatures.zip members |>.zip memberDeclarations |>.zip mutationMembers do
      let argumentTypesForMember ← if mutationMember then
        liftMacroM <| mutationArgumentTypes signature
      else
        pure signature.types
      let memberArgumentType ← liftMacroM <| argumentType argumentTypesForMember
      let short := mkIdentFrom ref (Name.mkSimple member.getString!)
      let logicalResult ← resultTypeOf short.raw
      let memberResultType ← if mutationMember then
        liftMacroM <| mutationResultType (declaredResultType declaration)
          logicalResult signature.mutableParameters
      else
        liftMacroM <| sourceResultType logicalResult signature.mutableParameters
      argumentTypes := argumentTypes.push memberArgumentType
      resultTypes := resultTypes.push memberResultType
    let recursor := mkIdentFrom ref (indexIdent.getId ++ `rec)
    let indexArg := mkIdentFrom ref `_moveSpecMutualIndex
    let typeMotive ← `(fun (_ : $indexIdent) => Type)
    let mut argumentFamily ← `(@$recursor:ident $typeMotive)
    let mut resultFamily ← `(@$recursor:ident $typeMotive)
    for argumentType in argumentTypes do argumentFamily ← `($argumentFamily $argumentType)
    for resultType in resultTypes do resultFamily ← `($resultFamily $resultType)
    argumentFamily ← `($argumentFamily $indexArg)
    resultFamily ← `($resultFamily $indexArg)
    elabCommand (← `(def $argsIdent $commonContext* ($indexArg : $indexIdent) : Type :=
      $argumentFamily))
    elabCommand (← `(def $resultIdent $commonContext* ($indexArg : $indexIdent) : Type :=
      $resultFamily))
    let world := mkIdentFrom ref `_moveSpecState
    let resourceTypes ← inferredResources (mkIdentFrom ref
      (Name.mkSimple anchor.getString!)).raw
    let worldBinder ← `(bracketedBinder| {$world : Type})
    let mut storeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
    for (head, index) in (distinctHeads resourceTypes).zipIdx do
      let storeName := mkIdentFrom ref (Name.mkSimple s!"_moveSpecStore{index}")
      storeBinders := storeBinders.push (← `(bracketedBinder|
        [$storeName : $(← storeType ⟨world⟩ head)]))
    let recursive := mkIdentFrom ref `_moveSpecMutualRecursive
    let familyType ← `(@Move.Semantics.Spec.Family $world $indexIdent
      $argsIdent $resultIdent)
    let mut recursiveSpecs : Array (Name × TSyntax `term) := #[]
    for ((member, constructor), argumentType) in
        members.zip constructorTerms |>.zip argumentTypes do
      let recursiveArgs := mkIdentFrom ref `_moveSpecMutualArgs
      let entry ← `(fun ($recursiveArgs : $argumentType) =>
        $recursive $constructor $recursiveArgs)
      recursiveSpecs := recursiveSpecs.push (member, entry)
    let bodyArg := mkIdentFrom ref `_moveSpecMutualBodyArgs
    let mut bodyBranches : Array (TSyntax `term) := #[]
    for ((((member, signature), argumentType), resultType), mutationMember) in
        members.zip signatures |>.zip argumentTypes |>.zip resultTypes |>.zip mutationMembers do
      let short := mkIdentFrom ref (Name.mkSimple member.getString!)
      let (translated, _) ← translateWithStores short.raw world resourceTypes none
        signature.mutableParameters recursiveSpecs mutationMember
      let sourceLambda ← liftMacroM <| unpackArguments signature.arguments translated
      let branch ← `(($sourceLambda : $argumentType →
        Move.Semantics.Spec $world $resultType))
      bodyBranches := bodyBranches.push branch
    let bodyIndex := mkIdentFrom ref `_moveSpecMutualBodyIndex
    let bodyMotive ← `(fun ($bodyIndex : $indexIdent) =>
      $argsIdent $bodyIndex → Move.Semantics.Spec $world ($resultIdent $bodyIndex))
    let mut bodyFamily ← `(@$recursor:ident $bodyMotive)
    for branch in bodyBranches do bodyFamily ← `($bodyFamily $branch)
    bodyFamily ← `($bodyFamily $bodyIndex)
    bodyFamily ← `($bodyFamily $bodyArg)
    let recursiveBinder ← `(bracketedBinder| ($recursive : $familyType))
    elabCommand (← `(noncomputable def $bodyIdent $commonContext* $worldBinder
      $storeBinders* $recursiveBinder : $familyType :=
      fun $bodyIndex $bodyArg => $bodyFamily))
    elabCommand (← `(noncomputable def $sourceIdent $commonContext* $worldBinder
      $storeBinders* : $familyType := Move.Semantics.Spec.fixFamily $bodyIdent))
    for ((((((member, signature), declaration), constructor), familyArgumentType),
        familyResultType), mutationMember) in
        members.zip signatures |>.zip memberDeclarations |>.zip constructorTerms
          |>.zip argumentTypes |>.zip resultTypes |>.zip mutationMembers do
      let memberArgs := mkIdentFrom ref `_moveSpecArgs
      if mutationMember then
        let memberMutation := mkIdentFrom ref
          (Name.mkSimple member.getString! ++ `mutationSpec)
        elabCommand (← `(noncomputable def $memberMutation $signature.context*
          $worldBinder $storeBinders* : $familyArgumentType →
            Move.Semantics.Spec $world $familyResultType :=
          fun $memberArgs => $sourceIdent $constructor $memberArgs))

        let memberSource := mkIdentFrom ref
          (Name.mkSimple member.getString! ++ `sourceSpec)
        let sourceArgumentType ← liftMacroM <| argumentType signature.types
        let logicalResult ← resultTypeOf
          (mkIdentFrom ref (Name.mkSimple member.getString!)).raw
        let sourceMemberResultType ← liftMacroM <|
          sourceResultType logicalResult signature.mutableParameters
        let packed ← packCallArguments ref
          (signature.arguments.map fun argument =>
            (⟨argument.raw⟩ : TSyntax `term))
        let mutationCall ← `($memberMutation $packed)
        let mutationOutput := mkIdentFrom ref `_moveSpecMutationOutput
        let returnedValue := mkIdentFrom ref `_moveSpecReturnedValue
        let transferred ← resolveMutationResult ref
          (resultLeaves (declaredResultType declaration))
          (← `($mutationOutput.1))
        let opened ← `(Move.Semantics.Spec.bind $mutationCall
          (fun $mutationOutput =>
            Move.Semantics.Spec.bind $transferred (fun $returnedValue =>
              Move.Semantics.Spec.pure ($returnedValue, $mutationOutput.2))))
        let wrapper ← withMutableParameters signature.mutableParameters opened
        let sourceLambda ← liftMacroM <|
          unpackArguments signature.arguments wrapper
        elabCommand (← `(noncomputable def $memberSource $signature.context*
          $worldBinder $storeBinders* : $sourceArgumentType →
            Move.Semantics.Spec $world $sourceMemberResultType := $sourceLambda))
      else
        let memberSource := mkIdentFrom ref
          (Name.mkSimple member.getString! ++ `sourceSpec)
        elabCommand (← `(noncomputable def $memberSource $signature.context*
          $worldBinder $storeBinders* : $familyArgumentType →
            Move.Semantics.Spec $world $familyResultType :=
          fun $memberArgs => $sourceIdent $constructor $memberArgs))
    let info : MutualFamilyInfo := {
      anchor
      indexType := namespace_ ++ indexIdent.getId
      argsFamily := namespace_ ++ argsIdent.getId
      resultFamily := namespace_ ++ resultIdent.getId
      body := namespace_ ++ bodyIdent.getId
      source := namespace_ ++ sourceIdent.getId
      members
      constructors := constructorTerms.map fun constructor =>
        namespace_ ++ constructor.getId
      hasMutationMembers := mutationMembers.any id }
    modifyEnv fun env => members.foldl (fun env member =>
      mutualFamilies.addEntry env (member, info)) env
/-- Make sure a Move callee has its relational semantics `f.sourceSpec`,
generating it from the retained source and the elaborated signature when no
`spec` has.  Generation happens in the callee's namespace, so the body's
names resolve as they did at its declaration, and only for callees of the
current module: an imported module's functions get theirs from their own
`spec`, where the declaration belongs. -/
private partial def ensureSourceSpec (functionName : Name) (ref : Syntax) :
    CommandElabM Unit := do
  let env ← getEnv
  if env.contains (functionName ++ `sourceSpec) then return
  unless (declarations.getState env).contains functionName do
    throwErrorAt ref
      "Move callee `{functionName}` has no retained source; declare it with `fun` so its semantics can be generated"
  if (env.getModuleIdxFor? functionName).isSome then
    throwErrorAt ref
      "imported Move callee `{functionName}` has no source specification; declare its `spec` in its module"
  let component ← recursiveComponent functionName
  if component.size > 1 then
    if component.any (← generationInProgress.get).contains then
      throwErrorAt ref
        "recursive source-specification generation re-entered mutual component containing `{functionName}`"
    generationInProgress.modify fun active =>
      component.foldl (fun active member => active.insert member) active
    try
      generateMutualSourceSpecs component ref
    finally
      generationInProgress.modify fun active =>
        component.foldl (fun active member => active.erase member) active
    return
  if (← generationInProgress.get).contains functionName then
    throwErrorAt ref
      "recursive source-specification generation re-entered `{functionName}`"
  generationInProgress.modify (·.insert functionName)
  try
    let signature ← signatureOf functionName
    let .str namespace_ shortName := functionName
      | throwErrorAt ref "cannot generate a source specification for `{functionName}`"
    let function := mkIdentFrom ref (Name.mkSimple shortName)
    withScope (fun scope => { scope with currNamespace := namespace_ }) do
      generateSourceSpec function signature
  finally
    generationInProgress.modify (·.erase functionName)

private partial def translate (function world : Syntax) (resources : Array ResourceBinding)
    (recursiveSpec? : Option (TSyntax `term) := none)
    (mutableParameters : Array (TSyntax `ident × TSyntax `term) := #[])
    (recursiveSpecs : Array (Name × TSyntax `term) := #[])
    (mutationBoundary : Bool := false) :
    CommandElabM (TSyntax `term × TSyntax `term) := do
  let declaration ← declarationFor function
  let resultType ← actionResultType declaration
  let declaredResult := declaredResultType declaration
  let resultLeaves := resultLeaves declaredResult
  if returnsMutableReferences declaration && !mutationBoundary then
    throwErrorAt function
      "automatic source specifications do not yet model a function returning a mutable reference"
  if mutationBoundary && !returnsMutableReferences declaration then
    throwErrorAt function
      "internal error: mutation-level result generation requires a mutable-reference result"
  let body ← sourceBody declaration
  let functionName := (← getCurrNamespace) ++ function.getId
  let parameters := (← signatureOf functionName).arguments.toList.map (·.getId)
  let mutationTypes ← mutableParameters.mapM fun (_, referent) =>
    referentTypeName? referent
  let mutation? := mutableParameters[0]?.map (·.1)
  let mutationType? := mutationTypes[0]?.join
  let mutationRefs := mutableParameters.extract 1 mutableParameters.size |>.map (·.1) |>.toList
  let mutationAncestors := (mutableParameters.extract 1 mutableParameters.size).zip
    (mutationTypes.extract 1 mutationTypes.size) |>.map
      (fun ((parameter, _), type?) => (parameter, type?)) |>.toList
  let spec ← translateTerm {
    world := ⟨world⟩
    resources
    functionName
    parameters
    recursiveSpec?
    recursiveSpecs
    mutation?
    mutationType?
    mutationAncestors
    mutationRefs
    rootMutations := mutableParameters.map (·.1)
    returnsMutation := mutationBoundary
    resultLeaves
  } body
  if mutationBoundary then
    return (spec, ← liftMacroM <|
      mutationResultType declaredResult resultType mutableParameters)
  if mutableParameters.isEmpty then
    pure (spec, resultType)
  else
    let wrapped ← withMutableParameters mutableParameters spec
    pure (wrapped, ← liftMacroM <| sourceResultType resultType mutableParameters)

/-- Translate against the abstract compositional resource-store interface. -/
private partial def translateWithStores (function : Syntax) (world : TSyntax `ident)
    (families : Array Family)
    (recursiveSpec? : Option (TSyntax `term) := none)
    (mutableParameters : Array (TSyntax `ident × TSyntax `term) := #[])
    (recursiveSpecs : Array (Name × TSyntax `term) := #[])
    (mutationBoundary : Bool := false) :
    CommandElabM (TSyntax `term × TSyntax `term) := do
  let mut resources : Array ResourceBinding := #[]
  for head in distinctHeads families do
    resources := resources.push {
      head
      descriptorFor := fun family => `(Move.Semantics.ResourceStore.descriptor
        (State := $world) (Value := $(family.term))) }
  translate function world.raw resources recursiveSpec? mutableParameters recursiveSpecs
    mutationBoundary

/-- A mutable borrow of a global resource at `key`, focused through `fields`
— none for the whole resource.  The resource is checked out by ownership for
the loan's lifetime, the focus is a prophecy mutation, and the reconciled
focus is written back when the loan dies; a certified resource is re-created
there, which is where its data invariant is owed.  The family's global
invariants are re-certified at the write. -/
private partial def globalMutableBorrow (context : TranslationContext)
    (name : TSyntax `ident) (place : Syntax) (family : Family)
    (key : TSyntax `term) (fields : Array (TSyntax `ident))
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  let (loanBody, continuation) ← mutableBorrowScope name.getId rest
  let descriptor ← resourceFor context.resources family
  let resourceName := family.head
  let referentType? ← pathTypeName? (some resourceName)
    (fields.toList.map (·.getId))
  let outer := liveMutations context
  let delaysControlExit := loanBody.any fun element =>
    containsLoanControlExit element.raw
  let normalValue := mkIdentFrom name `_moveSpecLoanValue
  let normal ← afterLoan context continuation ⟨normalValue.raw⟩
  let loanExit := mkLoanExitFrame context #[] normalValue normal
  let translateNested (deferContinuation : Bool) := translateDo
    { context with
        mutation? := some name, mutationType? := referentType?
        mutationRefs := outer
        loops := if deferContinuation then context.loops else []
        loanScope? := some name
        loanExits := if deferContinuation then loanExit :: context.loanExits else [] }
    loanBody
  let deferContinuation := delaysControlExit || !context.loanExits.isEmpty ||
    (← sourceMutatesAny (outer.map (·.getId)) loanBody)
  let nested ← translateNested deferContinuation
  let owner := mkIdentFrom place `_moveSpecOwner
  let replacement := mkIdentFrom place `_moveSpecReplacement
  let ownerTerm : TSyntax `term := ⟨owner.raw⟩
  let replacementTerm : TSyntax `term := ⟨replacement.raw⟩
  let focused ← projectPath ownerTerm fields
  let certified? := (Move.dataInvariant? (← getEnv) resourceName).map
    (resourceName, ·)
  let borrow ← match ← rebuildOwner ownerTerm replacementTerm fields.toList
      certified? with
    | some creation =>
        -- A certified resource is re-created when the loan dies: its
        -- data invariant is owed there, and the stored value stays
        -- certified.
        let output := mkIdentFrom place `_moveSpecFocusOutput
        let rebuilt := mkIdentFrom place `_moveSpecRebuilt
        let loan ← `(Move.Semantics.Spec.bind
                (Move.Semantics.withMutation $focused (fun $name => $nested))
                (fun $output =>
                  let $replacement := $output.2
                  Move.Semantics.Spec.bind $creation
                    (fun $rebuilt =>
                      Move.Semantics.Spec.pure ($output.1, $rebuilt))))
        let guarded ← if fields.isEmpty then pure loan
          else `(guardPath% $ownerTerm [$fields,*] $loan)
        `(Move.Semantics.Resource.withBorrowMutSpec $descriptor $key
            (fun $owner => $guarded))
    | none =>
        if ← pathCrossesEnum (some resourceName) (fields.toList.map (·.getId)) then
          -- Enum payload selection is partial, so guard it and rebuild the
          -- complete owner before restoring the resource.
          let focus ← projectPath ownerTerm fields
          let updated ← updatePath ownerTerm replacementTerm fields.toList
          let output := mkIdentFrom place `_moveSpecFocusOutput
          let loan ← `(Move.Semantics.Spec.bind
            (Move.Semantics.withMutation $focus (fun $name => $nested))
            (fun $output =>
              let $replacement := $output.2
              Move.Semantics.Spec.pure ($output.1, $updated)))
          let guarded ← `(guardPath% $ownerTerm [$fields,*] $loan)
          `(Move.Semantics.Resource.withBorrowMutSpec $descriptor $key
              (fun $owner => $guarded))
        else
          let focus ← if fields.isEmpty then pure ownerTerm
            else `(focusPath% $ownerTerm [$fields,*])
          let updated ← updatePath ownerTerm replacementTerm fields.toList
          `(Move.Semantics.Resource.withBorrowMutFocusSpec $descriptor $key
              (fun $owner => $focus)
              (fun $owner $replacement => $updated)
              (fun $name => $nested))
  -- Global invariants re-certify the store at this write: an `update`
  -- invariant wraps the write (relating pre/post state); a regular
  -- invariant is asserted immediately after it.
  let invariants := Move.globalInvariants (← getEnv) resourceName
  let mut borrow := borrow
  for (isUpdate, body, _) in invariants do
    if isUpdate then
      let bodyId := mkIdentFrom place body
      borrow ← `(Move.Semantics.Spec.certifyUpdate $bodyId $borrow)
  let regulars := invariants.filterMap fun (u, b, _) => if u then none else some b
  let assertGlobal (cont : TSyntax `term) : CommandElabM (TSyntax `term) := do
    let mut tail := cont
    for body in regulars.reverse do
      let bodyId := mkIdentFrom place body
      tail ← `(Move.Semantics.Spec.bind
        (Move.Semantics.Spec.certifyState $bodyId)
        (fun _moveSpecCertify => $tail))
    pure tail
  if deferContinuation then
    let tail ← assertGlobal (← `(_moveSpecBorrowResult))
    `(Move.Semantics.Spec.bind $borrow
      (fun _moveSpecBorrowResult => $tail))
  else if continuation.isEmpty && context.loops.isEmpty then
    if regulars.isEmpty then pure borrow
    else
      let tail ← assertGlobal
        (← `(Move.Semantics.Spec.pure _moveSpecBorrowResult))
      `(Move.Semantics.Spec.bind $borrow
          (fun _moveSpecBorrowResult => $tail))
  else
    -- Inside a loop, an empty continuation carries the iteration on.
    let after ← if continuation.isEmpty then emptyFinish context
      else translateDo context continuation
    let tail ← assertGlobal after
    `(Move.Semantics.Spec.bind $borrow (fun _moveSpecBorrowResult => $tail))

/-- An immutable borrow of a global resource, focused through `fields`: the
observed value itself, after immutable-reference erasure. -/
private partial def globalImmutableBorrow (context : TranslationContext)
    (name : TSyntax `ident) (place : Syntax) (family : Family)
    (key : TSyntax `term) (fields : Array (TSyntax `ident))
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  let nested ← if rest.isEmpty then emptyFinish context else translateDo context rest
  let descriptor ← resourceFor context.resources family
  let owner := mkIdentFrom place `_moveSpecOwner
  let ownerTerm : TSyntax `term := ⟨owner.raw⟩
  let body ← if fields.isEmpty then `(let $name := $ownerTerm; $nested)
    else `(bindSelectPath% $ownerTerm [$fields,*] $name => $nested)
  `(Move.Semantics.Spec.bind
      (Move.Semantics.Resource.borrowSpec $descriptor $key)
      (fun $owner => $body))

/-- A mutable borrow of an element of a local vector, or of the active vector
mutation, focused through `fields` — none for the element itself.  The
element is checked out through the checked `withBorrowElemMutSpec`; a field
path focuses it through a nested prophecy mutation whose reconciled value is
written back into the element before the element is written back into the
vector. -/
private partial def elementMutableBorrow (context : TranslationContext)
    (name : TSyntax `ident) (place : Syntax) (vector : TSyntax `ident)
    (index : TSyntax `term) (fields : Array (TSyntax `ident))
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  let ownerIsMutation := context.mutation?.any (·.getId == vector.getId)
  let (loanBody, continuation) ← mutableBorrowScope name.getId rest
  withHoisted context #[index] fun residuals => do
  let index := residuals[0]!
  if context.returnsMutation && ownerIsMutation && continuation.isEmpty &&
      isMutationResultComponent context name.getId loanBody then
    let current ← `(Move.Semantics.Mutation.read $vector)
    let element := mkIdentFrom place `_moveSpecReturnedElement
    let elementTerm : TSyntax `term := ⟨element.raw⟩
    let focused ← projectPath elementTerm fields
    let transfer := mkIdentFrom place `_moveSpecReturnedElementFocus
    let transferTerm : TSyntax `term := ⟨transfer.raw⟩
    let loanNormal ← transferredLoanNormal context #[name]
    let loanExit := mkTransferredLoanExitFrame context name loanNormal
    let nested ← translateDo
      { context with
          mutation? := some name
          mutationType? := none
          mutationAncestors :=
            (vector, context.mutationType?) :: context.mutationAncestors
          mutationOwnerAliases :=
            (vector.getId, name, none) :: context.mutationOwnerAliases
          mutationRefs := context.mutation?.toList ++ context.mutationRefs
          transferredReturns := context.transferredReturns.push name
          loanExits := loanExit :: context.loanExits
          loanScope? := some name }
      loanBody
    let updatedElement ← updatePath elementTerm (← `($transferTerm.2)) fields.toList
    let updatedVector ← `(Move.Vector.set $current $index $updatedElement)
    let updatedMutation ← `(Move.Semantics.Mutation.write $vector $updatedVector)
    let reborrow ← `(Move.Semantics.Spec.bind
      (Move.Semantics.reborrowMutation $focused)
      (fun $transfer =>
        let $name := $transferTerm.1
        let $vector := $updatedMutation
        $nested))
    let reborrow ← if fields.isEmpty then pure reborrow
      else `(guardPath% $elementTerm [$fields,*] $reborrow)
    return ← `(Move.Semantics.Spec.bind
      (Move.Semantics.Vector.borrowElemSpec $current $index)
      (fun $element => $reborrow))
  let outer := liveMutations context
  let delaysControlExit := loanBody.any fun element =>
    containsLoanControlExit element.raw
  let normalValue := mkIdentFrom name `_moveSpecLoanValue
  let normal ← afterLoan context continuation ⟨normalValue.raw⟩
  let loanExit := mkLoanExitFrame context #[vector] normalValue normal
  let translateNested (deferContinuation : Bool) := translateDo
    { context with
        mutation? := some name, mutationType? := none
        mutationRefs := if ownerIsMutation then context.mutationRefs else outer
        loops := if deferContinuation then context.loops else []
        loanScope? := some name
        loanExits := if deferContinuation then loanExit :: context.loanExits else [] }
    loanBody
  let deferContinuation := delaysControlExit || !context.loanExits.isEmpty ||
    (← sourceMutatesAny (outer.map (·.getId)) loanBody)
  let nested ← translateNested deferContinuation
  let body ← if fields.isEmpty then
      `(fun $name => $nested)
    else
      let element := mkIdentFrom place `_moveSpecElement
      let elementValue ← `(Move.Semantics.Mutation.read $element)
      let focused ← projectPath elementValue fields
      let fieldOutput := mkIdentFrom place `_moveSpecFieldOutput
      let fieldOutputTerm : TSyntax `term := ⟨fieldOutput.raw⟩
      let updated ← updatePath elementValue (← `($fieldOutputTerm.2)) fields.toList
      `(fun $element =>
          guardPath% $elementValue [$fields,*]
            (Move.Semantics.Spec.bind
              (Move.Semantics.withMutation $focused (fun $name => $nested))
              (fun $fieldOutput =>
                Move.Semantics.Spec.pure
                  ($fieldOutputTerm.1,
                    Move.Semantics.Mutation.write $element $updated))))
  let output := mkIdentFrom place `_moveSpecVectorOutput
  let outputTerm : TSyntax `term := ⟨output.raw⟩
  if ownerIsMutation then
    -- The vector is the active mutation: its current value is checked out
    -- and the updated vector is written back into it.
    let current ← `(Move.Semantics.Mutation.read $vector)
    let borrow ← `(Move.Semantics.Vector.withBorrowElemMutSpec $current $index $body)
    let updated ← `(Move.Semantics.Mutation.write $vector $outputTerm.2)
    let after ← if deferContinuation then
      `($outputTerm.1 $vector)
    else
      afterLoan context continuation (← `($outputTerm.1))
    `(Move.Semantics.Spec.bind $borrow (fun $output =>
        let $vector := $updated
        $after))
  else
    let borrow ← `(Move.Semantics.Vector.withBorrowElemMutSpec $vector $index $body)
    let after ← if deferContinuation then
      `($outputTerm.1 $outputTerm.2)
    else if continuation.isEmpty && context.loops.isEmpty then
      finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
    else
      -- The owner is refreshed before the continuation -- or, inside a
      -- loop, before the iteration carries it on.
      let continuationSpec ← if continuation.isEmpty then emptyFinish context
        else translateDo context continuation
      `(let $vector := $outputTerm.2; $continuationSpec)
    `(Move.Semantics.Spec.bind $borrow (fun $output => $after))

/-- The relational semantics of a call that passes the live mutable
reference, when `term` is one; the caller resumes the reference with the
callee's final referent. -/
private partial def mutableCallSpec? (context : TranslationContext)
    (term : TSyntax `term) :
    CommandElabM (Option (TSyntax `term × Array (TSyntax `ident))) := do
  match ← moveCallSpec? context term with
  | some (call, mutations, returnsMutation) =>
      if mutations.isEmpty || returnsMutation then return none
      return some (call, mutations)
  | _ => return none

private partial def returnedMutationCallSpec? (context : TranslationContext)
    (term : TSyntax `term) :
    CommandElabM (Option
      (TSyntax `term × Array (TSyntax `ident) × Array ResultLeaf)) := do
  match ← moveCallSpec? context term with
  | some (call, mutations, true) =>
      let (head, _) ← match term with
        | `(continue $head:term $_arguments:term*) => pure (head, true)
        | _ =>
            let some (head, _) := application? term | return none
            pure (head, false)
      unless head.raw.isIdent do return none
      let identifier : TSyntax `ident := ⟨head.raw⟩
      let some functionName ← resolveMoveFunction? identifier | return none
      let some declaration := (declarations.getState (← getEnv)).find? functionName
        | return none
      return some (call, mutations,
        resultLeaves (declaredResultType declaration))
  | _ => return none

private partial def writeMutableCallOutputs (mutations : Array (TSyntax `ident))
    (output : TSyntax `term) (continuation : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  let mut result := continuation
  for (mutation, index) in mutations.zipIdx.reverse do
    let finalValue ← if mutations.size == 1 then
      `($output.2)
    else
      liftMacroM <| argumentProjection (← `($output.2)) index mutations.size
    result ← `(let $mutation := Move.Semantics.Mutation.write $mutation $finalValue
      $result)
  pure result

/-- Sequence a call passing the live mutable reference before `continuation`:
the reference resumes with the callee's final referent, and the call's value
is bound to `name?` when given. -/
private partial def bindMutableCall
    (call : TSyntax `term) (mutations : Array (TSyntax `ident))
    (name? : Option (TSyntax `ident))
    (continuation : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let output := mkIdentFrom call `_moveSpecCallOutput
  let outputTerm : TSyntax `term := ⟨output.raw⟩
  let continuation ← match name? with
    | some name => `(let $name := $outputTerm.1; $continuation)
    | none => pure continuation
  let continuation ← writeMutableCallOutputs mutations outputTerm continuation
  match name? with
  | some _ | none => `(Move.Semantics.Spec.bind $call (fun $output => $continuation))

/-- Bind a mutable reference returned by a call. All mutable actuals are
unavailable for the result's live range, matching the bytecode verifier's
signature-only call rule. Once the result resolves, the caller continues with
the updated lender carriers supplied by the callee. -/
private partial def bindReturnedMutationCall (context : TranslationContext)
    (call : TSyntax `term) (mutations : Array (TSyntax `ident))
    (name : TSyntax `ident) (rest : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  let (loanBody, continuation) ← mutableBorrowScope name.getId rest
  if context.returnsMutation then
    unless continuation.isEmpty do
      throwErrorAt name
        "a forwarded mutable-reference result must remain live through the function return"
    for mutation in mutations do
      unless context.rootMutations.any (·.getId == mutation.getId) do
        throwErrorAt mutation
          "a forwarded mutable-reference result must derive directly from mutable-reference parameters"
      if loanBody.any (containsIdentifier mutation.getId ·.raw) then
        throwErrorAt name
          "every mutable argument is unavailable while a returned mutable reference is live"
    let callOutput := mkIdentFrom name `_moveSpecForwardedCallOutput
    let callOutputTerm : TSyntax `term := ⟨callOutput.raw⟩
    let poisoned (candidate : Name) := mutations.any (·.getId == candidate)
    let remaining := (liveMutations context).filter fun mutation =>
      !poisoned mutation.getId
    let nested ← translateDo
      { context with
          mutation? := some name
          mutationType? := none
          mutationAncestors := context.mutationAncestors.filter fun (mutation, _) =>
            !poisoned mutation.getId
          mutationOwnerAliases := context.mutationOwnerAliases.filter
            fun (_, mutation, _) => !poisoned mutation.getId
          mutationRefs := remaining
          transferredReturns := #[name]
          loanScope? := some name }
      loanBody
    let mut nested := nested
    nested ← `(let $name := $callOutputTerm.1; $nested)
    for (mutation, index) in mutations.zipIdx.reverse do
      let updated ← if mutations.size == 1 then
        `($callOutputTerm.2)
      else
        liftMacroM <| argumentProjection (← `($callOutputTerm.2)) index mutations.size
      nested ← `(let $mutation := $updated; $nested)
    return ← `(Move.Semantics.Spec.bind $call (fun $callOutput => $nested))
  for mutation in mutations do
    if loanBody.any (containsIdentifier mutation.getId ·.raw) then
      throwErrorAt name
        "every mutable argument is unavailable while a returned mutable reference is live"
  let poisoned (candidate : Name) :=
    mutations.any (·.getId == candidate)
  let remaining := (liveMutations context).filter fun mutation =>
    !poisoned mutation.getId
  let normalValue := mkIdentFrom name `_moveSpecLoanValue
  let normal ← afterLoan context continuation ⟨normalValue.raw⟩
  let loanExit := mkLoanExitFrame context mutations normalValue normal
  let nested ← translateDo
    { context with
        mutation? := some name
        mutationType? := none
        mutationAncestors := context.mutationAncestors.filter fun (mutation, _) =>
          !poisoned mutation.getId
        mutationOwnerAliases := context.mutationOwnerAliases.filter
          fun (_, mutation, _) => !poisoned mutation.getId
        mutationRefs := remaining
        loanScope? := some name
        loanExits := loanExit :: context.loanExits }
    loanBody
  let callOutput := mkIdentFrom name `_moveSpecReturnedCallOutput
  let callOutputTerm : TSyntax `term := ⟨callOutput.raw⟩
  let scopeOutput := mkIdentFrom name `_moveSpecReturnedScopeOutput
  let scopeOutputTerm : TSyntax `term := ⟨scopeOutput.raw⟩
  let transferred ← `(Move.Semantics.withTransferredMutation $callOutput.1
    (fun $name => $nested))
  let mut selected : TSyntax `term := scopeOutputTerm
  for (_, index) in mutations.zipIdx do
    let updated ← if mutations.size == 1 then
      `($callOutputTerm.2)
    else
      liftMacroM <| argumentProjection (← `($callOutputTerm.2))
        index mutations.size
    selected ← `($selected $updated)
  `(Move.Semantics.Spec.bind $call (fun $callOutput =>
    Move.Semantics.Spec.bind $transferred (fun $scopeOutput => $selected)))

/-- Bind a flattened multiple return containing mutable-reference components.
All returned mutations share one conservative live range; this keeps every
possible lender suspended until all components have resolved. -/
private partial def bindReturnedMutationPatternCall (context : TranslationContext)
    (call : TSyntax `term) (mutations : Array (TSyntax `ident))
    (leaves : Array ResultLeaf) (pattern : TSyntax `term)
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  let components := flattenResultTerms pattern
  unless components.size == leaves.size do
    throwErrorAt pattern
      "a multiple-return binding must match the flattened result shape"
  let mut names : Array (Option (TSyntax `ident)) := #[]
  let mut mutableNames : Array (TSyntax `ident) := #[]
  for (leaf, index) in leaves.zipIdx do
    let component := components[index]!
    let authored? : Option (TSyntax `ident) :=
      if component.raw.isIdent && !component.raw.getId.isAnonymous then
        some ⟨component.raw⟩
      else none
    match leaf with
    | .mutable _ =>
        let name := authored?.getD <| mkIdentFrom component
          (Name.mkSimple s!"_moveSpecDiscardedReturnedMutation{index}")
        names := names.push (some name)
        mutableNames := mutableNames.push name
    | .value _ | .immutable _ => names := names.push authored?
  let (loanBody, continuation) ← mutableResultScope mutableNames rest
  if context.returnsMutation then
    unless continuation.isEmpty do
      throwErrorAt pattern
        "forwarded mutable-reference results must remain live through the function return"
    for mutation in mutations do
      unless context.rootMutations.any (·.getId == mutation.getId) do
        throwErrorAt mutation
          "forwarded mutable-reference results must derive directly from mutable-reference parameters"
      if loanBody.any (containsIdentifier mutation.getId ·.raw) then
        throwErrorAt pattern
          "every mutable argument is unavailable while returned mutable references are live"
    let poisoned (candidate : Name) := mutations.any (·.getId == candidate)
    let remaining := (liveMutations context).filter fun mutation =>
      !poisoned mutation.getId
    let firstMutation := mutableNames[0]!
    let nested ← translateDo
      { context with
          mutation? := some firstMutation
          mutationType? := none
          mutationAncestors := context.mutationAncestors.filter fun (mutation, _) =>
            !poisoned mutation.getId
          mutationOwnerAliases := context.mutationOwnerAliases.filter
            fun (_, mutation, _) => !poisoned mutation.getId
          mutationRefs :=
            (mutableNames.extract 1 mutableNames.size).toList ++ remaining
          transferredReturns := mutableNames
          loanScope? := some firstMutation }
      loanBody
    let callOutput := mkIdentFrom pattern `_moveSpecForwardedCallOutput
    let callOutputTerm : TSyntax `term := ⟨callOutput.raw⟩
    let mut body := nested
    for (name?, index) in names.zipIdx.reverse do
      if let some name := name? then
        let component ← liftMacroM <|
          argumentProjection (← `($callOutputTerm.1)) index leaves.size
        body ← `(let $name := $component; $body)
    for (mutation, index) in mutations.zipIdx.reverse do
      let updated ← if mutations.size == 1 then
        `($callOutputTerm.2)
      else
        liftMacroM <| argumentProjection (← `($callOutputTerm.2)) index mutations.size
      body ← `(let $mutation := $updated; $body)
    return ← `(Move.Semantics.Spec.bind $call (fun $callOutput => $body))
  for mutation in mutations do
    if loanBody.any (containsIdentifier mutation.getId ·.raw) then
      throwErrorAt pattern
        "every mutable argument is unavailable while returned mutable references are live"
  let poisoned (candidate : Name) := mutations.any (·.getId == candidate)
  let remaining := (liveMutations context).filter fun mutation =>
    !poisoned mutation.getId
  let firstMutation := mutableNames[0]!
  let normalValue := mkIdentFrom pattern `_moveSpecLoanValue
  let normal ← afterLoan context continuation ⟨normalValue.raw⟩
  let loanExit := mkLoanExitFrame context mutations normalValue normal
  let nested ← translateDo
    { context with
        mutation? := some firstMutation
        mutationType? := none
        mutationAncestors := context.mutationAncestors.filter fun (mutation, _) =>
          !poisoned mutation.getId
        mutationOwnerAliases := context.mutationOwnerAliases.filter
          fun (_, mutation, _) => !poisoned mutation.getId
        mutationRefs :=
          (mutableNames.extract 1 mutableNames.size).toList ++ remaining
        rootMutations := mutableNames
        loanScope? := some firstMutation
        loanExits := loanExit :: context.loanExits }
    loanBody
  let callOutput := mkIdentFrom pattern `_moveSpecReturnedCallOutput
  let callOutputTerm : TSyntax `term := ⟨callOutput.raw⟩
  let scopeOutput := mkIdentFrom pattern `_moveSpecReturnedScopeOutput
  let scopeOutputTerm : TSyntax `term := ⟨scopeOutput.raw⟩
  let mut after ← `($scopeOutputTerm.1)
  for (_, index) in mutations.zipIdx do
    let updated ← liftMacroM <|
      argumentProjection (← `($callOutputTerm.2)) index mutations.size
    after ← `($after $updated)
  for index in (List.range mutableNames.size).reverse do
    let finalMutation ← liftMacroM <|
      argumentProjection (← `($scopeOutputTerm.2)) index mutableNames.size
    after ← `(Move.Semantics.Spec.bind
      (Move.Semantics.resolveMutation $finalMutation)
      (fun _moveSpecResolved => $after))
  let mut body ← `(Move.Semantics.Spec.bind $nested (fun $scopeOutput => $after))
  for (name?, index) in names.zipIdx.reverse do
    if let some name := name? then
      let component ← liftMacroM <|
        argumentProjection (← `($callOutputTerm.1)) index leaves.size
      body ← `(let $name := $component; $body)
  `(Move.Semantics.Spec.bind $call (fun $callOutput => $body))

private partial def vectorMutationSpec (context : TranslationContext)
    (mutation : TSyntax `ident) (call : VectorMutationCall) :
    CommandElabM (TSyntax `term) := do
  let check (reference : TSyntax `term) :=
    unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
      throwErrorAt reference "vector operation must use the currently borrowed vector"
  match call with
  | .insert reference index value =>
      check reference
      withHoisted context #[index, value] fun args =>
        `(Move.Semantics.Vector.insertSpec $mutation $(args[0]!) $(args[1]!))
  | .remove reference index =>
      check reference
      withHoisted context #[index] fun args =>
        `(Move.Semantics.Vector.removeSpec $mutation $(args[0]!))
  | .popBack reference => check reference; `(Move.Semantics.Vector.popBackSpec $mutation)
  | .swap reference i j =>
      check reference
      withHoisted context #[i, j] fun args =>
        `(Move.Semantics.Vector.swapSpec $mutation $(args[0]!) $(args[1]!))
  | .swapRemove reference i =>
      check reference
      withHoisted context #[i] fun args =>
        `(Move.Semantics.Vector.swapRemoveSpec $mutation $(args[0]!))
  | .append reference other =>
      check reference
      withHoisted context #[other] fun args =>
        `(Move.Semantics.Vector.appendSpec $mutation $(args[0]!))
  | .reverse reference => check reference; `(Move.Semantics.Vector.reverseSpec $mutation)
  | .reverseSlice reference left right =>
      check reference
      withHoisted context #[left, right] fun args =>
        `(Move.Semantics.Vector.reverseSliceSpec $mutation $(args[0]!) $(args[1]!))
  | .trim reference newLen =>
      check reference
      withHoisted context #[newLen] fun args =>
        `(Move.Semantics.Vector.trimSpec $mutation $(args[0]!))
  | .trimReverse reference newLen =>
      check reference
      withHoisted context #[newLen] fun args =>
        `(Move.Semantics.Vector.trimReverseSpec $mutation $(args[0]!))
  | .rotate reference rot =>
      check reference
      withHoisted context #[rot] fun args =>
        `(Move.Semantics.Vector.rotateSpec $mutation $(args[0]!))
  | .rotateSlice reference left rot right =>
      check reference
      withHoisted context #[left, rot, right] fun args =>
        `(Move.Semantics.Vector.rotateSliceSpec $mutation $(args[0]!) $(args[1]!) $(args[2]!))

/-- A `do`-level `match` through the live mutable reference `mutation`: the
dispatch is on the current referent; an alternative's payload binders are
loans of the selected variant's fields — prophecy mutations whose reconciled
values rebuild the variant, written back when the alternative's statements
end — and the statements after the match then continue with the reference,
as in `matchSpec`.  A wildcard payload position is not loaned.  The reference
itself stays readable inside an alternative but cannot be written while its
payload is borrowed (Move's borrow rules). -/
private partial def refMatchSpec (context : TranslationContext)
    (mutation : TSyntax `ident) (alternatives : Array Syntax)
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  let current ← `(Move.Semantics.Mutation.read $mutation)
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for alternative in alternatives do
    let `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $body) := alternative
      | throwErrorAt alternative
          "unsupported `match` alternative in automatic source specification"
    let patternList := patterns.getElems
    unless patternList.size == 1 do
      throwErrorAt alternative
        "a `match` through a mutable reference takes one pattern per alternative"
    let pattern := patternList[0]!
    let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨body.raw⟩
    let elements := Lean.Parser.Term.getDoElems body
    let (head, binders) : Syntax × Array Syntax :=
      if pattern.raw.isOfKind ``Lean.Parser.Term.app && pattern.raw.getNumArgs == 2 then
        (pattern.raw[0], pattern.raw[1].getArgs)
      else (pattern.raw, #[])
    let loans : Array (TSyntax `ident) := binders.filterMap fun binder =>
      if binder.isIdent then some ⟨binder⟩ else none
    if loans.isEmpty then
      -- No payload reference: the alternative is a plain continuation.
      let armSpec ← translateDo context (elements ++ rest)
      arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $armSpec))
    else
      -- Every payload position is named, so the variant can be rebuilt.
      let mut positions : Array (TSyntax `term) := #[]
      for (binder, index) in binders.zipIdx do
        if binder.isIdent then positions := positions.push ⟨binder⟩
        else
          let fresh : TSyntax `term :=
            ⟨mkIdentFrom binder (Name.mkSimple s!"_moveSpecPayload{index}")⟩
          positions := positions.push fresh
      let headTerm : TSyntax `term := ⟨head⟩
      let fullPattern ← `($headTerm $positions*)
      if context.returnsMutation && rest.isEmpty then
        let returnedLoans := loans.filter fun loan =>
          isMutationResultComponent context loan.getId elements
        if !returnedLoans.isEmpty then
          let isReturned (loan : TSyntax `ident) :=
            returnedLoans.any (·.getId == loan.getId)
          let scopedLoans := loans.filter fun loan => !isReturned loan
          let loanNormal ← transferredLoanNormal context loans
          let loanExits := loans.reverse.toList.map fun loan =>
            mkTransferredLoanExitFrame context loan loanNormal
          let nested ← translateDo
            { context with
                mutation? := some loans[0]!
                mutationType? := none
                mutationAncestors :=
                  (mutation, context.mutationType?) :: context.mutationAncestors
                mutationRefs :=
                  (loans.extract 1 loans.size).toList ++
                    context.mutation?.toList ++ context.mutationRefs
                transferredReturns :=
                  context.transferredReturns ++ returnedLoans
                resolveBeforeReturn :=
                  context.resolveBeforeReturn ++ scopedLoans
                loanExits := loanExits ++ context.loanExits
                loanScope? := some loans[0]! }
            elements
          let mut transfers : Array (TSyntax `ident × TSyntax `ident) := #[]
          for (loan, index) in loans.zipIdx do
            let transfer := mkIdentFrom pattern
              (Name.mkSimple s!"_moveSpecReturnedPayload{index}")
            transfers := transfers.push (loan, transfer)
          let mut rebuiltArguments : Array (TSyntax `term) := #[]
          for position in positions do
            let transfer? := transfers.find? fun (loan, _) =>
              position.raw.isIdent && position.raw.getId == loan.getId
            match transfer? with
            | some (_, transfer) =>
                let transferTerm : TSyntax `term := ⟨transfer.raw⟩
                rebuiltArguments := rebuiltArguments.push (← `($transferTerm.2))
            | none => rebuiltArguments := rebuiltArguments.push position
          let rebuilt ← `($headTerm $rebuiltArguments*)
          let updatedMutation ← `(Move.Semantics.Mutation.write $mutation $rebuilt)
          let mut armSpec ← `(let $mutation := $updatedMutation; $nested)
          for (loan, transfer) in transfers.reverse do
            let transferTerm : TSyntax `term := ⟨transfer.raw⟩
            armSpec ← `(Move.Semantics.Spec.bind
              (Move.Semantics.reborrowMutation $loan)
              (fun $transfer =>
                let $loan := $transferTerm.1
                $armSpec))
          arms := arms.push
            (← `(Lean.Parser.Term.matchAltExpr| | $fullPattern:term => $armSpec))
          continue
      let outer := liveMutations context
      let delaysControlExit := elements.any fun element =>
        containsLoanControlExit element.raw
      let normalValue := mkIdentFrom pattern `_moveSpecLoanValue
      let normal ← afterLoan context rest ⟨normalValue.raw⟩
      let loanExit := mkLoanExitFrame context #[mutation] normalValue normal
      let translateNested (deferContinuation : Bool) := translateDo
        { context with
            mutation? := some loans[0]!
            mutationType? := none
            mutationRefs := (loans.extract 1 loans.size).toList ++ outer
            rootMutations := loans
            loops := if deferContinuation then context.loops else []
            loanScope? := some loans[0]!
            loanExits := if deferContinuation then loanExit :: context.loanExits else [] }
        elements
      let deferContinuation := delaysControlExit || !context.loanExits.isEmpty ||
        (← sourceMutatesAny (outer.map (·.getId)) elements)
      let nested ← translateNested deferContinuation
      let output := mkIdentFrom pattern `_moveSpecPayloadOutput
      let outputTerm : TSyntax `term := ⟨output.raw⟩
      let finalOf (loanIndex : Nat) : CommandElabM (TSyntax `term) := do
        let finalOwners ← `($outputTerm.2)
        liftMacroM <| argumentProjection finalOwners loanIndex loans.size
      let mut rebuiltArguments : Array (TSyntax `term) := #[]
      for position in positions do
        match loans.findIdx? (·.getId == position.raw.getId) with
        | some loanIndex => rebuiltArguments := rebuiltArguments.push (← finalOf loanIndex)
        | none => rebuiltArguments := rebuiltArguments.push position
      let rebuilt ← `($headTerm $rebuiltArguments*)
      let borrow ← withInferredMutableValues loans nested
      let after ← if deferContinuation then
        `($outputTerm.1 $mutation)
      else
        afterLoan context rest (← `($outputTerm.1))
      let armSpec ← `(Move.Semantics.Spec.bind $borrow (fun $output =>
          let $mutation := Move.Semantics.Mutation.write $mutation $rebuilt
          $after))
      arms := arms.push
        (← `(Lean.Parser.Term.matchAltExpr| | $fullPattern:term => $armSpec))
  `(match $current:term with $arms:matchAlt*)

/-- A `do`-level `match`: each arm continues with the statements after the
match, as the then-branch of a statement `if` does. -/
private partial def matchSpec (context : TranslationContext)
    (discriminants : Array (TSyntax `term)) (alternatives : Array Syntax)
    (rest : Array Lean.DoElem) : CommandElabM (TSyntax `term) := do
  withHoisted context discriminants fun residuals => do
    let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
    for alternative in alternatives do
      let `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $body) := alternative
        | throwErrorAt alternative
            "unsupported `match` alternative in automatic source specification"
      let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨body.raw⟩
      let armSpec ← translateDo context (Lean.Parser.Term.getDoElems body ++ rest)
      arms := arms.push
        (← `(Lean.Parser.Term.matchAltExpr| | $patterns,* => $armSpec))
    if residuals.size == 1 then
      `(match $(residuals[0]!):term with $arms:matchAlt*)
    else if residuals.size == 2 then
      `(match $(residuals[0]!):term, $(residuals[1]!):term with $arms:matchAlt*)
    else
      throwError "automatic source specifications support at most two match discriminants"

/-- A mutable borrow of an owned local that is not a loaned owner: a loan of
the local's value. The loan body returns its continuation as a closure, so the
owner is reconciled before the continuation runs and writes to disjoint outer
mutations are retained in the closure. -/
private partial def ownedLocalMutableBorrow (context : TranslationContext)
    (name : TSyntax `ident) (place : TSyntax `term) (rest : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  let (loanBody, continuation) ← mutableBorrowScope name.getId rest
  let localIdent : TSyntax `ident := ⟨place.raw⟩
  let output := mkIdentFrom place `_moveSpecLocalOutput
  let outputTerm : TSyntax `term := ⟨output.raw⟩
  let outer := liveMutations context
  let delaysControlExit := loanBody.any fun element =>
    containsLoanControlExit element.raw
  let normalValue := mkIdentFrom name `_moveSpecLoanValue
  let normal ← afterLoan context continuation ⟨normalValue.raw⟩
  let loanExit := mkLoanExitFrame context #[localIdent] normalValue normal
  let translateNested (deferContinuation : Bool) := translateDo
    { context with
        mutation? := some name
        mutationType? := none
        mutationOwnerAliases :=
          (localIdent.getId, name, none) :: context.mutationOwnerAliases
        mutationRefs := outer
        loops := if deferContinuation then context.loops else []
        loanScope? := some name
        loanExits := if deferContinuation then loanExit :: context.loanExits else [] }
    loanBody
  let deferContinuation := delaysControlExit || !context.loanExits.isEmpty ||
    (← sourceMutatesAny (outer.map (·.getId)) loanBody)
  let nested ← translateNested deferContinuation
  let borrow ← `(Move.Semantics.withMutation $localIdent (fun $name => $nested))
  let after ← if deferContinuation then
    `($outputTerm.1 $outputTerm.2)
  else
    afterLoan context continuation (← `($outputTerm.1))
  `(Move.Semantics.Spec.bind $borrow (fun $output =>
      let $localIdent := $outputTerm.2
      $after))

/-- The code after a loan, translated outside it: the continuation; when
there is none, the loan body's value ends the block -- or, inside a loop,
the iteration. -/
private partial def afterLoan (context : TranslationContext)
    (continuation : Array Lean.DoElem) (value : TSyntax `term) :
    CommandElabM (TSyntax `term) := do
  if continuation.isEmpty then
    if context.loops.isEmpty then
      finish context (← `(Move.Semantics.Spec.pure $value))
    else
      emptyFinish context
  else
    translateDo context continuation

private partial def translateDo (context : TranslationContext)
    (elements : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  let translateRest (rest : Array Lean.DoElem) :=
    if rest.isEmpty then emptyFinish context else translateDo context rest
  if elements.isEmpty then return ← emptyFinish context
  let first : Lean.DoElem := elements[0]!
  let mut rest := elements.extract 1 elements.size
  let kind := first.raw.getKind
  if kind == ``Move.moveNamedStructLet then
    let fields : TSyntaxArray `term := first.raw[3].getSepArgs.map (⟨·⟩)
    let value : TSyntax `term := ⟨first.raw[6]⟩
    let expanded ← `(doElem| let ⟨$fields:term,*⟩ := $value)
    return ← translateDo context (#[expanded] ++ rest)
  if kind == ``Move.movePositionalStructLet then
    let fields : TSyntaxArray `term := first.raw[3].getSepArgs.map (⟨·⟩)
    let value : TSyntax `term := ⟨first.raw[6]⟩
    let expanded ← `(doElem| let ⟨$fields:term,*⟩ := $value)
    return ← translateDo context (#[expanded] ++ rest)
  if kind == ``Move.moveSpecAssert then
    match first with
    | `(doElem| assert $clause:term) =>
        let mut predicate ← inlineSpecPredicate context clause
        -- Consecutive assertions observe the same state.  Combining them
        -- avoids building a deeply nested chain of unit binds while retaining
        -- the exact conjunction of their proof obligations.
        let mut consumed := 0
        let mut collecting := true
        for next in rest do
          if collecting && next.raw.getKind == ``Move.moveSpecAssert then
            match next with
            | `(doElem| assert $nextClause:term) =>
                let nextPredicate ← inlineSpecPredicate context nextClause
                let state := mkIdentFrom next `_moveSpecAssertState
                predicate ← `(fun $state => $predicate $state ∧ $nextPredicate $state)
                consumed := consumed + 1
            | _ => throwErrorAt next "invalid in-body `assert`"
          else
            collecting := false
        rest := rest.extract consumed rest.size
        let nested ← translateRest rest
        let ignored := mkIdentFrom first `_moveSpecAssert
        return ← `(Move.Semantics.Spec.bind
          (Move.Semantics.Spec.certifyState $predicate) (fun $ignored => $nested))
    | _ => throwErrorAt first "invalid in-body `spec assert`"
  if kind == ``Move.moveSpecAssume then
    match first with
    | `(doElem| assume $clause:term) =>
        let predicate ← inlineSpecPredicate context clause
        let nested ← translateRest rest
        let ignored := mkIdentFrom first `_moveSpecAssume
        return ← `(Move.Semantics.Spec.bind
          (Move.Semantics.Spec.assumeState $predicate) (fun $ignored => $nested))
    | _ => throwErrorAt first "invalid in-body `spec assume`"
  if kind == ``Move.moveSpecCapture then
    match first with
    | `(doElem| __moveSpecCapture $label:num) =>
        let some labelValue := label.raw.isNatLit?
          | throwErrorAt label "expected a numeric state-anchor label"
        let observed := rest.foldl (fun observed element =>
          anchoredObservations labelValue element.raw observed) #[]
        let mut saved : Array (StateAnchorCapture × TSyntax `term) := #[]
        for (observedValue, index) in observed.zipIdx do
          let snapshot := mkIdentFrom first
            (Name.mkSimple s!"_moveSpecAnchor{labelValue}_{index}")
          let value ← rewritePure (liveMutations context) ⟨observedValue⟩
          saved := saved.push ({ label := labelValue, observed := observedValue, snapshot }, value)
        let captures := saved.map (·.1)
        let nested ← if rest.isEmpty then emptyFinish context else
          translateDo ({ context with stateAnchors := captures.toList ++ context.stateAnchors }) rest
        let mut nested := nested
        for (savedCapture, value) in saved.reverse do
          let snapshot := savedCapture.snapshot
          nested ← `(let $snapshot:ident := $value; $nested)
        return nested
    | _ => throwErrorAt first "invalid compiler-generated `spec capture`"
  if kind == ``Move.moveForRange then
    let index : TSyntax `ident := ⟨first.raw[2]⟩
    let lower : TSyntax `term := ⟨first.raw[4]⟩
    let upper : TSyntax `term := ⟨first.raw[6]⟩
    let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨first.raw[9]⟩
    let counter := mkIdentFrom index `_moveSpecForIndex
    let bodyElems := Lean.Parser.Term.getDoElems body |>.map (fun element => element.raw)
    let bindIndex := (← `(doElem| let $index := $counter)).raw
    let increment := (← `(doElem| $counter:ident := $counter + 1)).raw
    let loopBody := Move.mkDoSeq ((#[bindIndex] ++ bodyElems).push increment)
    let initialElement ← `(doElem| let mut $counter := $lower)
    let loopElement ← `(doElem| while $counter < $upper do $loopBody)
    return ← translateDo context (#[initialElement, loopElement] ++ rest)
  if kind == ``Move.moveAddAssign || kind == ``Move.moveSubAssign ||
      kind == ``Move.moveMulAssign || kind == ``Move.moveDivAssign ||
      kind == ``Move.moveModAssign then
    let name : TSyntax `ident := ⟨first.raw[0]⟩
    let rhs : TSyntax `term := ⟨first.raw[2]⟩
    let lhs : TSyntax `term := ⟨name.raw⟩
    let value ←
      if kind == ``Move.moveAddAssign then `($lhs + $rhs)
      else if kind == ``Move.moveSubAssign then `($lhs - $rhs)
      else if kind == ``Move.moveMulAssign then `($lhs * $rhs)
      else if kind == ``Move.moveDivAssign then `($lhs / $rhs)
      else `($lhs % $rhs)
    let assignment ← `(doElem| $name:ident := $value)
    return ← translateDo context (#[assignment] ++ rest)
  if first.raw.isOfKind ``Lean.Parser.Term.doReassign then
    let assignment : TSyntax ``Lean.Parser.Term.doReassign := ⟨first.raw⟩
    match assignment with
    | `(doReassign| $name:ident $[: $_]? :=%$_ $rhs:term) =>
        if (liveMutations context).any (·.getId == name.getId) then
          let rhsSpec ← expressionSpec context rhs
          let nested ← translateRest rest
          return ← `(Move.Semantics.Spec.bind $rhsSpec fun _moveSpecValue =>
              let $name := Move.Semantics.Mutation.write $name _moveSpecValue
              $nested)
        let rhsSpec ← expressionSpec context rhs
        let nested ← translateRest rest
        return ← `(Move.Semantics.Spec.bind $rhsSpec fun $name => $nested)
    | _ => throwErrorAt first "unsupported assignment in automatic source specification"
  if first.raw.isOfKind ``Lean.Parser.Term.doReassignArrow then
    -- `x ← e` on an already bound local: a bind, like `let x ← e`.
    let declaration := first.raw[0]
    unless declaration.isOfKind ``Lean.Parser.Term.doIdDecl do
      throwErrorAt first "unsupported assignment in automatic source specification"
    let name : TSyntax `ident := ⟨declaration[0]⟩
    -- The right-hand side is a `doElem`; a plain expression is wrapped.
    let rhs := declaration[3]
    let value : TSyntax `term :=
      if rhs.isOfKind ``Lean.Parser.Term.doExpr then ⟨rhs[0]⟩ else ⟨rhs⟩
    if (liveMutations context).any (·.getId == name.getId) then
      throwErrorAt first "a mutable reference cannot be rebound; write through it with `:=`"
    -- The same bind as `let x ← e` (reads, borrows, calls): translate it as
    -- that `let`.
    let bind ← `(doElem| let $name:ident ← $value:term)
    return ← translateDo context (#[bind] ++ rest)
  if first.raw.isOfKind ``Lean.Parser.Term.doMatch then
    -- `match e, f with | p, q => …`. Motives/generalization remain Lean's
    -- own elaboration concern; source translation preserves ordinary arms.
    let discriminants := first.raw[4].getSepArgs
    let terms : Array (TSyntax `term) := discriminants.map fun discriminant =>
      ⟨discriminant[1]⟩
    let alternatives := first.raw[6][0].getArgs
    if terms.size == 1 && terms[0]!.raw.isIdent &&
        (liveMutations context).any (·.getId == terms[0]!.raw.getId) then
      return ← refMatchSpec context ⟨terms[0]!.raw⟩ alternatives rest
    return ← matchSpec context terms alternatives rest
  if first.raw.isOfKind ``Move.moveInvariant then
    throwErrorAt first "`invariant` belongs at the head of a `loop` or `while` body"
  if first.raw.isOfKind ``Move.moveLoopDo ||
      first.raw.isOfKind ``Move.moveLoopLabeledDo then
    let sourceLabel? :=
      if first.raw.isOfKind ``Move.moveLoopLabeledDo then
        some first.raw[1]!.getId
      else
        none
    if let some sourceLabel := sourceLabel? then
      if context.loops.any (·.sourceLabel? == some sourceLabel) then
        throwErrorAt first "duplicate active loop label `{sourceLabel}`"
    let bodyIndex := if sourceLabel?.isSome then 2 else 1
    let body : TSyntax ``Lean.Parser.Term.doSeq := ⟨first.raw[bodyIndex]!⟩
    let body ← Lean.Elab.Command.liftCoreM <| Move.freshenLoopLocals body
    -- `reference := value` writes through an existing mutation; it does not
    -- rebind the reference.  The surface loop-assignment collector cannot
    -- distinguish that notation from an ordinary local reassignment, so do
    -- not carry live mutation handles as loop-state values.
    let liveMutationNames := (liveMutations context).map (·.getId)
    let assigned := Move.loopAssignedIdents body |>.filter fun name =>
      !liveMutationNames.any fun mutation =>
        mutation.getString! == name.getId.getString!
    let state := freshLoopStateIdents first.raw assigned
    let bodyElements := (Lean.Parser.Term.getDoElems body).map fun element =>
      (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
    let (invariants, bodyElements) := splitLoopInvariants bodyElements
    -- The live mutable references the body mentions travel with the state
    -- under their own names: a write through one inside the body is what
    -- the next iteration sees (and what the code after the loop resumes
    -- with).  One the body leaves alone keeps its value from the loop's entry.
    let handles := (liveMutations context).filter fun handle =>
      bodyElements.any fun element => containsIdentifier handle.getId element.raw
    let pack ← packLoopState (assigned ++ handles)
    let recName := mkIdentFrom first `_moveSpecLoop
    let recTerm : TSyntax `term := ⟨recName.raw⟩
    let afterElements := rest.map fun element =>
      (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
    let after ← translateRest afterElements
    let frame : VerificationLoopFrame := {
      sourceLabel?, recursive := recTerm, after
      assigned := assigned ++ handles, state := state ++ handles }
    let bodySpec ← translateDo
      { context with
        loops := frame :: context.loops }
      bodyElements
    let stateName := mkIdentFrom first `_moveSpecLoopState
    let stateTerm : TSyntax `term := ⟨stateName.raw⟩
    let unpacked ← unpackLoopState (state ++ handles) stateTerm bodySpec
    return ← loopFixpoint context invariants (state ++ handles) recName stateName unpacked pack
  -- An unactivated mutable handle with no source use has no prophecy.  Erase
  -- it before the eager `withMutation` cases below.  In particular, creating
  -- overlapping handles and discarding all but one must not constrain the
  -- owner's final value.
  match first with
  | `(doElem| let $name:ident ← &mut $_place:term) =>
      let (loanBody, continuation) ← mutableBorrowScope name.getId rest
      if loanBody.isEmpty then
        return ← translateDo context continuation
  | _ => pure ()
  match first with
  | `(doElem| let $name:ident ← &mut $vector:ident[$index:term]) =>
      if let some family ← familyOfTerm? vector then
        -- `&mut R[key]`: the whole resource.
        globalMutableBorrow context name first.raw family index #[] rest
      else if context.mutation?.any (·.getId == vector.getId) then
        -- An element of the active vector mutation.
        elementMutableBorrow context name first.raw vector index #[] rest
      else
        elementMutableBorrow context name first.raw vector index #[] rest
  | `(doElem| let $name:ident ← & $vector:ident[$index:term]) =>
      if let some family ← familyOfTerm? vector then
        globalImmutableBorrow context name first.raw family index #[] rest
      else
        withHoisted context #[index] fun residuals => do
          let nested ← translateRest rest
          let owner ← mutationValue context vector
          `(Move.Semantics.Spec.bind
              (Move.Semantics.Vector.borrowElemSpec $owner $(residuals[0]!))
              (fun $name => $nested))
  | `(doElem| let $name:ident ← &mut $place:term) =>
      if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        elementMutableBorrow context name first.raw vector index fields rest
      else if let some (family, key, fields) ← globalPlace? place then
        globalMutableBorrow context name first.raw family key fields rest
      else if place.raw.isIdent && place.raw.getId.isAtomic &&
          !(← hasResource context.resources ⟨place.raw⟩) &&
          !isLoanedOwner context place.raw.getId then
        -- A whole local (a dotted identifier is a field place).
        ownedLocalMutableBorrow context name place rest
      else if let some parent := context.mutation? then
        let (loanBody, continuation) ← mutableBorrowScope name.getId rest
        let some (owner, fields) ← localPlace? context.resources place
          | throwErrorAt place
              "a nested mutable borrow must select a field of the live mutable reference"
        let parentFrame? :=
          if owner.getId == parent.getId then
            some (parent, context.mutationType?)
          else if let some frame := context.mutationAncestors.find? fun (ancestor, _) =>
              ancestor.getId == owner.getId then
            some frame
          else
            context.mutationOwnerAliases.find? (fun (sourceOwner, _, _) =>
              sourceOwner == owner.getId) |>.map fun (_, mutation, type?) =>
                (mutation, type?)
        let some (parent, parentType?) := parentFrame?
          | throwErrorAt place
              "a nested mutable borrow must select a field of the live mutable reference or a retained ancestor"
        let fieldNames := fields.toList.map (·.getId)
        if context.mutationLoans.any fun (loanOwner, loanPath) =>
            loanOwner == owner.getId && !loanPath.isEmpty &&
              loanPath.head? == fieldNames.head? then
          throwErrorAt place
            "overlapping nested mutable borrows are not supported; sibling borrows must select distinct fields"
        let childType? ← pathTypeName? parentType?
          (fields.toList.map (·.getId))
        if context.returnsMutation && continuation.isEmpty &&
            isMutationResultComponent context name.getId loanBody then
          let parentValue ← `(Move.Semantics.Mutation.read $parent)
          let focused ← projectPath parentValue fields
          let transfer := mkIdentFrom place `_moveSpecReturnedFocus
          let transferTerm : TSyntax `term := ⟨transfer.raw⟩
          let loanNormal ← transferredLoanNormal context #[name]
          let loanExit := mkTransferredLoanExitFrame context name loanNormal
          let nested ← translateDo
            { context with
                mutation? := some name
                mutationType? := childType?
                mutationAncestors := (parent, parentType?) :: context.mutationAncestors
                mutationOwnerAliases :=
                  (owner.getId, name, childType?) :: context.mutationOwnerAliases
                mutationRefs := context.mutation?.toList ++ context.mutationRefs
                transferredReturns := context.transferredReturns.push name
                mutationLoans := (owner.getId, fieldNames) :: context.mutationLoans
                loanExits := loanExit :: context.loanExits
                loanScope? := some name }
            loanBody
          let certified? ← match parentType? with
            | none => pure none
            | some typeName =>
                pure <| (Move.dataInvariant? (← getEnv) typeName).map (typeName, ·)
          let body ← match ← rebuildOwner parentValue (← `($transferTerm.2))
              fields.toList certified? with
            | some creation =>
                let rebuilt := mkIdentFrom place `_moveSpecReturnedOwner
                let updatedParent ← `(Move.Semantics.Mutation.write $parent $rebuilt)
                `(Move.Semantics.Spec.bind $creation (fun $rebuilt =>
                    let $name := $transferTerm.1
                    let $parent := $updatedParent
                    $nested))
            | none =>
                let updated ← updatePath parentValue (← `($transferTerm.2)) fields.toList
                let updatedParent ← `(Move.Semantics.Mutation.write $parent $updated)
                `(let $name := $transferTerm.1
                  let $parent := $updatedParent
                  $nested)
          let returned ← `(Move.Semantics.Spec.bind
            (Move.Semantics.reborrowMutation $focused)
            (fun $transfer => $body))
          if fields.isEmpty then return returned
          return ← `(guardPath% $parentValue [$fields,*] $returned)
        let outer := liveMutations context
        let delaysControlExit := loanBody.any fun element =>
          containsLoanControlExit element.raw
        let normalValue := mkIdentFrom name `_moveSpecLoanValue
        let normal ← afterLoan context continuation ⟨normalValue.raw⟩
        let loanExit := mkLoanExitFrame context #[parent] normalValue normal
        let translateNested (deferContinuation : Bool) := translateDo
          { context with
              mutation? := some name
              mutationType? := childType?
              mutationAncestors := (parent, parentType?) :: context.mutationAncestors
              mutationOwnerAliases :=
                (owner.getId, name, childType?) :: context.mutationOwnerAliases
              mutationRefs := context.mutation?.toList ++ context.mutationRefs
              mutationLoans := (owner.getId, fieldNames) :: context.mutationLoans
              loops := if deferContinuation then context.loops else []
              loanScope? := some name
              loanExits := if deferContinuation then loanExit :: context.loanExits else [] }
          loanBody
        let deferContinuation := delaysControlExit || !context.loanExits.isEmpty ||
          (← sourceMutatesAny (outer.map (·.getId)) loanBody)
        let nested ← translateNested deferContinuation
        let parentValue ← `(Move.Semantics.Mutation.read $parent)
        let focused ← projectPath parentValue fields
        let output := mkIdentFrom place `_moveSpecFieldOutput
        let outputTerm : TSyntax `term := ⟨output.raw⟩
        let finalField ← `($outputTerm.2)
        let loan ← `(Move.Semantics.withMutation $focused (fun $name => $nested))
        let borrow ← if fields.isEmpty then pure loan
          else `(guardPath% $parentValue [$fields,*] $loan)
        -- Re-creating a certified owner is a creation site: its data
        -- invariant is owed here, when the loan dies, and nowhere else.
        let certified? ← match parentType? with
          | none => pure none
          | some typeName =>
              pure <| (Move.dataInvariant? (← getEnv) typeName).map (typeName, ·)
        match ← rebuildOwner parentValue finalField fields.toList certified? with
        | some creation =>
            let rebuilt := mkIdentFrom place `_moveSpecRebuilt
            let resumed ← if deferContinuation then
              `($outputTerm.1 $parent)
            else
              afterLoan context continuation (← `($outputTerm.1))
            `(Move.Semantics.Spec.bind $borrow (fun $output =>
                Move.Semantics.Spec.bind $creation (fun $rebuilt =>
                  let $parent := Move.Semantics.Mutation.write $parent $rebuilt
                  $resumed)))
        | none =>
            let updated ← updatePath parentValue finalField fields.toList
            let resumed ← if deferContinuation then
              `($outputTerm.1 $parent)
            else
              afterLoan context continuation (← `($outputTerm.1))
            `(Move.Semantics.Spec.bind $borrow (fun $output =>
                let $parent := Move.Semantics.Mutation.write $parent $updated
                $resumed))
      else
        let (family, key, fields) ← globalPlace place
        globalMutableBorrow context name first.raw family key fields rest
  | `(doElem| let $name:ident ← & $place:term) =>
      if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        withHoisted context #[index] fun residuals => do
          let nested ← translateRest rest
          let owner ← mutationValue context vector
          let element := mkIdentFrom place `_moveSpecVectorElement
          let elementTerm : TSyntax `term := ⟨element.raw⟩
          let body ← if fields.isEmpty then `(let $name := $elementTerm; $nested)
            else `(bindSelectPath% $elementTerm [$fields,*] $name => $nested)
          `(Move.Semantics.Spec.bind
              (Move.Semantics.Vector.borrowElemSpec $owner $(residuals[0]!))
              (fun $element => $body))
      else if let some (owner, fields) ←
          localPlace? context.resources place then
        let nested ← translateRest rest
        let ownerTerm ← mutationValue context owner
        if fields.isEmpty then `(let $name := $ownerTerm; $nested)
        else `(bindSelectPath% $ownerTerm [$fields,*] $name => $nested)
      else
        let (family, key, fields) ← globalPlace place
        globalImmutableBorrow context name first.raw family key fields rest
  | `(doElem| let $name:ident ← * $reference:term) =>
      let nested ← translateRest rest
      `(let $name := $(← dereferenceValue context reference); $nested)
  | `(doElem| let _ ← $value:term) =>
      -- A discarded result is a bind to a fresh name.
      let name := mkIdentFrom first `_moveSpecDiscarded
      let bind ← `(doElem| let $name:ident ← $value:term)
      translateDo context (#[bind] ++ rest)
  | `(doElem| let $name:ident ← $value:term) =>
      if let some (call, mutations, leaves) ← returnedMutationCallSpec? context value then
        if leaves.size == 1 then
          bindReturnedMutationCall context call mutations name rest
        else
          throwErrorAt name "a reference-containing multiple return must be destructured at its call site"
      else if let some (call, mutations) ← mutableCallSpec? context value then
        bindMutableCall call mutations (some name) (← translateRest rest)
      else if let some call ← vectorMutationCall? value then
        let some mutation := context.mutation?
          | throwErrorAt value
              "`vector::insert` and `vector::remove` require a live mutable vector borrow"
        let output := mkIdentFrom value `_moveSpecVectorMutationOutput
        let nested ← translateRest rest
        match call with
        | .insert reference index inserted =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector insert must use the currently borrowed vector"
            withHoisted context #[index, inserted] fun residuals =>
              `(Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.insertSpec $mutation $(residuals[0]!) $(residuals[1]!))
                  (fun $output =>
                    let $name := $output.1
                    let $mutation := $output.2
                    $nested))
        | .remove reference index =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector remove must use the currently borrowed vector"
            withHoisted context #[index] fun residuals =>
              `(Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.removeSpec $mutation $(residuals[0]!))
                  (fun $output =>
                    let $name := $output.1
                    let $mutation := $output.2
                    $nested))
        | .popBack reference =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector pop_back must use the currently borrowed vector"
            `(Move.Semantics.Spec.bind
                (Move.Semantics.Vector.popBackSpec $mutation)
                (fun $output =>
                  let $name := $output.1
                  let $mutation := $output.2
                  $nested))
        | call =>
            let operation ← vectorMutationSpec context mutation call
            `(Move.Semantics.Spec.bind $operation (fun $output =>
                let $name := $output.1
                let $mutation := $output.2
                $nested))
      else
        if receiverStyleVectorMutation? value then
          throwErrorAt value
            "automatic source specifications require fully qualified `Move.Vector.insert` or `Move.Vector.remove`"
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let $name:ident := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let $name:ident : $type:term := $value:term) =>
      -- The ascription directs the value's elaboration (a structure literal
      -- or numeral has no other source of its type).
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun ($name : $type) => $nested))
  | `(doElem| let $name:ident : $type:term ← $value:term) =>
      if let some (call, mutations, leaves) ← returnedMutationCallSpec? context value then
        if leaves.size == 1 then
          bindReturnedMutationCall context call mutations name rest
        else
          throwErrorAt name "a reference-containing multiple return must be destructured at its call site"
      else if let some (call, mutations) ← mutableCallSpec? context value then
        bindMutableCall call mutations (some name) (← translateRest rest)
      else
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $valueSpec (fun ($name : $type) => $nested))
  | `(doElem| let mut $name:ident := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let mut $name:ident : $type:term := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      `(Move.Semantics.Spec.bind $valueSpec (fun ($name : $type) => $nested))
  | `(doElem| let mut $name:ident : $type:term ← $value:term) =>
      if let some (call, mutations, leaves) ← returnedMutationCallSpec? context value then
        if leaves.size == 1 then
          bindReturnedMutationCall context call mutations name rest
        else
          throwErrorAt name "a reference-containing multiple return must be destructured at its call site"
      else if let some (call, mutations) ← mutableCallSpec? context value then
        bindMutableCall call mutations (some name) (← translateRest rest)
      else
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $valueSpec (fun ($name : $type) => $nested))
  | `(doElem| let mut $name:ident ← * $reference:term) =>
      let nested ← translateRest rest
      `(let $name := $(← dereferenceValue context reference); $nested)
  | `(doElem| let mut $name:ident ← $value:term) =>
      if let some (call, mutations, leaves) ← returnedMutationCallSpec? context value then
        if leaves.size == 1 then
          bindReturnedMutationCall context call mutations name rest
        else
          throwErrorAt name "a reference-containing multiple return must be destructured at its call site"
      else if let some (call, mutations) ← mutableCallSpec? context value then
        bindMutableCall call mutations (some name) (← translateRest rest)
      else
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $valueSpec (fun $name => $nested))
  | `(doElem| let $pattern:term := $value:term) =>
      let valueSpec ← expressionSpec context value
      let nested ← translateRest rest
      let packed := mkIdentFrom pattern `_moveSpecTuple
      let packedTerm : TSyntax `term := ⟨packed.raw⟩
      let matched ← `(match $packedTerm:term with | $pattern:term => $nested)
      `(Move.Semantics.Spec.bind $valueSpec (fun $packed => $matched))
  | `(doElem| let $pattern:term ← $value:term) =>
      if let some (call, mutations, leaves) ← returnedMutationCallSpec? context value then
        bindReturnedMutationPatternCall context call mutations leaves pattern rest
      else
        let valueSpec ← expressionSpec context value
        let nested ← translateRest rest
        let packed := mkIdentFrom pattern `_moveSpecTuple
        let packedTerm : TSyntax `term := ⟨packed.raw⟩
        let matched ← `(match $packedTerm:term with | $pattern:term => $nested)
        `(Move.Semantics.Spec.bind $valueSpec (fun $packed => $matched))
  | `(doElem| return $value:term) =>
      -- `return` ends the function: inside a loop too, since the loop's
      -- fixed point already produces the function's result (its `break`
      -- continuation is the rest of the function).
      if forwardsTransferredMutationResult context value then
        translateTerm context value
      else if !context.loanExits.isEmpty then
        let action ← translateTerm (resumeAfterAllLoans context) value
        finishLoanExits context (fun _ => true) action
      else
        translateTerm context value
  | `(doElem| break@$sourceLabel:ident) =>
      loopBreakSpec context (some sourceLabel.getId) first.raw
  | `(doElem| continue@$sourceLabel:ident) =>
      loopContinueSpec context (some sourceLabel.getId) first.raw
  | `(doElem| break) =>
      loopBreakSpec context none first.raw
  | `(doElem| continue) =>
      loopContinueSpec context none first.raw
  | `(doElem| while $condition:doIfCond do $body:doSeq) =>
      let (binder?, condition) ← plainCondition condition
      let body ← Lean.Elab.Command.liftCoreM <| Move.freshenLoopLocals body
      -- Writes through live mutable references use assignment notation but do
      -- not rebind those references.  Only ordinary locals belong in the
      -- loop's explicit fixed-point state.
      let liveMutationNames := (liveMutations context).map (·.getId)
      let assigned := Move.loopAssignedIdents body |>.filter fun name =>
        !liveMutationNames.any fun mutation =>
          mutation.getString! == name.getId.getString!
      let state := freshLoopStateIdents first.raw assigned
      let condition : TSyntax `term := ⟨replaceLoopState assigned state condition.raw⟩
      let bodyElements := (Lean.Parser.Term.getDoElems body).map fun element =>
        (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
      -- Invariants at the head of a `while` body hold before the condition.
      let (invariants, bodyElements) := splitLoopInvariants bodyElements
      -- The live mutable references the body mentions travel with the state
      -- (see `loop`).
      let handles := (liveMutations context).filter fun handle =>
        bodyElements.any fun element => containsIdentifier handle.getId element.raw
      let pack ← packLoopState (assigned ++ handles)
      let recName := mkIdentFrom first `_moveSpecLoop
      let recTerm : TSyntax `term := ⟨recName.raw⟩
      let afterElements := rest.map fun element =>
        (⟨replaceLoopState assigned state element.raw⟩ : Lean.DoElem)
      let after ← translateRest afterElements
      let frame : VerificationLoopFrame := {
        sourceLabel? := none
        recursive := recTerm
        after
        assigned := assigned ++ handles
        state := state ++ handles }
      let bodySpec ← translateDo
        { context with loops := frame :: context.loops }
        bodyElements
      let step ← withHoisted context #[condition] fun residuals =>
        match binder? with
        | some binder =>
            `(if $binder:ident : $(residuals[0]!) then $bodySpec else $after)
        | none => `(if $(residuals[0]!) then $bodySpec else $after)
      let stateName := mkIdentFrom first `_moveSpecLoopState
      let stateTerm : TSyntax `term := ⟨stateName.raw⟩
      let unpacked ← unpackLoopState (state ++ handles) stateTerm step
      loopFixpoint context invariants (state ++ handles) recName stateName unpacked pack
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq) =>
      conditionalSpec context condition
        (Lean.Parser.Term.getDoElems thenBranch ++ rest) rest
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
      -- Both branches continue with the statements after the conditional.
      conditionalSpec context condition
        (Lean.Parser.Term.getDoElems thenBranch ++ rest)
        (Lean.Parser.Term.getDoElems elseBranch ++ rest)
  | `(doElem| if $condition:doIfCond then $thenBranch:doSeq $[else if $conditions:doIfCond then $branches:doSeq]* else $elseBranch:doSeq) =>
      -- An `else if` chain is nested conditionals.
      let mut elseSeq := elseBranch
      for (c, b) in (conditions.zip branches).reverse do
        let nested ← `(doElem| if $c:doIfCond then $b:doSeq else $elseSeq:doSeq)
        elseSeq := ⟨mkNode ``Lean.Parser.Term.doSeqIndent #[mkNullNode #[mkNullNode #[nested.raw, mkNullNode]]]⟩
      let rebuilt ← `(doElem| if $condition:doIfCond then $thenBranch:doSeq else $elseSeq:doSeq)
      translateDo context (#[rebuilt] ++ rest)
  | `(doElem| let $name:ident ← if $condition:doIfCond then $thenBranch:doSeq else $elseBranch:doSeq) =>
      -- The value of each branch binds the name; the rest follows in both.
      -- A branch ending in another conditional binds the name in each of
      -- its branches in turn.
      let rec bindLast (branch : TSyntax ``Lean.Parser.Term.doSeq) :
          CommandElabM (Array Lean.DoElem) := do
        let elems := Lean.Parser.Term.getDoElems branch
        let some last := elems.back? | throwErrorAt branch "empty conditional branch"
        match last with
        | `(doElem| if $innerCondition:doIfCond then $innerThen:doSeq else $innerElse:doSeq) =>
            let innerThen' ← bindLast innerThen
            let innerElse' ← bindLast innerElse
            let thenSeq : TSyntax ``Lean.Parser.Term.doSeq := ⟨mkNode ``Lean.Parser.Term.doSeqIndent
              #[mkNullNode (innerThen'.map fun e => mkNullNode #[e.raw, mkNullNode])]⟩
            let elseSeq : TSyntax ``Lean.Parser.Term.doSeq := ⟨mkNode ``Lean.Parser.Term.doSeqIndent
              #[mkNullNode (innerElse'.map fun e => mkNullNode #[e.raw, mkNullNode])]⟩
            let rebuilt ← `(doElem| if $innerCondition:doIfCond then $thenSeq:doSeq else $elseSeq:doSeq)
            pure (elems.pop.push rebuilt)
        | _ =>
            unless last.raw.isOfKind ``Lean.Parser.Term.doExpr do
              throwErrorAt last "a conditional bound to a name must end each branch in its value"
            let value : TSyntax `term := ⟨last.raw[0]⟩
            let bind ← `(doElem| let $name:ident ← $value:term)
            pure (elems.pop.push bind)
      conditionalSpec context condition
        ((← bindLast thenBranch) ++ rest)
        ((← bindLast elseBranch) ++ rest)
  | `(doElem| $value:term) =>
      if value.raw.isOfKind ``Lean.Parser.Term.do then
        let effect ← translateTerm { context with mutation? := none, mutationType? := none } value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $effect (fun _moveSpecIgnored => $nested))
      else if let some (call, mutations) ← mutableCallSpec? context value then
        if rest.isEmpty then
          -- The call's value is the statement's value.
          let output := mkIdentFrom value `_moveSpecCallValue
          let outputTerm : TSyntax `term := ⟨output.raw⟩
          let after ← finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
          let after ← writeMutableCallOutputs mutations outputTerm after
          `(Move.Semantics.Spec.bind $call (fun $output => $after))
        else
          bindMutableCall call mutations none (← translateRest rest)
      else if let some vectorCall ← vectorMutationCall? value then
        let some mutation := context.mutation?
          | throwErrorAt value "a mutating vector operation requires a live mutable vector borrow"
        let output := mkIdentFrom value `_moveSpecVectorMutationOutput
        let outputTerm : TSyntax `term := ⟨output.raw⟩
        let continuation ← if rest.isEmpty then
          finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
        else
          translateRest rest
        match vectorCall with
        | .insert reference index inserted =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector insert must use the currently borrowed vector"
            withHoisted context #[index, inserted] fun residuals =>
              `(Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.insertSpec $mutation $(residuals[0]!) $(residuals[1]!))
                  (fun $output =>
                    let $mutation := $output.2
                    $continuation))
        | .remove reference index =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector remove must use the currently borrowed vector"
            withHoisted context #[index] fun residuals =>
              `(Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.removeSpec $mutation $(residuals[0]!))
                  (fun $output =>
                    let $mutation := $output.2
                    $continuation))
        | .popBack reference =>
            unless reference.raw.isIdent && reference.raw.getId == mutation.getId do
              throwErrorAt reference "vector pop_back must use the currently borrowed vector"
            `(Move.Semantics.Spec.bind
                (Move.Semantics.Vector.popBackSpec $mutation)
                (fun $output =>
                  let $mutation := $output.2
                  $continuation))
        | call =>
            let operation ← vectorMutationSpec context mutation call
            `(Move.Semantics.Spec.bind $operation (fun $output =>
                let $mutation := $output.2
                $continuation))
      else if receiverStyleVectorMutation? value then
        throwErrorAt value
          "automatic source specifications require fully qualified `Move.Vector.insert` or `Move.Vector.remove`"
      else if value.raw.isOfKind ``Move.abortTerm then
        -- Abort is terminal, so this branch never executes its syntactic rest.
        translateTerm context value
      else if rest.isEmpty then
        translateTerm context value
      else
        -- An effectful statement followed by more: its result is discarded.
        let effect ← translateTerm context value
        let nested ← translateRest rest
        `(Move.Semantics.Spec.bind $effect (fun _moveSpecIgnored => $nested))
  | _ => throwErrorAt first
      "unsupported `do` statement in automatic source specification: {first.raw}"

/-- A statement conditional, with the statements each branch continues
into.  A plain condition is a source `if`; a dependent condition `h : c`
keeps the branch hypothesis in scope (a `dite`); a pattern condition
`let pat := e` is a match with a wildcard fall-through arm. -/
private partial def conditionalSpec (context : TranslationContext)
    (condition : TSyntax ``Lean.Parser.Term.doIfCond)
    (thenElements elseElements : Array Lean.DoElem) :
    CommandElabM (TSyntax `term) := do
  let translateBranch (elements : Array Lean.DoElem) :=
    if elements.isEmpty then emptyFinish context else translateDo context elements
  if condition.raw.isOfKind ``Lean.Parser.Term.doIfLet then
    -- Both `if let pat := e` and `if let pat ← e` sequence the scrutinee;
    -- the latter may abort before either branch is selected.
    let pattern : TSyntax `term := ⟨condition.raw[1]⟩
    let binding := condition.raw[2]
    let scrutinee : TSyntax `term := ⟨binding[1]⟩
    let scrutineeSpec ← expressionSpec context scrutinee
    let thenSpec ← translateBranch thenElements
    let elseSpec ← translateBranch elseElements
    let value := mkIdentFrom pattern `_moveSpecIfLetValue
    let valueTerm : TSyntax `term := ⟨value.raw⟩
    let selected ← `(match $valueTerm:term with
      | $pattern:term => $thenSpec
      | _ => $elseSpec)
    return ← `(Move.Semantics.Spec.bind $scrutineeSpec (fun $value => $selected))
  let (binder?, condition) ← plainCondition condition
  withHoisted context #[condition] fun residuals => do
    let condition := residuals[0]!
    let thenSpec ← translateBranch thenElements
    let elseSpec ← translateBranch elseElements
    match binder? with
    | some binder => `(if $binder:ident : $condition then $thenSpec else $elseSpec)
    | none => `(if $condition then $thenSpec else $elseSpec)

/-- Translate a multiple-return value containing mutable references. Mutable
components are transferred left-to-right, threading the updated lender tuple;
ordinary and immutable-reference components retain their value semantics. -/
private partial def finishMutationResult (context : TranslationContext)
    (value : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let values := flattenResultTerms value
  unless values.size == context.resultLeaves.size do
    throwErrorAt value
      "a reference-containing result must have the same flattened tuple shape as its declared type"
  let initialRoots := context.rootMutations.map fun root =>
    (⟨root.raw⟩ : TSyntax `term)
  let rec go (index : Nat) (roots results : Array (TSyntax `term)) :
      CommandElabM (TSyntax `term) := do
    if index == context.resultLeaves.size then
      let packedResult ← packCallArguments value.raw results
      let packedRoots ← packCallArguments value.raw roots
      return ← resolveMutationReturns context
        (← `(Move.Semantics.Spec.pure ($packedResult, $packedRoots)))
    let leaf := context.resultLeaves[index]!
    let component := values[index]!
    match leaf with
    | .mutable _ =>
        unless component.raw.isIdent do
          throwErrorAt component
            "a mutable-reference result component must be a live reference local"
        if context.transferredReturns.any (·.getId == component.raw.getId) then
          return ← go (index + 1) roots (results.push component)
        let some rootIndex := context.rootMutations.findIdx?
            (·.getId == component.raw.getId) | throwErrorAt component
          "a mutable-reference result component must derive from a live mutable-reference parameter"
        let output := mkIdentFrom component
          (Name.mkSimple s!"_moveSpecTransferredMutation{index}")
        let outputTerm : TSyntax `term := ⟨output.raw⟩
        let nextRoots := roots.set! rootIndex (← `($outputTerm.2))
        let tail ← go (index + 1) nextRoots (results.push (← `($outputTerm.1)))
        `(Move.Semantics.Spec.bind
            (Move.Semantics.transferMutation $(roots[rootIndex]!))
            (fun $output => $tail))
    | .value _ | .immutable _ =>
        let output := mkIdentFrom component
          (Name.mkSimple s!"_moveSpecResultComponent{index}")
        let tail ← go (index + 1) roots (results.push ⟨output.raw⟩)
        `(Move.Semantics.Spec.bind $(← expressionSpec context component)
            (fun $output => $tail))
  go 0 initialRoots #[]

private partial def translateTerm (context : TranslationContext)
    (term : TSyntax `term) : CommandElabM (TSyntax `term) := do
  if context.returnsMutation then
    match term with
    | `(($inner:term)) => return ← translateTerm context inner
    | `(pure $value:term) =>
        if context.resultLeaves.size == 1 && value.raw.isIdent then
          return ← finishMutationReturn context ⟨value.raw⟩
        if context.resultLeaves.size > 1 then
          return ← finishMutationResult context value
    | _ =>
        if context.resultLeaves.size == 1 && term.raw.isIdent then
          return ← finishMutationReturn context ⟨term.raw⟩
        if let some (call, mutations, true) ← moveCallSpec? context term then
          -- A reference-returning tail call is already in mutation form.
          -- Forward its returned loan unchanged and splice the callee's
          -- updated lender carriers back into this function's complete root
          -- tuple. This is the expression-form equivalent of
          -- `let returned ← call; pure returned`.
          for mutation in mutations do
            unless context.rootMutations.any (·.getId == mutation.getId) do
              throwErrorAt mutation
                "a forwarded mutable-reference result must derive directly from mutable-reference parameters"
          let output := mkIdentFrom term `_moveSpecTailCallOutput
          let outputTerm : TSyntax `term := ⟨output.raw⟩
          let mut roots : Array (TSyntax `term) := #[]
          for root in context.rootMutations do
            if let some mutationIndex := mutations.findIdx?
                (·.getId == root.getId) then
              let updated ← if mutations.size == 1 then
                `($outputTerm.2)
              else
                liftMacroM <| argumentProjection (← `($outputTerm.2))
                  mutationIndex mutations.size
              roots := roots.push updated
            else
              roots := roots.push ⟨root.raw⟩
          let packedRoots ← packCallArguments term.raw roots
          return ← resolveMutationReturns context
            (← `(Move.Semantics.Spec.bind $call (fun $output =>
              Move.Semantics.Spec.pure ($outputTerm.1, $packedRoots))))
  match term with
  | `(do $sequence:doSeq) =>
      translateDo context (Lean.Parser.Term.getDoElems sequence)
  | `(& $place:term) =>
      if let some (vector, index, fields) ←
          localVectorPlace? context.resources place then
        withHoisted context #[index] fun residuals => do
          let owner ← mutationValue context vector
          let element := mkIdentFrom place `_moveSpecVectorElement
          let elementTerm : TSyntax `term := ⟨element.raw⟩
          let focused ← projectPath elementTerm fields
          let result ← finish context (← `(Move.Semantics.Spec.pure $focused))
          `(Move.Semantics.Spec.bind
              (Move.Semantics.Vector.borrowElemSpec $owner $(residuals[0]!))
              (fun $element => $result))
      else if let some (owner, fields) ←
          localPlace? context.resources place then
        let ownerTerm ← mutationValue context owner
        let focused ← projectPath ownerTerm fields
        finish context (← `(Move.Semantics.Spec.pure $focused))
      else
        let (family, key, fields) ← globalPlace place
        let descriptor ← resourceFor context.resources family
        let owner := mkIdentFrom place `_moveSpecOwner
        let ownerTerm : TSyntax `term := ⟨owner.raw⟩
        let focused ← projectPath ownerTerm fields
        let result ← finish context (← `(Move.Semantics.Spec.pure $focused))
        `(Move.Semantics.Spec.bind
            (Move.Semantics.Resource.borrowSpec $descriptor $key)
            (fun $owner => $result))
  | `(abort $code:term) =>
      let codeSpec ← expressionSpec context code
      `(Move.Semantics.Spec.bind $codeSpec fun _moveSpecAbortCode =>
          Move.Semantics.Spec.abort
            (Move.Spec.abortCodeOf _moveSpecAbortCode))
  | `(pure $value:term) => finish context (← expressionSpec context value)
  | _ =>
      if let some (call, mutations) ← mutableCallSpec? context term then
        let output := mkIdentFrom term `_moveSpecCallValue
        let outputTerm : TSyntax `term := ⟨output.raw⟩
        let after ← finish context (← `(Move.Semantics.Spec.pure $outputTerm.1))
        let after ← writeMutableCallOutputs mutations outputTerm after
        return ← `(Move.Semantics.Spec.bind $call (fun $output => $after))
      let valueSpec ← expressionSpec context term
      finish context valueSpec

/-- A plain or dependent `if`/`while` condition: the optional hypothesis
binder and the condition term. -/
private partial def plainCondition (condition : TSyntax ``Lean.Parser.Term.doIfCond) :
    CommandElabM (Option (TSyntax `ident) × TSyntax `term) := do
  match condition with
  | `(doIfCond| $term:term) => pure (none, term)
  | `(doIfCond| $binder:ident : $term:term) => pure (some binder, term)
  | _ => throwErrorAt condition
      "this `if` condition is not supported by source specification generation"
end


end Move.Verify.Source

namespace Move.Spec

open Lean Elab Command Parser Command Macro
open scoped Move

private def nameSuffix? : Name → Option String
  | .str _ suffix => some suffix
  | _ => none

private def isRecursiveSourceSpec (env : Environment) (name : Name) : Bool :=
  nameSuffix? name == some "sourceSpec" &&
    match env.find? name with
    | some info => match info.value? (allowOpaque := true) with
      | some value =>
          let constants := value.getUsedConstants
          constants.contains ``Move.Semantics.Spec.fix ||
            constants.contains ``Move.Semantics.Spec.withInvariant ||
            constants.contains ``Move.Semantics.Spec.fixFamily
      | none => false
    | none => false

declare_syntax_cat moveSpecBinder
syntax "(" ident " : " term ")" : moveSpecBinder
syntax "{" ident " : " term "}" : moveSpecBinder
syntax "{" ident "}" : moveSpecBinder
syntax "[" term "]" : moveSpecBinder

declare_syntax_cat moveSpecResource
syntax ident " => " term : moveSpecResource

private def associatedName (function : TSyntax `ident) (suffix : Name) : TSyntax `ident :=
  mkIdentFrom function (function.getId ++ suffix)

private def applyArguments (function : TSyntax `ident)
    (arguments : Array (TSyntax `ident)) : MacroM (TSyntax `term) := do
  arguments.foldlM (init := ⟨function.raw⟩) fun application argument =>
    `($application $argument)

private def quantifyArguments (arguments : Array (TSyntax `ident))
    (types : Array (TSyntax `term)) (body : TSyntax `term) : MacroM (TSyntax `term) := do
  arguments.zip types |>.foldrM (init := body) fun (argument, type) body =>
    `(∀ ($argument : $type), $body)

private structure SpecParameters where
  arguments : Array (TSyntax `ident) := #[]
  types : Array (TSyntax `term) := #[]
  context : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
  mutableParameters : Array (TSyntax `ident × TSyntax `term) := #[]

private partial def mutableReferent? (type : TSyntax `term) : Option (TSyntax `term) :=
  match type with
  | `(($inner:term)) => mutableReferent? inner
  | `(&mut $referent:term) => some referent
  | _ => none

/-- Separate runtime arguments from generic proof context.  Every Move type
parameter contributes an internal `Inhabited` instance, mirroring the `fun`
command without exposing that compiler requirement in source contracts. -/
private def unpackSpecParameters
    (binders : Array (TSyntax `moveSpecBinder)) : MacroM SpecParameters := do
  let mut result : SpecParameters := {}
  for binder in binders do
    match binder with
    | `(moveSpecBinder|($argument:ident : $type:term)) =>
        let mut logicalType := type
        let mut mutableParameters := result.mutableParameters
        if let some referent := mutableReferent? type then
          logicalType := referent
          mutableParameters := mutableParameters.push (argument, referent)
        result := { result with
          arguments := result.arguments.push argument
          types := result.types.push logicalType
          mutableParameters }
    | `(moveSpecBinder|{$typeName:ident : $type:term}) =>
        let typeBinder ← `(bracketedBinder| {$typeName : $type})
        let inhabitedBinder ← `(bracketedBinder| [Inhabited $typeName])
        result := { result with
          context := result.context.push typeBinder |>.push inhabitedBinder }
    | `(moveSpecBinder|{$typeName:ident}) =>
        let typeBinder ← `(bracketedBinder| {$typeName : Type})
        let inhabitedBinder ← `(bracketedBinder| [Inhabited $typeName])
        result := { result with
          context := result.context.push typeBinder |>.push inhabitedBinder }
    | `(moveSpecBinder|[$instanceType:term]) =>
        let instanceBinder ← `(bracketedBinder| [$instanceType])
        result := { result with context := result.context.push instanceBinder }
    | _ => Macro.throwErrorAt binder "invalid specification binder"
  pure result

private def quantifyContext
    (binders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder))
    (body : TSyntax `term) : MacroM (TSyntax `term) :=
  binders.foldrM (init := body) fun binder body => `(∀ $binder, $body)

/-- Apply a function's relational semantics to the type parameters of its
specification context by name: a parameter no argument determines (the `T`
of a body `existsAt (Vault T) a`) would otherwise be left to inference. -/
private def applyTypeParameters (function : TSyntax `term)
    (binders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder)) :
    MacroM (TSyntax `term) := do
  -- One application node: named arguments resolve against the head's
  -- binders, and a nested application would already have instantiated the
  -- remaining implicit ones.
  let mut namedArguments : Array (TSyntax `term) := #[]
  for binder in binders do
    match binder with
    | `(bracketedBinder| {$name:ident : $_}) =>
        let argument ← `(Lean.Parser.Term.namedArgument| ($name := $name))
        namedArguments := namedArguments.push ⟨argument.raw⟩
    | _ => pure ()
  if namedArguments.isEmpty then return function
  `($function $namedArguments*)

private partial def findResult? (stx : Syntax) : Option Syntax :=
  if stx.isIdent && stx.getId == `result then
    some stx
  else
    stx.getArgs.findSome? findResult?

private partial def bindResult (stx binder : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == `result then
    binder
  else
    stx.setArgs (stx.getArgs.map (bindResult · binder))

private partial def bindImplicit (name : Name) (stx binder : Syntax) : Syntax :=
  if stx.isIdent && stx.getId == name then
    binder
  else
    stx.setArgs (stx.getArgs.map (bindImplicit name · binder))

private def specFieldParts : Name → List String
  | .anonymous => []
  | .str baseName part => specFieldParts baseName ++ [part]
  | .num _ _ => []

private def specIdentifierExtends (root candidate : Name) : Bool :=
  let rootParts := specFieldParts root
  let candidateParts := specFieldParts candidate
  !rootParts.isEmpty && candidateParts.take rootParts.length == rootParts

/-- In a mutable-reference postcondition, the parameter name denotes the
post-call referent while `old(parameter)` denotes its pre-call value. -/
private partial def rewriteMutablePost (parameter : TSyntax `ident)
    (finalValue : TSyntax `term) (stx : Syntax) : MacroM Syntax := do
  let term : TSyntax `term := ⟨stx⟩
  match term with
  | `(old($value:term)) =>
      if value.raw.isIdent && value.raw.getId == parameter.getId then
        return parameter.raw
  | _ => pure ()
  if stx.isIdent && specIdentifierExtends parameter.getId stx.getId then
    let parameterParts := specFieldParts parameter.getId
    let suffix := (specFieldParts stx.getId).drop parameterParts.length
    let mut rewritten := finalValue
    for field in suffix do
      let field := mkIdentFrom stx (Name.mkSimple field)
      rewritten ← `($rewritten.$field)
    return rewritten.raw
  let args ← stx.getArgs.mapM (rewriteMutablePost parameter finalValue)
  pure (stx.setArgs args)

private def clauseLambda (arguments : Array (TSyntax `ident))
    (extra : Array (Name × TSyntax `ident)) (clause : TSyntax `term) :
    MacroM (TSyntax `term) := do
  let rewritten := extra.foldl (init := clause.raw) fun result (name, binder) =>
    bindImplicit name result binder.raw
  let mut body : TSyntax `term := ⟨rewritten⟩
  for (_, binder) in extra.reverse do
    body ← `(fun $binder => $body)
  Move.Verify.Source.unpackArguments arguments body

/-- The frame condition of a specification: every resource family the
function uses is unchanged except at the addresses the `modifies` clause
lists.  Without a clause no global memory changes at all, which the abstract
state expresses directly. -/
private def frameCondition (resourceTypes : Array Move.Verify.Source.Family)
    (world initial final : TSyntax `term)
    (clause? : Option Syntax) : CommandElabM (Option (TSyntax `term)) := do
  let some clause := clause?
    | return some (← `($final = $initial))
  let mut targets : Array (String × Array (TSyntax `term)) := #[]
  -- `*`: the families the clause does not list are unconstrained.
  let mut loose := false
  for target in clause[1].getSepArgs do
    if target.isOfKind `Move.Spec.modifiesAny then
      loose := true
      continue
    let (typeStx, address?) : Syntax × Option (TSyntax `term) :=
      -- The target kinds are declared below, with the `spec` syntax.
      if target.isOfKind `Move.Spec.modifiesAddress then (target[0], some ⟨target[2]⟩)
      else if target.isOfKind `Move.Spec.modifiesFamily then (target[0], none)
      else if target.isOfKind `Move.Spec.modifiesGenericAddress then (target[1], some ⟨target[4]⟩)
      else (target[1], none)
    let some family ← Move.Verify.Source.familyOfTerm? typeStx
      | throwErrorAt target "`modifies` expects a resource family"
    let family := family.key
    match targets.findIdx? (·.1 == family) with
    | some index =>
        let (name, addresses) := targets[index]!
        let addresses := match address? with
          | none => #[]
          | some address =>
              if addresses.isEmpty then #[] else addresses.push address
        targets := targets.set! index (name, addresses)
    | none =>
        targets := targets.push (family, address?.toArray)
  let mut conjuncts : Array (TSyntax `term) := #[]
  for family in resourceTypes.filter (·.concrete) do
    let addresses? := (targets.find? (·.1 == family.key)).map (·.2)
    if let some addresses := addresses? then
      if addresses.isEmpty then
        continue
    else if loose then
      -- Not listed, and the frame is loose: unconstrained.
      continue
    let resourceType := family.term
    let address := mkIdentFrom resourceType `_moveSpecAddress
    let addressTerm : TSyntax `term := ⟨address.raw⟩
    let mut body ←
      `(Move.Semantics.ResourceStore.lookup (State := $world)
            (Value := $resourceType) $final $addressTerm =
          Move.Semantics.ResourceStore.lookup (State := $world)
            (Value := $resourceType) $initial $addressTerm)
    for modified in (addresses?.getD #[]).reverse do
      body ← `($addressTerm ≠ $modified → $body)
    conjuncts := conjuncts.push (← `(∀ $address:ident, $body))
  if conjuncts.isEmpty then
    return none
  let mut frame := conjuncts[0]!
  for index in [1:conjuncts.size] do
    frame ← `($frame ∧ $(conjuncts[index]!))
  return some frame

/-- Where a contract excuses its postcondition.  A written `may_abort`
component is used directly; otherwise it is the states in which the declared
abort predicate admits some outcome. -/
private def mayAbortLambdaFor (arguments : Array (TSyntax `ident))
    (initial : TSyntax `ident) (abortsLambda : TSyntax `term)
    (written? : Option (TSyntax `term)) : MacroM (TSyntax `term) := do
  match written? with
  | some condition => clauseLambda arguments #[( `initial, initial)] condition
  | none =>
      `(fun _moveSpecArgs _moveSpecInitial =>
          ∃ _moveSpecAbortCode,
            $abortsLambda _moveSpecArgs _moveSpecInitial _moveSpecAbortCode)

/-- Where a contract requires an abort: a written `must_abort` component, or
`False` (no declared condition is sufficient). -/
private def mustAbortLambdaFor (arguments : Array (TSyntax `ident))
    (initial : TSyntax `ident) (written? : Option (TSyntax `term)) : MacroM (TSyntax `term) := do
  match written? with
  | some condition => clauseLambda arguments #[( `initial, initial)] condition
  | none => `(fun _moveSpecArgs _moveSpecInitial => False)

/-- Further invariant clauses of a data specification; they are conjoined. -/
declare_syntax_cat moveExtraInvariant
scoped syntax ";" "invariant " term : moveExtraInvariant

/-- A data invariant: every value of the named struct or enum satisfies the
declared conditions, and carries their proof.  `this` denotes the value being
constrained; a leading `.field` is the spelling to use, and `this` is only needed where the value as a whole is.  Clauses read
like the other spec blocks and may be repeated:

```lean
spec Map {K} {V} where
  invariant Model.SortedEntries .entries.toList
```

The declaration is consumed by the enclosing `module`, which attaches
the invariant to the type it names. -/
scoped syntax (name := dataInvariantSpec)
  "spec " ident moveSpecBinder* " where "
    "invariant " term moveExtraInvariant* : command

/-- A global-invariant predicate, quantified over an address.  A *regular*
invariant `∀ a, … R[a] …` constrains the current state; an *update*
invariant `update ∀ a, … old(R[a]) … R[a] …` relates the pre- and
post-state of a change.  The `existsAt<R>(a)` guard is implicit: the address
ranges over the stored resources, so absent addresses are unconstrained. -/
declare_syntax_cat moveGlobalInvariant
scoped syntax (name := globalInvariantRegular)
  "∀ " ident ", " term : moveGlobalInvariant
scoped syntax (name := globalInvariantUpdate)
  "update " "∀ " ident ", " term : moveGlobalInvariant

/-- Further clauses of a global-invariant spec. -/
declare_syntax_cat moveExtraGlobalInvariant
scoped syntax ";" "invariant " moveGlobalInvariant : moveExtraGlobalInvariant

/-- A global invariant over the resource state, re-established at each change
(not at function end).  A regular invariant is assumed at reads and asserted
at writes; an `update` invariant is asserted at each write only:

```lean
spec module where
  invariant ∀ a, 0 < Counter[a].value.toNat;
  invariant update ∀ a, old(Counter[a]).value ≤ Counter[a].value
```

Consumed by the enclosing `module`. -/
scoped syntax (name := globalInvariantSpec) (priority := high)
  "spec " &"module" " where "
    "invariant " moveGlobalInvariant moveExtraGlobalInvariant* : command

/-- Whether `name` occurs as an identifier anywhere in `stx`. -/
partial def occursIdentifier (name : Name) (stx : Syntax) : Bool :=
  (stx.isIdent && stx.getId == name) ||
    stx.getArgs.any (occursIdentifier name)

/-- Whether an `old(…)` pre-state observation occurs anywhere in `stx`. -/
partial def occursOld (stx : Syntax) : Bool :=
  stx.getKind == ``oldResourceTerm || stx.getArgs.any occursOld

/-- Rewrite the resource places of a global-invariant body over the bound
address `addr` into a state predicate: `R[addr]` becomes `get (Value := R)
state addr` and `existsAt<R>(addr)` becomes `contains (Value := R) state addr`,
where `state` is the post-state (`old(…)` selects the pre-state).  Returns the
rewritten body, the families accessed *by value* with their `old`-ness (needing
an existence guard), and every family the body mentions. -/
partial def rewriteStatePlaces (addr : Name) (preState postState addrIdent : Syntax)
    (inOld : Bool) (body : Syntax) :
    MacroM (Syntax × Array (Syntax × Bool) × Array Syntax) := do
  let state := if inOld then preState else postState
  let stateT : TSyntax `term := ⟨state⟩
  let addrT : TSyntax `term := ⟨addrIdent⟩
  if body.getKind == ``oldResourceTerm && body.getNumArgs == 3 then
    rewriteStatePlaces addr preState postState addrIdent true body[1]
  else if body.getKind == `«term__[_]» && body.getNumArgs == 4 &&
      body[2].isIdent && body[2].getId == addr then
    let famT : TSyntax `term := ⟨body[0]⟩
    let repl ← `(Move.Semantics.ResourceStore.get (Value := $famT) $stateT $addrT)
    return (repl.raw, #[(body[0], inOld)], #[body[0]])
  else if body.getKind == ``resourceExistsTerm && body.getNumArgs == 5 &&
      body[3].isIdent && body[3].getId == addr then
    let famT : TSyntax `term := ⟨body[1]⟩
    let repl ← `(Move.Semantics.ResourceStore.contains (Value := $famT) $stateT $addrT)
    return (repl.raw, #[], #[body[1]])
  else
    let mut valueAccess : Array (Syntax × Bool) := #[]
    let mut allFamilies : Array Syntax := #[]
    let mut args : Array Syntax := #[]
    for child in body.getArgs do
      let (child', va, fs) ← rewriteStatePlaces addr preState postState addrIdent inOld child
      args := args.push child'; valueAccess := valueAccess ++ va; allFamilies := allFamilies ++ fs
    return (body.setArgs args, valueAccess, allFamilies)

/-- Desugar one global-invariant clause into `(isUpdate, families, addr,
atBody)`.  `atBody` is the *per-address* predicate `guard → body` over the
fixed binders `_moveSpecState` (regular) or `_moveSpecPre`/`_moveSpecPost`
(update) and the address `addr`; the full invariant is `∀ addr, atBody`.
Factoring out `atBody` lets the generated reestablishment lemmas name the
changed address's obligation without unfolding the whole quantifier.  The guard
requires existence of each value-accessed family at `addr`.  `families` is
every family the clause mentions — the invariant is registered under each. -/
def elabGlobalInvariantClause (clause : Syntax) :
    MacroM (Bool × Array (TSyntax `term) × TSyntax `ident × TSyntax `term) := do
  let isUpdate := clause.isOfKind ``globalInvariantUpdate
  let offset := if isUpdate then 1 else 0
  let addr := clause[offset + 1]
  let bodyStx := clause[offset + 3]
  unless addr.isIdent do
    Macro.throwErrorAt clause "a global invariant must bind an address, `∀ a, …`"
  unless isUpdate do
    if occursOld bodyStx then
      Macro.throwErrorAt clause
        "a regular global invariant may not use `old`; write `invariant update ∀ a, …`"
  let addrIdent := mkIdentFrom bodyStx `_moveSpecAddr
  let preState := mkIdentFrom bodyStx `_moveSpecPre
  let postState := mkIdentFrom bodyStx (if isUpdate then `_moveSpecPost else `_moveSpecState)
  let (rewritten, valueAccess, allFamilies) ←
    rewriteStatePlaces addr.getId preState.raw postState.raw addrIdent.raw false bodyStx
  let mut families : Array (TSyntax `term) := #[]
  for f in allFamilies do
    unless families.any (·.raw == f) do families := families.push ⟨f⟩
  if families.isEmpty then
    Macro.throwErrorAt clause
      "a global invariant must reference a stored resource `R[a]` or `existsAt<R>(a)`"
  if occursIdentifier addr.getId rewritten then
    Macro.throwErrorAt addr
      "the quantified address may appear only inside resource places `R[a]`"
  -- Existence guards for value-accessed families (deduplicated by family and
  -- `old`-ness), so `R[a].field` constrains only addresses where `R` exists.
  let mut guards : Array (TSyntax `term) := #[]
  let mut seen : Array (Syntax × Bool) := #[]
  for (fam, fromOld) in valueAccess do
    unless seen.any (fun (f, o) => f == fam && o == fromOld) do
      seen := seen.push (fam, fromOld)
      let famT : TSyntax `term := ⟨fam⟩
      let stateT : TSyntax `term := ⟨(if fromOld then preState else postState).raw⟩
      let addrT : TSyntax `term := ⟨addrIdent.raw⟩
      guards := guards.push
        (← `(Move.Semantics.ResourceStore.contains (Value := $famT) $stateT $addrT))
  let bodyTerm : TSyntax `term := ⟨rewritten⟩
  let guarded ← if h : guards.size = 0 then pure bodyTerm else do
    let mut conjunction := guards[guards.size - 1]!
    for i in [1:guards.size] do
      conjunction ← `($(guards[guards.size - 1 - i]!) ∧ $conjunction)
    `($conjunction → $bodyTerm)
  return (isUpdate, families, addrIdent, guarded)

@[macro globalInvariantSpec] def expandGlobalInvariantSpec : Macro := fun stx =>
  Macro.throwErrorAt stx
    "a global invariant must be declared inside a `module`"

@[macro dataInvariantSpec] def expandDataInvariantSpec : Macro := fun stx =>
  Macro.throwErrorAt stx
    ("a data invariant must be declared inside the `module` which " ++
      "declares its type")

/-- Total primitives of the relational semantics, with the lemma that says
so.  Every step of the discharger below applies exactly one of these, or one
combinator rule, chosen by the head symbol of the goal — it never lets
unification unfold a body. -/
private def totalPrimitiveLemma : Name → Option Name
  | ``Move.Semantics.Spec.pure => some ``Move.Semantics.Spec.pure_undefined
  | ``Move.Semantics.Spec.abort => some ``Move.Semantics.Spec.abort_undefined
  | ``Move.Semantics.Spec.bottom => some ``Move.Semantics.Spec.bottom_undefined
  | ``Move.Semantics.Spec.get => some ``Move.Semantics.Spec.total_get
  | ``Move.Semantics.Spec.set => some ``Move.Semantics.Spec.total_set
  | ``Move.Semantics.Spec.modify => some ``Move.Semantics.Spec.total_modify
  | ``Move.Semantics.Spec.ofTxn => some ``Move.Semantics.Spec.total_ofTxn
  | ``Move.Semantics.Checked.addSpec => some ``Move.Semantics.Checked.total_addSpec
  | ``Move.Semantics.Checked.subSpec => some ``Move.Semantics.Checked.total_subSpec
  | ``Move.Semantics.Checked.mulSpec => some ``Move.Semantics.Checked.total_mulSpec
  | ``Move.Semantics.Checked.divSpec => some ``Move.Semantics.Checked.total_divSpec
  | ``Move.Semantics.Checked.modSpec => some ``Move.Semantics.Checked.total_modSpec
  | ``Move.Semantics.Checked.shlSpec => some ``Move.Semantics.Checked.total_shlSpec
  | ``Move.Semantics.Checked.shrSpec => some ``Move.Semantics.Checked.total_shrSpec
  | ``Move.Semantics.Checked.castSpec => some ``Move.Semantics.Checked.total_castSpec
  | ``Move.Semantics.Resource.containsSpec =>
      some ``Move.Semantics.Resource.total_containsSpec
  | ``Move.Semantics.Resource.borrowSpec =>
      some ``Move.Semantics.Resource.total_borrowSpec
  | ``Move.Semantics.Resource.moveFromSpec =>
      some ``Move.Semantics.Resource.total_moveFromSpec
  | ``Move.Semantics.Resource.moveToSpec =>
      some ``Move.Semantics.Resource.total_moveToSpec
  | ``Move.Semantics.Vector.borrowElemSpec =>
      some ``Move.Semantics.Vector.total_borrowElemSpec
  | ``Move.Semantics.variantFieldSpec =>
      some ``Move.Semantics.total_variantFieldSpec
  | ``Move.Semantics.Vector.setSpec => some ``Move.Semantics.Vector.total_setSpec
  | ``Move.Semantics.Vector.insertSpec =>
      some ``Move.Semantics.Vector.insertSpec_undefined
  | ``Move.Semantics.Vector.removeSpec =>
      some ``Move.Semantics.Vector.removeSpec_undefined
  | ``Move.Semantics.Vector.popBackSpec =>
      some ``Move.Semantics.Vector.popBackSpec_undefined
  | ``Move.Semantics.Vector.swapSpec => some ``Move.Semantics.Vector.swapSpec_undefined
  | ``Move.Semantics.Vector.swapRemoveSpec =>
      some ``Move.Semantics.Vector.swapRemoveSpec_undefined
  | ``Move.Semantics.Vector.appendSpec => some ``Move.Semantics.Vector.appendSpec_undefined
  | ``Move.Semantics.Vector.reverseSpec => some ``Move.Semantics.Vector.reverseSpec_undefined
  | ``Move.Semantics.Vector.reverseSliceSpec =>
      some ``Move.Semantics.Vector.reverseSliceSpec_undefined
  | ``Move.Semantics.Vector.trimSpec => some ``Move.Semantics.Vector.trimSpec_undefined
  | ``Move.Semantics.Vector.trimReverseSpec =>
      some ``Move.Semantics.Vector.trimReverseSpec_undefined
  | ``Move.Semantics.Vector.rotateSpec => some ``Move.Semantics.Vector.rotateSpec_undefined
  | ``Move.Semantics.Vector.rotateSliceSpec =>
      some ``Move.Semantics.Vector.rotateSliceSpec_undefined
  | ``Move.Semantics.Vector.destroyEmptySpec =>
      some ``Move.Semantics.Vector.destroyEmptySpec_undefined
  | _ => none

/-- The computation a well-definedness goal `¬ X.undefined s` is about. -/
private def definednessSubject? (target : Expr) : Option Expr := do
  guard (target.isAppOfArity ``Not 1)
  let inner := target.appArg!
  guard (inner.isAppOfArity ``Move.Semantics.Spec.undefined 4)
  return inner.getArg! 2

/-- A hypothesis `∀ a s, ¬ (recursive a).undefined s` for the recursive call
of an enclosing fixed point. -/
private def recursiveDefinedHypothesis? (recursive : FVarId) :
    Lean.Elab.Tactic.TacticM (Option FVarId) := do
  let lctx ← getLCtx
  for decl in lctx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let isHypothesis ← Lean.Meta.forallTelescopeReducing type fun _ body => do
      match definednessSubject? body with
      | some subject =>
          pure (subject.getAppFn == .fvar recursive && subject.getAppNumArgs == 1)
      | none => pure false
    if isHypothesis then return some decl.fvarId
  return none

/-- Establish that a source computation owes no proof, by its structure: the
primitives are total by definition, the combinators preserve it, and the only
obligation that survives is the data invariant of a created value, which is
left as the goal `Invariant`.  Each step is chosen by the goal's head symbol
and applies one lemma, so the cost is linear in the body and a body the
discharger does not understand fails immediately with its head symbol. -/
syntax (name := specDefined) "spec_defined" : tactic

private partial def dischargeDefined (fuel : Nat := 10000) :
    Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  if fuel == 0 then throwError "spec_defined: body too large"
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← instantiateMVars (← goal.getType)
  let some subject := definednessSubject? target
    | throwError "spec_defined: expected a goal `¬ X.undefined s`, got{indentExpr target}"
  let recurse := dischargeDefined (fuel - 1)
  let step (stx : TSyntax `tactic) : Lean.Elab.Tactic.TacticM Unit := do
    Lean.Elab.Tactic.evalTactic stx
  let onEveryGoal (action : Lean.Elab.Tactic.TacticM Unit) :
      Lean.Elab.Tactic.TacticM Unit := do
    let produced ← Lean.Elab.Tactic.getGoals
    let mut remaining := #[]
    for produced in produced do
      Lean.Elab.Tactic.setGoals [produced]
      action
      remaining := remaining ++ (← Lean.Elab.Tactic.getGoals)
    Lean.Elab.Tactic.setGoals remaining.toList
  match subject.getAppFn with
  | .lam .. | .letE .. | .mdata .. | .proj .. =>
      step (← `(tactic| dsimp only))
      Lean.Elab.Tactic.withMainContext do
        let after ← instantiateMVars (← (← Lean.Elab.Tactic.getMainGoal).getType)
        if after == target then
          throwError "spec_defined: cannot reduce{indentExpr subject}"
        recurse
  | .fvar recursive =>
      let some hypothesis ← recursiveDefinedHypothesis? recursive
        | throwError "spec_defined: no well-definedness hypothesis for recursive call{indentExpr subject}"
      step (← `(tactic| exact $(mkIdent (← hypothesis.getUserName)) _ _))
  | .const name _ =>
      if name == ``Move.Semantics.Spec.bind then
        step (← `(tactic| refine Move.Semantics.Spec.bind_defined ?_ (fun _ _ _ => ?_)))
        onEveryGoal recurse
      else if name == ``Move.Semantics.Spec.fix then
        step (← `(tactic| refine Move.Semantics.Spec.fix_defined
          (fun _recursive _recursiveDefined _ _ => ?_)))
        recurse
      else if name == ``Move.Semantics.withMutation then
        step (← `(tactic| refine Move.Semantics.withMutation_defined (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Resource.withBorrowMutSpec then
        step (← `(tactic| refine Move.Semantics.Resource.withBorrowMutSpec_defined
          (fun _ _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Resource.withBorrowMutFocusSpec then
        step (← `(tactic| refine Move.Semantics.Resource.withBorrowMutFocusSpec_defined
          (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Vector.withBorrowElemMutSpec then
        step (← `(tactic| refine Move.Semantics.Vector.withBorrowElemMutSpec_defined
          (fun _ => ?_)))
        recurse
      else if name == ``Move.Semantics.Spec.certified then
        -- The one genuine obligation: the data invariant of a created value.
        step (← `(tactic| rw [Move.Semantics.Spec.certified_defined_iff]))
      else if name == ``ite || name == ``dite ||
          (← Lean.Meta.isMatcher name) then
        step (← `(tactic| split))
        onEveryGoal recurse
      else if let some lemma := totalPrimitiveLemma name then
        step (← `(tactic| apply $(mkIdent lemma)))
      else
        throwError "spec_defined: no well-definedness rule for `{name}`"
  | other =>
      throwError "spec_defined: unexpected head{indentExpr other}"

@[tactic specDefined] private def elabSpecDefined : Lean.Elab.Tactic.Tactic :=
  fun _ => dischargeDefined

/-- Discharge the data invariant of a value being created.  Creation is the
only place an invariant is owed, so this runs wherever a literal of a
certified type is elaborated; when it cannot close the goal the error points
at the literal. -/
syntax "move_invariant" : tactic
macro_rules
  | `(tactic| move_invariant) =>
    `(tactic| first
        | trivial
        | rfl
        | decide
        | assumption
        -- Every alternative must close the goal, or `first` would stop at
        -- a partial simplification and report it as the failure.
        | (simp [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat,
            Nat.reducePow, Nat.reduceMod]
           done)
        | (simp [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat,
            Nat.reducePow, Nat.reduceMod]
           omega)
        | (simp_all [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat, Nat.reducePow,
            Nat.reduceMod]
           done)
        | (simp_all [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat, Nat.reducePow,
            Nat.reduceMod]
           omega)
        -- Quantified invariants (`∀ i j, i < len → …`): introduce, then
        -- arithmetic over the simplified bounds.
        | (simp [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat,
            Nat.reducePow, Nat.reduceMod]
           intros
           simp_all [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat]
           omega)
        | (intros
           simp_all [move_invariant_norm, move_norm, Move.UInt.toInt_eq_toNat,
             Nat.reducePow, Nat.reduceMod]
           omega)
        | fail "cannot establish the data invariant of this value here (if this is a pattern of a certified enum, bind the proof with a trailing `_`)")

/-- Rewrite the leading-dot field abbreviations of an invariant condition to
projections of the constrained value. -/
partial def bindInvariantValue (this : TSyntax `ident) (condition : Syntax) :
    Syntax :=
  if condition.isOfKind ``Lean.Parser.Term.dotIdent then
    mkNode ``Lean.Parser.Term.proj #[this.raw, mkAtom ".", condition[1]]
  else if condition.isOfKind ``Lean.Parser.Term.matchAlt then
    -- The patterns of a `match this with | .ctor …` are constructor names,
    -- not field abbreviations: rewrite only the right-hand side.
    let last := condition.getNumArgs - 1
    condition.setArg last (bindInvariantValue this condition[last])
  else
    condition.setArgs (condition.getArgs.map (bindInvariantValue this))

/-- Data invariants are assembled by the enclosing-module macro rather than
the command elaborator. Clean length/size expressions use mathematical
integers, while legacy invariants which explicitly select `toNat`/`toInt`
retain their ordinary Lean relation and arithmetic. -/
partial def rewriteInvariantClause (condition : Syntax)
    (mathematical : Bool := false) : MacroM Syntax := do
  let mathematicalSurface (term : TSyntax `term) : Bool :=
    let text := term.raw.reprint.getD term.raw.prettyPrint.pretty
    (text.contains ".length" || text.contains ".size") && !text.contains ".toList" &&
      !text.contains ".toNat" && !text.contains ".toInt"
  let rewrite (term : TSyntax `term) (mathematical := mathematical) :
      MacroM (TSyntax `term) := do
    pure ⟨← rewriteInvariantClause term.raw mathematical⟩
  let term : TSyntax `term := ⟨condition⟩
  match term with
  | `($left:term + $right:term) =>
      let left ← rewrite left
      let right ← rewrite right
      if mathematical then return (← `(Move.Spec.intAdd $left $right)).raw
      else return (← `($left + $right)).raw
  | `($left:term - $right:term) =>
      let left ← rewrite left
      let right ← rewrite right
      if mathematical then return (← `(Move.Spec.intSub $left $right)).raw
      else return (← `($left - $right)).raw
  | `($left:term * $right:term) =>
      let left ← rewrite left
      let right ← rewrite right
      if mathematical then return (← `(Move.Spec.intMul $left $right)).raw
      else return (← `($left * $right)).raw
  | `($left:term / $right:term) =>
      let left ← rewrite left
      let right ← rewrite right
      if mathematical then return (← `(Move.Spec.intDiv $left $right)).raw
      else return (← `($left / $right)).raw
  | `($left:term % $right:term) =>
      let left ← rewrite left
      let right ← rewrite right
      if mathematical then return (← `(Move.Spec.intMod $left $right)).raw
      else return (← `($left % $right)).raw
  | `(-$value:term) =>
      let value ← rewrite value
      if mathematical then return (← `(Move.Spec.intNeg $value)).raw
      else return (← `(-$value)).raw
  | `($value:term <<< $amount:term) =>
      let value ← rewrite value
      let amount ← rewrite amount
      if mathematical then return (← `(Move.Spec.intShiftLeft $value $amount)).raw
      else return (← `($value <<< $amount)).raw
  | `($value:term >>> $amount:term) =>
      let value ← rewrite value
      let amount ← rewrite amount
      if mathematical then return (← `(Move.Spec.intShiftRight $value $amount)).raw
      else return (← `($value >>> $amount)).raw
  | `($left:term = $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(Move.Spec.logicalEq $left $right)).raw
      else return (← `($left = $right)).raw
  | `($left:term ≠ $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(¬Move.Spec.logicalEq $left $right)).raw
      else return (← `($left ≠ $right)).raw
  | `($left:term < $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(Move.Spec.logicalLT $left $right)).raw
      else return (← `($left < $right)).raw
  | `($left:term > $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(Move.Spec.logicalLT $right $left)).raw
      else return (← `($left > $right)).raw
  | `($left:term ≤ $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(Move.Spec.logicalLE $left $right)).raw
      else return (← `($left ≤ $right)).raw
  | `($left:term ≥ $right:term) =>
      let mathematical := mathematical || mathematicalSurface left || mathematicalSurface right
      let left ← rewrite left mathematical
      let right ← rewrite right mathematical
      if mathematical then return (← `(Move.Spec.logicalLE $right $left)).raw
      else return (← `($left ≥ $right)).raw
  | _ =>
      return condition.setArgs (← condition.getArgs.mapM fun child =>
        rewriteInvariantClause child mathematical)

/-- A contract stating only a postcondition.  For a *pure* function it is a
value predicate over the function applied to its arguments.  For an *effectful*
function it routes through the effectful path (trivial precondition and abort
behavior) so resource observations — `R[a]`, `existsAt<R>`, `old` — in the
postcondition are interpreted against global state. -/
scoped syntax (name := ensuresOnlySpec) "spec " ident moveSpecBinder* " where "
    "ensures " term : command

/-- Build the pure value contract `def f.contract : Prop := ∀ args, ensures`. -/
private def pureEnsuresContract (function : TSyntax `ident)
    (binders : Array (TSyntax `moveSpecBinder)) (postcondition : TSyntax `term) :
    MacroM (TSyntax `command) := do
  let contractName := associatedName function `contract
  let parameters ← unpackSpecParameters binders
  let application ← applyArguments function parameters.arguments
  let result := findResult? postcondition.raw |>.getD (mkIdentFrom postcondition `result)
  let ensured : TSyntax `term := ⟨bindResult postcondition.raw result⟩
  let result : TSyntax `ident := ⟨result⟩
  let body ← `((fun $result => $ensured) $application)
  let contract ← quantifyArguments parameters.arguments parameters.types body
  let contract ← quantifyContext parameters.context contract
  let command : TSyntax `command ← `(def $contractName : Prop := $contract)
  return command

/-- One global location a specification is allowed to change: a resource
family, optionally narrowed to one address. -/
declare_syntax_cat moveModifiesTarget
scoped syntax (name := modifiesAddress) ident "[" term "]" : moveModifiesTarget
scoped syntax (name := modifiesFamily) ident : moveModifiesTarget
/-- A generic family, written with its type arguments: `(Vault T)[addr]`. -/
scoped syntax (name := modifiesGenericAddress) "(" term ")" "[" term "]" : moveModifiesTarget
scoped syntax (name := modifiesGenericFamily) "(" term ")" : moveModifiesTarget
/-- The loose frame: every family the clause does not list is unconstrained.
`modifies R[a], *` closes `R` at `a` and leaves the rest open; `modifies *`
alone states no frame at all — the reading the Move Prover gives a
specification without `modifies` clauses. -/
scoped syntax (name := modifiesAny) "*" : moveModifiesTarget

/-- The global memory a function may change.  Everything else is unchanged,
so contracts never state a frame condition explicitly.  An omitted clause
means the function changes no global memory at all; the `*` target makes the
frame loose for the families the clause does not list. -/
declare_syntax_cat moveModifiesClause
scoped syntax "modifies " moveModifiesTarget,+ ";" : moveModifiesClause

/-- A declarative contract for an effectful Move source function. The
function's relational semantics is generated from its retained `fun` body.
`initial`, `final`, `result`, and `abortCode` are implicit clause binders.
Resource descriptors only define the typed representation of global storage;
they do not restate the function's behavior. -/
scoped syntax (name := effectfulSourceSpec)
  "spec " ident moveSpecBinder* " on " term
    " using " "[" moveSpecResource,* "]" " where "
    "requires " term ";"
    (moveModifiesClause)?
    "ensures " term ";"
    "aborts " term (";" "may_abort " term)? (";" "must_abort " term)? : command

/-- User-facing effectful contract. The global state and one typed store
instance per borrowed resource are implicit and universally quantified. -/
scoped syntax (name := inferredEffectfulSourceSpec)
  "spec " ident moveSpecBinder* " where "
    "requires " term ";"
    (moveModifiesClause)?
    "ensures " term ";"
    "aborts " term (";" "may_abort " term)? (";" "must_abort " term)? : command

/-- Omitted effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " on " world:term
    " using " "[" resources:moveSpecResource,* "]" " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* on $world using [$resources,*] where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortCondition)

/-- Omitted inferred effectful preconditions mean `True`. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term ";"
    "aborts " abortCondition:term : command =>
  `(spec $function $binder* where
      requires True;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortCondition)

/-- A Move-style pragma clause.  Leaner interprets the two pragmas that bear
on abort semantics: `aborts_if_is_partial` — the `aborts_if` clauses state
only where the function *must* abort (with the written code), and where no
declared condition holds any outcome is permitted — and
`aborts_if_is_strict` — an empty clause list means the function never aborts
(`aborts_if False`) instead of leaving abort behavior uninterpreted.  Any
other pragma is rejected. -/
declare_syntax_cat movePragma
scoped syntax "pragma " (ident <|> "opaque") ";" : movePragma

/-- The abort-semantics pragmas of a clause list: (partial, strict). -/
private def readPragmas (pragmas : Array Syntax) : MacroM (Bool × Bool × Bool) := do
  let mut isPartial := false
  let mut isStrict := false
  let mut isOpaque := false
  for clause in pragmas do
    -- `opaque` is a Lean keyword, so the clause admits it as an atom.
    let rec leaf (stx : Syntax) (fuel : Nat) : String :=
      match fuel with
      | 0 => ""
      | fuel + 1 =>
        if stx.isIdent then stx.getId.toString
        else if stx.isAtom then stx.getAtomVal
        else if stx.getNumArgs > 0 then leaf stx[0] fuel
        else ""
    let name := leaf clause[1] 4
    match name with
    | "aborts_if_is_partial" => isPartial := true
    | "aborts_if_is_strict" => isStrict := true
    | "opaque" => isOpaque := true
    | name =>
      Macro.throwErrorAt clause s!"unknown pragma `{name}`; Leaner interprets `aborts_if_is_partial`, `aborts_if_is_strict`, and `opaque`"
  return (isPartial, isStrict, isOpaque)

/-- Under `pragma opaque` the canonical specification is elaborated with the
summary option set. -/
private def opaqueWrap (isOpaque : Bool) (command : TSyntax `command) :
    MacroM (TSyntax `command) :=
  if isOpaque then `(command| set_option move.specOpaque true in $command:command) else pure command

/-- The three abort components of a Move-style clause list `aborts_if Pᵢ
[with Cᵢ]`: the permitted abort outcomes (`aborts`), the states excusing the
postcondition (`may_abort`), and the states forcing an abort (`must_abort`).
Without clauses the behavior is uninterpreted (strict: never aborts).  With
clauses, every Pᵢ forces an abort; a non-partial list also permits only
outcomes matching a clause (with its code), while a partial list permits any
outcome where no Pᵢ holds. -/
private def abortComponents (isPartial isStrict : Bool)
    (conditions : Array (TSyntax `term)) (codes : Array (Option (TSyntax `term)))
    (abortCode : TSyntax `ident) :
    MacroM (TSyntax `term × TSyntax `term × TSyntax `term) := do
  if conditions.isEmpty then
    if isStrict then return (← `(False), ← `(False), ← `(False))
    else return (← `(True), ← `(False), ← `(False))
  let clause (i : Nat) : MacroM (TSyntax `term) := do
    match codes[i]! with
    | some code => `($(conditions[i]!) ∧ $abortCode = Move.Spec.abortCodeOf $code)
    | none => pure conditions[i]!
  let mut matched ← clause 0
  let mut any := conditions[0]!
  for i in [1:conditions.size] do
    matched ← `($matched ∨ $(← clause i))
    any ← `($any ∨ $(conditions[i]!))
  let permitted ← if isPartial then `($matched ∨ ¬$any) else pure matched
  return (permitted, any, any)

/-- An effectful contract that declares no abort condition. Abort behavior is
then uninterpreted: every abort code is permitted, no state excuses the
postcondition, and no state forces an abort, so every successful execution
must still establish `ensures`.  Under `pragma aborts_if_is_strict` the
function never aborts instead. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    pragmas:movePragma*
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term : command => do
  let (isPartial, isStrict, isOpaque) ← readPragmas (pragmas.map (·.raw))
  let abortCode := mkIdentFrom postcondition `abortCode
  let (abortsTerm, mayAbortTerm, mustAbortTerm) ←
    abortComponents isPartial isStrict #[] #[] abortCode
  opaqueWrap isOpaque (← `(spec $function $binder* where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts $abortsTerm;
      may_abort $mayAbortTerm;
      must_abort $mustAbortTerm))

/-- A frame and a postcondition without precondition or abort clauses: the
loose or closed frame with uninterpreted abort behavior. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    modifiesClause:moveModifiesClause
    "ensures " postcondition:term : command =>
  `(spec $function $binder* where
      requires True;
      $modifiesClause:moveModifiesClause
      ensures $postcondition)

/-- A pragma-led contract with only a postcondition: the pragmas select the
relational reading (uninterpreted or never-aborting abort behavior) even for
a pure function, whose plain `ensures` would be a value contract. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " where "
    pragmas:movePragma+
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term : command =>
  `(spec $function $binder* where
      $pragmas:movePragma*
      requires True;
      $[$modifiesClause]?
      ensures $postcondition)

/-- An explicit-resource contract that declares no abort condition. -/
scoped macro "spec " function:ident binder:moveSpecBinder* " on " world:term
    " using " "[" resources:moveSpecResource,* "]" " where "
    "requires " precondition:term ";"
    modifiesClause:(moveModifiesClause)?
    "ensures " postcondition:term : command =>
  `(spec $function $binder* on $world using [$resources,*] where
      requires $precondition;
      $[$modifiesClause]?
      ensures $postcondition;
      aborts True;
      may_abort False)

/-- Further Move-style abort clauses of a specification, each with an optional
exact code. -/
declare_syntax_cat moveExtraAbortsIf
scoped syntax ";" "aborts_if " term (" with " term)? : moveExtraAbortsIf

/-- The canonical expansion of a Move-style clause list: pragmas, an optional
precondition, and one or more `aborts_if` clauses with optional codes. -/
private def abortsIfSpec (function : TSyntax `ident) (binders : Array (TSyntax `moveSpecBinder))
    (pragmas : Array Syntax) (precondition : TSyntax `term) (modifiesClause? : Option Syntax)
    (postcondition condition : TSyntax `term) (withCode : Option (TSyntax `term))
    (more : Array Syntax) : MacroM (TSyntax `command) := do
  let (isPartial, isStrict, isOpaque) ← readPragmas pragmas
  -- The optional `with code` group arrives as `[" with ", code]`, possibly
  -- wrapped in a singleton null node.
  let codeOf (optional : Syntax) : Option (TSyntax `term) :=
    let node := if optional.getNumArgs == 1 then optional[0] else optional
    if node.getNumArgs == 2 then some ⟨node[1]⟩ else none
  let mut conditions := #[condition]
  let mut codes := #[withCode]
  for clause in more do
    conditions := conditions.push ⟨clause[2]⟩
    codes := codes.push (codeOf clause[3])
  let abortCode := mkIdentFrom condition `abortCode
  let (abortsTerm, mayAbortTerm, mustAbortTerm) ←
    abortComponents isPartial isStrict conditions codes abortCode
  let modifiesSyntax : Option (TSyntax `moveModifiesClause) := modifiesClause?.map (⟨·⟩)
  opaqueWrap isOpaque (← `(spec $function $binders* where
      requires $precondition;
      $[$modifiesSyntax]?
      ensures $postcondition;
      aborts $abortsTerm;
      may_abort $mayAbortTerm;
      must_abort $mustAbortTerm))

/-- Move-style abort clauses, one or more, each with an optional exact code,
optionally led by abort-semantics pragmas and a precondition (omitted: `True`).
Every declared condition forces an abort; without `pragma
aborts_if_is_partial` the declared conditions are also the only permitted
aborts, each with its code.  That the postcondition needs to hold only where
the declared aborts are ruled out is the semantics of the contract
(`Move.Verify.Satisfies`), not anything written into the clauses. -/
scoped syntax (name := abortsIfSourceSpec)
  "spec " ident moveSpecBinder* " where "
    movePragma* ("requires " term ";")? (moveModifiesClause)?
    "ensures " term ";"
    "aborts_if " term (" with " term)? moveExtraAbortsIf* : command

macro_rules
  | `(spec $function $binder* where $[$pragmas]* $[requires $precondition;]? $[$modifiesClause]?
        ensures $postcondition;
        aborts_if $condition $[with $code]? $[$more]*) => do
    let precondition ← match precondition with
      | some precondition => pure precondition
      | none => `(True)
    abortsIfSpec function binder (pragmas.map (·.raw)) precondition
      (modifiesClause.map (·.raw)) postcondition condition code (more.map (·.raw))

/-- A specification function.  `spec fun f binders := body` gives `f` a
meaning in specification expressions.  When `f` is a Move function of this
module, the declaration is its *specification version* — the value-level
reading under which the Move Prover lets a pure Move function be called in a
specification; Move derives it, Leaner asks for it to be written (and the
transpiler writes it) — and `f args` in a `spec` clause denotes the
definition `f.specFun args`.  Otherwise it declares a new specification
function under its own name.  Binders are those of `spec`: values (a Move
function's reference parameters are written as the values they observe),
`{T}` type parameters, and `[C]` instance assumptions; the optional result
type is Lean's.  The body is a specification term: it may read global memory
through `R[a]` and `existsAt<R>(a)`, which makes the function *stateful* —
its definition takes the store instances of the families it reads and the
state to read, and a clause that applies it passes its own state (the
pre-state under `old(…)`) — but it cannot use `old` itself. -/
scoped syntax (name := specFunctionDecl)
  (docComment)? "spec " "fun " ident moveSpecBinder* (" : " term)? " := " term : command

/-- An uninterpreted specification function. Like `spec fun`, its direct
integer domain and codomain are `Int`; unlike `spec fun`, it has no defining
body. Applications are still registered for clean argument projection. -/
scoped syntax (name := opaqueSpecFunctionDecl)
  (docComment)? "spec " "opaque " ident moveSpecBinder* " : " term : command

@[command_elab specFunctionDecl]
private def elabSpecFunctionDecl : CommandElab := fun stx => do
  let doc? : Option (TSyntax ``Lean.Parser.Command.docComment) :=
    if stx[0].getNumArgs == 1 then some ⟨stx[0][0]⟩ else none
  let function : TSyntax `ident := ⟨stx[3]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[4].getArgs.map (⟨·⟩)
  let resultType? : Option (TSyntax `term) :=
    if stx[5].getNumArgs == 2 then some ⟨stx[5][1]⟩ else none
  let body : TSyntax `term := ⟨stx[7]⟩
  let env ← getEnv
  let fullName := (← getCurrNamespace) ++ function.getId
  let attached := Move.isMoveFunction env fullName
  if attached && env.contains (fullName ++ `specFun) then
    throwErrorAt function (m!"Move function `{fullName}` has a derived specification version; " ++
      m!"`spec fun` declares one only for a Move function without (a native, or a body with " ++
      m!"no pure reading)")
  if Move.Verify.Source.isSpecFunction env fullName then
    throwErrorAt function "specification function `{fullName}` is already declared"
  if !attached && env.contains fullName then
    throwErrorAt function (m!"`{fullName}` is already declared; a specification function takes " ++
      m!"a name of its own, or the name of the Move function it is the specification version of")
  let parameters ← liftMacroM <| unpackSpecParameters binders
  unless parameters.mutableParameters.isEmpty do
    throwErrorAt stx[4] "a specification function takes values; write `(x : T)` for a reference"
  Move.Verify.Source.declareSpecFunction function fullName attached parameters.context
    parameters.arguments parameters.types resultType? body doc?

@[command_elab opaqueSpecFunctionDecl]
private def elabOpaqueSpecFunctionDecl : CommandElab := fun stx => do
  let doc? : Option (TSyntax ``Lean.Parser.Command.docComment) :=
    if stx[0].getNumArgs == 1 then some ⟨stx[0][0]⟩ else none
  let function : TSyntax `ident := ⟨stx[3]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[4].getArgs.map (⟨·⟩)
  let resultType : TSyntax `term := ⟨stx[6]⟩
  let env ← getEnv
  let fullName := (← getCurrNamespace) ++ function.getId
  let attached := Move.isMoveFunction env fullName
  if Move.Verify.Source.isSpecFunction env fullName then
    throwErrorAt function "specification function `{fullName}` is already declared"
  if !attached && env.contains fullName then
    throwErrorAt function m!"`{fullName}` is already declared"
  let parameters ← liftMacroM <| unpackSpecParameters binders
  unless parameters.mutableParameters.isEmpty do
    throwErrorAt stx[4] "a specification function takes values; write `(x : T)` for a reference"
  let integerTypes ← parameters.types.mapM Move.Verify.Source.integerSpecTypeName?
  for (type, integerType?) in parameters.types.zip integerTypes do
    if let some integerType := integerType? then
      if integerType != ``Int then
        throwErrorAt type
          "specification functions use mathematical `Int`, not Move integer type `{integerType}`"
  let integerResultType? ← Move.Verify.Source.integerSpecTypeName? resultType
  if let some integerType := integerResultType? then
    if integerType != ``Int then
      throwErrorAt resultType
        "specification functions return mathematical `Int`, not Move integer type `{integerType}`"
  let declIdent := mkIdentFrom function
    (if attached then function.getId ++ `specFun else function.getId)
  let declName := if attached then fullName ++ `specFun else fullName
  let mut declBinders := parameters.context
  for (argument, type) in parameters.arguments.zip parameters.types do
    declBinders := declBinders.push (← `(bracketedBinder| ($argument : $type)))
  let command ← `(opaque $declIdent $declBinders* : $resultType)
  let command := match doc? with
    | some doc => command.raw.setArg 0 (command.raw[0].setArg 0 (mkNullNode #[doc.raw]))
    | none => command.raw
  elabCommand command
  unless (← getEnv).contains declName do
    throwErrorAt function m!"opaque specification function `{declName}` could not be declared"
  Move.Verify.Source.registerSpecFunction fullName {
    decl := declName
    stateful := false
    families := #[]
    integerArguments := integerTypes.map (·.isSome)
    integerResult := integerResultType?.isSome }

/-- The registered global-invariant body for each resource family that has
one, among the families a function uses. -/
private def globalInvariantsFor (resourceTypes : Array Move.Verify.Source.Family) :
    CommandElabM (Array (TSyntax `ident)) := do
  let env ← getEnv
  let mut result := #[]
  for family in resourceTypes do
    -- Only regular invariants are assumed on entry; `update` invariants
    -- constrain transitions and are asserted at writes only.
    for (isUpdate, body, _) in Move.globalInvariants env family.head do
      unless isUpdate do
        result := result.push (mkIdentFrom family.term body)
  return result

/-- Conjoin each regular global invariant `Inv initial` into the entry
precondition: the invariant is assumed on entry (it holds because every prior
write re-established it) and re-established at each write. -/
private def assumeGlobalInvariants (initial : TSyntax `term)
    (invariants : Array (TSyntax `ident))
    (precondition : TSyntax `term) : CommandElabM (TSyntax `term) := do
  let mut condition := precondition
  for body in invariants do
    condition ← `($condition ∧ $body $initial)
  return condition

/-- Build `def f.contract : Prop := …Satisfies…` for an effectful function from
its declarative clauses.  Shared by the surface elaborator and the ensures-only
routing (which supplies a trivial precondition and abort behavior). -/
def buildEffectfulContract (function : TSyntax `ident)
    (binders : Array (TSyntax `moveSpecBinder))
    (precondition : TSyntax `term) (modifiesClause? : Option Syntax)
    (postcondition abortCondition : TSyntax `term)
    (mayAbortCondition? : Option (TSyntax `term))
    (mustAbortCondition? : Option (TSyntax `term) := none) : CommandElabM Unit := do
  let parameters ← liftMacroM <| unpackSpecParameters binders
  let arguments := parameters.arguments
  let argsType ← liftMacroM <| Move.Verify.Source.argumentType parameters.types
  let world := mkIdentFrom function `_moveSpecState
  let mut resourceTypes ← Move.Verify.Source.inferredResources function.raw
  for clause in #[precondition.raw, postcondition.raw, abortCondition.raw] ++
      (mayAbortCondition?.map (·.raw)).toArray ++
      (mustAbortCondition?.map (·.raw)).toArray ++ modifiesClause?.toArray do
    resourceTypes ← Move.Verify.Source.addMentionedFamilies clause resourceTypes
  let sourceSpecName := associatedName function `sourceSpec
  let sourceSpecFullName := (← getCurrNamespace) ++ sourceSpecName.getId
  let hasSourceSpec := (← getEnv).contains sourceSpecFullName
  let resultType ← Move.Verify.Source.resultTypeOf function.raw
  let sourceResultType ← liftMacroM <|
    Move.Verify.Source.sourceResultType resultType parameters.mutableParameters
  -- A native, or a function specified `pragma opaque`, is *summarized*: its
  -- callers reason through the contract (`f.summarySpec`, the computation
  -- of every outcome the contract permits), not through its body.  The body
  -- of an opaque function is still what `verify` checks, when its source
  -- semantics can be generated; a native has none.
  let functionFullName := (← getCurrNamespace) ++ function.getId
  let isNative := Move.moveNativeAttr.hasTag (← getEnv) functionFullName
  let summarize := isNative || (← getOptions).getBool `move.specOpaque false
  if summarize && !isNative then
    liftCoreM (Move.moveOpaqueAttr.setTag functionFullName)
  let mut sourceAvailable := hasSourceSpec
  if !hasSourceSpec then
    if summarize then
      unless isNative do
        -- Best effort: a body the translator cannot follow leaves the
        -- contract assumed (warned), not the module broken.
        let saved := (← get).messages
        try
          Move.Verify.Source.ensureSourceSpec sourceSpecFullName.getPrefix function
          if (← get).messages.hasErrors && !saved.hasErrors then
            modify fun st => { st with messages := saved }
            logWarning m!"opaque function `{functionFullName}`: its source semantics could not be generated, so its contract is assumed by callers and cannot be verified here"
          else
            sourceAvailable := true
        catch e =>
          modify fun st => { st with messages := saved }
          logWarning m!"opaque function `{functionFullName}`: its source semantics could not be generated, so its contract is assumed by callers and cannot be verified here: {e.toMessageData}"
    else
      Move.Verify.Source.ensureSourceSpec sourceSpecFullName.getPrefix function
      sourceAvailable := true
  let contractName := associatedName function `contract
  let initial := mkIdentFrom precondition `_moveSpecInitial
  let final := mkIdentFrom postcondition `_moveSpecFinal
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let initialTerm : TSyntax `term := ⟨initial.raw⟩
  let finalTerm : TSyntax `term := ⟨final.raw⟩
  let globalInvariants ← globalInvariantsFor resourceTypes
  let bound := arguments.map (·.getId) ++ Move.Verify.Source.implicitClauseBinders
  let precondition ← Move.Verify.Source.rewriteClause
    resourceTypes initialTerm initialTerm precondition bound
  let precondition ← assumeGlobalInvariants initialTerm globalInvariants precondition
  let output := mkIdentFrom postcondition `_moveSpecOutput
  let outputTerm : TSyntax `term := ⟨output.raw⟩
  let mut postconditionRaw := postcondition.raw
  if !parameters.mutableParameters.isEmpty then
    let resultValue ← `($outputTerm.1)
    postconditionRaw := bindImplicit `result postconditionRaw resultValue.raw
    for ((parameter, _), index) in parameters.mutableParameters.zipIdx do
      let finalReferent ← if parameters.mutableParameters.size == 1 then
        `($outputTerm.2)
      else
        liftMacroM <| Move.Verify.Source.argumentProjection
          (← `($outputTerm.2)) index parameters.mutableParameters.size
      postconditionRaw ← liftMacroM <|
        rewriteMutablePost parameter finalReferent postconditionRaw
  let postcondition ← Move.Verify.Source.rewriteClause
    resourceTypes finalTerm initialTerm ⟨postconditionRaw⟩ bound
  let frame? ← frameCondition resourceTypes ⟨world.raw⟩ initialTerm finalTerm
    modifiesClause?
  let abortCondition ← Move.Verify.Source.rewriteClause
    resourceTypes initialTerm initialTerm abortCondition bound
  let requiresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial)] precondition
  let ensuresLambda ← liftMacroM <| if parameters.mutableParameters.isEmpty then
      clauseLambda arguments
        #[( `initial, initial), (`result, result), (`final, final)] postcondition
    else clauseLambda arguments
        #[( `initial, initial), (`_moveSpecOutput, output), (`final, final)] postcondition
  let abortsLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`abortCode, abortCode)] abortCondition
  let mayAbortCondition? ← match mayAbortCondition? with
    | none => pure none
    | some condition => do
        let rewritten ← Move.Verify.Source.rewriteClause
          resourceTypes initialTerm initialTerm condition bound
        pure (some rewritten)
  let mayAbortLambda ← liftMacroM <|
    mayAbortLambdaFor arguments initial abortsLambda mayAbortCondition?
  let mustAbortCondition? ← match mustAbortCondition? with
    | none => pure none
    | some condition => do
        let rewritten ← Move.Verify.Source.rewriteClause
          resourceTypes initialTerm initialTerm condition bound
        pure (some rewritten)
  let mustAbortLambda ← liftMacroM <| mustAbortLambdaFor arguments initial mustAbortCondition?
  let frameLambda ← liftMacroM <| match frame? with
    | none => `(fun _moveSpecArgs _moveSpecInitial _moveSpecFinal => True)
    | some frame => clauseLambda arguments
        #[( `initial, initial), (`final, final)] frame
  let contractRecord ← `(@Move.Verify.Contract.mk $world $argsType $sourceResultType
    $requiresLambda $ensuresLambda $abortsLambda $mayAbortLambda $mustAbortLambda $frameLambda)
  let heads := Move.Verify.Source.distinctHeads resourceTypes
  if summarize then
    -- The summary callers use, over the same world and stores as the
    -- contract it is drawn from.
    let summarySpecName := associatedName function `summarySpec
    let worldBinder ← `(bracketedBinder| {$world : Type})
    let mut storeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
    for (head, index) in heads.zipIdx do
      let storeName := mkIdentFrom function
        (Name.mkSimple s!"_moveSpecStore{index}")
      storeBinders := storeBinders.push (← `(bracketedBinder|
        [$storeName : $(← Move.Verify.Source.storeType ⟨world⟩ head)]))
    elabCommand (← `(noncomputable def $summarySpecName $parameters.context* $worldBinder
      $storeBinders* : $argsType → Move.Semantics.Spec $world $sourceResultType :=
      fun _moveSpecArgs => Move.Verify.Contract.summary $contractRecord _moveSpecArgs))
    -- Nothing to verify without the body's semantics.
    unless sourceAvailable do return
  let sourceSpecApplied ← liftMacroM <| applyTypeParameters sourceSpecName parameters.context
  let mut contractBody ← `(Move.Verify.Satisfies $sourceSpecApplied $contractRecord)
  let mut resourcePairs : Array (Name × Name) := #[]
  for leftIndex in [:heads.size] do
    for rightIndex in [leftIndex + 1:heads.size] do
      -- Distinct heads are independent at every instantiation; two
      -- instantiations of one head may coincide, so nothing is assumed about
      -- them.  Both directions: independence is symmetric, and a
      -- global-invariant reestablishment lemma frames the *other* family
      -- across the written one, needing whichever direction the write
      -- happens to take.
      resourcePairs := resourcePairs.push (heads[leftIndex]!, heads[rightIndex]!)
      resourcePairs := resourcePairs.push (heads[rightIndex]!, heads[leftIndex]!)
  let functionFullName := (← getCurrNamespace) ++ function.getId
  if (Move.Verify.Source.mutualFamilies.getState (← getEnv)).contains functionFullName then
    let contractSpecName := associatedName function `contractSpec
    let worldBinder ← `(bracketedBinder| {$world : Type})
    let mut instanceBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
    for (head, index) in heads.zipIdx do
      let storeName := mkIdentFrom function
        (Name.mkSimple s!"_moveSpecStore{index}")
      instanceBinders := instanceBinders.push (← `(bracketedBinder|
        [$storeName : $(← Move.Verify.Source.storeType ⟨world⟩ head)]))
    for ((left, right), index) in resourcePairs.zipIdx do
      let independenceName := mkIdentFrom function
        (Name.mkSimple s!"_moveSpecIndependent{index}")
      instanceBinders := instanceBinders.push (← `(bracketedBinder|
        [$independenceName :
          $(← Move.Verify.Source.independenceType ⟨world⟩ left right)]))
    elabCommand (← `(def $contractSpecName $parameters.context* $worldBinder
      $instanceBinders* : Move.Verify.Contract $world $argsType $sourceResultType :=
      $contractRecord))
  for ((left, right), index) in resourcePairs.zipIdx.reverse do
    let independenceName := mkIdentFrom function
      (Name.mkSimple s!"_moveSpecIndependent{index}")
    contractBody ← `(∀ [$independenceName :
      $(← Move.Verify.Source.independenceType ⟨world⟩ left right)], $contractBody)
  for (head, index) in heads.zipIdx.reverse do
    let storeName := mkIdentFrom function
      (Name.mkSimple s!"_moveSpecStore{index}")
    contractBody ← `(∀ [$storeName : $(← Move.Verify.Source.storeType ⟨world⟩ head)],
      $contractBody)
  contractBody ← `(∀ ($world : Type), $contractBody)
  contractBody ← liftMacroM <| quantifyContext parameters.context contractBody
  let contractCommand ← `(def $contractName : Prop := $contractBody)
  elabCommand contractCommand

@[command_elab inferredEffectfulSourceSpec]
private def elabInferredEffectfulSourceSpec : CommandElab := fun stx => do
  let modifiesClause? : Option Syntax :=
    if stx[7].getNumArgs == 1 then some stx[7][0] else none
  let mayAbortCondition? : Option (TSyntax `term) :=
    if stx[13].getNumArgs == 3 then some ⟨stx[13][2]⟩ else none
  let mustAbortCondition? : Option (TSyntax `term) :=
    if stx[14].getNumArgs == 3 then some ⟨stx[14][2]⟩ else none
  buildEffectfulContract ⟨stx[1]⟩ (stx[2].getArgs.map (⟨·⟩)) ⟨stx[5]⟩
    modifiesClause? ⟨stx[9]⟩ ⟨stx[12]⟩ mayAbortCondition? mustAbortCondition?

@[command_elab ensuresOnlySpec]
private def elabEnsuresOnlySpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let postcondition : TSyntax `term := ⟨stx[5]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  -- Effectful, or a postcondition that observes global state (a place, or a
  -- stateful specification function): a trivial precondition and
  -- uninterpreted aborts, routed so the postcondition observes global state.
  if (← Move.Verify.Source.isEffectfulFunction function.raw) ||
      (← Move.Verify.Source.mentionsState postcondition.raw) then
    buildEffectfulContract function binders (← `(True)) none postcondition
      (← `(True)) (some (← `(False)))
  else
    -- A value contract still applies specification functions by their
    -- definitions (no state to pass).
    let parameters ← liftMacroM <| unpackSpecParameters binders
    let postcondition ← Move.Verify.Source.rewriteClause #[] (← `(True)) (← `(True))
      postcondition (parameters.arguments.map (·.getId) ++ Move.Verify.Source.implicitClauseBinders)
    elabCommand (← liftMacroM (pureEnsuresContract function binders postcondition))
    -- A pure function's relational semantics serves its callers' automatic
    -- specifications.  Its generation is best-effort here: the value contract
    -- above does not depend on it, and a caller that needs it reports the
    -- unsupported form.
    try
      Move.Verify.Source.ensureSourceSpec ((← getCurrNamespace) ++ function.getId) function
    catch _ => pure ()

@[command_elab effectfulSourceSpec]
private def elabEffectfulSourceSpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let binders : Array (TSyntax `moveSpecBinder) := stx[2].getArgs.map (⟨·⟩)
  let world : TSyntax `term := ⟨stx[4]⟩
  let resourceSyntax := stx[7].getSepArgs
  let mut resources := #[]
  let mut resourceTypes : Array Move.Verify.Source.Family := #[]
  for resource in resourceSyntax do
    let resource : TSyntax `moveSpecResource := ⟨resource⟩
    let `(moveSpecResource| $typeName:ident => $descriptor:term) := resource
      | throwErrorAt resource "invalid resource descriptor"
    let some family ← Move.Verify.Source.familyOfTerm? typeName.raw
      | throwErrorAt typeName "expected a resource type"
    resourceTypes := resourceTypes.push family
    resources := resources.push { head := family.head, descriptorFor := fun _ => pure descriptor }
  let precondition : TSyntax `term := ⟨stx[11]⟩
  let modifiesClause? : Option Syntax :=
    if stx[13].getNumArgs == 1 then some stx[13][0] else none
  let postcondition : TSyntax `term := ⟨stx[15]⟩
  let abortCondition : TSyntax `term := ⟨stx[18]⟩
  let mayAbortCondition? : Option (TSyntax `term) :=
    if stx[19].getNumArgs == 3 then some ⟨stx[19][2]⟩ else none
  let mustAbortCondition? : Option (TSyntax `term) :=
    if stx[20].getNumArgs == 3 then some ⟨stx[20][2]⟩ else none
  let parameters ← liftMacroM <| unpackSpecParameters binders
  let arguments := parameters.arguments
  let argsType ← liftMacroM <| Move.Verify.Source.argumentType parameters.types
  let recursive ← Move.Verify.Source.isRecursive function.raw
  let recursiveName := mkIdentFrom function `_moveSpecRecursive
  let recursiveTerm : TSyntax `term := ⟨recursiveName.raw⟩
  let (body, resultType) ← Move.Verify.Source.translate function world.raw resources
    (if recursive then some recursiveTerm else none)
  let sourceSpecName := associatedName function `sourceSpec
  let contractName := associatedName function `contract
  let initial := mkIdentFrom precondition `_moveSpecInitial
  let final := mkIdentFrom postcondition `_moveSpecFinal
  let result := mkIdentFrom postcondition `result
  let abortCode := mkIdentFrom abortCondition `abortCode
  let initialTerm : TSyntax `term := ⟨initial.raw⟩
  let finalTerm : TSyntax `term := ⟨final.raw⟩
  let frame? ← frameCondition resourceTypes world
    initialTerm finalTerm modifiesClause?
  let requiresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial)] precondition
  let ensuresLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`result, result), (`final, final)] postcondition
  let abortsLambda ← liftMacroM <| clauseLambda arguments
    #[( `initial, initial), (`abortCode, abortCode)] abortCondition
  let mayAbortLambda ← liftMacroM <|
    mayAbortLambdaFor arguments initial abortsLambda mayAbortCondition?
  let mustAbortLambda ← liftMacroM <| mustAbortLambdaFor arguments initial mustAbortCondition?
  let frameLambda ← liftMacroM <| match frame? with
    | none => `(fun _moveSpecArgs _moveSpecInitial _moveSpecFinal => True)
    | some frame => clauseLambda arguments
        #[( `initial, initial), (`final, final)] frame
  let contractStruct ← `(@Move.Verify.Contract.mk $world $argsType $resultType
        $requiresLambda $ensuresLambda $abortsLambda $mayAbortLambda
        $mustAbortLambda $frameLambda)
  let sourceLambda ← liftMacroM <| Move.Verify.Source.unpackArguments arguments body
  if recursive then
    let bodySpecName := associatedName function `bodySpec
    let recursiveBinder ← `(bracketedBinder|
      ($recursiveName : $argsType → Move.Semantics.Spec $world $resultType))
    let bodyCommand ← `(noncomputable def $bodySpecName $parameters.context* $recursiveBinder :
        $argsType → Move.Semantics.Spec $world $resultType := $sourceLambda)
    elabCommand bodyCommand
    let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* : $argsType →
        Move.Semantics.Spec $world $resultType :=
        Move.Semantics.Spec.fix $bodySpecName)
    elabCommand sourceCommand
  else
    let sourceCommand ← `(noncomputable def $sourceSpecName $parameters.context* : $argsType →
        Move.Semantics.Spec $world $resultType := $sourceLambda)
    elabCommand sourceCommand
  let sourceSpecApplied ← liftMacroM <| applyTypeParameters sourceSpecName parameters.context
  let contractBody ← `(Move.Verify.Satisfies $sourceSpecApplied $contractStruct)
  let contractBody ← liftMacroM <| quantifyContext parameters.context contractBody
  let contractCommand ← `(def $contractName : Prop := $contractBody)
  elabCommand contractCommand

private partial def introUntilSatisfies : Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  if target.getAppFn.constName? == some ``Move.Verify.Satisfies then
    return
  match target with
  | .forallE .. =>
      let goal ← Lean.Elab.Tactic.getMainGoal
      let (_, next) ← goal.intro1P
      Lean.Elab.Tactic.replaceMainGoal [next]
      introUntilSatisfies
  | _ =>
      throwError
        "expected generated contract context followed by `Move.Verify.Satisfies`, got {target}"

private def introNamed (name : Name) : Lean.Elab.Tactic.TacticM Unit := Lean.Elab.Tactic.withMainContext do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let (_, next) ← goal.intro name
  Lean.Elab.Tactic.replaceMainGoal [next]

private def hasLastName (name : Name) (suffix : String) : Bool :=
  match name with
  | .str _ last => last == suffix
  | _ => false

/-- Whether a source semantics *is* a fixed point — `Spec.fix body`, possibly
eta-expanded over the function's argument — as opposed to a function whose
body merely contains a loop (`fun n => Spec.fix loop ()`), which opens with
the ordinary rule and meets the loop as a `wp (fix …)` sub-goal. -/
private partial def hasFixHead : Expr → Bool
  | .lam _ _ body _ =>
      body.getAppFn.constName? == some ``Move.Semantics.Spec.fix &&
        body.getAppNumArgs == 5 && body.appArg! == .bvar 0
  | .letE _ _ _ body _ => hasFixHead body
  | expression =>
      expression.getAppFn.constName? == some ``Move.Semantics.Spec.fix &&
        expression.getAppNumArgs == 4

private def targetUsesFix : Lean.Elab.Tactic.TacticM Bool := Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  unless target.getAppFn.constName? == some ``Move.Verify.Satisfies do
    throwError "expected a `Move.Verify.Satisfies` goal, got {target}"
  let arguments := target.getAppArgs
  if h : 2 ≤ arguments.size then
    let function := arguments[arguments.size - 2]'(by omega)
    return hasFixHead function
  return false

/-- Normalize the semantic `¬ mayAbort → ...` guard on the postcondition into
one negated hypothesis per declared abort condition — and none at all when no
abort condition is declared or it is `False`. `contract_intro` applies this
automatically; use it directly after a manual
`satisfies_of_wp`/`satisfies_fix_of_wp`. -/
syntax "abort_norm" : tactic
macro_rules
  | `(tactic| abort_norm) =>
    `(tactic|
      try simp only [false_and, and_false, exists_false, exists_const,
        not_false_eq_true, true_implies, not_true_eq_false, false_implies,
        implies_true, exists_and_left, exists_eq, exists_eq', and_true,
        not_or, exists_or, and_imp])

/-- Open the generated contract at the current goal and switch to weakest-
precondition reasoning. The source function is recovered from a goal of the
form `f.contract`. Nonrecursive functions use `satisfies_of_wp`; recursive
functions unfold `f.sourceSpec`, use `satisfies_fix_of_wp`, and expose
`recursive` and `recursiveVerified`. In both cases the authored source body is
unfolded and the remaining binders are named `args`, `initial`, and
`permitted`. -/
syntax (name := contractIntro) "contract_intro" : tactic

private def normalizeMayAbort : Lean.Elab.Tactic.TacticM Unit := do
  Lean.Elab.Tactic.evalTactic (← `(tactic| abort_norm))

/-- Verify the loops of a body from their stated invariants: at every
`wp (Spec.withInvariant I (Spec.fix body init)) …` goal, the invariant is
established on entry and preserved by each iteration, whose body is opened
by the weakest-precondition rules with the next iteration as
`recursiveVerified` (closed by `loop_continue`).  Repeats for nested loops
and loops in sequence. -/
syntax (name := loopInvariants) "loop_invariants" : tactic

/-- Case-split a conditional the goal is, when an annotated loop
(`Spec.withInvariant`) lies beneath it: the way to the loop's own goal. -/
syntax (name := splitTowardLoop) "split_toward_loop" : tactic

@[tactic splitTowardLoop]
private def elabSplitTowardLoop : Lean.Elab.Tactic.Tactic := fun _ =>
    Lean.Elab.Tactic.withMainContext do
  Lean.Elab.Tactic.evalTactic (← `(tactic| intros))
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  unless (target.find? fun e => e.isConstOf ``Move.Semantics.Spec.withInvariant).isSome do
    throwError "`split_toward_loop`: no annotated loop beneath"
  unless target.isAppOf ``ite || target.isAppOf ``dite do
    throwError "`split_toward_loop`: the goal is not a conditional"
  Lean.Elab.Tactic.evalTactic (← `(tactic| split))

macro_rules
  | `(tactic| loop_invariants) => do
    -- The iteration's names are accessible to the tactics that follow.
    let recursive := mkIdent `recursive
    let recursiveVerified := mkIdent `recursiveVerified
    let state := mkIdent `_moveSpecLoopState
    let invariantHyp := mkIdent `_moveSpecInvariant
    `(tactic| repeat' (first | (intros; refine Move.Verify.wp_withInvariant_fix_frame ?_ (fun $recursive $recursiveVerified $state $invariantHyp => ?_); all_goals try simp only [wp_norm, move_norm, move_data, Nat.reducePow, Nat.reduceMod,
          and_imp, forall_eq, forall_eq', Classical.not_not,
          exists_eq_left, exists_eq_left', exists_eq_right, exists_and_left,
          exists_and_right, and_true, true_and, true_implies, implies_true,
          and_self, Prod.mk.injEq, not_false_eq_true, not_true_eq_false,
          ite_true, ite_false, dite_true, dite_false] at $invariantHyp:ident ⊢) | split_toward_loop))

/-- The next iteration of a loop under verification: `recursiveVerified`
closes its `wp` goal, leaving the invariant to be re-established. -/
syntax (name := loopContinue) "loop_continue" : tactic
macro_rules
  | `(tactic| loop_continue) => do
    let recursiveVerified := mkIdent `recursiveVerified
    `(tactic| (intros; apply $recursiveVerified))

/-- Split a goal that is syntactically a conjunction — not a definition
that unfolds to one, such as `wp`, which `constructor` would open. -/
syntax (name := splitConjunction) "split_conjunction" : tactic

@[tactic splitConjunction]
private def elabSplitConjunction : Lean.Elab.Tactic.Tactic := fun stx =>
    Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  unless target.isAppOfArity ``And 2 do
    throwErrorAt stx "`split_conjunction` expects a conjunction"
  Lean.Elab.Tactic.evalTactic (← `(tactic| refine And.intro ?_ ?_))

/-- Case-split the `match`es (an enum `match` at the head of a body) the goal
contains, and nothing else: conditionals are the weakest-precondition rules'
business. -/
syntax (name := splitMatches) "split_matches" : tactic

@[tactic splitMatches]
private def elabSplitMatches : Lean.Elab.Tactic.Tactic := fun _ =>
    Lean.Elab.Tactic.withMainContext do
  let env ← getEnv
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  let hasMatch := (target.find? fun e =>
    match e.getAppFn.constName? with
    | some name => (Lean.Meta.getMatcherInfoCore? env name).isSome
    | none => false).isSome
  unless hasMatch do throwError "`split_matches`: no match to split"
  Lean.Elab.Tactic.evalTactic (← `(tactic| split))

/-- A `match` on a Move enum value in `e` whose discriminants are closed
(not under a binder), if any. -/
private partial def enumMatch? (e : Expr) : MetaM (Option Expr) := do
  let env ← getEnv
  if let some info := e.getAppFn.constName?.bind (Lean.Meta.getMatcherInfoCore? env) then
    let arguments := e.getAppArgs
    let discriminants := arguments.extract info.getFirstDiscrPos
      (info.getFirstDiscrPos + info.numDiscrs)
    if discriminants.size == info.numDiscrs && discriminants.all (!·.hasLooseBVars) then
      for discriminant in discriminants do
        let type ← Lean.Meta.whnf (← Lean.Meta.inferType discriminant)
        if let .const typeName _ := type.getAppFn then
          if Move.moveEnumAttr.hasTag env typeName then return some e
  match e with
  | .app f a => do
      if let some found ← enumMatch? f then return some found
      enumMatch? a
  | .lam _ _ b _ | .forallE _ _ b _ => enumMatch? b
  | .letE _ _ v b _ => do
      if let some found ← enumMatch? v then return some found
      enumMatch? b
  | .mdata _ b | .proj _ _ b => enumMatch? b
  | _ => return none

/-- Case-split a `match` on a Move enum value the goal contains — the
dispatch of a `match` through a reference, exposed under the mutation's
prophecy quantifier by the wp rules — and nothing else: a `match` on an
optional result (an element borrow's) is the simplifier's. -/
syntax (name := splitEnumMatches) "split_enum_matches" : tactic

@[tactic splitEnumMatches]
private def elabSplitEnumMatches : Lean.Elab.Tactic.Tactic := fun _ =>
    Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  let some application ← enumMatch? target
    | throwError "`split_enum_matches`: no enum match to split"
  let goals ← Lean.Meta.Split.splitMatch (← Lean.Elab.Tactic.getMainGoal) application
  Lean.Elab.Tactic.replaceMainGoal goals

/-- Inside a loop iteration under verification (`recursiveVerified` is in
the context): the exit test is a case split, a conjunction its parts, the
next iteration `recursiveVerified`. -/
syntax (name := loopIterationCases) "loop_iteration_cases" : tactic

@[tactic loopIterationCases]
private def elabLoopIterationCases : Lean.Elab.Tactic.Tactic := fun _ =>
    Lean.Elab.Tactic.withMainContext do
  unless (← getLCtx).any (fun decl => !decl.isImplementationDetail &&
      decl.userName == `recursiveVerified) do
    throwError "`loop_iteration_cases`: not inside a loop iteration"
  Lean.Elab.Tactic.evalTactic (← `(tactic| (repeat' split)))
  Lean.Elab.Tactic.evalTactic
    (← `(tactic| all_goals try (repeat' (first | split_conjunction | loop_continue))))

/-- Discharge a concrete `wp (callee.sourceSpec args) …` goal from the
callee's generated `callee.verified` theorem.  Automatic verification keeps
verified recursive callees opaque and invokes this bridge instead of trying
to unfold their fixed point in the caller. -/
syntax (name := verifiedCall) "verified_call" : tactic

@[tactic verifiedCall]
private def elabVerifiedCall : Lean.Elab.Tactic.Tactic := fun stx =>
    Lean.Elab.Tactic.withMainContext do
  -- Mutable-reference calls are guarded by one prophecy quantifier (and
  -- potentially reconciliation hypotheses) before the callee's `wp` is
  -- exposed. Introduce those binders just as the ordinary wp simplifier does.
  Lean.Elab.Tactic.evalTactic (← `(tactic| intros))
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  unless target.getAppFn.constName? == some ``Move.Verify.wp do
    throwErrorAt stx "`verified_call` expects a weakest-precondition goal"
  let action := target.getArg! 2
  let some sourceSpecName := action.getAppFn.constName?
    | throwErrorAt stx "the action is not a named source specification"
  let .str functionName "sourceSpec" := sourceSpecName
    | throwErrorAt stx "the action is not a generated `sourceSpec`"
  let verifiedName := functionName ++ `verified
  let contractName := functionName ++ `contract
  unless (← getEnv).contains verifiedName do
    throwErrorAt stx "`{verifiedName}` is not available"
  unless (← getEnv).contains contractName do
    throwErrorAt stx "`{contractName}` is not available"
  let sourceSpec := mkIdentFrom stx sourceSpecName
  let contract := mkIdentFrom stx contractName
  let verified := mkIdentFrom stx verifiedName
  Lean.Elab.Tactic.evalTactic (← `(tactic| refine Move.Verify.wp_mono
    (Move.Verify.wp_of_satisfies
      (show Move.Verify.Satisfies $sourceSpec _ from
        (by
          have established : $contract := $verified
          unfold $contract at established
          exact established _))
      ?_ (noAbort := ?_)) ?_ ?_))

@[tactic contractIntro]
private def elabContractIntro : Lean.Elab.Tactic.Tactic := fun stx => Lean.Elab.Tactic.withMainContext do
  let target ← instantiateMVars (← Lean.Elab.Tactic.getMainTarget)
  let some contractName := target.getAppFn.constName?
    | throwErrorAt stx
        "`contract_intro` must start on a generated goal of the form `f.contract`"
  let .str functionName "contract" := contractName
    | throwErrorAt stx
        "`contract_intro` expected a generated `f.contract` goal, got `{contractName}`"
  let sourceSpecName := functionName ++ `sourceSpec
  let bodySpecName := functionName ++ `bodySpec
  let env ← getEnv
  unless env.contains sourceSpecName do
    throwErrorAt stx
      "`contract_intro` supports effectful source contracts, but `{sourceSpecName}` is not defined"
  let mutualInfo? := Move.Verify.Source.mutualFamilies.getState env |>.find? functionName
  let mutationFamily := mutualInfo?.map (·.hasMutationMembers) |>.getD false
  let mutualInfo? := mutualInfo?.filter fun info => !info.hasMutationMembers
  let contract := mkIdentFrom stx contractName
  let sourceSpec := mkIdentFrom stx sourceSpecName
  Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $contract))
  introUntilSatisfies
  Lean.Elab.Tactic.withMainContext do
    Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $sourceSpec))
    Lean.Elab.Tactic.evalTactic (← `(tactic|
      try simp only [Move.Semantics.Spec.pure_bind]))
    if let some mutualInfo := mutualInfo? then
      let mutualSource := mkIdentFrom stx mutualInfo.source
      Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $mutualSource))
      let indexType := mkIdentFrom stx mutualInfo.indexType
      let argsFamily := mkIdentFrom stx mutualInfo.argsFamily
      let resultFamily := mkIdentFrom stx mutualInfo.resultFamily
      let recursor := mkIdentFrom stx (mutualInfo.indexType ++ `rec)
      let familyIndex := mkIdentFrom stx `_moveSpecContractIndex
      let motive ← `(fun ($familyIndex : $indexType) =>
        Move.Verify.Contract _ ($argsFamily $familyIndex) ($resultFamily $familyIndex))
      let mut contracts ← `(@$recursor:ident $motive)
      let mut contractSpecs : Array (TSyntax `ident) := #[]
      for member in mutualInfo.members do
        let contractSpec := mkIdentFrom stx (member ++ `contractSpec)
        unless env.contains (member ++ `contractSpec) do
          throwErrorAt stx
            "mutual contract `{member ++ `contractSpec}` is not defined; declare every member's `spec` before verifying the family"
        contractSpecs := contractSpecs.push contractSpec
        contracts ← `($contracts $contractSpec)
      let body := mkIdentFrom stx mutualInfo.body
      let some memberIndex := mutualInfo.members.findIdx? (· == functionName)
        | throwErrorAt stx "current function is missing from its mutual component"
      let constructor := mkIdentFrom stx mutualInfo.constructors[memberIndex]!
      Lean.Elab.Tactic.evalTactic (← `(tactic| change Move.Verify.Satisfies
        (Move.Semantics.Spec.fixFamily $body $constructor) ($contracts $constructor)))
      Lean.Elab.Tactic.evalTactic (← `(tactic|
        apply Move.Verify.satisfies_fixFamily_of_wp $body $contracts))
      introNamed `recursive
      introNamed `recursiveVerified
      introNamed `index
      let indexLocal := mkIdent `index
      Lean.Elab.Tactic.evalTactic (← `(tactic| cases $indexLocal:ident))
      let branchGoals ← Lean.Elab.Tactic.getGoals
      let mut remaining : List MVarId := []
      for goal in branchGoals do
        Lean.Elab.Tactic.setGoals [goal]
        Lean.Elab.Tactic.withMainContext do
          Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $body))
          Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $argsFamily $resultFamily))
          for contractSpec in contractSpecs do
            try Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $contractSpec))
            catch _ => pure ()
          introNamed `args
          introNamed `initial
          introNamed `permitted
          normalizeMayAbort
        remaining := remaining ++ (← Lean.Elab.Tactic.getGoals)
      Lean.Elab.Tactic.setGoals remaining
    else if mutationFamily then
      -- A public value contract does not describe the prophecy carriers
      -- needed as induction hypotheses for mutation-level SCC entries. As
      -- with direct reference-returning recursion, expose the wrapper WP so
      -- the proof can establish and apply an explicit mutation invariant.
      Lean.Elab.Tactic.evalTactic (← `(tactic| apply Move.Verify.satisfies_of_wp))
      introNamed `args
      introNamed `initial
      introNamed `permitted
      normalizeMayAbort
    else if ← targetUsesFix then
      let bodySpec := mkIdentFrom stx bodySpecName
      Lean.Elab.Tactic.evalTactic (← `(tactic| apply Move.Verify.satisfies_fix_of_wp))
      introNamed `recursive
      introNamed `recursiveVerified
      introNamed `args
      introNamed `initial
      introNamed `permitted
      if env.contains bodySpecName then
        Lean.Elab.Tactic.evalTactic (← `(tactic| unfold $bodySpec))
      normalizeMayAbort
    else
      Lean.Elab.Tactic.evalTactic (← `(tactic| apply Move.Verify.satisfies_of_wp))
      introNamed `args
      introNamed `initial
      introNamed `permitted
      normalizeMayAbort

/-- Wrap a proof to record its cost when benchmarking.  With the environment
variable `MOVE_PROOF_BENCH` unset it is exactly the wrapped tactics, so it is
free to leave in every generated proof; when set it logs a parseable line

    ‖MOVE_BENCH‖\t<name>\t<heartbeats>\t<elapsed-ms>

per verified function.  Heartbeats are deterministic — independent of the
machine, load, and the `aptos` CLI the test suite otherwise spends its wall
time in — so summing them over the suite benchmarks proof work directly. -/
scoped syntax (name := moveBench) "move_bench " tacticSeq : tactic

@[tactic moveBench]
private def elabMoveBench : Lean.Elab.Tactic.Tactic := fun stx => do
  let proof := stx[1]
  match ← IO.getEnv "MOVE_PROOF_BENCH" with
  | none => Lean.Elab.Tactic.evalTactic proof
  | some _ =>
      let name := (← Lean.Elab.Term.getDeclName?).getD `_
      let name := name.replacePrefix (`_root_) Name.anonymous
      let start ← IO.monoNanosNow
      let (_, heartbeats) ← Lean.withHeartbeats (Lean.Elab.Tactic.evalTactic proof)
      let elapsed := (← IO.monoNanosNow) - start
      Lean.logInfo m!"‖MOVE_BENCH‖\t{name}\t{heartbeats}\t{elapsed / 1000000}"

/-- Prove the contract associated with the named source function with an
explicit tactic proof. Requiring `by` keeps the command unambiguous when the
next Move declaration starts with the term-level keyword `fun`. -/
scoped macro "verify " function:ident " by " proof:tacticSeq : command => do
  let contractName := associatedName function `contract
  let verifiedName := associatedName function `verified
  `(theorem $verifiedName : $contractName := by move_bench $proof)

/-- Symbolically execute the supported effectful source fragment and discharge
its declarative contract using the typed store laws and arithmetic solver. -/
scoped syntax (name := automaticSourceVerify) "verify " ident : command

/-- Derive a `fun`'s relational semantics `f.sourceSpec` from its retained
source at its declaration.  The function items emit this command for every
function with a body, so a function without a `spec` block carries its
semantics with its module: its callers, in this module or in importing ones,
reason through its body.  The derivation is best effort: a body outside the
translator's coverage derives nothing and reports nothing here -- the `spec`
or the caller that needs the semantics reports the limitation where it
matters, and a `spec` block, when there is one, is the declared semantics
regardless. -/
syntax (name := deriveSourceSpec) "#derive_move_source_spec " ident : command

@[command_elab deriveSourceSpec]
private def elabDeriveSourceSpec : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let functionName := (← getCurrNamespace) ++ function.getId
  let env ← getEnv
  if env.contains (functionName ++ `sourceSpec) then return
  -- Translate against an empty message log: errors mean the derivation is
  -- abandoned, with the environment (a partially generated mutual component)
  -- restored; otherwise the messages are kept.
  let saved := (← get).messages
  modify fun st => { st with messages := {} }
  let report := (← getOptions).getBool `move.reportDerivation false
  let abandon (reason : MessageData) : CommandElabM Unit := do
    setEnv env
    modify fun st => { st with messages := saved }
    if report then
      logWarningAt function m!"source semantics of `{functionName}` not derived: {reason}"
  try
    Move.Verify.Source.ensureSourceSpec functionName function
    if (← get).messages.hasErrors then
      let errors ← (← get).messages.toList.filterM fun message =>
        pure (message.severity == .error)
      let reasons ← errors.mapM (·.data.toString)
      abandon m!"{String.intercalate "; " reasons}"
    else
      modify fun st => { st with messages := saved ++ st.messages }
  catch e =>
    abandon e.toMessageData

/-- Derive a `fun`'s specification version `f.specFun` from its retained
source at its declaration (`Move.Verify.Source.ensureSpecFunction`): the
pure reading of its body, what `f args` denotes in a specification clause.
The function items emit this command for every function with a body, so the
version travels with its module and serves the specifications of importing
modules.  Best effort and silent, like `#derive_move_source_spec`: a body
without a pure reading derives nothing here — a specification that applies
the function reports why, and `spec fun f …` can declare a version by hand
(`set_option move.reportDerivation true` reports the failures here). -/
syntax (name := deriveSpecFunction) "#derive_move_spec_function " ident : command

@[command_elab deriveSpecFunction]
private def elabDeriveSpecFunction : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let functionName := (← getCurrNamespace) ++ function.getId
  let env ← getEnv
  if Move.Verify.Source.isSpecFunction env functionName then return
  let saved := (← get).messages
  modify fun st => { st with messages := {} }
  let report := (← getOptions).getBool `move.reportDerivation false
  let abandon (reason : MessageData) : CommandElabM Unit := do
    setEnv env
    modify fun st => { st with messages := saved }
    if report then
      logWarningAt function m!"specification version of `{functionName}` not derived: {reason}"
  try
    let _ ← Move.Verify.Source.ensureSpecFunction functionName function
    if (← get).messages.hasErrors then
      let errors ← (← get).messages.toList.filterM fun message =>
        pure (message.severity == .error)
      let reasons ← errors.mapM (·.data.toString)
      abandon m!"{String.intercalate "; " reasons}"
    else
      modify fun st => { st with messages := saved ++ st.messages }
  catch e =>
    abandon e.toMessageData

/-- End an automatic verification attempt with a concise, source-oriented
diagnostic instead of exposing the automation tactic's internal search state. -/
scoped syntax (name := reportVerificationFailure)
  "report_verification_failure " ident : tactic

@[tactic reportVerificationFailure]
private def elabReportVerificationFailure : Lean.Elab.Tactic.Tactic := fun stx => do
  throwErrorAt stx[1]
    "verification failed for `{stx[1].getId}`: the implementation does not prove its contract; use `verify {stx[1].getId} by` to inspect and prove the remaining obligation"

/-- A Move named integer constant: a definition of a bounded integer type.
The automatic proofs unfold the ones a function or its contract mentions, so
their values reach the arithmetic. -/
private def isMoveIntConstant (env : Environment) (name : Name) : Bool :=
  match env.find? name with
  | some (.defnInfo info) =>
      match info.type.getAppFn.constName? with
      | some head =>
          [``Move.U8, ``Move.U16, ``Move.U32, ``Move.U64, ``Move.U128, ``Move.U256,
            ``Move.I8, ``Move.I16, ``Move.I32, ``Move.I64, ``Move.I128, ``Move.I256,
            ``Move.MoveInt, ``Move.UInt, ``Move.SInt].contains head
      | none => false
  | _ => false

/-- The Move named integer constants a declaration's value mentions. -/
private def moveIntConstantsOf (env : Environment) (declName : Name) : Array Name :=
  match env.find? declName >>= (·.value? (allowOpaque := true)) with
  | some value => value.getUsedConstants.filter (isMoveIntConstant env)
  | none => #[]

@[command_elab automaticSourceVerify]
private def elabAutomaticSourceVerify : CommandElab := fun stx => do
  let function : TSyntax `ident := ⟨stx[1]⟩
  let functionName := (← getCurrNamespace) ++ function.getId
  -- Parse the fully qualified source name as a term. Tactic identifiers do not
  -- inherit the declaration-name expansion used by command elaboration.
  let parseTerm (name : Name) : CommandElabM (TSyntax `term) := do
    match Lean.Parser.runParserCategory (← getEnv) `term name.toString with
    | .ok parsed => pure ⟨parsed⟩
    | .error message => throwErrorAt function
        "failed to generate verification name `{name}`: {message}"
  let sourceSpecName := functionName ++ `sourceSpec
  let contractName := associatedName function `contract
  let verifiedName := associatedName function `verified
  let qualifiedFunction := mkIdent functionName
  -- A pure value contract reduces the function; a relational one opens the
  -- generated `Satisfies`.  The contract's own shape decides: a pure function
  -- may well have a `sourceSpec` too, generated for its callers.
  let relational := match (← getEnv).find? (functionName ++ `contract) with
    | some info => match info.value? (allowOpaque := true) with
      | some value => value.getUsedConstants.contains ``Move.Verify.Satisfies
      | none => false
    | none => false
  unless relational do
    let functionTerm ← parseTerm functionName
    let functionLemma ←
      `(Lean.Parser.Tactic.simpLemma| $functionTerm:term)
    let env ← getEnv
    let mut unfoldLemmas := #[functionLemma]
    if let some info := env.find? functionName then
      if let some value := info.value? (allowOpaque := true) then
        for dependency in value.getUsedConstants do
          if dependency != functionName &&
              (Move.isMoveFunction env dependency || isMoveIntConstant env dependency) then
            let dependencyTerm ← parseTerm dependency
            let dependencyLemma ←
              `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
            unfoldLemmas := unfoldLemmas.push dependencyLemma
    -- Named constants the contract mentions.
    for constant in moveIntConstantsOf env (functionName ++ `contract) do
      let constantTerm ← parseTerm constant
      unfoldLemmas := unfoldLemmas.push (← `(Lean.Parser.Tactic.simpLemma| $constantTerm:term))
    -- Specification functions the contract applies, transitively: unfolded,
    -- as the Move Prover inlines non-recursive specification functions.
    for definition in Move.Verify.Source.specFunctionDependencies env (functionName ++ `contract) do
      let definitionTerm ← parseTerm definition
      unfoldLemmas := unfoldLemmas.push (← `(Lean.Parser.Tactic.simpLemma| $definitionTerm:term))
    let command ← `(theorem $verifiedName : $contractName := by
      move_bench
      unfold $contractName
      simp_all [$unfoldLemmas,*, move_spec, move_invariant_norm, move_norm,
        Nat.reducePow, Nat.reduceMod, Move.UInt.numeral_eq_ofNat, and_assoc,
        Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size,
        exists_const] <;>
      -- An enum `match` or a conditional the unfolding exposes is a case
      -- split, each branch normalized again.
      (try (repeat' split) <;> simp_all [$unfoldLemmas,*, move_spec,
        move_invariant_norm, move_norm, Nat.reducePow, Nat.reduceMod,
        Move.UInt.numeral_eq_ofNat, and_assoc, exists_const]) <;>
      -- A second pass for `Int`-valued specifications over unsigned values
      -- (subtraction, signed or `num` operands): the `toInt` view becomes the
      -- `Nat` view once the checked-operation lemmas have fired.
      (try simp_all (config := { contextual := true })
        [Move.UInt.toInt_eq_toNat, ← Int.ofNat_sub, Int.ofNat_inj, move_norm,
          Nat.reducePow, Nat.reduceMod, and_assoc]) <;>
      (try uint_bounds) <;>
      try (grind [Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16,
        Move.UInt.toNat_ofNat_u32, Move.UInt.toNat_ofNat_u64,
        Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256,
        Move.UInt.toNat_zero, Move.UInt.toNat_one,
        Move.UInt.toNat_cast, Move.UInt.toInt_eq_toNat,
        Move.UInt.toNat_ofNat_sub, Move.UInt.toNat_ofNat_div,
        Move.UInt.toNat_ofNat_mod,
        Move.UInt.toNat_lt,
        Move.Semantics.ResourceStore.get, Move.Semantics.ResourceStore.contains,
        Move.Semantics.ResourceStore.get_insert_same,
        Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le,
        List.getElem!_of_getElem?,
        Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size])
      all_goals report_verification_failure $qualifiedFunction)
    elabCommand command
    return
  let sourceSpecTerm ← parseTerm sourceSpecName
  let sourceSpecLemma ←
    `(Lean.Parser.Tactic.simpLemma| $sourceSpecTerm:term)
  let env ← getEnv
  let mut sourceUnfoldLemmas := #[sourceSpecLemma]
  if let some info := env.find? sourceSpecName then
    if let some value := info.value? (allowOpaque := true) then
      for dependency in value.getUsedConstants do
          let verifiedSource := match dependency with
            | .str callee "sourceSpec" =>
                env.contains (callee ++ `verified) &&
                  isRecursiveSourceSpec env dependency
            | _ => false
          if dependency != sourceSpecName && !verifiedSource &&
              (Move.isMoveFunction env dependency ||
                isMoveIntConstant env dependency ||
                nameSuffix? dependency == some "sourceSpec" ||
                nameSuffix? dependency == some "mutationSpec" ||
                nameSuffix? dependency == some "summarySpec" ||
                nameSuffix? dependency == some "bodySpec") then
          let dependencyTerm ← parseTerm dependency
          let dependencyLemma ←
            `(Lean.Parser.Tactic.simpLemma| $dependencyTerm:term)
          sourceUnfoldLemmas := sourceUnfoldLemmas.push dependencyLemma
  -- Named constants the contract mentions, and those callee summaries mention.
  for constant in moveIntConstantsOf env (functionName ++ `contract) do
    let constantTerm ← parseTerm constant
    sourceUnfoldLemmas := sourceUnfoldLemmas.push (← `(Lean.Parser.Tactic.simpLemma| $constantTerm:term))
  -- Specification functions the contract applies, transitively: unfolded, as
  -- the Move Prover inlines non-recursive specification functions.
  for definition in Move.Verify.Source.specFunctionDependencies env (functionName ++ `contract) do
    let definitionTerm ← parseTerm definition
    sourceUnfoldLemmas := sourceUnfoldLemmas.push (← `(Lean.Parser.Tactic.simpLemma| $definitionTerm:term))
  -- Callees are inlined: their `sourceSpec`s are unfolded into the caller's
  -- body before symbolic execution (the function's own `sourceSpec` and
  -- `bodySpec` are already opened by `contract_intro`).
  let calleeUnfoldLemmas := sourceUnfoldLemmas.filter fun lemma =>
    let name := lemma.raw.getId
    let verifiedCallee := match name with
      | .str callee "sourceSpec" =>
          env.contains (callee ++ `verified) && isRecursiveSourceSpec env name
      | _ => false
    name != sourceSpecName && name != functionName ++ `bodySpec && !verifiedCallee
  let wpNormalize ← `(tactic| simp only [$calleeUnfoldLemmas,*, wp_norm, move_norm, move_data,
        Nat.reducePow, Nat.reduceMod, and_imp, forall_eq, forall_eq',
        Classical.not_not,
        exists_eq_left, exists_eq_left', exists_eq_right, exists_and_left,
        exists_and_right, and_true, true_and, true_implies, implies_true,
        and_self, Prod.mk.injEq, not_false_eq_true, not_true_eq_false,
        ite_true, ite_false, dite_true, dite_false])
  let command ← `(set_option maxHeartbeats 800000 in
    theorem $verifiedName : $contractName := by
      move_bench
      -- Open the contract into one weakest-precondition goal, then execute
      -- the body symbolically by the wp rules: linear in the body, no
      -- existentials, well-definedness discharged per primitive, the only
      -- residue being a created value's data invariant.
      contract_intro
      -- An enum `match` at the head of the body is a case split: the wp
      -- rules then apply to each branch.
      all_goals try (repeat' split_matches)
      all_goals try $wpNormalize:tactic
      -- A `match` through a mutable reference dispatches on the referent,
      -- which the prophecy rule exposes under its quantifier: introduce,
      -- split, and normalize each branch again (nothing is introduced when
      -- there is no enum match to split).
      all_goals try (intros; split_enum_matches; repeat' split_enum_matches)
      all_goals try $wpNormalize:tactic
      all_goals try verified_call
      -- Loops with stated invariants: each iteration's exit test is a case
      -- split, a conjunction its parts, the next iteration `recursiveVerified`.
      all_goals try loop_invariants
      all_goals try loop_iteration_cases
      all_goals
        simp_all (config := { maxSteps := 1000000 })
          [$calleeUnfoldLemmas,*, move_spec, move_data, move_invariant_norm,
            move_norm, Nat.reducePow, Nat.reduceMod,
            Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size,
            and_assoc, exists_const] <;>
        -- A second pass for `Int`-valued specifications over unsigned values
        -- (subtraction, signed or `num` operands): the `toInt` view becomes
        -- the `Nat` view once the checked-operation lemmas have fired.
        (try simp_all (config := { maxSteps := 1000000, contextual := true })
          [Move.UInt.toInt_eq_toNat, ← Int.ofNat_sub, Int.ofNat_inj, move_norm,
            Nat.reducePow, Nat.reduceMod, and_assoc]) <;>
        (try uint_bounds) <;>
        try (grind [Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16,
          Move.UInt.toNat_ofNat_u32, Move.UInt.toNat_ofNat_u64,
          Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256,
          Move.UInt.toNat_zero, Move.UInt.toNat_one,
          -- `Int`-valued specifications (subtraction, signed or `num`
          -- operands) over unsigned values reduce to the `Nat` view; the
          -- range-preserving operations keep their exposed value.
          Move.UInt.toNat_cast, Move.UInt.toInt_eq_toNat,
          Move.UInt.toNat_ofNat_sub, Move.UInt.toNat_ofNat_div,
          Move.UInt.toNat_ofNat_mod,
          Move.UInt.toNat_lt, Nat.shiftRight_le,
          Move.Semantics.ResourceStore.get, Move.Semantics.ResourceStore.contains,
          Move.Semantics.ResourceStore.get_insert_same,
          Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le,
          List.getElem!_of_getElem?,
          Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size])
      -- Loop goals the normalization uncovers (an element borrow's match,
      -- say): the loop passes and the normalization once more.
      all_goals try loop_iteration_cases
      all_goals try
        simp_all (config := { maxSteps := 1000000 })
          [$calleeUnfoldLemmas,*, move_spec, move_data, move_invariant_norm,
            move_norm, Nat.reducePow, Nat.reduceMod,
            Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size,
            and_assoc, exists_const] <;>
        -- A second pass for `Int`-valued specifications over unsigned values
        -- (subtraction, signed or `num` operands): the `toInt` view becomes
        -- the `Nat` view once the checked-operation lemmas have fired.
        (try simp_all (config := { maxSteps := 1000000, contextual := true })
          [Move.UInt.toInt_eq_toNat, ← Int.ofNat_sub, Int.ofNat_inj, move_norm,
            Nat.reducePow, Nat.reduceMod, and_assoc]) <;>
        (try uint_bounds) <;>
        try (grind [Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16,
          Move.UInt.toNat_ofNat_u32, Move.UInt.toNat_ofNat_u64,
          Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256,
          Move.UInt.toNat_zero, Move.UInt.toNat_one,
          -- `Int`-valued specifications (subtraction, signed or `num`
          -- operands) over unsigned values reduce to the `Nat` view; the
          -- range-preserving operations keep their exposed value.
          Move.UInt.toNat_cast, Move.UInt.toInt_eq_toNat,
          Move.UInt.toNat_ofNat_sub, Move.UInt.toNat_ofNat_div,
          Move.UInt.toNat_ofNat_mod,
          Move.UInt.toNat_lt, Nat.shiftRight_le,
          Move.Semantics.ResourceStore.get, Move.Semantics.ResourceStore.contains,
          Move.Semantics.ResourceStore.get_insert_same,
          Move.UInt.lt_iff_toNat_lt, Move.UInt.le_iff_toNat_le,
          List.getElem!_of_getElem?,
          Move.Vector.toList_length_lt_size, Move.Vector.elems_length_lt_size])
      -- What arithmetic leaves: a loop iteration whose invariant is the
      -- function's own contract closes by the induction hypothesis; an
      -- equality of bounded integers is decided by their values.
      all_goals try first
        | (intros; exact Move.Verify.wp_of_satisfies $(mkIdent `recursiveVerified) trivial)
        | (apply Move.UInt.ext
           (try simp_all [Move.UInt.toNat_zero, Move.UInt.toNat_one,
             Move.UInt.toNat_ofNat_u8, Move.UInt.toNat_ofNat_u16, Move.UInt.toNat_ofNat_u32,
             Move.UInt.toNat_ofNat_u64, Move.UInt.toNat_ofNat_u128, Move.UInt.toNat_ofNat_u256])
           <;> omega)
      all_goals report_verification_failure $qualifiedFunction)
  elabCommand command

end Move.Spec

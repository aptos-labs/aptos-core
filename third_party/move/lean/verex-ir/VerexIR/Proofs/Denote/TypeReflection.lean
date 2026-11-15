-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.NativeMaps

/-! # Type reflection

The Move Prover's translator computes `type_info::type_name`, `type_of`, and
`spec_is_struct` at a type without parameters. These definitions read a
native type the same way; at a type that mentions a parameter the value stays
the uninterpreted function it falls back to. -/

namespace VerexIR.Proofs.Denote

open VerexIR.Validation

/-- A name's bytes: Move identifiers and addresses are ASCII. -/
def nameBytes (name : String) : List Nat := name.toList.map Char.toNat

/-- The address, module, and name of a nominal declaration. -/
def nominalIdentity? (unit : ValidatedUnit) (source : StructHandle) :
    Option (String × String × String) := do
  let ns ← unit.namespaces[source.namespaceId.index]?
  let path ← unit.tables.namespaces[source.namespaceId.index]?
  let declaration ← ns.structs[source.structId]?
  let name ← (ns.nameOf declaration.name)
  match path.segments.toList with
  | [address, module] => some (address, module, name.name)
  | _ => none

/-- `name<a, b>` of a name and its type arguments' names. -/
def withTypeArguments (name : List Nat) : List (List Nat) → List Nat
  | [] => name
  | first :: rest =>
      name ++ nameBytes "<" ++ first ++
        rest.flatMap (fun argument => nameBytes ", " ++ argument) ++ nameBytes ">"

/-- The canonical name of a nominal type at its type arguments' names. -/
def nominalName? (unit : ValidatedUnit) (source : StructHandle) (arguments : List (List Nat)) :
    Option (List Nat) := do
  let (account, module, name) ← nominalIdentity? unit source
  some (nameBytes (account ++ "::" ++ module ++ "::") ++
    withTypeArguments (nameBytes name) arguments)

mutual
/-- The canonical name of a type, unless it mentions a parameter. -/
def NTy.reflectedName? (unit : ValidatedUnit) : NTy → Option (List Nat)
  | .bool => some (nameBytes "bool")
  | .int width signed =>
      some (nameBytes ((if signed then "i" else "u") ++ toString width))
  | .address => some (nameBytes "address")
  | .signer => some (nameBytes "signer")
  | .vector element => do
      let name ← NTy.reflectedName? unit element
      some (nameBytes "vector<" ++ name ++ nameBytes ">")
  | .struct source arguments _ => do
      nominalName? unit source (← NRow.reflectedNames? unit arguments)
  | .enum source arguments _ _ _ => do
      nominalName? unit source (← NRow.reflectedNames? unit arguments)
  | _ => none
termination_by structural τ => τ

/-- The canonical names of a row of types. -/
def NRow.reflectedNames? (unit : ValidatedUnit) : NRow → Option (List (List Nat))
  | .nil => some []
  | .cons τ rest => do
      let name ← NTy.reflectedName? unit τ
      let names ← NRow.reflectedNames? unit rest
      some (name :: names)
termination_by structural row => row
end

/-- The address, module, and name with type arguments of a nominal type that
mentions no parameter. -/
def NTy.reflectedInfo? (unit : ValidatedUnit) (τ : NTy) :
    Option (String × List Nat × List Nat) :=
  let info (source : StructHandle) (arguments : NRow) :
      Option (String × List Nat × List Nat) := do
    let (account, module, name) ← nominalIdentity? unit source
    let names ← NRow.reflectedNames? unit arguments
    some (account, nameBytes module, withTypeArguments (nameBytes name) names)
  match τ with
  | .struct source arguments _ => info source arguments
  | .enum source arguments _ _ _ => info source arguments
  | _ => none

/-- The bytes of a name as a byte vector: the ones of a declared name fit. -/
def bytesVector (bytes : List Nat) : SpecVector (SpecInt (.bits 8) false) :=
  let values := (bytes.map fun byte => SpecInt.ofInt ⟨0, by decide⟩ (Int.ofNat byte)).toArray
  if bounded : values.size < 2 ^ 64 then ⟨values, bounded⟩ else ⟨#[], by decide⟩

/-- `type_name` at `τ`: the value `pack` builds of its name, or `fallback`. -/
def reflectedTypeName {α : Type} (unit : ValidatedUnit) (τ : NTy)
    (pack : SpecVector (SpecInt (.bits 8) false) → α) (fallback : α) : α :=
  match τ.reflectedName? unit with
  | some name => pack (bytesVector name)
  | none => fallback

/-- `type_of` at `τ`: the value `pack` builds of its address, module, and
name, or `fallback`. -/
def reflectedTypeInfo {α : Type} (unit : ValidatedUnit) (τ : NTy)
    (pack : String → SpecVector (SpecInt (.bits 8) false) →
      SpecVector (SpecInt (.bits 8) false) → α) (fallback : α) : α :=
  match τ.reflectedInfo? unit with
  | some (address, module, name) => pack address (bytesVector module) (bytesVector name)
  | none => fallback

/-- `spec_is_struct` at `τ`: a parameter's is `fallback`, an uninterpreted
value of the parameter. -/
def reflectedIsStruct (τ : NTy) (fallback : Bool) : Bool :=
  match τ with
  | .struct .. | .enum .. => true
  | .param _ => fallback
  | _ => false

section Aggregator
variable {unit : ValidatedUnit} [FunctionCarriers unit]

/-- Whether the prelude's `aggregator_v2` computes on a type: `u64`, `u128`. -/
def NTy.isAggregatorElement : NTy → Bool
  | .int 64 false | .int 128 false => true
  | _ => false

/-- The largest value of an aggregator element. -/
def NTy.aggregatorMax : NTy → Int
  | .int width _ => 2 ^ width - 1
  | _ => 0

/-- An aggregator element's integer at the plain layer: the native model
reads fields and values through this one observation. -/
def NTy.aggregatorValuePlain : (τ : NTy) → @NTy.plain (Carriers.runtime unit) τ → Int
  | .int _ _, value => SpecInt.val value
  | _, _ => 0

/-- `NTy.isAggregatorElement` of what a type resolves to at a frame. -/
def Skolems.isAggregatorElement (Θ : Skolems unit) (τ : NTy) : Bool :=
  (Θ.resolve τ).isAggregatorElement

/-- `NTy.aggregatorMax` of what a type resolves to at a frame. -/
def Skolems.aggregatorMax (Θ : Skolems unit) (τ : NTy) : Int :=
  (Θ.resolve τ).aggregatorMax

/-- `NTy.aggregatorValuePlain` of a lent value of a frame. -/
def Skolems.aggregatorValuePlain (Θ : Skolems unit) (τ : NTy)
    (value : @NTy.plain Θ.toCarriers τ) : Int :=
  NTy.aggregatorValuePlain (Θ.resolve τ) (Θ.plainToRuntime τ value)

@[vir_denote, vir_denote_norm] theorem Skolems.isAggregatorElement_instantiate
    (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy) :
    (Skolems.instantiate θ Θ).isAggregatorElement τ = Θ.isAggregatorElement (τ.subst θ.1) := rfl

@[vir_denote, vir_denote_norm] theorem Skolems.aggregatorMax_instantiate
    (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy) :
    (Skolems.instantiate θ Θ).aggregatorMax τ = Θ.aggregatorMax (τ.subst θ.1) := rfl

@[vir_denote, vir_denote_norm] theorem Skolems.aggregatorValuePlain_instantiate
    (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy)
    (value : @NTy.plain (Skolems.instantiate θ Θ).toCarriers τ) :
    (Skolems.instantiate θ Θ).aggregatorValuePlain τ value =
      Θ.aggregatorValuePlain (τ.subst θ.1)
        (@NTy.plainOfSkolem Θ.toCarriers inferInstance θ τ value) := rfl

@[vir_denote, vir_denote_norm] theorem Skolems.isAggregatorElement_runtime
    [CarriersNonempty (Carriers.runtime unit)] (τ : NTy) :
    (Skolems.runtime unit).isAggregatorElement τ = τ.isAggregatorElement := rfl

@[vir_denote, vir_denote_norm] theorem Skolems.isAggregatorElement_int (Θ : Skolems unit)
    (width : Nat) (signed : Bool) :
    Θ.isAggregatorElement (.int width signed) = (NTy.int width signed).isAggregatorElement := by
  simp only [Skolems.isAggregatorElement, Θ.resolve_int]

@[vir_denote, vir_denote_norm] theorem Skolems.aggregatorMax_int (Θ : Skolems unit)
    (width : Nat) (signed : Bool) :
    Θ.aggregatorMax (.int width signed) = 2 ^ width - 1 := by
  simp only [Skolems.aggregatorMax, Θ.resolve_int, NTy.aggregatorMax]

/-- At an integer type every frame's plain value is the integer itself. -/
@[vir_denote, vir_denote_norm] theorem Skolems.aggregatorValuePlain_int (Θ : Skolems unit)
    (width : Nat) (signed : Bool) (value : SpecInt (.bits width) signed) :
    Θ.aggregatorValuePlain (.int width signed) value = value.val := by
  have encoded := Θ.plainEncode_plainToRuntime (.int width signed) value
  have resolved := Θ.resolve_int width signed
  unfold Skolems.aggregatorValuePlain
  generalize Θ.plainToRuntime (.int width signed) value = moved at encoded ⊢
  generalize Θ.resolve (.int width signed) = τ at resolved moved encoded ⊢
  subst resolved
  have same := RuntimeValue.integer.inj encoded
  exact same

end Aggregator

open Lean Meta

/-- A literal byte vector, as a clause's `b"…"` denotes one. -/
private def bytesLiteral (bytes : List Nat) : MetaM Lean.Expr := do
  let width := mkApp (mkConst ``IntWidth.bits) (mkNatLit 8)
  let element := mkApp2 (mkConst ``SpecInt) width (toExpr false)
  let values ← bytes.mapM fun byte => do
    let literal := toExpr (Int.ofNat byte)
    let fits ← mkDecideProof (mkApp3 (mkConst ``IntegerValueFits) width (toExpr false) literal)
    return mkApp4 (mkConst ``SpecInt.mk) width (toExpr false) literal fits
  let array ← mkArrayLit element values
  let bounded ← mkDecideProof
    (← mkAppM ``LT.lt #[← mkAppM ``Array.size #[array], mkNatLit (2 ^ 64)])
  return mkApp3 (mkConst ``SpecVector.mk) element array bounded

/-- A list of natural-number literals, read back. -/
private partial def natList? (e : Lean.Expr) : Option (List Nat) :=
  match_expr e with
  | List.nil _ => some []
  | List.cons _ head tail => do
      let byte ← head.rawNatLit? <|> head.nat?
      return byte :: (← natList? tail)
  | _ => none

/-- A type term that evaluates: no local or metavariable occurs in it. -/
private def closedType (τ : Lean.Expr) : Bool := !τ.hasFVar && !τ.hasMVar

/-- Evaluate a function reading the declarations of a constant unit: a walk
of its tables nests as deep as the tables are long. -/
private def evaluate (e : Lean.Expr) : MetaM Lean.Expr :=
  withTheReader Core.Context (fun context => { context with maxRecDepth := 1 <<< 20 }) <|
    withDefault <| reduce e

/-- The value of `type_name` at a closed type and a constant unit. -/
dsimproc [vir_denote, vir_denote_norm] evaluateTypeName (reflectedTypeName _ _ _ _) := fun e => do
  let_expr reflectedTypeName _ unit τ pack fallback := e | return .continue
  unless unit.isConst && closedType τ do return .continue
  let value ← evaluate (mkApp2 (mkConst ``NTy.reflectedName?) unit τ)
  match_expr value with
  | Option.none _ => return .done fallback
  | Option.some _ name =>
      let some name := natList? name | return .continue
      return .done (mkApp pack (← bytesLiteral name)).headBeta
  | _ => return .continue

/-- The value of `type_of` at a closed type and a constant unit. -/
dsimproc [vir_denote, vir_denote_norm] evaluateTypeInfo (reflectedTypeInfo _ _ _ _) := fun e => do
  let_expr reflectedTypeInfo _ unit τ pack fallback := e | return .continue
  unless unit.isConst && closedType τ do return .continue
  let value ← evaluate (mkApp2 (mkConst ``NTy.reflectedInfo?) unit τ)
  match_expr value with
  | Option.none _ => return .done fallback
  | Option.some _ info =>
      let_expr Prod.mk _ _ address rest := info | return .continue
      let_expr Prod.mk _ _ module name := rest | return .continue
      let some module := natList? module | return .continue
      let some name := natList? name | return .continue
      return .done (mkApp3 pack address (← bytesLiteral module) (← bytesLiteral name)).headBeta
  | _ => return .continue

/-- `spec_is_struct` at a type whose head is known. -/
dsimproc [vir_denote, vir_denote_norm] evaluateIsStruct (reflectedIsStruct _ _) := fun e => do
  let_expr reflectedIsStruct τ fallback := e | return .continue
  let τ ← whnfR τ
  let .const head _ := τ.getAppFn | return .continue
  unless (← getConstInfo head).isCtor do return .continue
  if τ.isAppOf ``NTy.param then return .done fallback
  return .done (toExpr (τ.isAppOf ``NTy.struct || τ.isAppOf ``NTy.enum))

/-- Whether a type whose head is known is an aggregator element. -/
dsimproc [vir_denote, vir_denote_norm] evaluateIsAggregatorElement (NTy.isAggregatorElement _) :=
    fun e => do
  let_expr NTy.isAggregatorElement τ := e | return .continue
  let τ ← whnfR τ
  let .const head _ := τ.getAppFn | return .continue
  unless (← getConstInfo head).isCtor do return .continue
  let value ← evaluate (mkApp (mkConst ``NTy.isAggregatorElement) τ)
  return .done value

end VerexIR.Proofs.Denote

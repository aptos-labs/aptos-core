-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Quote
import LeanerLang.SpecTypes

/-!
# Native value representations

The native representation of one LIR type: the Lean carrier a contract
clause reasons over and the codec relating it to the runtime value.  Shared
by the clause translator and the denotation route.
-/

namespace LeanerLang.Typed

open Lean Meta Elab Command
open LeanerIR (IntWidth RuntimeValue TypeId)
open LeanerIR.Validation (ValidatedUnit)

/-- Native representation available for one function-boundary value. -/
inductive ValueRep where
  | int (width : IntWidth) (signed : Bool)
  | bool
  | string
  | address
  | signer
  | bytes
  | unit
  | vector (element : ValueRep) (bounded : Bool)
  | tuple (elements : Array ValueRep)
  | twin (name : Name) (arguments : Array ValueRep)
  | parameter (index : Nat)
  deriving Repr, Inhabited, BEq

partial def ValueRep.mentionsParameter : ValueRep → Bool
  | .parameter _ => true
  | .vector element _ => element.mentionsParameter
  | .tuple elements => elements.any mentionsParameter
  | .twin _ arguments => arguments.any mentionsParameter
  | _ => false

/-- Replace each type parameter by its representation where one is given. -/
partial def ValueRep.substitute (arguments : Array (Option ValueRep)) : ValueRep → ValueRep
  | .parameter index => (arguments[index]?).join.getD (.parameter index)
  | .vector element bounded => .vector (element.substitute arguments) bounded
  | .tuple elements => .tuple (elements.map (·.substitute arguments))
  | .twin name twinArguments => .twin name (twinArguments.map (·.substitute arguments))
  | .int width signed => .int width signed
  | .bool => .bool
  | .string => .string
  | .address => .address
  | .signer => .signer
  | .bytes => .bytes
  | .unit => .unit

partial def valueRep? (unit : ValidatedUnit)
    (twins : Array SpecTypes.TwinInfo) (typeId : TypeId)
    (profile : Option LeanerIR.Profile) : Option ValueRep := do
  let ty ← unit.tables.types[typeId.index]?
  match ty with
  | .integer .pointer _ | .integer (.bits 0) _ => none
  | .integer width signed => some (.int width signed)
  | .bool => some .bool
  | .string => some .string
  | .address => some .address
  | .signer => some .signer
  | .bytes => some .bytes
  | .unit => some .unit
  | .vector element _ =>
      some (.vector (← valueRep? unit twins element profile) (profile == some .move))
  | .tuple elements => some (.tuple (← elements.mapM (fun ty => valueRep? unit twins ty profile)))
  | .typeParameter index => some (.parameter index)
  | .nominal name arguments => do
      let qualified ← unit.tables.names[name.index]?
      let twin ← twins.find? (·.qualified == qualified)
      let arguments ← arguments.mapM fun argument => match argument with
        | .typeArg value => valueRep? unit twins value.typeId profile
        | .const _ | .lifetime _ | .evidence _ => none
      guard (arguments.size == twin.typeParameterCount)
      some (.twin twin.twin arguments)
  | _ => none

partial def ValueRep.leanType (rep : ValueRep) (carrier? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  match rep with
  | .int width signed => mkAppM ``LeanerIR.SpecInt #[toExpr width, toExpr signed]
  | .bool => return mkConst ``Bool
  | .string | .address | .signer => return mkConst ``String
  | .bytes => mkAppM ``Array #[mkConst ``UInt8]
  | .unit => return mkConst ``Unit
  | .vector element bounded =>
      mkAppM (if bounded then ``LeanerIR.SpecVector else ``Array) #[← element.leanType carrier?]
  | .tuple elements =>
      elements.foldrM (init := mkConst ``Unit) fun element tail => do
        let head ← element.leanType carrier?
        mkAppM ``Prod #[head, tail]
  | .twin name arguments =>
      arguments.foldlM (init := mkConst name) fun type argument =>
        return mkApp type (← argument.leanType carrier?)
  | .parameter index =>
      let some carrier := carrier?
        | throwError "a type-parameter representation has no carrier"
      return mkApp carrier (toExpr index)

/-- Meta-level codec, used while generating the typed contract. -/
partial def ValueRep.codec (rep : ValueRep) (codecs? : Option Lean.Expr) :
    MetaM Lean.Expr := do
  match rep with
  | .int width signed =>
      mkAppM ``LeanerIR.Proofs.Codec.specInt #[toExpr width, toExpr signed]
  | .bool => return mkConst ``LeanerIR.Proofs.Codec.bool
  | .string => return mkConst ``LeanerIR.Proofs.Codec.string
  | .address => return mkConst ``LeanerIR.Proofs.Codec.address
  | .signer => return mkConst ``LeanerIR.Proofs.Codec.signer
  | .bytes => return mkConst ``LeanerIR.Proofs.Codec.bytes
  | .unit => return mkConst ``LeanerIR.Proofs.Codec.unit
  | .vector element bounded =>
      mkAppM (if bounded then ``LeanerIR.Proofs.Codec.boundedVector else
        ``LeanerIR.Proofs.Codec.vector) #[← element.codec codecs?]
  | .tuple elements => do
      let row ← elements.foldrM (init := mkConst ``LeanerIR.Proofs.Codec.tupleNil)
        fun element tail => do
          let head ← element.codec codecs?
          mkAppM ``LeanerIR.Proofs.Codec.tupleCons #[head, tail]
      mkAppM ``LeanerIR.Proofs.Codec.tuple #[row]
  | .twin name arguments => do
      let codecs ← arguments.mapM (·.codec codecs?)
      mkAppM (name ++ `codec) codecs
  | .parameter index =>
      let some codecs := codecs?
        | throwError "a type-parameter representation has no codec family"
      return mkApp codecs (toExpr index)

/-- Runtime encoding of a native value. -/
def ValueRep.encode (rep : ValueRep) (codecs? : Option Lean.Expr)
    (value : Lean.Expr) : MetaM Lean.Expr := do
  mkAppM ``LeanerIR.Proofs.Codec.encode #[← rep.codec codecs?, value]

end LeanerLang.Typed

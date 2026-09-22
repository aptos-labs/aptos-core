-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Denotation
import LeanerLang.SpecTypes

/-!
# Generated native function signatures

V1 denotations retain runtime rows at their semantic boundary.  This module
derives one native argument type and one native result codec per function,
then exposes the denotation through `LeanerIR.Proofs.typedFunction`.

Storage-parametric type parameters become abstract carriers equipped with a
codec.  The generated declarations are polymorphic in both; the eventual
runtime theorem instantiates them with `RuntimeValue` and the identity codec,
while the native theorem remains valid for every represented carrier.
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

/-- How a source parameter crosses the call boundary. -/
inductive ArgumentKind where
  | plain
  | shared
  | mutable
  deriving Repr, BEq, Inhabited

structure ArgumentInfo where
  name : Name
  kind : ArgumentKind
  rep : ValueRep
  deriving Repr, Inhabited

structure ResultInfo where
  name : Name
  kind : ArgumentKind
  rep : ValueRep
  deriving Repr, Inhabited

/-- The native representation of one declaration-local slot.  Unlike the
runtime frame, the slot keeps its source type: a generic local is a value of
`Carrier i`, not a `RuntimeValue`.  Availability is represented separately by
the generated frame's `Option` field. -/
structure LocalInfo where
  name : Name
  kind : ArgumentKind
  rep : ValueRep
  deriving Repr, Inhabited

structure SignatureInfo where
  typeParameterCount : Nat
  arguments : Array ArgumentInfo
  results : Array ResultInfo
  locals : Array LocalInfo := #[]
  /-- Move lowers a source multi-result as one tuple-valued runtime slot.
  The native boundary exposes its components directly and the row codec
  performs that one physical packing step. -/
  packedResults : Bool := false
  deriving Repr, Inhabited

/-- Names and representation data emitted for one typed wrapper. -/
structure Artifacts where
  signature : SignatureInfo
  argumentsType : Name
  argumentsCodec : Name
  localsType : Name
  initialLocals : Name
  resultsType? : Option Name
  resultsCodec : Name
  denotation : Name
  deriving Repr, Inhabited

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

private def root (segments : Array String) (function : String) : Name :=
  Name.str (pathName segments) function

def argumentsTypeName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `Arguments

def argumentsCodecName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `argumentsCodec

def localsTypeName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `Locals

def initialLocalsName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `initialLocals

def resultsTypeName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `Results

def resultsCodecName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `resultsCodec

def denotationName (segments : Array String) (function : String) : Name :=
  (root segments function) ++ `typedDenotation

private def rootIdent (name : Name) : Ident :=
  mkIdent (rootNamespace ++ name)

private def natLit (value : Nat) : Term := Syntax.mkNatLit value

/-- Resolve one LIR type to its native boundary representation. -/
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

/-- Classify a function signature for V3.  Unsupported boundary types are a
reason to retain the existing runtime-row route, never to invent a codec. -/
def signatureInfo? (unit : ValidatedUnit)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo) : Except String SignatureInfo := do
  unless declaration.signature.generics.all (fun binder => binder.kind == .typeArg) do
    throw "native wrappers support type parameters only"
  let mut arguments : Array ArgumentInfo := #[]
  for (parameter, index) in declaration.signature.parameters.zipIdx do
    let some declared := unit.tables.types[parameter.typeUse.typeId.index]?
      | throw s!"parameter type {parameter.typeUse.typeId.index} is out of range"
    let (kind, physical) ← match declared with
      | .reference reference =>
          pure (if reference.kind == .mutable then .mutable else .shared,
            reference.referent)
      | _ => pure (.plain, parameter.typeUse.typeId)
    let some rep := valueRep? unit twins physical (some declaration.profile)
      | throw s!"parameter `{parameter.name}` has no native V3 representation"
    arguments := arguments.push {
      name := Name.mkSimple (if parameter.name.isEmpty then s!"argument{index}"
        else parameter.name)
      kind, rep }
  let (resultTypes, packedResults) : Array TypeId × Bool :=
    match declaration.profile, declaration.signature.results.toList with
    | .move, [result] =>
        match unit.tables.types[result.typeId.index]? with
        | some (.tuple components) => (components, true)
        | _ => (#[result.typeId], false)
    | _, _ => (declaration.signature.results.map fun result => result.typeId, false)
  let mut results : Array ResultInfo := #[]
  for (resultType, index) in resultTypes.zipIdx do
    let some declared := unit.tables.types[resultType.index]?
      | throw s!"result type {resultType.index} is out of range"
    let (kind, physical) : ArgumentKind × TypeId ← match declared with
      | .reference reference =>
          pure (if reference.kind == .mutable then .mutable else .shared,
            reference.referent)
      | _ => pure (.plain, resultType)
    let some rep := valueRep? unit twins physical (some declaration.profile)
      | throw s!"result {index} has no native V3 representation"
    results := results.push {
      name := Name.mkSimple (if resultTypes.size == 1 then
        "result" else s!"result{index}")
      kind, rep }
  let mut locals : Array LocalInfo := #[]
  for (localDecl, index) in declaration.locals.zipIdx do
    let some declared := unit.tables.types[localDecl.type.typeId.index]?
      | throw s!"local type {localDecl.type.typeId.index} is out of range"
    let (kind, physical) : ArgumentKind × TypeId ← match declared with
      | .reference reference =>
          pure (if reference.kind == .mutable then .mutable else .shared,
            reference.referent)
      | _ => pure (.plain, localDecl.type.typeId)
    let some rep := valueRep? unit twins physical (some declaration.profile)
      | throw s!"local `{localDecl.name}` has no native V3 representation"
    locals := locals.push {
      name := Name.mkSimple s!"local{index}"
      kind, rep }
  return {
    typeParameterCount := declaration.signature.generics.size
    arguments, results, locals, packedResults }

/-- Lean type of one native representation in a generated declaration. -/
partial def ValueRep.typeSyntax (rep : ValueRep) (carrier : Ident) :
    CommandElabM Term :=
  match rep with
  | .int (.bits width) signed =>
      ``(LeanerIR.SpecInt (LeanerIR.IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.SpecInt LeanerIR.IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width native value"
  | .bool => ``(Bool)
  | .string | .address | .signer => ``(String)
  | .bytes => ``(Array UInt8)
  | .unit => ``(Unit)
  | .vector element bounded => do
      if bounded then ``(LeanerIR.SpecVector $(← element.typeSyntax carrier))
      else ``(Array $(← element.typeSyntax carrier))
  | .tuple elements => do
      let mut result ← ``(Unit)
      for element in elements.reverse do
        let head ← element.typeSyntax carrier
        result ← ``($head × $result)
      return result
  | .twin name arguments => do
      let arguments ← arguments.mapM (·.typeSyntax carrier)
      if arguments.isEmpty then return rootIdent name
      ``($(rootIdent name) $arguments*)
  | .parameter index => ``($carrier $(natLit index))

/-- Codec term of one native representation in a generated declaration. -/
partial def ValueRep.codecSyntax (rep : ValueRep) (codecs : Ident) :
    CommandElabM Term :=
  match rep with
  | .int (.bits width) signed =>
      ``(LeanerIR.Proofs.Codec.specInt
        (LeanerIR.IntWidth.bits $(natLit width)) $(quote signed))
  | .int .unbounded signed =>
      ``(LeanerIR.Proofs.Codec.specInt
        LeanerIR.IntWidth.unbounded $(quote signed))
  | .int .pointer _ => throwError "internal: pointer-width native codec"
  | .bool => ``(LeanerIR.Proofs.Codec.bool)
  | .string => ``(LeanerIR.Proofs.Codec.string)
  | .address => ``(LeanerIR.Proofs.Codec.address)
  | .signer => ``(LeanerIR.Proofs.Codec.signer)
  | .bytes => ``(LeanerIR.Proofs.Codec.bytes)
  | .unit => ``(LeanerIR.Proofs.Codec.unit)
  | .vector element bounded => do
      if bounded then ``(LeanerIR.Proofs.Codec.boundedVector $(← element.codecSyntax codecs))
      else ``(LeanerIR.Proofs.Codec.vector $(← element.codecSyntax codecs))
  | .tuple elements => do
      let mut row ← ``(LeanerIR.Proofs.Codec.tupleNil)
      for element in elements.reverse do
        row ← ``(LeanerIR.Proofs.Codec.tupleCons
          $(← element.codecSyntax codecs) $row)
      ``(LeanerIR.Proofs.Codec.tuple $row)
  | .twin name arguments => do
      let arguments ← arguments.mapM (·.codecSyntax codecs)
      ``($(rootIdent (name ++ `codec)) $arguments*)
  | .parameter index => ``($codecs $(natLit index))

private def ArgumentInfo.typeSyntax (info : ArgumentInfo) (carrier : Ident) :
    CommandElabM Term := do
  let base ← info.rep.typeSyntax carrier
  if info.kind == .mutable then
    ``(LeanerIR.Proofs.MutableArgument $base)
  else return base

private def ArgumentInfo.codecSyntax (info : ArgumentInfo) (codecs : Ident) :
    CommandElabM Term := do
  let base ← info.rep.codecSyntax codecs
  if info.kind == .mutable then
    ``(LeanerIR.Proofs.Codec.mutable $base)
  else return base

private def ResultInfo.typeSyntax (info : ResultInfo) (carrier : Ident) :
    CommandElabM Term := do
  let base ← info.rep.typeSyntax carrier
  if info.kind == .mutable then
    ``(LeanerIR.Proofs.MutableArgument $base)
  else return base

private def ResultInfo.codecSyntax (info : ResultInfo) (codecs : Ident) :
    CommandElabM Term := do
  let base ← info.rep.codecSyntax codecs
  if info.kind == .mutable then
    ``(LeanerIR.Proofs.Codec.mutable $base)
  else return base

private def LocalInfo.typeSyntax (info : LocalInfo) (carrier : Ident) :
    CommandElabM Term := do
  let base ← info.rep.typeSyntax carrier
  if info.kind == .mutable then
    ``(LeanerIR.Proofs.MutableArgument $base)
  else return base

/-- Meta-level native type, used while generating the typed contract. -/
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

/-- Clause-level view of a native value.  Integers expose their mathematical
value.  Twins and abstract carriers are erased only at the operations that
still use the runtime aggregate vocabulary. -/
def ValueRep.logical (rep : ValueRep) (codecs? : Option Lean.Expr)
    (value : Lean.Expr) : MetaM Lean.Expr := do
  match rep with
  | .int _ _ => mkAppM ``LeanerIR.SpecInt.val #[value]
  | .vector _ _ | .tuple _ | .twin _ _ | .parameter _ => rep.encode codecs? value
  | _ => return value

def resultTypeSyntax (signature : SignatureInfo) (carrier : Ident) :
    CommandElabM Term := do
  match signature.results.toList with
  | [] => ``(Unit)
  | [result] => result.typeSyntax carrier
  | _ =>
      throwError "internal: a multi-result wrapper must name its result structure"

/-- Emit a row codec for a generated structure. -/
private def emitArguments (signature : SignatureInfo) (typeName codecName : Name) :
    CommandElabM Unit := do
  let carrier := mkIdent `Carrier
  let codecs := mkIdent `codecs
  let typeIdent := rootIdent typeName
  let codecIdent := rootIdent codecName
  let fieldIds := signature.arguments.map (mkIdent ·.name)
  let fieldTypes ← signature.arguments.mapM (·.typeSyntax carrier)
  if signature.typeParameterCount == 0 then
    elabCommand (← `(@[ext] structure $typeIdent:ident where
      $[($fieldIds:ident : $fieldTypes:term)]*))
  else
    elabCommand (← `(@[ext] structure $typeIdent:ident
        ($carrier:ident : Nat → Type) where
      $[($fieldIds:ident : $fieldTypes:term)]*))
  let runtimeIds := signature.arguments.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"runtime{index}")
  let decodedIds := signature.arguments.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"decoded{index}")
  let codecTerms ← signature.arguments.mapM (·.codecSyntax codecs)
  let projections := signature.arguments.map fun argument =>
    (rootIdent (typeName ++ argument.name), mkIdent `arguments)
  let encoded : Array Term ←
    (codecTerms.zip projections).mapM fun (codec, (projection, value)) =>
      ``(LeanerIR.Proofs.Codec.encode $codec ($projection $value))
  let mut decodeChain ← ``(some ⟨$decodedIds,*⟩)
  for index in (List.range signature.arguments.size).reverse do
    decodeChain ← ``(Option.bind
        (LeanerIR.Proofs.Codec.decode? $(codecTerms[index]!)
          $(runtimeIds[index]!))
        fun $(decodedIds[index]!):ident => $decodeChain)
  let value := mkIdent `arguments
  let values := mkIdent `values
  let body ← `(term| {
    encode := fun $value:ident => #[$encoded,*],
    decode? := fun $values:ident =>
      (match ($values:ident).toList with
      | [$runtimeIds,*] => $decodeChain
      | _ => none),
    decode_encode := by
      intro $value:ident
      cases $value:ident
      simp })
  if signature.typeParameterCount == 0 then
    elabCommand (← `(def $codecIdent:ident :
        LeanerIR.Proofs.Codec $typeIdent (Array LeanerIR.RuntimeValue) := $body))
  else
    elabCommand (← `(def $codecIdent:ident
        {$carrier:ident : Nat → Type}
        ($codecs:ident : ∀ index,
          LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue) :
        LeanerIR.Proofs.Codec ($typeIdent $carrier)
      (Array LeanerIR.RuntimeValue) := $body))

/-- Emit the function's typed local frame and its parameter initialization.
Every local has a statically known field, so generated reads and writes can
reduce to projections and record updates without a heterogeneous lookup or a
runtime cast. -/
private def emitLocals (signature : SignatureInfo) (argumentsType typeName
    initialName : Name) : CommandElabM Unit := do
  let carrier := mkIdent `Carrier
  let frame := rootIdent typeName
  let arguments := rootIdent argumentsType
  let fieldIds := signature.locals.map (mkIdent ·.name)
  let fieldTypes ← signature.locals.mapM fun localInfo => do
    ``(Option $(← localInfo.typeSyntax carrier))
  if signature.typeParameterCount == 0 then
    elabCommand (← `(@[ext] structure $frame:ident where
      $[($fieldIds:ident : $fieldTypes:term)]*))
  else
    elabCommand (← `(@[ext] structure $frame:ident
        ($carrier:ident : Nat → Type) where
      $[($fieldIds:ident : $fieldTypes:term)]*))
  let defaults : Array Term ← signature.locals.mapM fun _ => ``(none)
  let values ← signature.locals.mapIdxM fun index _ => do
    if h : index < signature.arguments.size then
      let projection := rootIdent
        (argumentsType ++ signature.arguments[index].name)
      ``(some ($projection $(mkIdent `arguments)))
    else
      ``(none)
  let initial := rootIdent initialName
  if signature.typeParameterCount == 0 then
    elabCommand (← `(instance : Inhabited $frame := ⟨⟨$defaults,*⟩⟩))
    elabCommand (← `(def $initial:ident
        ($(mkIdent `arguments):ident : $arguments) : $frame :=
      ⟨$values,*⟩))
  else
    elabCommand (← `(instance : Inhabited ($frame $carrier) :=
      ⟨⟨$defaults,*⟩⟩))
    elabCommand (← `(def $initial:ident
        {$(mkIdent `Carrier):ident : Nat → Type}
        ($(mkIdent `arguments):ident : $arguments $carrier) : $frame $carrier :=
      ⟨$values,*⟩))

/-- Emit the result type when needed and its row codec. -/
private def emitResults (signature : SignatureInfo) (typeName codecName : Name) :
    CommandElabM (Option Name) := do
  let carrier := mkIdent `Carrier
  let codecs := mkIdent `codecs
  let codecIdent := rootIdent codecName
  let mut typeName? : Option Name := none
  if signature.results.size > 1 then
    typeName? := some typeName
    let typeIdent := rootIdent typeName
    let fieldIds := signature.results.map (mkIdent ·.name)
    let fieldTypes ← signature.results.mapM fun result =>
      result.typeSyntax carrier
    if signature.typeParameterCount == 0 then
      elabCommand (← `(@[ext] structure $typeIdent:ident where
        $[($fieldIds:ident : $fieldTypes:term)]*))
    else
      elabCommand (← `(@[ext] structure $typeIdent:ident
          ($carrier:ident : Nat → Type) where
        $[($fieldIds:ident : $fieldTypes:term)]*))
  let resultType ← match typeName? with
    | some name =>
        if signature.typeParameterCount == 0 then
          pure (⟨(rootIdent name).raw⟩ : Term)
        else ``($(rootIdent name) $carrier)
    | none => resultTypeSyntax signature carrier
  let runtimeIds := signature.results.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"runtime{index}")
  let decodedIds := signature.results.mapIdx fun index _ =>
    mkIdent (Name.mkSimple s!"decoded{index}")
  let codecTerms ← signature.results.mapM fun result =>
    result.codecSyntax codecs
  let value := mkIdent `result
  let nativeValues : Array Term ← match signature.results.toList with
    | [] => pure #[]
    | [_] => pure #[⟨value.raw⟩]
    | _ => signature.results.mapM fun result =>
        ``($(rootIdent (typeName ++ result.name)) $value)
  let encoded : Array Term ←
    (codecTerms.zip nativeValues).mapM fun (codec, native) =>
      ``(LeanerIR.Proofs.Codec.encode $codec $native)
  let encodedRow : Array Term ← if signature.packedResults then
    pure #[← `(LeanerIR.RuntimeValue.tuple #[$encoded,*])]
  else pure encoded
  let constructor ← match signature.results.toList with
    | [] => ``(())
    | [_] => pure (⟨decodedIds[0]!.raw⟩ : Term)
    | _ => ``(⟨$decodedIds,*⟩)
  let mut decodeChain ← ``(some $constructor)
  for index in (List.range signature.results.size).reverse do
    decodeChain ← ``(Option.bind
        (LeanerIR.Proofs.Codec.decode? $(codecTerms[index]!)
          $(runtimeIds[index]!))
        fun $(decodedIds[index]!):ident => $decodeChain)
  let values := mkIdent `values
  let decodeBody ← if signature.packedResults then
    let packed := mkIdent `packed
    `(term|
      (match ($values:ident).toList with
      | [LeanerIR.RuntimeValue.tuple $packed:ident] =>
          match ($packed:ident).toList with
          | [$runtimeIds,*] => $decodeChain
          | _ => none
      | _ => none))
  else
    `(term|
      (match ($values:ident).toList with
      | [$runtimeIds,*] => $decodeChain
      | _ => none))
  let casesResult ← if signature.results.size > 1 then
    `(tactic| cases $value:ident)
  else
    `(tactic| skip)
  let body ← `(term| {
    encode := fun $value:ident => #[$encodedRow,*],
    decode? := fun $values:ident => $decodeBody,
    decode_encode := by
      intro $value:ident
      $casesResult
      simp })
  if signature.typeParameterCount == 0 then
    elabCommand (← `(def $codecIdent:ident :
        LeanerIR.Proofs.Codec $resultType (Array LeanerIR.RuntimeValue) := $body))
  else
    elabCommand (← `(def $codecIdent:ident
        {$carrier:ident : Nat → Type}
        ($codecs:ident : ∀ index,
          LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue) :
        LeanerIR.Proofs.Codec $resultType
          (Array LeanerIR.RuntimeValue) := $body))
  return typeName?

/-- Generate one function's native signature and certified row codecs.  This
stage does not need a denotation, so `#leaner_contract` can expose the same
typed boundary as `verify`. -/
def ensureSignatureDefinitions (unit : ValidatedUnit) (segments : Array String)
    (function : String)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo) :
    CommandElabM (Except String Artifacts) := do
  let signature ← match signatureInfo? unit declaration twins with
    | .error reason => return .error reason
    | .ok signature => pure signature
  let argumentsType := argumentsTypeName segments function
  let argumentsCodec := argumentsCodecName segments function
  let localsType := localsTypeName segments function
  let initialLocals := initialLocalsName segments function
  let resultsType := resultsTypeName segments function
  let resultsCodec := resultsCodecName segments function
  let denotation := denotationName segments function
  let env ← getEnv
  unless env.contains argumentsCodec do
    emitArguments signature argumentsType argumentsCodec
  let env ← getEnv
  unless env.contains localsType do
    emitLocals signature argumentsType localsType initialLocals
  let env ← getEnv
  let resultsType? ← if env.contains resultsCodec then
    pure (if signature.results.size > 1 then some resultsType else none)
  else do
    let emitted ← emitResults signature resultsType resultsCodec
    elabCommand (← `(attribute [lir_data_norm] $(rootIdent resultsCodec)))
    pure emitted
  return .ok {
    signature := signature
    argumentsType := argumentsType
    argumentsCodec := argumentsCodec
    localsType := localsType
    initialLocals := initialLocals
    resultsType? := resultsType?
    resultsCodec := resultsCodec
    denotation := denotation }

/-- Add the typed view of a V1 denotation after its signature artifacts are
available. -/
private def ensureDenotationDefinition (artifacts : Artifacts)
    (runtimeDenotation : Name) : CommandElabM Unit := do
  let signature := artifacts.signature
  let env ← getEnv
  unless env.contains artifacts.denotation do
    -- The multi-result spelling is rooted alongside the codec; pass it
    -- directly rather than deriving it from source syntax.
    let carrier := mkIdent `Carrier
    let codecs := mkIdent `codecs
    let executable := mkIdent `executable
    let typeInstantiation := mkIdent `typeInstantiation
    let argsTerm ← if signature.typeParameterCount == 0 then
      pure (⟨(rootIdent artifacts.argumentsType).raw⟩ : Term)
    else ``($(rootIdent artifacts.argumentsType) $carrier)
    let resultTerm ← match artifacts.resultsType? with
      | some name =>
          if signature.typeParameterCount == 0 then
            pure (⟨(rootIdent name).raw⟩ : Term)
          else ``($(rootIdent name) $carrier)
      | none => resultTypeSyntax signature carrier
    let name := rootIdent artifacts.denotation
    if signature.typeParameterCount == 0 then
      elabCommand (← `(def $name:ident
          ($executable:ident : LeanerIR.Validation.ExecutableUnit) :
          $argsTerm → LeanerIR.Proofs.Spec LeanerIR.RuntimeState
            LeanerIR.Proofs.Failure $resultTerm :=
        LeanerIR.Proofs.typedFunction $(rootIdent artifacts.argumentsCodec)
          $(rootIdent artifacts.resultsCodec)
          ($(rootIdent runtimeDenotation) $executable)))
    else
      elabCommand (← `(def $name:ident
          {$carrier:ident : Nat → Type}
          ($codecs:ident : ∀ index,
            LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue)
          ($typeInstantiation:ident :
            Array (LeanerIR.TypeId × LeanerIR.TypeId))
          ($executable:ident : LeanerIR.Validation.ExecutableUnit) :
          $argsTerm → LeanerIR.Proofs.Spec LeanerIR.RuntimeState
            LeanerIR.Proofs.Failure $resultTerm :=
        LeanerIR.Proofs.typedFunction
          ($(rootIdent artifacts.argumentsCodec) $codecs)
          ($(rootIdent artifacts.resultsCodec) $codecs)
          ($(rootIdent runtimeDenotation) $executable $typeInstantiation)))

/-- Generate one function's native signature, codecs, and denotation view. -/
def ensureDefinitions (unit : ValidatedUnit) (segments : Array String)
    (function : String)
    (declaration : LeanerIR.FunctionDecl LeanerIR.Validation.FunctionBody)
    (twins : Array SpecTypes.TwinInfo) (runtimeDenotation : Name) :
    CommandElabM (Except String Artifacts) := do
  let result ← ensureSignatureDefinitions unit segments function declaration twins
  match result with
  | .error reason => return .error reason
  | .ok artifacts =>
      ensureDenotationDefinition artifacts runtimeDenotation
      return .ok artifacts

end LeanerLang.Typed

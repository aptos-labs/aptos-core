-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Term

/-!
# Compilation of validated LIR to typed terms

`compileFunction` is a Lean function of the validated unit: it reads a
function's body out of the expression arena and produces the typed term
that denotes it, or names the construct the denotation does not carry.
It is structural recursion on a fuel, so the kernel unfolds it on a quoted
unit and a per-target certificate is a `rfl`.

Everything it decides is decided on identities and indexes — never on
strings — so that reduction stays cheap.  A rejected construct is a
negative check naming it, never an approximation.
-/

namespace LeanerIR.Proofs.Denote

open LeanerIR.Validation LeanerIR.SemanticOperations

mutual
/-- The native type an LIR type denotes.  A shared reference denotes its
referent: certified exclusivity erases it at run time.  A nominal type
denotes its declaration's rows, resolved in the declaring namespace; the
fuel bounds the nesting of declarations. -/
def ntyOfFuel (unit : ValidatedUnit) : Nat → NamespaceId → TypeId → Option NTy
  | 0, _, _ => none
  | fuel + 1, namespaceId, typeId => do
      let ns ← unit.namespaces[namespaceId.index]?
      let ty ← ns.tables.types[typeId.index]?
      match ty with
      | .unit => some .unit
      | .bool => some .bool
      | .integer (.bits width) signed => if width == 0 then none else some (.int width signed)
      | .address => some .address
      | .signer => some .signer
      | .string => some .string
      | .bytes => some .bytes
      | .reference reference =>
          match reference.kind with
          | .shared => ntyOfFuel unit fuel namespaceId reference.referent
          | .mutable => .ref <$> ntyOfFuel unit fuel namespaceId reference.referent
      | .tuple elements =>
          (.tuple ∘ NRow.ofList) <$> elements.toList.mapM (ntyOfFuel unit fuel namespaceId)
      | .vector element none => .vector <$> ntyOfFuel unit fuel namespaceId element
      | .nominal name arguments => do
          let handle ← resolveNominal? unit ns name
          let generic ← structNTyFuel unit fuel handle
          if arguments.isEmpty then some generic else
            let θ ← arguments.toList.mapM fun argument => match argument with
              | .typeArg value => ntyOfFuel unit fuel namespaceId value.typeId
              | _ => none
            some (generic.subst (NRow.ofList θ))
      | .typeParameter index => some (.param index)
      | _ => none

/-- The native type of a nominal declaration: its field row, or its named
variant rows. -/
def structNTyFuel (unit : ValidatedUnit) : Nat → StructHandle → Option NTy
  | 0, _ => none
  | fuel + 1, handle => do
      let targetNs ← unit.namespaces[handle.namespaceId.index]?
      let declaration ← targetNs.structs[handle.structId]?
      let row := fun (fields : Array FieldDecl) =>
        NRow.ofList <$> fields.toList.mapM fun field =>
          ntyOfFuel unit fuel handle.namespaceId field.type.typeId
      if declaration.variants.isEmpty then .struct handle <$> row declaration.fields
      else
        let names ← declaration.variants.toList.mapM fun variant =>
          (targetNs.tables.names[variant.name.index]?).map (·.name)
        let rows ← NRows.ofList <$> declaration.variants.toList.mapM fun variant =>
          row variant.fields
        if distinct : names.Nodup then some (.enum handle names rows distinct) else none
end

/-- The fuel that bounds the nesting of every type in a unit: two steps per
type-table entry, since validated types are not recursive and a nominal
type costs a step of its own. -/
def typeFuel (unit : ValidatedUnit) : Nat :=
  2 * unit.namespaces.foldl (fun total ns => total + ns.tables.types.size) 0 + 2

def ntyOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  ntyOfFuel unit (typeFuel unit) namespaceId typeId

/-- The native type of a nominal declaration. -/
def structNTy (unit : ValidatedUnit) (handle : StructHandle) : Option NTy :=
  structNTyFuel unit (typeFuel unit) handle

/-- The construct a node uses, for a rejection message. -/
private def describeKind : ExprKind → String
  | .value .. => "value"
  | .constant _ => "constant"
  | .localVar _ => "local"
  | .operation operation _ _ _ =>
      match operation with
      | .move _ => "move of a place"
      | .copy _ => "copy of a place"
      | .borrow .mutable _ => "mutable borrow"
      | .borrow _ _ => "borrow"
      | .read _ => "read of a place"
      | .write _ => "write of a place"
      | .call _ => "call"
      | .global _ => "global storage operation"
      | .primitive primitive => s!"primitive {repr primitive}"
      | .reference _ => "reference operation"
      | .data _ => "data operation"
      | .specification _ => "specification operation"
      | .assert => "assert"
      | .drop _ => "drop"
      | .profile .. => "profile operation"
  | .block .. => "block"
  | .letDecl .. => "let"
  | .ifElse .. => "if"
  | .match_ .. => "match"
  | .loop .. => "loop"
  | .break_ _ (some _) => "break with a value"
  | .break_ .. => "break"
  | .continue_ _ => "continue"
  | .return_ _ => "return"
  | .throw_ .. => "throw"
  | .assign .. => "assignment"
  | .assignPattern .. => "pattern assignment"
  | .quantifier .. => "quantifier"
  | .spec _ => "specification block"

/-- The rejection of a construct. -/
def notCarried (what : String) : Except String α :=
  .error s!"{what} is not carried by the denotation"

/-- The native type of a node, `none` for the `never` type of a node that
transfers control. -/
private def typeOf? (unit : ValidatedUnit) (ns : ValidatedNamespace) (namespaceId : NamespaceId)
    (typeId : TypeId) : Except String (Option NTy) :=
  match ns.tables.types[typeId.index]? with
  | some .never => .ok none
  | _ => match ntyOf unit namespaceId typeId with
    | some τ => .ok (some τ)
    | none => notCarried "the type of an expression"

/-- The typed position of a declaration index in a row. -/
def Var.ofIndex : (Γ : NRow) → Nat → Option (Σ τ : NTy, Var Γ τ)
  | .nil, _ => none
  | .cons τ _, 0 => some ⟨τ, .here⟩
  | .cons _ Γ, index + 1 => do
      let ⟨τ, x⟩ ← Var.ofIndex Γ index
      some ⟨τ, .there x⟩

/-- The position of a declaration index at an expected type. -/
def Var.at? (Γ : NRow) (index : Nat) (τ : NTy) : Except String (Var Γ τ) :=
  match Var.ofIndex Γ index with
  | none => .error "a local is out of range"
  | some ⟨τ', x⟩ =>
      if equal : τ' = τ then .ok (equal ▸ x) else .error "a local has an unexpected type"

/-- The variant choice a name denotes. -/
def Which.ofName : (names : List String) → (rows : NRows) → String →
    Option (Σ σs : NRow, Which names rows σs)
  | _, .nil, _ => none
  | [], .cons _ _, _ => none
  | name :: names, .cons fields rest, wanted =>
      if name = wanted then some ⟨fields, .here⟩
      else do
        let ⟨σs, later⟩ ← Which.ofName names rest wanted
        some ⟨σs, .there later⟩

/-- The payload choices of a variant-field selection at an expected type. -/
def Choices.ofList (names : List String) (rows : NRows) (τ : NTy) :
    List (String × Nat) → Except String (Choices names rows τ)
  | [] => .ok .nil
  | (variant, index) :: rest => do
      let some ⟨σs, choice⟩ := Which.ofName names rows variant
        | .error "a variant field selection names an unknown variant"
      let x ← Var.at? σs index τ
      let rest ← Choices.ofList names rows τ rest
      .ok (.cons choice x rest)

/-- A compiled expression: a term at its declared type, or a term of every
type, which is what a node of type `never` denotes. -/
inductive Compiled (ρ : ResultShape) (Γ : NRow) : Type where
  | at (τ : NTy) (term : Term ρ Γ τ)
  | never (term : (τ : NTy) → Term ρ Γ τ)

/-- A compiled expression at an expected type. -/
def Compiled.at? {ρ : ResultShape} {Γ : NRow} (τ : NTy) :
    Compiled ρ Γ → Except String (Term ρ Γ τ)
  | .at τ' term =>
      if equal : τ' = τ then .ok (equal ▸ term) else .error "an expression has an unexpected type"
  | .never term => .ok (term τ)

/-- A compiled expression at its own type, taking unit for one of every type. -/
def Compiled.some {ρ : ResultShape} {Γ : NRow} : Compiled ρ Γ → Σ τ : NTy, Term ρ Γ τ
  | .at τ term => ⟨τ, term⟩
  | .never term => ⟨.unit, term .unit⟩

/-- Wrap a compiled expression in a type-preserving context. -/
def Compiled.map {ρ : ResultShape} {Γ : NRow}
    (wrap : (τ : NTy) → Term ρ Γ τ → Term ρ Γ τ) : Compiled ρ Γ → Compiled ρ Γ
  | .at τ term => .at τ (wrap τ term)
  | .never term => .never fun τ => wrap τ (term τ)

mutual
/-- The native value of a constant at its declared type. -/
def literalValue : (τ : NTy) → ConstValue → Except String τ.groundCarrier
  | .int width signed, .integer value =>
      if fits : IntegerValueFits (.bits width) signed value then .ok ⟨value, fits⟩
      else .error "an integer literal is out of range"
  | .bool, .bool value => .ok value
  | .unit, .unit => .ok ()
  | .address, .address value => .ok value
  | .string, .string value => .ok value
  | .bytes, .bytes value => .ok value
  | .tuple elements, .tuple values => literalRow elements values.toList
  | _, _ => notCarried "a literal of this type"

def literalRow : (row : NRow) → List ConstValue → Except String (@HList Skolems.ground row)
  | .nil, [] => .ok ()
  | .cons τ rest, value :: values => do
      let head ← literalValue τ value
      let tail ← literalRow rest values
      .ok (head, tail)
  | _, _ => .error "a tuple literal's arity differs from its type's"
end

/-- A typed place: the local it is rooted in and the path to its component. -/
structure TypedPlace (Γ : NRow) where
  root : NTy
  component : NTy
  x : Var Γ root
  path : Proj root component

/-- Extend a path by one more step at its end. -/
def Proj.append : {τ σ υ : NTy} → Proj τ σ → Proj σ υ → Proj τ υ
  | _, _, _, .nil, rest => rest
  | _, _, _, .deref path, rest => .deref (path.append rest)
  | _, _, _, .field x path, rest => .field x (path.append rest)
  | _, _, _, .index position path, rest => .index position (path.append rest)
  | _, _, _, .variant choices path, rest => .variant choices (path.append rest)

/-- The typed place a place names. -/
def compilePlace (unit : ValidatedUnit) (Γ : NRow) (ns : ValidatedNamespace) :
    Nat → PlaceId → Except String (TypedPlace Γ)
  | 0, _ => .error "the compiler ran out of fuel"
  | fuel + 1, placeId => do
      let some place := ns.places[placeId.index]? | .error "a place is out of range"
      match place with
      | .localVar localId =>
          let some ⟨τ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
          .ok ⟨τ, τ, x, .nil⟩
      | .deref base =>
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          match component, path with
          | .ref referent, path => .ok ⟨root, referent, x, path.append (.deref .nil)⟩
          | _, _ => notCarried "a dereference of a place that is not a mutable reference"
      | .field base owner field =>
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          match component, path with
          | .struct source fields, path =>
              let some handle := resolveStruct? unit ns.identity owner
                | .error "a field place does not resolve"
              unless handle = source do .error "a field place's declaration differs from its type"
              let some name := sourceFieldName? ns field | .error "a field place has no name"
              let some index := handleFieldIndex? unit handle none name
                | .error "a field place names an unknown field"
              let some ⟨σ, position⟩ := Var.ofIndex fields index
                | .error "a field place is out of range"
              .ok ⟨root, σ, x, path.append (.field position .nil)⟩
          | .enum source names rows _, path =>
              let some handle := resolveStruct? unit ns.identity owner
                | .error "a variant field place does not resolve"
              unless handle = source do .error "a variant field place's declaration differs from its type"
              let some name := sourceFieldName? ns field | .error "a variant field place has no name"
              let some choices := variantFieldChoices? unit handle #[name]
                | .error "a variant field place has no choices"
              let (firstVariant, firstIndex) :: _ := choices.toList
                | .error "a variant field place names a field of no variant"
              let some ⟨σs, _⟩ := Which.ofName names rows firstVariant
                | .error "a variant field place names an unknown variant"
              let some ⟨σ, _⟩ := Var.ofIndex σs firstIndex
                | .error "a variant field place is out of range"
              let choices ← Choices.ofList names rows σ choices.toList
              .ok ⟨root, σ, x, path.append (.variant choices .nil)⟩
          | _, _ => notCarried "a field place on a non-struct"
      | .index base indexExpr =>
          let ⟨root, component, x, path⟩ ← compilePlace unit Γ ns fuel base
          let some form := placeIndexForm? ns indexExpr
            | notCarried "an element place with this index form"
          let position ← match form with
            | .literal value =>
                if value < 0 then notCarried "a negative element index"
                else .ok (PlaceIndex.literal value.toNat)
            | .local localId | .copyLocal localId => .ok (PlaceIndex.slot localId.index)
            | .fromEnd source offset =>
                if source == base then .ok (PlaceIndex.fromEnd offset)
                else notCarried "an element index from the end of another place"
          match component, path with
          | .vector σ, path => .ok ⟨root, σ, x, path.append (.index position .nil)⟩
          | _, _ => notCarried "an element place on a non-vector"
      | _ => notCarried "a place of this kind"

/-- Whether the components of a tuple hold references only at their top. -/
def NRow.componentsLendable : NRow → Bool
  | .nil => true
  | .cons (.ref referent) rest => referent.refFree && rest.componentsLendable
  | .cons τ rest => τ.refFree && rest.componentsLendable

/-- Whether a value's references are where the prophetic meaning lends
them: at its top, or as components of a tuple. -/
def NTy.lendable : NTy → Bool
  | .ref referent => referent.refFree
  | .tuple elements => elements.componentsLendable
  | τ => τ.refFree

/-- Whether a result's references are lendable. -/
def ResultShape.lendable : ResultShape → Bool
  | .none => true
  | .one τ => τ.lendable

/-- The locals the borrow analysis found holding a function's lexical
loan's reference. -/
def loanHolders? (unit : ValidatedUnit) (handle : FunctionHandle) (loan : LoanId) :
    Option (Array LocalId) := do
  let certificate ← unit.borrowCertificates.find? fun certificate =>
    certificate.namespaceId == handle.namespaceId && certificate.functionId == handle.functionId
  let fact ← certificate.loans[loan.index]?
  some fact.holders

/-- The literal of a constant at its declared type. -/
def compileLiteral {ρ : ResultShape} {Γ : NRow} (τ : NTy) (literal : ConstValue) :
    Except String (Term ρ Γ τ) :=
  (.lit ·) <$> literalValue τ literal

/-- The local a place names, when it is a bare local. -/
def placeLocal? (ns : ValidatedNamespace) (place : PlaceId) : Except String LocalId :=
  match ns.places[place.index]? with
  | some (.localVar localId) => .ok localId
  | _ => notCarried "a place other than a local"

/-- The local an operand reads a reference from: the local itself, or a
copy or move of it. -/
def heldLocal? (ns : ValidatedNamespace) (operand : ExprId) : Option LocalId :=
  match ns.expressions[operand.index]? with
  | some { kind := .localVar localId, .. } => some localId
  | some { kind := .operation (.move place) _ #[] _, .. }
  | some { kind := .operation (.copy place) _ #[] _, .. } =>
      match ns.places[place.index]? with
      | some (.localVar localId) => some localId
      | _ => none
  | _ => none

mutual
  /-- The term of one expression. -/
  def compileExpr (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → ExprId → Except String (Compiled ρ Γ)
    | 0, _ => .error "the compiler ran out of fuel"
    | fuel + 1, id => do
        let some expression := ns.expressions[id.index]?
          | .error "an expression is out of range"
        let τ? ← typeOf? unit ns namespaceId expression.typeId
        match expression.kind with
        | .value literal _ =>
            let some τ := τ? | notCarried "a literal of type never"
            let term ← compileLiteral τ literal
            .ok (.at τ term)
        | .localVar localId =>
            let some τ := τ? | notCarried "a local of type never"
            let x ← Var.at? Γ localId.index τ
            -- A mutable reference used as a value moves out of its slot.
            match τ with
            | .ref _ => .ok (.at τ (.take x))
            | _ => .ok (.at τ (.var x))
        | .constant reference =>
            let some τ := τ? | notCarried "a constant of type never"
            let some handle := resolveConstant? unit namespaceId reference
              | .error "a constant does not resolve"
            let some targetNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a constant's namespace is out of range"
            let some declaration := targetNs.constants[handle.constantId]?
              | .error "a constant is out of range"
            let value ← compileExpr unit function ρ .nil targetNs handle.namespaceId fuel declaration.value
            let value ← value.at? τ
            .ok (.at τ (.const value))
        | .operation (.call (.function reference)) instantiations arguments _ =>
            let some handle := resolveFunction? unit namespaceId reference
              | .error "a callee does not resolve"
            let some calleeNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a callee's namespace is out of range"
            let some callee := calleeNs.functions[handle.functionId.index]?
              | .error "a callee is out of range"
            let some parameters := callee.signature.parameters.toList.mapM fun parameter =>
                ntyOf unit handle.namespaceId parameter.typeUse.typeId
              | notCarried "the type of a callee parameter"
            unless parameters.all NTy.lendable do notCarried "a reference inside an aggregate"
            let parameters := NRow.ofList parameters
            let shape ← match callee.signature.results.toList with
              | [] => .ok ResultShape.none
              | [result] => match ntyOf unit handle.namespaceId result.typeId with
                  | some τ => .ok (ResultShape.one τ)
                  | none => notCarried "the type of a callee result"
              | _ => notCarried "a callee with several results"
            unless shape.lendable do notCarried "a reference inside an aggregate"
            if instantiations.isEmpty then
              let arguments ← compileArgs unit function ρ Γ ns namespaceId fuel arguments.toList
                parameters
              let value : Term ρ Γ shape.bodyType := .call handle shape arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
              | none => notCarried "a call of type never"
            else
              let some typeArgs := instantiations.toList.mapM fun argument => match argument with
                  | .typeArg value => some value
                  | _ => none
                | notCarried "a generic argument that is not a type"
              let some θ := typeArgs.mapM fun value => ntyOf unit namespaceId value.typeId
                | notCarried "the type of a type argument"
              let θ := NRow.ofList θ
              unless θ.refFree do notCarried "a reference type argument"
              let some (θ : TypeArgs) := if inhabitable : θ.inhabitable = true then
                  some ⟨θ, inhabitable⟩ else none
                | notCarried "a type argument without values"
              let arguments ← compileArgs unit function ρ Γ ns namespaceId fuel arguments.toList
                (NRow.subst θ.1 parameters)
              let value : Term ρ Γ (shape.subst θ.1).bodyType :=
                .callGeneric handle typeArgs.toArray θ shape arguments
              match τ? with
              | some τ => .ok (.at τ (← Compiled.at? τ (.at (shape.subst θ.1).bodyType value)))
              | none => notCarried "a call of type never"
        | .operation (.borrow .mutable _) _ _ _ =>
            let some τ := τ? | notCarried "a borrow of type never"
            match expression.kind with
            | .operation operation _ arguments _ =>
                compileOperation unit function ρ Γ ns namespaceId fuel τ operation arguments.toList
            | _ => .error "internal: a borrow is not an operation"
        | .operation (.global kind) instantiations arguments _ =>
            let some τ := τ? | notCarried "a storage operation of type never"
            let #[.typeArg resource] := instantiations
              | notCarried "a storage operation without one resource type"
            let family : Family := ⟨namespaceId, resource.typeId⟩
            match kind, arguments.toList with
            | .contains, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel key).some
                .ok (.at .bool (.globalContains family key))
            | .borrow .immutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel key).some
                .ok (.at τ (.globalRead family key))
            | .borrow .mutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel key).some
                match τ with
                | .ref referent => .ok (.at (.ref referent) (.globalBorrow (τ := referent) family key))
                | _ => .error "a mutable global borrow has a non-reference type"
            | .take, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel key).some
                .ok (.at τ (.globalTake family key))
            | .publish, [key, value] => do
                let ⟨_, key⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel key).some
                let ⟨_, value⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel value).some
                .ok (.at .unit (.globalPublish family key value))
            | _, _ => notCarried "a storage operation of this kind or arity"
        | .operation (.call (.constructor reference variant)) _ arguments _ =>
            let some τ := τ? | notCarried "a constructor of type never"
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a constructor does not resolve"
            match τ, variant with
            | .struct source σs, none =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let fields ← compileArgs unit function ρ Γ ns namespaceId fuel arguments.toList σs
                .ok (.at (.struct source σs) (.pack source fields))
            | .enum source names rows distinct, some name =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let some ⟨σs, choice⟩ := Which.ofName names rows name
                  | .error "a constructor names an unknown variant"
                let fields ← compileArgs unit function ρ Γ ns namespaceId fuel arguments.toList σs
                .ok (.at (.enum source names rows distinct) (.variant source distinct choice fields))
            | _, _ => .error "a constructor's type is not its declaration's"
        | .operation operation _ arguments _ =>
            let some τ := τ? | notCarried "an operation of type never"
            compileOperation unit function ρ Γ ns namespaceId fuel τ operation arguments.toList
        | .block statements result =>
            compileBlock unit function ρ Γ ns namespaceId fuel statements.toList result
        | .letDecl _ none body =>
            compileExpr unit function ρ Γ ns namespaceId fuel body
        | .letDecl pattern (some value) body => do
            let compiledValue ← compileExpr unit function ρ Γ ns namespaceId fuel value
            -- A value that never completes never binds: the body is unreachable.
            if let .never term := compiledValue then return .never term
            let ⟨σ, value⟩ := compiledValue.some
            let body ← compileExpr unit function ρ Γ ns namespaceId fuel body
            let some pattern := ns.patterns[pattern.index]? | .error "a pattern is out of range"
            match pattern.kind with
            | .variable localId => do
                let x ← Var.at? Γ localId.index σ
                .ok (body.map fun _ body => .let_ x value body)
            | .wildcard => .ok (body.map fun _ body => .drop value body)
            | .tuple elements =>
                match σ, value with
                | .tuple σs, value => do
                    let targets ← compileTargets Γ ns fuel elements.toList σs
                    .ok (body.map fun _ body => .letRow targets value body)
                | _, _ => .error "a tuple pattern binds a non-tuple"
            | .constructor _ _ none fields =>
                match σ, value with
                | .struct _ σs, value => do
                    let targets ← compileTargets Γ ns fuel fields.toList σs
                    .ok (body.map fun _ body => .letFields targets value body)
                | _, _ => .error "a struct pattern binds a non-struct"
            | _ => notCarried "a destructuring let over this pattern"
        | .ifElse condition thenBranch elseBranch => do
            let condition ← (← compileExpr unit function ρ Γ ns namespaceId fuel condition).at? .bool
            let thenBranch ← compileExpr unit function ρ Γ ns namespaceId fuel thenBranch
            let elseBranch ← match elseBranch with
              | some elseBranch => compileExpr unit function ρ Γ ns namespaceId fuel elseBranch
              | none => .ok (.at .unit (.lit ()))
            match τ? with
            | some τ => do
                let thenBranch ← thenBranch.at? τ
                let elseBranch ← elseBranch.at? τ
                .ok (.at τ (.ite condition thenBranch elseBranch))
            | none =>
                match thenBranch, elseBranch with
                | .never thenBranch, .never elseBranch =>
                    .ok (.never fun τ => .ite condition (thenBranch τ) (elseBranch τ))
                | .at σ thenBranch, elseBranch => do
                    let elseBranch ← elseBranch.at? σ
                    .ok (.at σ (.ite condition thenBranch elseBranch))
                | thenBranch, .at σ elseBranch => do
                    let thenBranch ← thenBranch.at? σ
                    .ok (.at σ (.ite condition thenBranch elseBranch))
        | .throw_ kind arguments =>
            match arguments.toList with
            | [] => .ok (.never fun _ => .throw0 kind)
            | [code] => do
                let ⟨_, code⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel code).some
                .ok (.never fun _ => .throw1 kind code)
            | _ => notCarried "a throw with several arguments"
        | .return_ values =>
            match values.toList with
            | [] => do
                let value ← Compiled.at? ρ.bodyType (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit ()))
                .ok (.never fun _ => .return_ value)
            | [value] => do
                let value ← (← compileExpr unit function ρ Γ ns namespaceId fuel value).at? ρ.bodyType
                .ok (.never fun _ => .return_ value)
            | _ => notCarried "a return of several values"
        | .assign place value =>
            match ns.places[place.index]? with
            | some (.localVar localId) => do
                let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
                let value ← (← compileExpr unit function ρ Γ ns namespaceId fuel value).at? σ
                .ok (.at .unit (.assign x value))
            | _ => do
                let ⟨_, σ, x, path⟩ ← compilePlace unit Γ ns fuel place
                let value ← (← compileExpr unit function ρ Γ ns namespaceId fuel value).at? σ
                .ok (.at .unit (.writePlace x path value))
        | .loop _ body => do
            let body ← (← compileExpr unit function ρ Γ ns namespaceId fuel body).at? .unit
            .ok (.at .unit (.loop id.index body))
        | .break_ nest none => .ok (.never fun _ => .break_ nest)
        | .continue_ nest => .ok (.never fun _ => .continue_ nest)
        | .spec _ => .ok (.at .unit (.lit ()))
        | kind => notCarried (describeKind kind)

  /-- A read of a place at its declared type; `consume` moves a mutable
  reference out of its local. -/
  def compileRead (unit : ValidatedUnit) (Γ : NRow) (ns : ValidatedNamespace) (consume : Bool) :
      Nat → NTy → PlaceId → Except String (Compiled ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, place =>
        match ns.places[place.index]? with
        | some (.localVar localId) => do
            let x ← Var.at? Γ localId.index τ
            match consume, τ with
            | true, .ref _ => .ok (.at τ (.take x))
            | _, _ => .ok (.at τ (.var x))
        | _ => do
            let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
            if equal : component = τ then .ok (.at τ (.readPlace x (equal ▸ path)))
            else .error "a place read has an unexpected type"

  /-- An argument row at the expected types. -/
  def compileArgs (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → (σs : NRow) → Except String (Args ρ Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, argument :: arguments, .cons σ σs => do
        let head ← (← compileExpr unit function ρ Γ ns namespaceId fuel argument).at? σ
        let tail ← compileArgs unit function ρ Γ ns namespaceId fuel arguments σs
        .ok (.cons head tail)
    | _ + 1, _, _ => .error "an argument row's arity differs from its declaration's"

  /-- The slots a row of variable or wildcard patterns binds. -/
  def compileTargets (Γ : NRow) (ns : ValidatedNamespace) :
      Nat → List PatternId → (σs : NRow) → Except String (Vars Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, pattern :: patterns, .cons σ σs => do
        let some pattern := ns.patterns[pattern.index]? | .error "a pattern is out of range"
        let target ← match pattern.kind with
          | .variable localId => some <$> Var.at? Γ localId.index σ
          | .wildcard => .ok none
          | _ => notCarried "a nested destructuring pattern"
        let rest ← compileTargets Γ ns fuel patterns σs
        .ok (.cons target rest)
    | _ + 1, _, _ => .error "a pattern's arity differs from its declaration's"

  /-- The statements of a block, then its result. -/
  def compileBlock (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → Option ExprId → Except String (Compiled ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], none => .ok (.at .unit (.lit ()))
    | fuel + 1, [], some result => compileExpr unit function ρ Γ ns namespaceId fuel result
    | fuel + 1, statement :: statements, result => do
        let ⟨_, statement⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel statement).some
        let rest ← compileBlock unit function ρ Γ ns namespaceId fuel statements result
        .ok (rest.map fun _ rest => .drop statement rest)

  /-- An operation at its declared result type. -/
  def compileOperation (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Operation → List ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .primitive primitive, arguments =>
        compilePrimitive unit function ρ Γ ns namespaceId fuel τ primitive arguments
    | fuel + 1, τ, .copy place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, τ, .move place, [] => compileRead unit Γ ns true fuel τ place
    | fuel + 1, τ, .read place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, τ, .borrow .immutable place, [] => compileRead unit Γ ns false fuel τ place
    | fuel + 1, .ref referent, .borrow .mutable place, [] => do
        let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
        if equal : component = referent then .ok (.at (.ref referent) (.borrowPlace x (equal ▸ path)))
        else .error "a mutable borrow has an unexpected type"
    | fuel + 1, τ, .reference .dereference, [operand] => do
        -- Reading through a local reference leaves it in place.
        if let some { kind := .localVar localId, .. } := ns.expressions[operand.index]? then
          let x ← Var.at? Γ localId.index (.ref τ)
          return .at τ (.deref (.var x))
        let ⟨σ, operand⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .ref referent, operand =>
            if equal : referent = τ then .ok (.at τ (.deref (equal ▸ operand)))
            else .error "a dereference has an unexpected type"
        | _, _ => notCarried "a dereference of a non-reference"
    | _ + 1, τ, .reference (.freeze _), [operand] => do
        -- A freeze is a shared reborrow: the result is the current value,
        -- and the loan's death, not the freeze, resolves its prophecy.
        let some localId := heldLocal? ns operand
          | notCarried "a freeze of a reference that no local holds"
        let x ← Var.at? Γ localId.index (.ref τ)
        .ok (.at τ (.deref (.var x)))
    | fuel + 1, .unit, .reference .mutate, [target, value] => do
        let some { kind := .localVar localId, .. } := ns.expressions[target.index]?
          | notCarried "a mutation of a reference that is not a local"
        let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
        match σ, x with
        | .ref referent, x => do
            let value ← (← compileExpr unit function ρ Γ ns namespaceId fuel value).at? referent
            .ok (.at .unit (.mutate x value))
        | _, _ => notCarried "a mutation of a non-reference local"
    | fuel + 1, τ, .reference (.endLoan loans), arguments => do
        let anchor ← match arguments with
          | [] => .ok (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit ()))
          | [anchor] => compileExpr unit function ρ Γ ns namespaceId fuel anchor
          | _ => notCarried "a loan death marker with several operands"
        let anchor ← anchor.at? τ
        -- A loan's death resolves the reference wherever it is held; a
        -- holder whose slot is empty passed it on, and a loan consumed by a
        -- call has no holder here, since the callee resolves it.  A holder
        -- without references observes the loan through an erased shared
        -- borrow and resolves nothing.
        let deaths ← loans.toList.reverse.mapM fun loan => do
          let some holders := loanHolders? unit function loan
            | .error "a loan death names an unknown loan"
          holders.toList.filterMapM fun holder => do
            let some ⟨σ, x⟩ := Var.ofIndex Γ holder.index | .error "a local is out of range"
            match σ, x with
            | .ref _, x => .ok (some (Term.resolve (ρ := ρ) x))
            | σ, _ =>
                if σ.refFree then .ok none else notCarried "a loan held inside an aggregate"
        let effects := deaths.flatten
        if effects.isEmpty then .ok (.at τ anchor) else
        let effect := effects.foldr (fun effect rest => Term.drop effect rest) (.lit ())
        .ok (.at τ (.seqAfter anchor effect))
    | fuel + 1, τ, .data (.select reference field), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .struct _ σs, operand => do
            let some index := referencedFieldIndex? unit namespaceId reference none field
              | .error "a field selection does not resolve"
            let x ← Var.at? σs index τ
            .ok (.at τ (.field x operand))
        | _, _ => notCarried "a field selection on a non-struct"
    | fuel + 1, .bool, .data (.testVariants _ variants), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum _ _ _ _, operand => .ok (.at .bool (.isVariant variants.toList operand))
        | _, _ => notCarried "a variant test on a non-enum"
    | fuel + 1, τ, .data (.selectVariants reference fields), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum source names rows _, operand => do
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a variant field selection does not resolve"
            unless handle = source do .error "a variant field selection's declaration differs"
            let some choices := variantFieldChoices? unit handle fields
              | .error "a variant field selection has no choices"
            let choices ← Choices.ofList names rows τ choices.toList
            .ok (.at τ (.payload choices operand))
        | _, _ => notCarried "a variant field selection on a non-enum"
    | _ + 1, _, operation, _ => notCarried (describeKind (.operation operation #[] #[] none))

  /-- A pure primitive at its declared result type. -/
  def compilePrimitive (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → PrimitiveOperation → List ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .copyValue, [operand] => do
        let operand ← (← compileExpr unit function ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, τ, .moveValue, [operand] => do
        let operand ← (← compileExpr unit function ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, .tuple σs, .tuple, elements => do
        let elements ← compileArgs unit function ρ Γ ns namespaceId fuel elements σs
        .ok (.at (.tuple σs) (.tuple elements))
    | fuel + 1, τ, .add, [left, right] =>
        compileModular unit function ρ Γ ns namespaceId fuel τ .add left right
    | fuel + 1, τ, .subtract, [left, right] =>
        compileModular unit function ρ Γ ns namespaceId fuel τ .subtract left right
    | fuel + 1, τ, .multiply, [left, right] =>
        compileModular unit function ρ Γ ns namespaceId fuel τ .multiply left right
    | fuel + 1, τ, .checkedAdd failure, [left, right] =>
        compileChecked unit function ρ Γ ns namespaceId fuel τ .add failure left right
    | fuel + 1, τ, .checkedSubtract failure, [left, right] =>
        compileChecked unit function ρ Γ ns namespaceId fuel τ .subtract failure left right
    | fuel + 1, τ, .checkedMultiply failure, [left, right] =>
        compileChecked unit function ρ Γ ns namespaceId fuel τ .multiply failure left right
    | fuel + 1, τ, .checkedDivide failure, [left, right] =>
        compileChecked unit function ρ Γ ns namespaceId fuel τ .divide failure left right
    | fuel + 1, τ, .checkedModulo failure, [left, right] =>
        compileChecked unit function ρ Γ ns namespaceId fuel τ .modulo failure left right
    | fuel + 1, τ, .less, [left, right] =>
        compileCompare unit function ρ Γ ns namespaceId fuel τ .less left right
    | fuel + 1, τ, .greater, [left, right] =>
        compileCompare unit function ρ Γ ns namespaceId fuel τ .greater left right
    | fuel + 1, τ, .lessEqual, [left, right] =>
        compileCompare unit function ρ Γ ns namespaceId fuel τ .lessEqual left right
    | fuel + 1, τ, .greaterEqual, [left, right] =>
        compileCompare unit function ρ Γ ns namespaceId fuel τ .greaterEqual left right
    | fuel + 1, .bool, .equal, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        -- `NTy.eqb` is equality only on a ref-free type; at a reference it is
        -- `false` however the executor compares the borrows. Decline rather
        -- than carry an equality the execution does not agree with.
        unless σ.refFree do notCarried "an equality of a value holding a reference"
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal false left right))
    | fuel + 1, .bool, .notEqual, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        unless σ.refFree do notCarried "an inequality of a value holding a reference"
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal true left right))
    | fuel + 1, .bool, .logicalNot, [operand] => do
        let operand ← (← compileExpr unit function ρ Γ ns namespaceId fuel operand).at? .bool
        .ok (.at .bool (.not operand))
    | fuel + 1, .bool, .logicalAnd, [left, right] => do
        let left ← (← compileExpr unit function ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical true left right))
    | fuel + 1, .bool, .logicalOr, [left, right] => do
        let left ← (← compileExpr unit function ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical false left right))
    | fuel + 1, .int width false, .bitwiseAnd, [left, right] => do
        let left ← (← compileExpr unit function ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .and left right))
    | fuel + 1, τ, .checkedShiftLeft failure, [value, distance] =>
        compileShift unit function ρ Γ ns namespaceId fuel τ true failure value distance
    | fuel + 1, τ, .checkedShiftRight failure, [value, distance] =>
        compileShift unit function ρ Γ ns namespaceId fuel τ false failure value distance
    | fuel + 1, .int width' signed', .checkedCast failure, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value => .ok (.at (.int width' signed') (.cast failure value))
        | _, _ => notCarried "a cast from a non-integer"
    | fuel + 1, .vector τ, .vector, elements => do
        let count := elements.length
        let elements ← compileArgs unit function ρ Γ ns namespaceId fuel elements
          (NRow.replicate count τ)
        .ok (.at (.vector τ) (.vectorLit count elements))
    | fuel + 1, .address, .signerAddress, [signer] => do
        let ⟨σ, signer⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel signer).some
        match σ, signer with
        | .signer, signer => .ok (.at .address (.signerAddress signer))
        | _, _ => notCarried "a signer's address of a non-signer"
    | fuel + 1, .int 64 false, .length, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector _, vector => .ok (.at (.int 64 false) (.length vector))
        | _, _ => notCarried "a length of a non-vector"
    | fuel + 1, _, .index, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => .ok (.at τ (.index vector position))
            | _, _ => notCarried "an element read at a non-integer position"
        | _, _ => notCarried "an element read of a non-vector"
    | fuel + 1, .unit, .checkVectorIndex failure, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector _, vector =>
            match π, position with
            | .int _ _, position => .ok (.at .unit (.checkIndex failure vector position))
            | _, _ => notCarried "a bounds check at a non-integer position"
        | _, _ => notCarried "a bounds check of a non-vector"
    | fuel + 1, _, .pushVector, [vector, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            let element ← (← compileExpr unit function ρ Γ ns namespaceId fuel element).at? τ
            .ok (.at (.vector τ) (.push vector element))
        | _, _ => notCarried "a push onto a non-vector"
    | fuel + 1, _, .insertVector, [vector, position, element] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position => do
                let element ← (← compileExpr unit function ρ Γ ns namespaceId fuel element).at? τ
                .ok (.at (.vector τ) (.insert vector position element))
            | _, _ => notCarried "an insertion at a non-integer position"
        | _, _ => notCarried "an insertion into a non-vector"
    | fuel + 1, _, .removeVector, [vector, position] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, position⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel position).some
        match σ, vector with
        | .vector τ, vector =>
            match π, position with
            | .int _ _, position =>
                .ok (.at (.tuple (.cons τ (.cons (.vector τ) .nil))) (.remove vector position))
            | _, _ => notCarried "a removal at a non-integer position"
        | _, _ => notCarried "a removal from a non-vector"
    | fuel + 1, _, .swapVector, [vector, left, right] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        match σ, vector with
        | .vector τ, vector =>
          match π, left with
          | .int width signed, left => do
              let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at?
                (.int width signed)
              .ok (.at (.vector τ) (.swap vector left right))
          | _, _ => notCarried "a swap at non-integer positions"
        | _, _ => notCarried "a swap in a non-vector"
    | fuel + 1, _, .concatVector, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .vector τ, left => do
            let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? (.vector τ)
            .ok (.at (.vector τ) (.concat left right))
        | _, _ => notCarried "a concatenation of non-vectors"
    | fuel + 1, _, .slice, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit function ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.slice vector start stop))
          | _, _ => notCarried "a slice at non-integer positions"
        | _, _ => notCarried "a slice of a non-vector"
    | fuel + 1, _, .reverseSliceVector, [vector, start, stop] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        let ⟨π, start⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel start).some
        match σ, vector with
        | .vector τ, vector =>
          match π, start with
          | .int width signed, start => do
              let stop ← (← compileExpr unit function ρ Γ ns namespaceId fuel stop).at?
                (.int width signed)
              .ok (.at (.vector τ) (.reverseSlice vector start stop))
          | _, _ => notCarried "a reversal at non-integer positions"
        | _, _ => notCarried "a reversal of a non-vector"
    | fuel + 1, .unit, .destroyEmptyVector, [vector] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector _, vector => .ok (.at .unit (.destroyEmpty vector))
        | _, _ => notCarried "a destruction of a non-vector"
    | fuel + 1, _, .containsVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            let needle ← (← compileExpr unit function ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at .bool (.contains vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, _, .indexOfVector, [vector, needle] => do
        let ⟨σ, vector⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel vector).some
        match σ, vector with
        | .vector τ, vector => do
            let needle ← (← compileExpr unit function ρ Γ ns namespaceId fuel needle).at? τ
            .ok (.at (.tuple (.cons .bool (.cons (.int 64 false) .nil))) (.indexOf vector needle))
        | _, _ => notCarried "a search in a non-vector"
    | fuel + 1, .int 8 true, .compare, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        -- A reference is carried as its (current, prophecy) pair, so the
        -- denotation would order it by its contents. The executor orders a
        -- borrow by its loan before its contents, which the pair does not
        -- record, so the order of a value holding one is not carried.
        unless σ.refFree do notCarried "a structural order of a value holding a reference"
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at (.int 8 true) (.order ns.variantOrders left right))
    | _ + 1, _, primitive, _ => notCarried s!"primitive {repr primitive} at this type or arity"

  def compileChecked (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CheckedOp → ThrowKind → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width signed, op, failure, left, right => do
        let left ← (← compileExpr unit function ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.checked op failure left right))
    | _ + 1, _, _, _, _, _ => notCarried "checked arithmetic at a non-integer type"

  def compileModular (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → ModularOp → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width signed, op, left, right => do
        let left ← (← compileExpr unit function ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.modular op left right))
    | _ + 1, _, _, _, _ => notCarried "modular arithmetic at a non-integer type"

  def compileCompare (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CompareOp → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .bool, op, left, right => do
        let ⟨σ, left⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .int width signed, left => do
            let right ← (← compileExpr unit function ρ Γ ns namespaceId fuel right).at? (.int width signed)
            .ok (.at .bool (.compare op left right))
        | _, _ => notCarried "a comparison of non-integers"
    | _ + 1, _, _, _, _ => notCarried "a comparison at a non-Boolean type"

  def compileShift (unit : ValidatedUnit) (function : FunctionHandle)
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Bool → ThrowKind → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width false, left, failure, value, distance => do
        let value ← (← compileExpr unit function ρ Γ ns namespaceId fuel value).at? (.int width false)
        let ⟨σ, distance⟩ := (← compileExpr unit function ρ Γ ns namespaceId fuel distance).some
        match σ, distance with
        | .int _ false, distance => .ok (.at (.int width false) (.shift left failure value distance))
        | _, _ => notCarried "a shift by a signed or non-integer distance"
    | _ + 1, _, _, _, _, _ => notCarried "a shift of a signed integer"
end

/-- The fuel that bounds every body in a unit.  A term visits each node at
most once and each node costs at most three steps of the mutual recursion,
so three times the expression count suffices. -/
def unitFuel (unit : ValidatedUnit) : Nat :=
  3 * unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 0 + 3

/-- The mutable-reference slots among the first `count` slots from `index`. -/
def mutableSlots (Γ : NRow) : Nat → Nat → Except String (Mutables Γ)
  | _, 0 => .ok .nil
  | index, count + 1 => do
      let rest ← mutableSlots Γ (index + 1) count
      match Var.ofIndex Γ index with
      | some ⟨.ref _, x⟩ => .ok (.cons x rest)
      | some _ => .ok rest
      | none => .error "a parameter is out of range"

/-- Whether a type is logical only: it types specification values, never
executable ones, so no body reads a local of it. -/
def logicalOnly (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  match unit.namespaces[namespaceId.index]?.bind (·.tables.types[typeId.index]?) with
  | some (.integer .unbounded _) | some .range | some (.typeDomain _)
  | some (.resourceDomain ..) | some .stateDomain => true
  | _ => false

/-- The compiled function a handle names, or the construct that stops it. -/
def compileFunction (unit : ValidatedUnit) (handle : FunctionHandle) : Except String Function := do
  let some ns := unit.namespaces[handle.namespaceId.index]? | .error "a namespace is out of range"
  let some declaration := ns.functions[handle.functionId.index]? | .error "a function is out of range"
  let .structured root := declaration.body | notCarried "a function without a body"
  -- A specification local (a quantifier's binder) keeps an empty slot.
  let some allLocals := declaration.locals.toList.mapM fun localDecl =>
      match ntyOf unit handle.namespaceId localDecl.type.typeId with
      | some τ => some τ
      | none => if logicalOnly unit handle.namespaceId localDecl.type.typeId then some .unit else none
    | notCarried "the type of a local"
  let paramCount := declaration.signature.parameters.size
  if paramCount > allLocals.length then .error "fewer locals than parameters" else
  let params := NRow.ofList (allLocals.take paramCount)
  let locals := NRow.ofList (allLocals.drop paramCount)
  let some paramTypes := declaration.signature.parameters.toList.mapM fun parameter =>
      ntyOf unit handle.namespaceId parameter.typeUse.typeId
    | notCarried "the type of a parameter"
  if NRow.ofList paramTypes != params then
    .error "parameter types differ from the leading locals" else
  let result ← match declaration.signature.results.toList with
    | [] => .ok ResultShape.none
    | [result] => match ntyOf unit handle.namespaceId result.typeId with
        | some τ => .ok (ResultShape.one τ)
        | none => notCarried "the type of the result"
    | _ => notCarried "several results"
  unless paramTypes.all NTy.lendable && result.lendable do
    notCarried "a reference inside an aggregate"
  let body ← compileExpr unit handle result (params ++ locals) ns handle.namespaceId (unitFuel unit) root
  let body ← body.at? result.bodyType
  let mutables ← mutableSlots (params ++ locals) 0 paramCount
  .ok { params, locals, result, body, mutables }

end LeanerIR.Proofs.Denote

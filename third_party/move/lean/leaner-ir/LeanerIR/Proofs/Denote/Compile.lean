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
      | .nominal name arguments => do
          unless arguments.isEmpty do none
          let handle ← resolveNominal? unit ns name
          structNTyFuel unit fuel handle
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
def literalValue : (τ : NTy) → ConstValue → Except String τ.carrier
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

def literalRow : (row : NRow) → List ConstValue → Except String (HList row)
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
          | _, _ => notCarried "a field place on a non-struct"
      | _ => notCarried "a place of this kind"

/-- The role of a mutable borrow site: passed straight to a callee, whose
export settles it, or bound to a local, whose death marker writes it back
to the lender. -/
inductive LoanRole where
  | consumed
  | bound (target : LocalId) (lender : PlaceId)
  | boundGlobal (target : LocalId)
  deriving Repr, Inhabited

/-- The expression a value position ends in, through the bindings and
blocks the frontend wraps around it. -/
def resultSite (ns : ValidatedNamespace) : Nat → ExprId → ExprId
  | 0, id => id
  | fuel + 1, id =>
      match ns.expressions[id.index]? with
      | some { kind := .letDecl _ _ body, .. } => resultSite ns fuel body
      | some { kind := .block _ (some result), .. } => resultSite ns fuel result
      | _ => id

def mutableBorrowPlace? (ns : ValidatedNamespace) (id : ExprId) : Option PlaceId := do
  let expression ← ns.expressions[(resultSite ns ns.expressions.size id).index]?
  match expression.kind with
  | .operation (.borrow .mutable place) _ _ _ => some place
  | _ => none

def isGlobalMutableBorrow (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  match ns.expressions[(resultSite ns ns.expressions.size id).index]? with
  | some { kind := .operation (.global (.borrow .mutable)) .., .. } => true
  | _ => false

/-- The roles of the mutable borrow sites of a namespace, by site. -/
def siteRoles (ns : ValidatedNamespace) : Array (ExprId × LoanRole) :=
  ns.expressions.foldl (init := #[]) fun roles expression =>
    match expression.kind with
    | .letDecl pattern (some value) _ =>
        let site := resultSite ns ns.expressions.size value
        match mutableBorrowPlace? ns value, ns.patterns[pattern.index]? with
        | some lender, some { kind := .variable target, .. } =>
            roles.push (site, .bound target lender)
        | none, some { kind := .variable target, .. } =>
            if isGlobalMutableBorrow ns value then roles.push (site, .boundGlobal target) else roles
        | _, _ => roles
    | .operation (.call (.function _)) _ arguments _ =>
        arguments.foldl (init := roles) fun roles argument =>
          if (mutableBorrowPlace? ns argument).isSome then
            roles.push (resultSite ns ns.expressions.size argument, .consumed)
          else roles
    | _ => roles

/-- The borrow site a function's lexical loan names. -/
def loanSite? (unit : ValidatedUnit) (handle : FunctionHandle) (loan : LoanId) : Option ExprId := do
  let certificate ← unit.borrowCertificates.find? fun certificate =>
    certificate.namespaceId == handle.namespaceId && certificate.functionId == handle.functionId
  let fact ← certificate.loans[loan.index]?
  some fact.expression

/-- The literal of a constant at its declared type. -/
def compileLiteral {ρ : ResultShape} {Γ : NRow} (τ : NTy) (literal : ConstValue) :
    Except String (Term ρ Γ τ) :=
  (.lit ·) <$> literalValue τ literal

/-- The local a place names, when it is a bare local. -/
def placeLocal? (ns : ValidatedNamespace) (place : PlaceId) : Except String LocalId :=
  match ns.places[place.index]? with
  | some (.localVar localId) => .ok localId
  | _ => notCarried "a place other than a local"

mutual
  /-- The term of one expression. -/
  def compileExpr (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
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
            .ok (.at τ (.var x))
        | .constant reference =>
            let some τ := τ? | notCarried "a constant of type never"
            let some handle := resolveConstant? unit namespaceId reference
              | .error "a constant does not resolve"
            let some targetNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a constant's namespace is out of range"
            let some declaration := targetNs.constants[handle.constantId]?
              | .error "a constant is out of range"
            let value ← compileExpr unit function roles ρ .nil targetNs handle.namespaceId fuel declaration.value
            let value ← value.at? τ
            .ok (.at τ (.const value))
        | .operation (.call (.function reference)) instantiations arguments _ =>
            unless instantiations.isEmpty do notCarried "a generic call"
            let some handle := resolveFunction? unit namespaceId reference
              | .error "a callee does not resolve"
            let some calleeNs := unit.namespaces[handle.namespaceId.index]?
              | .error "a callee's namespace is out of range"
            let some callee := calleeNs.functions[handle.functionId.index]?
              | .error "a callee is out of range"
            let some parameters := callee.signature.parameters.toList.mapM fun parameter =>
                ntyOf unit handle.namespaceId parameter.typeUse.typeId
              | notCarried "the type of a callee parameter"
            let parameters := NRow.ofList parameters
            let shape ← match callee.signature.results.toList with
              | [] => .ok ResultShape.none
              | [result] => match ntyOf unit handle.namespaceId result.typeId with
                  | some τ => .ok (ResultShape.one τ)
                  | none => notCarried "the type of a callee result"
              | _ => notCarried "a callee with several results"
            let arguments ← compileArgs unit function roles ρ Γ ns namespaceId fuel arguments.toList parameters
            let value : Term ρ Γ shape.bodyType := .call handle shape arguments
            match τ? with
            | some τ => .ok (.at τ (← Compiled.at? τ (.at shape.bodyType value)))
            | none => notCarried "a call of type never"
        | .operation (.borrow .mutable _) _ _ _ =>
            if (roles.find? (·.1 == id)).isNone then
              notCarried "a mutable borrow outside a binding or a call argument"
            else
              let some τ := τ? | notCarried "a borrow of type never"
              let some expression := ns.expressions[id.index]? | .error "an expression is out of range"
              match expression.kind with
              | .operation operation _ arguments _ =>
                  compileOperation unit function roles ρ Γ ns namespaceId fuel τ operation
                    arguments.toList
              | _ => .error "internal: a borrow is not an operation"
        | .operation (.global kind) instantiations arguments _ =>
            let some τ := τ? | notCarried "a storage operation of type never"
            let #[.typeArg resource] := instantiations
              | notCarried "a storage operation without one resource type"
            let family : Family := ⟨namespaceId, resource.typeId⟩
            match kind, arguments.toList with
            | .contains, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel key).some
                .ok (.at .bool (.globalContains family key))
            | .borrow .immutable, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel key).some
                .ok (.at τ (.globalRead family key))
            | .borrow .mutable, [key] => do
                if (roles.find? (·.1 == id)).isNone then
                  notCarried "a mutable global borrow outside a binding"
                let ⟨_, key⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel key).some
                match τ with
                | .ref referent => .ok (.at (.ref referent) (.globalBorrow (τ := referent) family key))
                | _ => .error "a mutable global borrow has a non-reference type"
            | .take, [key] => do
                let ⟨_, key⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel key).some
                .ok (.at τ (.globalTake family key))
            | .publish, [key, value] => do
                let ⟨_, key⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel key).some
                let ⟨_, value⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).some
                .ok (.at .unit (.globalPublish family key value))
            | _, _ => notCarried "a storage operation of this kind or arity"
        | .operation (.call (.constructor reference variant)) instantiations arguments _ =>
            unless instantiations.isEmpty do notCarried "a generic constructor"
            let some τ := τ? | notCarried "a constructor of type never"
            let some handle := resolveStruct? unit namespaceId reference
              | .error "a constructor does not resolve"
            match τ, variant with
            | .struct source σs, none =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let fields ← compileArgs unit function roles ρ Γ ns namespaceId fuel arguments.toList σs
                .ok (.at (.struct source σs) (.pack source fields))
            | .enum source names rows distinct, some name =>
                unless handle = source do .error "a constructor's declaration differs from its type"
                let some ⟨σs, choice⟩ := Which.ofName names rows name
                  | .error "a constructor names an unknown variant"
                let fields ← compileArgs unit function roles ρ Γ ns namespaceId fuel arguments.toList σs
                .ok (.at (.enum source names rows distinct) (.variant source distinct choice fields))
            | _, _ => .error "a constructor's type is not its declaration's"
        | .operation operation _ arguments _ =>
            let some τ := τ? | notCarried "an operation of type never"
            compileOperation unit function roles ρ Γ ns namespaceId fuel τ operation arguments.toList
        | .block statements result =>
            compileBlock unit function roles ρ Γ ns namespaceId fuel statements.toList result
        | .letDecl _ none body =>
            compileExpr unit function roles ρ Γ ns namespaceId fuel body
        | .letDecl pattern (some value) body => do
            let ⟨σ, value⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).some
            let body ← compileExpr unit function roles ρ Γ ns namespaceId fuel body
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
            let condition ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel condition).at? .bool
            let thenBranch ← compileExpr unit function roles ρ Γ ns namespaceId fuel thenBranch
            let elseBranch ← match elseBranch with
              | some elseBranch => compileExpr unit function roles ρ Γ ns namespaceId fuel elseBranch
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
                let ⟨_, code⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel code).some
                .ok (.never fun _ => .throw1 kind code)
            | _ => notCarried "a throw with several arguments"
        | .return_ values =>
            match values.toList with
            | [] => do
                let value ← Compiled.at? ρ.bodyType (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit ()))
                .ok (.never fun _ => .return_ value)
            | [value] => do
                let value ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).at? ρ.bodyType
                .ok (.never fun _ => .return_ value)
            | _ => notCarried "a return of several values"
        | .assign place value =>
            match ns.places[place.index]? with
            | some (.localVar localId) => do
                let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
                let value ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).at? σ
                .ok (.at .unit (.assign x value))
            | _ => do
                let ⟨_, σ, x, path⟩ ← compilePlace unit Γ ns fuel place
                let value ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).at? σ
                .ok (.at .unit (.writePlace x path value))
        | .loop _ body => do
            let body ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel body).at? .unit
            .ok (.at .unit (.loop id.index body))
        | .break_ nest none => .ok (.never fun _ => .break_ nest)
        | .continue_ nest => .ok (.never fun _ => .continue_ nest)
        | .spec _ => .ok (.at .unit (.lit ()))
        | kind => notCarried (describeKind kind)

  /-- A read of a place at its declared type. -/
  def compileRead (unit : ValidatedUnit) (Γ : NRow) (ns : ValidatedNamespace) :
      Nat → NTy → PlaceId → Except String (Compiled ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, place =>
        match ns.places[place.index]? with
        | some (.localVar localId) => do
            let x ← Var.at? Γ localId.index τ
            .ok (.at τ (.var x))
        | _ => do
            let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
            if equal : component = τ then .ok (.at τ (.readPlace x (equal ▸ path)))
            else .error "a place read has an unexpected type"

  /-- An argument row at the expected types. -/
  def compileArgs (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → (σs : NRow) → Except String (Args ρ Γ σs)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], .nil => .ok .nil
    | fuel + 1, argument :: arguments, .cons σ σs =>
        match mutableBorrowPlace? ns argument, σ with
        | some place, .ref referent => do
            let tail ← compileArgs unit function roles ρ Γ ns namespaceId fuel arguments σs
            let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
            if equal : component = referent then
              let arguments : Args ρ Γ (.cons (.ref component) σs) := .reborrow x path tail
              .ok (equal ▸ arguments)
            else .error "a reborrowed argument has an unexpected type"
        | _, σ => do
            let tail ← compileArgs unit function roles ρ Γ ns namespaceId fuel arguments σs
            let head ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel argument).at? σ
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
  def compileBlock (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → List ExprId → Option ExprId → Except String (Compiled ρ Γ)
    | 0, _, _ => .error "the compiler ran out of fuel"
    | _ + 1, [], none => .ok (.at .unit (.lit ()))
    | fuel + 1, [], some result => compileExpr unit function roles ρ Γ ns namespaceId fuel result
    | fuel + 1, statement :: statements, result => do
        let ⟨_, statement⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel statement).some
        let rest ← compileBlock unit function roles ρ Γ ns namespaceId fuel statements result
        .ok (rest.map fun _ rest => .drop statement rest)

  /-- An operation at its declared result type. -/
  def compileOperation (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Operation → List ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .primitive primitive, arguments =>
        compilePrimitive unit function roles ρ Γ ns namespaceId fuel τ primitive arguments
    | fuel + 1, τ, .copy place, [] => compileRead unit Γ ns fuel τ place
    | fuel + 1, τ, .read place, [] => compileRead unit Γ ns fuel τ place
    | fuel + 1, τ, .borrow .immutable place, [] => compileRead unit Γ ns fuel τ place
    | fuel + 1, .ref referent, .borrow .mutable place, [] => do
        let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel place
        if equal : component = referent then .ok (.at (.ref referent) (.borrowPlace x (equal ▸ path)))
        else .error "a mutable borrow has an unexpected type"
    | fuel + 1, τ, .reference .dereference, [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .ref referent, operand =>
            if equal : referent = τ then .ok (.at τ (.deref (equal ▸ operand)))
            else .error "a dereference has an unexpected type"
        | _, _ => notCarried "a dereference of a non-reference"
    | fuel + 1, .unit, .reference .mutate, [target, value] => do
        let some { kind := .localVar localId, .. } := ns.expressions[target.index]?
          | notCarried "a mutation of a reference that is not a local"
        let some ⟨σ, x⟩ := Var.ofIndex Γ localId.index | .error "a local is out of range"
        match σ, x with
        | .ref referent, x => do
            let value ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).at? referent
            .ok (.at .unit (.mutate x value))
        | _, _ => notCarried "a mutation of a non-reference local"
    | fuel + 1, τ, .reference (.endLoan loans), arguments => do
        let anchor ← match arguments with
          | [] => .ok (Compiled.at (ρ := ρ) (Γ := Γ) .unit (.lit ()))
          | [anchor] => compileExpr unit function roles ρ Γ ns namespaceId fuel anchor
          | _ => notCarried "a loan death marker with several operands"
        let anchor ← anchor.at? τ
        let deaths ← loans.toList.reverse.mapM fun loan => do
          let some site := loanSite? unit function loan | .error "a loan death names an unknown loan"
          let some (_, role) := roles.find? (·.1 == site)
            | notCarried "a mutable borrow outside a binding or a call argument"
          match role with
          | .consumed => .ok none
          | .bound target lender => do
              let ⟨_, component, x, path⟩ ← compilePlace unit Γ ns fuel lender
              let borrow ← Var.at? Γ target.index (.ref component)
              .ok (some (Term.writeBack (ρ := ρ) x path borrow))
          | .boundGlobal target => do
              let some ⟨σ, borrow⟩ := Var.ofIndex Γ target.index | .error "a local is out of range"
              match σ, borrow with
              | .ref _, borrow => .ok (some (Term.publishBack (ρ := ρ) borrow))
              | _, _ => .error "a global loan is bound to a non-reference local"
        let effects := deaths.filterMap id
        if effects.isEmpty then .ok (.at τ anchor) else
        let effect := effects.foldr (fun effect rest => Term.drop effect rest) (.lit ())
        .ok (.at τ (.seqAfter anchor effect))
    | fuel + 1, τ, .data (.select reference field), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .struct _ σs, operand => do
            let some index := referencedFieldIndex? unit namespaceId reference none field
              | .error "a field selection does not resolve"
            let x ← Var.at? σs index τ
            .ok (.at τ (.field x operand))
        | _, _ => notCarried "a field selection on a non-struct"
    | fuel + 1, .bool, .data (.testVariants _ variants), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).some
        match σ, operand with
        | .enum _ _ _ _, operand => .ok (.at .bool (.isVariant variants.toList operand))
        | _, _ => notCarried "a variant test on a non-enum"
    | fuel + 1, τ, .data (.selectVariants reference fields), [operand] => do
        let ⟨σ, operand⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).some
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
  def compilePrimitive (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → PrimitiveOperation → List ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, τ, .copyValue, [operand] => do
        let operand ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, τ, .moveValue, [operand] => do
        let operand ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).at? τ
        .ok (.at τ operand)
    | fuel + 1, .tuple σs, .tuple, elements => do
        let elements ← compileArgs unit function roles ρ Γ ns namespaceId fuel elements σs
        .ok (.at (.tuple σs) (.tuple elements))
    | fuel + 1, τ, .checkedAdd failure, [left, right] =>
        compileChecked unit function roles ρ Γ ns namespaceId fuel τ .add failure left right
    | fuel + 1, τ, .checkedSubtract failure, [left, right] =>
        compileChecked unit function roles ρ Γ ns namespaceId fuel τ .subtract failure left right
    | fuel + 1, τ, .checkedMultiply failure, [left, right] =>
        compileChecked unit function roles ρ Γ ns namespaceId fuel τ .multiply failure left right
    | fuel + 1, τ, .checkedDivide failure, [left, right] =>
        compileChecked unit function roles ρ Γ ns namespaceId fuel τ .divide failure left right
    | fuel + 1, τ, .checkedModulo failure, [left, right] =>
        compileChecked unit function roles ρ Γ ns namespaceId fuel τ .modulo failure left right
    | fuel + 1, τ, .less, [left, right] =>
        compileCompare unit function roles ρ Γ ns namespaceId fuel τ .less left right
    | fuel + 1, τ, .greater, [left, right] =>
        compileCompare unit function roles ρ Γ ns namespaceId fuel τ .greater left right
    | fuel + 1, τ, .lessEqual, [left, right] =>
        compileCompare unit function roles ρ Γ ns namespaceId fuel τ .lessEqual left right
    | fuel + 1, τ, .greaterEqual, [left, right] =>
        compileCompare unit function roles ρ Γ ns namespaceId fuel τ .greaterEqual left right
    | fuel + 1, .bool, .equal, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).some
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal false left right))
    | fuel + 1, .bool, .notEqual, [left, right] => do
        let ⟨σ, left⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).some
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? σ
        .ok (.at .bool (.equal true left right))
    | fuel + 1, .bool, .logicalNot, [operand] => do
        let operand ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel operand).at? .bool
        .ok (.at .bool (.not operand))
    | fuel + 1, .bool, .logicalAnd, [left, right] => do
        let left ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical true left right))
    | fuel + 1, .bool, .logicalOr, [left, right] => do
        let left ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).at? .bool
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? .bool
        .ok (.at .bool (.logical false left right))
    | fuel + 1, .int width false, .bitwiseAnd, [left, right] => do
        let left ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).at? (.int width false)
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? (.int width false)
        .ok (.at (.int width false) (.bitwise .and left right))
    | fuel + 1, τ, .checkedShiftLeft failure, [value, distance] =>
        compileShift unit function roles ρ Γ ns namespaceId fuel τ true failure value distance
    | fuel + 1, τ, .checkedShiftRight failure, [value, distance] =>
        compileShift unit function roles ρ Γ ns namespaceId fuel τ false failure value distance
    | fuel + 1, .int width' signed', .checkedCast failure, [value] => do
        let ⟨σ, value⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).some
        match σ, value with
        | .int _ _, value => .ok (.at (.int width' signed') (.cast failure value))
        | _, _ => notCarried "a cast from a non-integer"
    | _ + 1, _, primitive, _ => notCarried s!"primitive {repr primitive} at this type or arity"

  def compileChecked (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CheckedOp → ThrowKind → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width signed, op, failure, left, right => do
        let left ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).at? (.int width signed)
        let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? (.int width signed)
        .ok (.at (.int width signed) (.checked op failure left right))
    | _ + 1, _, _, _, _, _ => notCarried "checked arithmetic at a non-integer type"

  def compileCompare (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → CompareOp → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .bool, op, left, right => do
        let ⟨σ, left⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel left).some
        match σ, left with
        | .int width signed, left => do
            let right ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel right).at? (.int width signed)
            .ok (.at .bool (.compare op left right))
        | _, _ => notCarried "a comparison of non-integers"
    | _ + 1, _, _, _, _ => notCarried "a comparison at a non-Boolean type"

  def compileShift (unit : ValidatedUnit) (function : FunctionHandle) (roles : Array (ExprId × LoanRole))
      (ρ : ResultShape) (Γ : NRow) (ns : ValidatedNamespace) (namespaceId : NamespaceId) :
      Nat → NTy → Bool → ThrowKind → ExprId → ExprId → Except String (Compiled ρ Γ)
    | 0, _, _, _, _, _ => .error "the compiler ran out of fuel"
    | fuel + 1, .int width false, left, failure, value, distance => do
        let value ← (← compileExpr unit function roles ρ Γ ns namespaceId fuel value).at? (.int width false)
        let ⟨σ, distance⟩ := (← compileExpr unit function roles ρ Γ ns namespaceId fuel distance).some
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
def exportSlots (Γ : NRow) : Nat → Nat → Except String (Exports Γ)
  | _, 0 => .ok .nil
  | index, count + 1 => do
      let rest ← exportSlots Γ (index + 1) count
      match Var.ofIndex Γ index with
      | some ⟨.ref _, x⟩ => .ok (.cons x rest)
      | some _ => .ok rest
      | none => .error "a parameter is out of range"

/-- The compiled function a handle names, or the construct that stops it. -/
def compileFunction (unit : ValidatedUnit) (handle : FunctionHandle) : Except String Function := do
  let some ns := unit.namespaces[handle.namespaceId.index]? | .error "a namespace is out of range"
  let some declaration := ns.functions[handle.functionId.index]? | .error "a function is out of range"
  let .structured root := declaration.body | notCarried "a function without a body"
  let some allLocals := declaration.locals.toList.mapM fun localDecl =>
      ntyOf unit handle.namespaceId localDecl.type.typeId
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
  let roles := siteRoles ns
  let body ← compileExpr unit handle roles result (params ++ locals) ns handle.namespaceId (unitFuel unit) root
  let body ← body.at? result.bodyType
  let exports ← exportSlots (params ++ locals) 0 paramCount
  .ok { params, locals, result, body, exports }

end LeanerIR.Proofs.Denote

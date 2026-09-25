-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Profile
import LeanerMove.Frontend.Effects
import LeanerMove.Frontend.Frames
import LeanerLang.AddressAlias
import LeanerMove.Frontend.LIR.Codec

/-!
# XAST to raw LIR

This is the transitional compiler-v2 frontend.  It interns XAST semantic data
bottom-up into the neutral arenas and retains only Move-specific leaf choices
as checked profile tags.  No XAST tree is kept beside the resulting unit.
-/

namespace LeanerMove.Frontend.LIR.Encode

open LeanerMove.Frontend Xast Effects

structure BuildState where
  sourceNamespaces : Array ModuleRef := #[]
  sourceNames : Array QualifiedName := #[]
  sourceTypes : Array Xast.Ty := #[]
  sourceLocals : Array (String × Xast.Ty × LeanerIR.LocalId) := #[]
  files : Array LeanerIR.SourceFile := #[]
  fileOffset : Nat := 0
  locations : Array LeanerIR.Location := #[]
  origins : Array LeanerIR.Origin := #[]
  alignments : Array LeanerIR.Alignment := #[]
  lifetimes : Array LeanerIR.Lifetime := #[]
  types : Array LeanerIR.Ty := #[]
  names : Array LeanerIR.QualifiedName := #[]
  expressions : Array LeanerIR.Expr := #[]
  patterns : Array LeanerIR.Pattern := #[]
  places : Array LeanerIR.Place := #[]
  locals : Array LeanerIR.LocalDecl := #[]
  /-- Whether the encoder is inside specification content, where local
  occurrences arrive projected and may name executable locals. -/
  logical : Bool := false
  /-- Hidden temporaries the encoder introduced, named `$t<n>` like the
  LeanerLang lowering's, so the two lowerings of one element access agree. -/
  temporaryLocals : Nat := 0
  /-- Whether the module being built is not a verification target: its
  pragmas then resolve `verify` to false. -/
  nonTarget : Bool := false
  /-- The package's functions that may write global memory. -/
  globalWriters : List QualifiedName := []
  /-- The package's and its dependencies' structs declaring `copy`, with
  their phantom-free type parameter positions. -/
  copyableStructs : List (QualifiedName × List Bool) := []
  /-- Whether each type parameter of the function being built has `copy`. -/
  typeParameterCopy : Array Bool := #[]

abbrev BuildM := StateT BuildState (Except String)

/-- Run an action in specification context, restoring the flag after. -/
private def logically (act : BuildM α) : BuildM α := do
  let saved := (← get).logical
  modify fun state => { state with logical := true }
  let result ← act
  modify fun state => { state with logical := saved }
  return result

/-- Whether values of a type are copyable: primitives and references are, a
vector or tuple when its elements are, a struct declaring `copy` when its
non-phantom arguments are, a type parameter when it declares `copy`. -/
private partial def copyable (ty : Xast.Ty) : BuildM Bool := do
  match ty with
  | .signer => pure false
  | .vector element => copyable element
  | .tuple elements => elements.allM copyable
  | .struct name arguments =>
      let some (_, relevant) := (← get).copyableStructs.find? (·.1 == name) | pure false
      (arguments.zip relevant).allM fun (argument, counts) =>
        if counts then copyable argument else pure true
  | .function _ _ abilities => pure (abilities.contains .copy)
  | .typeParam index => pure ((← get).typeParameterCopy[index]?.getD false)
  | _ => pure true

private def addLocation (loc : Loc) : BuildM LeanerIR.LocId := do
  let state ← get
  let id : LeanerIR.LocId := ⟨state.locations.size⟩
  let range : LeanerIR.SourceRange := {
    file := ⟨state.fileOffset + loc.file⟩, startByte := loc.start, endByte := loc.stop }
  set { state with locations := state.locations.push { primary := some range } }
  return id

/-- Record a semantic node synthesized by the compatibility adapter when XAST
v3 provides only the containing declaration's location. -/
private def addGeneratedLocation (parent : LeanerIR.LocId) : BuildM LeanerIR.LocId := do
  let state ← get
  let id : LeanerIR.LocId := ⟨state.locations.size⟩
  set { state with locations := state.locations.push {
    generatedBy := some "LeanerMove.Frontend.LIR.Encode"
    parent := some parent } }
  return id

/-- A module's address in its canonical spelling. -/
private def canonicalModuleAddress (module : ModuleRef) : String :=
  (LeanerLang.addressValue? module.address).map LeanerLang.canonicalAddress |>.getD module.address

/-- The namespace of a module: one per address and name, however a reference
spells the address; the first reference naming an alias gives it. -/
private def addModuleRef (module : ModuleRef) : BuildM LeanerIR.NamespaceId := do
  let state ← get
  let module := { module with address := canonicalModuleAddress module }
  if let some index := state.sourceNamespaces.findIdx? fun candidate =>
      candidate.address == module.address && candidate.name == module.name then
    if state.sourceNamespaces[index]!.addressAlias.isNone && module.addressAlias.isSome then
      set { state with sourceNamespaces := state.sourceNamespaces.set! index module }
    return ⟨index⟩
  let id : LeanerIR.NamespaceId := ⟨state.sourceNamespaces.size⟩
  set { state with sourceNamespaces := state.sourceNamespaces.push module }
  return id

private def addName (name : QualifiedName) : BuildM LeanerIR.NameId := do
  let state ← get
  if let some index := state.sourceNames.findIdx? (· == name) then
    return ⟨index⟩
  let namespaceId ← addModuleRef name.module
  let state ← get
  let id : LeanerIR.NameId := ⟨state.sourceNames.size⟩
  set { state with
    sourceNames := state.sourceNames.push name
    names := state.names.push { namespaceId, name := name.name } }
  return id

private def addQualifiedRef (name : QualifiedName) : BuildM LeanerIR.QualifiedRef := do
  let nameId ← addName name
  let state ← get
  let sourceName := state.sourceNames[nameId.index]!
  let namespaceId ← addModuleRef sourceName.module
  return { namespaceId, name := nameId }

private def pragmaName : LeanerIR.Attribute → String
  | .assign name .. | .call name .. => name

/-- The pragmas of a declaration that is not a verification target: any
`verify` it sets gives way, and a namespace states `verify = false` last,
which its functions inherit. -/
private def nonTargetPragmas (pragmas : Array LeanerIR.Attribute) (namespace_ : Bool) :
    Array LeanerIR.Attribute :=
  let kept := pragmas.filter (pragmaName · != "verify")
  if namespace_ then kept.push (.assign "verify" (.constant (.bool false))) else kept

private def lirAbility : Ability → LeanerIR.Ability
  | .copy => .copy
  | .drop => .drop
  | .store => .store
  | .key => .key

private def lirBorrowKind : RefKind → LeanerIR.BorrowKind
  | .immutable => .immutable
  | .mutable => .mutable

private def addInferredLifetime (loc : LeanerIR.LocId) : BuildM LeanerIR.LifetimeId := do
  let state ← get
  let id : LeanerIR.LifetimeId := ⟨state.lifetimes.size⟩
  set { state with lifetimes := state.lifetimes.push { kind := .inference, loc } }
  return id

/-- LIR's unit type, which the validator expects of unit-valued operations
such as an index check. Move spells unit as the empty tuple, which `addType`
keeps as a tuple; the entry here is keyed under a source spelling no XAST
type produces, so the two tables stay aligned without a lookup hit. -/
private def unitTypeId : BuildM LeanerIR.TypeId := do
  let state ← get
  if let some index := state.types.findIdx? (· == .unit) then return ⟨index⟩
  let id : LeanerIR.TypeId := ⟨state.types.size⟩
  set { state with
    sourceTypes := state.sourceTypes.push (.typeDomain (.tuple []))
    types := state.types.push .unit }
  return id

private partial def addType (loc : LeanerIR.LocId) (source : Xast.Ty) : BuildM LeanerIR.TypeId := do
  let state ← get
  unless source matches .reference .. do
    if let some index := state.sourceTypes.findIdx? (· == source) then
      return ⟨index⟩
  let ty ← match source with
    | .bool => pure .bool
    | .u8 => pure <| .integer (.bits 8) false
    | .u16 => pure <| .integer (.bits 16) false
    | .u32 => pure <| .integer (.bits 32) false
    | .u64 => pure <| .integer (.bits 64) false
    | .u128 => pure <| .integer (.bits 128) false
    | .u256 => pure <| .integer (.bits 256) false
    | .i8 => pure <| .integer (.bits 8) true
    | .i16 => pure <| .integer (.bits 16) true
    | .i32 => pure <| .integer (.bits 32) true
    | .i64 => pure <| .integer (.bits 64) true
    | .i128 => pure <| .integer (.bits 128) true
    | .i256 => pure <| .integer (.bits 256) true
    | .address => pure .address
    | .signer => pure .signer
    | .num => pure <| .integer .unbounded true
    | .range => pure .range
    | .eventStore => pure .eventStore
    | .tuple elements => .tuple <$> (elements.toArray.mapM (addType loc))
    | .vector element => do
        let element ← addType loc element
        pure <| .vector element
    | .struct name arguments => do
        let name ← addName name
        let arguments ← arguments.toArray.mapM fun argument => do
          let typeId ← addType loc argument
          pure <| LeanerIR.GenericArgument.typeArg { typeId, loc }
        pure <| .nominal name arguments
    | .function arguments result abilities => do
        let arguments ← match arguments with
          | .tuple elements => elements.toArray.mapM (addType loc)
          | argument => do
              let argument ← addType loc argument
              pure #[argument]
        let result ← addType loc result
        pure <| .function arguments result (abilities.toArray.map lirAbility)
    | .typeParam index => pure <| .typeParameter index
    | .reference mutable ty => do
        let ty ← addType loc ty
        let lifetime ← addInferredLifetime loc
        pure <| LeanerIR.Ty.reference {
          profile := .move
          kind := if mutable then .mutable else .shared
          referent := ty
          lifetime }
    | .typeDomain ty => do
      let ty ← addType loc ty
      pure <| .typeDomain ty
    | .resourceDomain name arguments => do
        let name ← addName name
        let arguments ← arguments.mapM fun arguments => arguments.toArray.mapM (addType loc)
        pure <| .resourceDomain name arguments
    | .stateDomain => pure .stateDomain
  let state ← get
  let id : LeanerIR.TypeId := ⟨state.types.size⟩
  set { state with sourceTypes := state.sourceTypes.push source, types := state.types.push ty }
  return id

private def typeUse (loc : LeanerIR.LocId) (ty : Xast.Ty) : BuildM LeanerIR.TypeUse := do
  return { typeId := ← addType loc ty, loc }

private def generatedTypeUse (parent : LeanerIR.LocId)
    (ty : Xast.Ty) : BuildM LeanerIR.TypeUse := do
  typeUse (← addGeneratedLocation parent) ty

private def addExprNode (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.ExprKind) : BuildM LeanerIR.ExprId := do
  let state ← get
  let id : LeanerIR.ExprId := ⟨state.expressions.size⟩
  set { state with expressions := state.expressions.push { loc, typeId, kind } }
  return id

private def addPlaceNode (place : LeanerIR.Place) : BuildM LeanerIR.PlaceId := do
  let state ← get
  let id : LeanerIR.PlaceId := ⟨state.places.size⟩
  set { state with places := state.places.push place }
  return id

private def addPatternNode (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.PatternKind) : BuildM LeanerIR.PatternId := do
  let state ← get
  let id : LeanerIR.PatternId := ⟨state.patterns.size⟩
  set { state with patterns := state.patterns.push { loc, typeId, kind } }
  return id

/-- Move specification functions erase one reference layer and interpret a
direct fixed-width integer parameter or result in MSL's mathematical integer
domain.  Nested integers remain part of their enclosing data representation. -/
private def specificationType : Xast.Ty → Xast.Ty
  | .reference _ ty => specificationType ty
  | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 |
      .i8 | .i16 | .i32 | .i64 | .i128 | .i256 | .num => .num
  | ty => ty

private def localId (name : String) (ty : Xast.Ty) (loc : LeanerIR.LocId) : BuildM LeanerIR.LocalId := do
  let state ← get
  -- Exact identity first: distinct executable locals must never unify, or a
  -- use site's annotated type contradicts the shared declaration.
  if let some (_, _, id) := state.sourceLocals.find? fun entry =>
      entry.1 == name && entry.2.1 == ty then
    return id
  -- A specification occurrence arrives already projected (dereferenced, with
  -- mathematical integers) and may refer to the executable local it names.
  -- The fallback is specification-only: an executable pattern binding that
  -- shadows a reference parameter by name must get a fresh local, not the
  -- parameter's slot.
  if (← get).logical && specificationType ty == ty then
    if let some (_, _, id) := state.sourceLocals.find? fun entry =>
        entry.1 == name && specificationType entry.2.1 == ty then
      return id
  let type ← typeUse loc ty
  let state ← get
  let id : LeanerIR.LocalId := ⟨state.locals.size⟩
  set { state with
    sourceLocals := state.sourceLocals.push (name, ty, id)
    locals := state.locals.push { id, name, type, loc } }
  return id

private def addParameter (parameter : Param) (loc : LeanerIR.LocId) : BuildM LeanerIR.Parameter := do
  let loc ← addGeneratedLocation loc
  let id ← localId parameter.name parameter.ty loc
  let state ← get
  let some declaration := state.locals[id.index]?
    | throw s!"parameter `{parameter.name}` has no local declaration"
  return { name := parameter.name, typeUse := declaration.type }

private def addSpecificationParameter (parameter : Param)
    (loc : LeanerIR.LocId) : BuildM LeanerIR.Parameter := do
  addParameter { parameter with ty := specificationType parameter.ty } loc

/-- Mark the locals a body borrows mutably as mutable declarations.

Move has no immutable local: every local is assignable and can be borrowed
mutably, while LIR carries writability on the declaration. A local the body
mutably borrows must therefore say so, or the borrow targets an immutable
place and the unit does not prepare. Locals the body never borrows mutably
keep the default, which leaves the printed source honest about what a
function actually mutates rather than marking everything `mut`.

Parameters are the leading locals, and LIR requires the two declarations to
agree, so the parameter records are refreshed from the locals they name.

Only expressions this function contributed are scanned: local ids restart per
function while the expression arena does not. -/
private def withMutablyBorrowedLocals (expressionsBefore : Nat)
    (parameters : Array LeanerIR.Parameter) : BuildM (Array LeanerIR.Parameter) := do
  let state ← get
  let body := state.expressions.extract expressionsBefore state.expressions.size
  -- A place borrow names its root local through the place arena; a value
  -- borrow names it through its operand expression. Both forms make the local
  -- writable.
  let rec rootLocal? (place : LeanerIR.PlaceId) (fuel : Nat) : Option LeanerIR.LocalId :=
    match fuel, state.places[place.index]? with
    | 0, _ => none
    | _, some (.localVar id) => some id
    | fuel + 1, some (.deref base) => rootLocal? base fuel
    | fuel + 1, some (.field base ..) => rootLocal? base fuel
    | fuel + 1, some (.index base _) => rootLocal? base fuel
    | fuel + 1, some (.subslice base ..) => rootLocal? base fuel
    | fuel + 1, some (.downcast base _) => rootLocal? base fuel
    | _, _ => none
  let borrowedLocal? (expression : LeanerIR.Expr) : Option LeanerIR.LocalId :=
    match expression.kind with
    | .operation (.borrow .mutable place) _ _ _ => rootLocal? place state.places.size
    | .operation (.reference (.borrow .mutable)) _ operands _ => do
        let operand ← operands[0]?
        let target : LeanerIR.Expr ← state.expressions[operand.index]?
        match target.kind with
        | .localVar id => some id
        | _ => none
    | _ => none
  let borrowed : Array LeanerIR.LocalId := body.foldl (init := #[]) fun borrowed expression =>
    match borrowedLocal? expression with
    | some id => if borrowed.contains id then borrowed else borrowed.push id
    | none => borrowed
  let locals := state.locals.map fun (declaration : LeanerIR.LocalDecl) =>
    if borrowed.contains declaration.id then { declaration with mutable := true }
    else declaration
  set { state with locals }
  return parameters.zipIdx.map fun (parameter, index) =>
    match locals[index]? with
    | some declaration => { parameter with mutable := declaration.mutable }
    | none => parameter

private def resetLocals : BuildM Unit := modify fun state => {
  state with sourceLocals := #[], locals := #[] }

/-- Names a `std::vector` native, whose semantics LIR owns directly.

Move's vector operations are `native fun` declarations with no body: in
stackless bytecode they are instructions, and the logical bytecode model
likewise makes them operations rather than callable functions. Lowering them
to the LIR operations that already mean the same thing is what makes vector
code executable at all — left as calls they resolve to absent bodies. The
remaining native, `move_range`, has no LIR counterpart yet and is deliberately
not listed, so it stays an ordinary call and surfaces as an absent body rather
than as silently wrong semantics. The functions the Move Prover treats as
intrinsic although they have bodies (`contains`, `index_of`, `remove`) are
LIR operations too: their bodies are loops whose invariants the standard
library does not state, and the operations mean the same thing. -/
private def vectorNative? (name : Xast.QualifiedName) : Option String :=
  let inVectorModule :=
    name.module.name == "vector" && canonicalModuleAddress name.module == "0x1"
  if inVectorModule &&
      ["empty", "length", "push_back", "pop_back", "swap", "destroy_empty", "borrow",
        "borrow_mut", "contains", "index_of", "remove"].contains name.name then
    some name.name
  else
    none

/-- Names the specification version of a value-level `std::vector` native,
which means the same LIR operation as the native. -/
private def vectorSpecNative? (name : Xast.QualifiedName) : Option String :=
  if name.module.name == "vector" && canonicalModuleAddress name.module == "0x1" &&
      (name.name == "$empty" || name.name == "$length") then
    some (name.name.drop 1).toString
  else
    none

/-- The width of an unsigned fixed-width integer type. -/
private def unsignedBits? : Option Xast.Ty → Option Nat
  | some .u8 => some 8 | some .u16 => some 16 | some .u32 => some 32
  | some .u64 => some 64 | some .u128 => some 128 | some .u256 => some 256
  | _ => none

/-- `std::signer::address_of` is LIR's signer-address primitive, which takes
the signer by reference as the native does. A specification applies it
through its specification version, `$address_of`, with the same meaning. -/
private def isSignerAddressOf (name : Xast.QualifiedName) : Bool :=
  name.module.name == "signer" && canonicalModuleAddress name.module == "0x1" &&
    (name.name == "address_of" || name.name == "$address_of")

private def lirOperation : Xast.Operation → BuildM LeanerIR.Operation
  | .moveFunction name => return .call (.function (← addQualifiedRef name))
  | .pack name variant => return .call (.constructor (← addQualifiedRef name) variant)
  | .exists _ => return .global .contains
  | .borrowGlobal kind => return .global (.borrow (lirBorrowKind kind))
  | .moveFrom => return .global .take
  | .moveTo => return .global .publish
  | .borrow kind => return .reference (.borrow (lirBorrowKind kind))
  | .deref => return .reference .dereference
  | .freeze explicit => return .reference (.freeze explicit)
  | .select name field => return .data (.select (← addQualifiedRef name) field)
  | .selectVariants name fields =>
      return .data (.selectVariants (← addQualifiedRef name) fields.toArray)
  | .testVariants name variants =>
      return .data (.testVariants (← addQualifiedRef name) variants.toArray)
  | .updateField name field => return .data (.updateField (← addQualifiedRef name) field)
  | .specFunction name range =>
      return .specification (.functionCall (← addQualifiedRef name) {
        pre := range.pre, post := range.post })
  | operation => do
      match Codec.primitiveOperation? operation with
      | some primitive => return .primitive primitive
      | none =>
          match Codec.specOperation? operation with
          | some specification => return .specification specification
          | none =>
              let encoded := Codec.encodeOperation operation
              let targets ← encoded.targets.mapM addQualifiedRef
              return .profile {
                profile := .move, tag := encoded.tag, payload := encoded.payload } targets

private def surfaceSyntax : Option Xast.SurfaceSyntax → Option LeanerIR.SurfaceSyntax
  | none => none
  | some .receiverCall => some .receiverCall
  | some .indexNotation => some .indexNotation

private partial def constValue : Value → LeanerIR.ConstValue
  | .address value => .address value
  | .number value => .integer value
  | .bool value => .bool value
  | .vector elements => .vector (elements.toArray.map constValue)
  | .tuple elements => .tuple (elements.toArray.map constValue)

private def toPragma (pragma : Xast.Pragma) : LeanerIR.Attribute :=
  .assign pragma.name (match pragma.value with
    | .value value => .constant (constValue value)
    | .name name => .name none name
    | .qualifiedName name => .qualifiedName name)

/-- The `proof_file` pragma naming the proof file beside a module's source,
which every function of the module inherits. -/
private def proofFilePragma (module : Xast.Module) : Array LeanerIR.Attribute :=
  (module.proofFile.map fun path =>
    LeanerIR.Attribute.assign "proof_file" (.constant (.string path))).toArray

/-- A module's pragmas: its own, and the proof file named on it. -/
private def modulePragmas (module : Xast.Module) : Array LeanerIR.Attribute :=
  module.pragmas.toArray.map toPragma ++ proofFilePragma module

mutual
  private partial def addExpr : Exp → BuildM LeanerIR.ExprId
    | .mk ty sourceLoc node => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc ty
        let kind ← match node with
          | .value value constant => pure <| .value (constValue value) constant
          | .«local» name => .localVar <$> localId name ty loc
          | .param index => pure <| .localVar ⟨index⟩
          | .call operation inst arguments surface => do
              let inst := if inst.isEmpty then
                  match operation, ty, arguments with
                  | .slice, .vector element, _ => [element]
                  | .len, _, .mk (.vector element) _ _ :: _ => [element]
                  | .len, _, .mk (.reference _ (.vector element)) _ _ :: _ => [element]
                  | .index, _, .mk (.vector element) _ _ :: _ => [element]
                  | .index, _, .mk (.reference _ (.vector element)) _ _ :: _ => [element]
                  | _, _, _ => inst
                else inst
              let instantiations ← match operation with
                -- Compiler-v2 retains the physical Move type in `old`'s
                -- inferred node instantiation even when its specification
                -- expression has already been projected (for example `u64`
                -- to mathematical `num`). LIR defines this generic argument
                -- as the operation's value/result type, so normalize it at
                -- the source boundary.
                | .old => do
                    let instantiationLoc ← addGeneratedLocation loc
                    pure #[.typeArg { typeId, loc := instantiationLoc }]
                | _ => inst.toArray.mapM fun ty =>
                    return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
              let sourceArguments := arguments
              let arguments ← sourceArguments.toArray.mapM addExpr
              let dereferenceFirst (referent : Xast.Ty) (first : LeanerIR.ExprId) := do
                let dereferenceLoc ← addGeneratedLocation loc
                let referentType ← addType dereferenceLoc referent
                let dereference ← addExprNode dereferenceLoc referentType <|
                  .operation (.reference .dereference) #[] #[first]
                pure (arguments.set! 0 dereference)
              let arguments ← match operation, ty, sourceArguments, arguments[0]? with
                -- A Move field borrow is represented by compiler-v2 as a
                -- selection whose operand and result are references. Keep
                -- that reference intact so LIR can project the borrow onto
                -- the selected field. By-value selection still needs the
                -- historical implicit dereference normalization below.
                | .select .., .reference .., .mk (.reference ..) _ _ :: _, _ => pure arguments
                | .select .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | .selectVariants .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | .testVariants .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                -- A slice reads the vector a reference points to, as a
                -- length does.
                | .slice, _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | _, _, _, _ => pure arguments
              -- A `std::vector` native is an LIR operation, not a call. Its
              -- vector operand arrives as a reference, so it is dereferenced
              -- the same way a by-value field selection is; element access
              -- then borrows the index place LIR already models.
              let vectorNativeNode : Option String → BuildM (Option LeanerIR.ExprKind)
                | none => pure none
                | some native => do
                    -- LIR's vector primitives take the vector by value, while
                    -- the Move natives take a reference. The operand is
                    -- almost always a literal borrow, and reading through it
                    -- is the borrowed expression itself: taking that directly
                    -- keeps the tree free of a synthesized dereference the
                    -- canonical round trip would fold away, which would make
                    -- the printer's fixed point depend on this lowering.
                    let vectorOperand : BuildM LeanerIR.ExprId := do
                      match sourceArguments.head? with
                      | some (.mk _ _ (.call (.borrow _) _ [borrowed@(.mk ty _ _)] _)) =>
                          -- Only a copyable value is read bare; a value that is
                          -- not stays read through its borrow, since spelled
                          -- bare it would be moved.
                          if ← copyable ty then addExpr borrowed else
                            let some first := arguments[0]?
                              | throw s!"`vector::{native}` has no vector operand"
                            let dereferenceLoc ← addGeneratedLocation loc
                            let referentType ← addType dereferenceLoc ty
                            addExprNode dereferenceLoc referentType <|
                              .operation (.reference .dereference) #[] #[first]
                      | some (.mk (.reference _ referent) _ _) => do
                          let some first := arguments[0]?
                            | throw s!"`vector::{native}` has no vector operand"
                          let dereferenceLoc ← addGeneratedLocation loc
                          let referentType ← addType dereferenceLoc referent
                          addExprNode dereferenceLoc referentType <|
                            .operation (.reference .dereference) #[] #[first]
                      | _ =>
                          match arguments[0]? with
                          | some first => pure first
                          | none => throw s!"`vector::{native}` has no vector operand"
                    match native with
                    | "empty" =>
                        pure <| some <| .operation (.primitive .vector) instantiations #[]
                          (surfaceSyntax surface)
                    | "length" =>
                        pure <| some <| .operation (.primitive .length) #[]
                          #[← vectorOperand] (surfaceSyntax surface)
                    | "swap" => do
                        -- Exchanging two elements is the value-level swap
                        -- stored back where it came from, the same shape as
                        -- `push_back`.
                        let some left := arguments[1]?
                          | throw "`vector::swap` has no first index operand"
                        let some right := arguments[2]?
                          | throw "`vector::swap` has no second index operand"
                        if let some (.mk _ _ (.call (.borrow _) _
                            [target@(.mk targetTy _ _)] _)) := sourceArguments.head? then
                          if let some place ← placeOf? target then
                            let swapLoc ← addGeneratedLocation loc
                            let vectorType ← addType swapLoc targetTy
                            let current ← addExpr target
                            let swapped ← addExprNode swapLoc vectorType <|
                              .operation (.primitive .swapVector) #[] #[current, left, right]
                            return some (.assign place swapped)
                        let some reference := arguments[0]?
                          | throw "`vector::swap` has no vector operand"
                        let some (.mk (.reference _ referent) _ _) := sourceArguments.head?
                          | throw "`vector::swap` does not take a reference"
                        let swapLoc ← addGeneratedLocation loc
                        let vectorType ← addType swapLoc referent
                        let current ← addExprNode swapLoc vectorType <|
                          .operation (.reference .dereference) #[] #[reference]
                        let swapped ← addExprNode swapLoc vectorType <|
                          .operation (.primitive .swapVector) #[] #[current, left, right]
                        pure <| some <| .operation (.reference .mutate) #[]
                          #[reference, swapped] (surfaceSyntax surface)
                    | "pop_back" => do
                        -- Removing the last element is the value-level
                        -- removal at the last index, the rest written back
                        -- through the reference; an empty vector aborts with
                        -- the VM's vector error 2.
                        let some reference := arguments[0]?
                          | throw "`vector::pop_back` has no vector operand"
                        let some (.mk referenceTy@(.reference _ referent) _ _) :=
                            sourceArguments.head?
                          | throw "`vector::pop_back` does not take a reference"
                        let popLoc ← addGeneratedLocation loc
                        let referenceType ← addType popLoc referenceTy
                        let vectorType ← addType popLoc referent
                        let indexType ← addType popLoc .u64
                        let boolType ← addType popLoc .bool
                        let pairType ← addType popLoc (.tuple [ty, referent])
                        let unitType ← unitTypeId
                        let slot ← temporaryLocal referenceType popLoc
                        let current : BuildM LeanerIR.ExprId := do
                          let read ← addExprNode popLoc referenceType (.localVar slot)
                          addExprNode popLoc vectorType <|
                            .operation (.reference .dereference) #[] #[read]
                        let length : BuildM LeanerIR.ExprId := do
                          addExprNode popLoc indexType <|
                            .operation (.primitive .length) #[] #[← current]
                        let zero ← addExprNode popLoc indexType (.value (.integer 0))
                        let empty ← addExprNode popLoc boolType <|
                          .operation (.primitive .equal) #[] #[← length, zero]
                        let code ← addExprNode popLoc indexType (.value (.integer 2))
                        let failure ← addExprNode popLoc unitType <|
                          .throw_ (.profile { profile := .move, tag := "runtime.vector_error" })
                            #[code]
                        let check ← addExprNode popLoc unitType (.ifElse empty failure none)
                        let one ← addExprNode popLoc indexType (.value (.integer 1))
                        let last ← addExprNode popLoc indexType <|
                          .operation (.primitive (.checkedSubtract .abort)) #[] #[← length, one]
                        let removal ← addExprNode popLoc pairType <|
                          .operation (.primitive .removeVector) #[] #[← current, last]
                        let removed ← temporaryLocal typeId popLoc
                        let rest ← temporaryLocal vectorType popLoc
                        let removedPattern ← addPatternNode popLoc typeId (.variable removed)
                        let restPattern ← addPatternNode popLoc vectorType (.variable rest)
                        let pairPattern ← addPatternNode popLoc pairType
                          (.tuple #[removedPattern, restPattern])
                        let restRead ← addExprNode popLoc vectorType (.localVar rest)
                        let writeReference ← addExprNode popLoc referenceType (.localVar slot)
                        let write ← addExprNode popLoc unitType <|
                          .operation (.reference .mutate) #[] #[writeReference, restRead]
                        let result ← addExprNode popLoc typeId (.localVar removed)
                        let written ← addExprNode popLoc typeId (.block #[write] (some result))
                        let popped ← addExprNode popLoc typeId
                          (.letDecl pairPattern (some removal) written)
                        let checked ← addExprNode popLoc typeId (.block #[check] (some popped))
                        let slotPattern ← addPatternNode popLoc referenceType (.variable slot)
                        pure <| some <| .letDecl slotPattern (some reference) checked
                    | "destroy_empty" =>
                        pure <| some <| .operation (.primitive .destroyEmptyVector) #[]
                          arguments (surfaceSyntax surface)
                    | "contains" | "index_of" => do
                        -- The element arrives as a reference too, and is read
                        -- through it the way the vector is.
                        let element : BuildM LeanerIR.ExprId := do
                          match (sourceArguments[1]? : Option Xast.Exp), arguments[1]? with
                          | some (.mk _ _ (.call (.borrow _) _ [borrowed@(.mk ty _ _)] _)), some second =>
                              if ← copyable ty then addExpr borrowed else
                                let dereferenceLoc ← addGeneratedLocation loc
                                let referentType ← addType dereferenceLoc ty
                                addExprNode dereferenceLoc referentType <|
                                  .operation (.reference .dereference) #[] #[second]
                          | some (.mk (.reference _ referent) _ _), some second => do
                              let dereferenceLoc ← addGeneratedLocation loc
                              let referentType ← addType dereferenceLoc referent
                              addExprNode dereferenceLoc referentType <|
                                .operation (.reference .dereference) #[] #[second]
                          | _, some second => pure second
                          | _, none => throw s!"`vector::{native}` has no element operand"
                        let operation := if native == "contains" then LeanerIR.PrimitiveOperation.containsVector
                          else .indexOfVector
                        pure <| some <| .operation (.primitive operation) #[]
                          #[← vectorOperand, ← element] (surfaceSyntax surface)
                    | "remove" => do
                        -- Removing at an index is the value-level removal, the
                        -- rest written back through the reference; an index
                        -- past the end aborts with the library's
                        -- `EINDEX_OUT_OF_BOUNDS`.
                        let some reference := arguments[0]?
                          | throw "`vector::remove` has no vector operand"
                        let some index := arguments[1]?
                          | throw "`vector::remove` has no index operand"
                        let some (.mk referenceTy@(.reference _ referent) _ _) :=
                            sourceArguments.head?
                          | throw "`vector::remove` does not take a reference"
                        let removeLoc ← addGeneratedLocation loc
                        let referenceType ← addType removeLoc referenceTy
                        let vectorType ← addType removeLoc referent
                        let indexType ← addType removeLoc .u64
                        let boolType ← addType removeLoc .bool
                        let pairType ← addType removeLoc (.tuple [ty, referent])
                        let unitType ← unitTypeId
                        let slot ← temporaryLocal referenceType removeLoc
                        let current : BuildM LeanerIR.ExprId := do
                          let read ← addExprNode removeLoc referenceType (.localVar slot)
                          addExprNode removeLoc vectorType <|
                            .operation (.reference .dereference) #[] #[read]
                        let length ← addExprNode removeLoc indexType <|
                          .operation (.primitive .length) #[] #[← current]
                        let inBounds ← addExprNode removeLoc boolType <|
                          .operation (.primitive .less) #[] #[index, length]
                        let code ← addExprNode removeLoc indexType (.value (.integer 131072))
                        let failure ← addExprNode removeLoc unitType <| .throw_ .abort #[code]
                        let skip ← addExprNode removeLoc unitType (.block #[] none)
                        let check ← addExprNode removeLoc unitType (.ifElse inBounds skip (some failure))
                        let removal ← addExprNode removeLoc pairType <|
                          .operation (.primitive .removeVector) #[] #[← current, index]
                        let removed ← temporaryLocal typeId removeLoc
                        let rest ← temporaryLocal vectorType removeLoc
                        let removedPattern ← addPatternNode removeLoc typeId (.variable removed)
                        let restPattern ← addPatternNode removeLoc vectorType (.variable rest)
                        let pairPattern ← addPatternNode removeLoc pairType
                          (.tuple #[removedPattern, restPattern])
                        let restRead ← addExprNode removeLoc vectorType (.localVar rest)
                        let writeReference ← addExprNode removeLoc referenceType (.localVar slot)
                        let write ← addExprNode removeLoc unitType <|
                          .operation (.reference .mutate) #[] #[writeReference, restRead]
                        let result ← addExprNode removeLoc typeId (.localVar removed)
                        let written ← addExprNode removeLoc typeId (.block #[write] (some result))
                        let taken ← addExprNode removeLoc typeId
                          (.letDecl pairPattern (some removal) written)
                        let checked ← addExprNode removeLoc typeId (.block #[check] (some taken))
                        let slotPattern ← addPatternNode removeLoc referenceType (.variable slot)
                        pure <| some <| .letDecl slotPattern (some reference) checked
                    | "push_back" => do
                        -- Growing a vector in place is the value-level push
                        -- stored back where it came from. When the native's
                        -- reference is a borrow of a storage path — which it
                        -- is for every Move call — write the path directly:
                        -- borrowing it and then reading through the borrow
                        -- would conflict with the loan taken for the write.
                        let some element := arguments[1]?
                          | throw "`vector::push_back` has no element operand"
                        if let some (.mk _ _ (.call (.borrow _) _
                            [target@(.mk targetTy _ _)] _)) := sourceArguments.head? then
                          if let some place ← placeOf? target then
                            let pushLoc ← addGeneratedLocation loc
                            let vectorType ← addType pushLoc targetTy
                            let current ← addExpr target
                            let pushed ← addExprNode pushLoc vectorType <|
                              .operation (.primitive .pushVector) #[] #[current, element]
                            return some (.assign place pushed)
                        let some reference := arguments[0]?
                          | throw "`vector::push_back` has no vector operand"
                        let some (.mk (.reference _ referent) _ _) := sourceArguments.head?
                          | throw "`vector::push_back` does not take a reference"
                        let pushLoc ← addGeneratedLocation loc
                        let vectorType ← addType pushLoc referent
                        -- Read through the reference rather than through the
                        -- borrowed local: the loan taken for the write is
                        -- live here, and a second read of the local under it
                        -- is a borrow conflict.
                        let current ← addExprNode pushLoc vectorType <|
                          .operation (.reference .dereference) #[] #[reference]
                        let pushed ← addExprNode pushLoc vectorType <|
                          .operation (.primitive .pushVector) #[] #[current, element]
                        pure <| some <| .operation (.reference .mutate) #[]
                          #[reference, pushed] (surfaceSyntax surface)
                    | "borrow" | "borrow_mut" => do
                        -- Borrowing an element is a borrow of the indexed
                        -- place, which is the executable form. Without a
                        -- storage path there is nothing to point at, so the
                        -- native stays a call and reports an absent body.
                        let some (.mk _ _ (.call (.borrow _) _ [target] _)) :=
                            sourceArguments.head?
                          | pure none
                        let some place ← placeOf? target
                          | pure none
                        let some index := arguments[1]?
                          | throw s!"`vector::{native}` has no index operand"
                        let kind := if native == "borrow_mut" then LeanerIR.BorrowKind.mutable
                          else .immutable
                        let collection ← vectorOperand
                        some <$> checkedElementAccess loc collection index typeId fun index => do
                          let elementPlace ← addPlaceNode (.index place index)
                          pure <| .operation (.borrow kind elementPlace) #[] #[] (surfaceSyntax surface)
                    | other => throw s!"unhandled vector native `{other}`"
              -- A borrow whose operand names a storage path is a place
              -- borrow, which is the executable form; the value borrow is
              -- kept only for operands that name no path.
              let placeBorrow? ← match operation, sourceArguments with
                | .borrow kind, [operand] => do
                    match ← placeOf? operand with
                    | some place => pure (some (lirBorrowKind kind, place))
                    | none => pure none
                | _, _ => pure none
              match operation with
              | .abort _ => pure <| .throw_ .abort arguments
              | .borrow kind =>
                  match placeBorrow? with
                  | some (borrowKind, place) =>
                      pure <| .operation (.borrow borrowKind place) #[] #[]
                        (surfaceSyntax surface)
                  | none =>
                      pure <| .operation (.reference (.borrow (lirBorrowKind kind)))
                        instantiations arguments (surfaceSyntax surface)
              | .moveFunction name =>
                  if isSignerAddressOf name then
                    pure <| .operation (.primitive .signerAddress) #[] arguments
                      (surfaceSyntax surface)
                  else match ← vectorNativeNode (vectorNative? name) with
                    | some node => pure node
                    | none =>
                        pure <| .operation (← lirOperation operation) instantiations arguments
                          (surfaceSyntax surface)
              | .specFunction name _ =>
                  if isSignerAddressOf name then
                    pure <| .operation (.primitive .signerAddress) #[] arguments
                      (surfaceSyntax surface)
                  else match ← vectorNativeNode (vectorSpecNative? name) with
                    | some node => pure node
                    | none =>
                        pure <| .operation (← lirOperation operation) instantiations arguments
                          (surfaceSyntax surface)
              | .shl =>
                  -- A specification's shift of a fixed-width value wraps at
                  -- its width, as the Move Prover reads it.
                  match (← get).logical, unsignedBits? (sourceArguments.head?.map (·.ty)) with
                  | true, some bits => do
                      -- The unbounded shift, reduced modulo the width.
                      let numType ← addType loc .num
                      let shifted ← addExprNode loc numType <|
                        .operation (.primitive .shiftLeft) #[] arguments (surfaceSyntax surface)
                      let modulus ← addExprNode loc numType (.value (.integer (2 ^ bits)))
                      pure <| .operation (.primitive .modulo) #[] #[shifted, modulus]
                  | _, _ =>
                      pure <| .operation (← lirOperation operation) instantiations arguments
                        (surfaceSyntax surface)
              | .and | .or =>
                  -- Move's `&&` and `||` short-circuit. In executable code
                  -- they are the conditional the LeanerLang lowering also
                  -- produces, so the two lowerings of one source agree and
                  -- the right operand's effects and aborts stay guarded;
                  -- a specification keeps the logical primitive.
                  if (← get).logical then
                    pure <| .operation (← lirOperation operation) instantiations arguments
                      (surfaceSyntax surface)
                  else
                    let some left := arguments[0]? | throw "a logical operator has no left operand"
                    let some right := arguments[1]? | throw "a logical operator has no right operand"
                    let constant ← addExprNode loc typeId (.value (.bool (operation matches .or)))
                    pure <| if operation matches .and then .ifElse left right (some constant)
                      else .ifElse left constant (some right)
              | operation =>
                  pure <| .operation (← lirOperation operation) instantiations arguments
                    (surfaceSyntax surface)
          | .invoke function arguments => do
              let arguments ← (function :: arguments).toArray.mapM addExpr
              pure <| .operation (.call .invoke) #[] arguments
          | .block pattern binding body => do
              pure <| .letDecl (← addPattern pattern) (← binding.mapM addExpr) (← addExpr body)
          | .ite condition thenBranch elseBranch =>
              pure <| .ifElse (← addExpr condition) (← addExpr thenBranch) (some (← addExpr elseBranch))
          | .«match» scrutinee arms => do
              let scrutinee ← addExpr scrutinee
              let arms ← arms.toArray.mapM fun arm => match arm with
                | .mk _ pattern guard body => do
                    let pattern ← addPattern pattern
                    let guard ← guard.mapM addExpr
                    let body ← addExpr body
                    return { pattern, guard, body }
              pure <| .match_ scrutinee arms
          | .sequence expressions =>
              let expressions ← expressions.toArray.mapM addExpr
              match expressions.back? with
              | none => pure <| .block #[] none
              | some result => pure <| .block (expressions.pop) (some result)
          | .loop body => pure <| .loop none (← addExpr body)
          | .loopCont nest isContinue =>
              pure <| if isContinue then .continue_ nest else .break_ nest none
          | .«return» value => pure <| .return_ #[← addExpr value]
          | .assign pattern value => pure <| .assignPattern (← addPattern pattern) (← addExpr value)
          | .mutate target value =>
              pure <| .operation (.reference .mutate)
                #[] #[← addMutationTarget target, ← addExpr value]
          | .specBlock (.mk specLoc pragmas conditions frame) => do
              -- An `update x = e` of a spec variable is a write of the ghost
              -- resource backing `x`; the other conditions stay a
              -- specification block.
              let writes ← conditions.toArray.filterMapM (ghostUpdate loc typeId)
              if writes.isEmpty then .spec <$> addSpecBlock (.mk specLoc pragmas conditions frame) loc
              else
                let rest := conditions.filter fun
                  | .mk .update .. => false
                  | _ => true
                let statements ← if rest.isEmpty then pure writes else do
                  let block ← addSpecBlock (.mk specLoc pragmas rest frame) loc
                  pure (writes.push (← addExprNode loc typeId (.spec block)))
                pure <| .block statements.pop (some statements.back!)
          | .quant kind ranges triggers condition body => do
              let kind := match kind with
                | .forall => LeanerIR.QuantifierKind.forall
                | .exists => .exists
                | .choose => .choose
                | .chooseMin => .chooseMin
              let binders ← ranges.toArray.mapM fun range => match range with
                | .mk pattern domain => return { pattern := ← addPattern pattern, domain := ← addExpr domain }
              let triggers ← triggers.toArray.mapM fun trigger => trigger.toArray.mapM addExpr
              pure <| .quantifier kind binders triggers (← condition.mapM addExpr) (← addExpr body)
        addExprNode loc typeId kind

  /-- The write an `update x = e` condition denotes: the model states the
  target as the field `v` of the ghost resource `global<Ghost$x>(@0)`
  backing the spec variable, and the write replaces that resource, whose
  only field it is. -/
  private partial def ghostUpdate (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId) :
      Xast.Condition → BuildM (Option LeanerIR.ExprId)
    | .mk .update _ _ value _ _ _ _
        (some (.mk _ _ (.call (.select ghostName "v") _
          [.mk ghostType _ (.call (.global _) _ [address] _)] _))) => do
        let .struct _ arguments := ghostType
          | throw s!"the ghost memory of `{ghostName.name}` is not a struct type"
        let writeLoc ← addGeneratedLocation loc
        let ghostTypeId ← addType writeLoc ghostType
        let referenceTypeId ← addType writeLoc (.reference true ghostType)
        let address ← addExpr address
        let ghostArgument := LeanerIR.GenericArgument.typeArg (← generatedTypeUse writeLoc ghostType)
        let instantiations ← arguments.toArray.mapM fun argument =>
          return LeanerIR.GenericArgument.typeArg (← generatedTypeUse writeLoc argument)
        let reference ← addExprNode writeLoc referenceTypeId <|
          .operation (.global (.borrow .mutable)) #[ghostArgument] #[address]
        let packed ← addExprNode writeLoc ghostTypeId <|
          .operation (.call (.constructor (← addQualifiedRef ghostName))) instantiations
            #[← addExpr value]
        return some (← addExprNode writeLoc typeId <|
          .operation (.reference .mutate) #[] #[reference, packed])
    | .mk .update .. => throw "an `update` condition must target a spec variable"
    | _ => pure none

  /-- A hidden temporary holding a computed element index, named like the
  LeanerLang lowering's `$t<n>`. -/
  private partial def temporaryLocal (typeId : LeanerIR.TypeId) (loc : LeanerIR.LocId) :
      BuildM LeanerIR.LocalId := do
    let state ← get
    let id : LeanerIR.LocalId := ⟨state.locals.size⟩
    set { state with
      temporaryLocals := state.temporaryLocals + 1
      locals := state.locals.push
        { id, name := s!"$t{state.temporaryLocals}", type := { typeId, loc }, loc } }
    return id

  /-- An element access in the checked form the LeanerLang lowering
  produces: a computed index is evaluated once into a temporary, the index
  is checked against the vector (`checkVectorIndex`, aborting with Move's
  vector error), and only then is the element place formed. Out-of-range
  resolution of an LIR index place is undefined, so the check is the
  semantic carrier of the abort; without it every Move element access was
  undefined past the end. `access` receives the checked index expression
  and builds the access node, whose type the whole binding takes. -/
  private partial def checkedElementAccess (loc : LeanerIR.LocId) (collection : LeanerIR.ExprId)
      (index : LeanerIR.ExprId) (accessType : LeanerIR.TypeId)
      (access : LeanerIR.ExprId → BuildM LeanerIR.ExprKind) : BuildM LeanerIR.ExprKind := do
    let state ← get
    let some indexNode := state.expressions[index.index]?
      | throw "an element index is out of range"
    let simple := match indexNode.kind with
      | .value .. | .localVar .. => true
      | _ => false
    let checkLoc ← addGeneratedLocation loc
    let unitType ← unitTypeId
    let (indexId, temporary?) ← if simple then pure (index, none) else do
      let slot ← temporaryLocal indexNode.typeId checkLoc
      let pattern ← addPatternNode checkLoc indexNode.typeId (.variable slot)
      let read ← addExprNode checkLoc indexNode.typeId (.localVar slot)
      pure (read, some (pattern, index))
    let check ← addExprNode checkLoc unitType <| .operation
      (.primitive (.checkVectorIndex (.profile { profile := .move, tag := "runtime.vector_error" })))
      #[] #[collection, indexId]
    let wildcard ← addPatternNode checkLoc unitType .wildcard
    let body ← addExprNode loc accessType (← access indexId)
    match temporary? with
    | none => pure (.letDecl wildcard (some check) body)
    | some (pattern, value) =>
        let checked ← addExprNode loc accessType (.letDecl wildcard (some check) body)
        pure (.letDecl pattern (some value) checked)

  /-- The storage path an expression denotes, when it denotes one.

  LIR distinguishes borrowing a *place* from borrowing a *value*, and only the
  place form is executable — a value borrow has no location to point at. Move
  borrows always name a place, so recognizing the path here is what makes
  `&x`, `&mut x`, and their indexed and dereferenced forms run at all.

  Paths this does not recognize fall back to the value borrow, which still
  validates and prints; it simply is not executable yet. -/
  private partial def placeOf? : Exp → BuildM (Option LeanerIR.PlaceId)
    | .mk ty sourceLoc (.«local» name) => do
        let loc ← addLocation sourceLoc
        some <$> addPlaceNode (.localVar (← localId name ty loc))
    | .mk _ _ (.param index) => some <$> addPlaceNode (.localVar ⟨index⟩)
    | .mk _ _ (.call .deref _ [reference] _) => do
        match ← placeOf? reference with
        | some base => some <$> addPlaceNode (.deref base)
        | none => pure none
    | .mk _ _ (.call .index _ [base, index] _) => do
        match ← placeOf? base with
        | some basePlace => some <$> addPlaceNode (.index basePlace (← addExpr index))
        | none => pure none
    | _ => pure none

  /-- Compiler-v2 represents assignment targets as value-typed XAST place
  trees. LIR mutation instead consumes a concrete reference. Reconstruct that
  reference while the place tree is still available, preserving a global or
  explicit borrow at its root and projecting it through struct fields. -/
  private partial def addMutationTarget : Exp → BuildM LeanerIR.ExprId
    | .mk fieldType sourceLoc
        (.call (.select owner field) inst [base] surface) => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc (.reference true fieldType)
        let instantiations ← inst.toArray.mapM fun ty =>
          return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
        let base ← addMutationTarget base
        addExprNode loc typeId <| .operation
          (.data (.select (← addQualifiedRef owner) field)) instantiations #[base]
            (surfaceSyntax surface)
    | .mk fieldType sourceLoc
        (.call (.selectVariants owner fields) inst [base] surface) => do
        -- A variant field written through a reference selects mutably the
        -- same way a struct field does.
        let loc ← addLocation sourceLoc
        let typeId ← addType loc (.reference true fieldType)
        let instantiations ← inst.toArray.mapM fun ty =>
          return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
        let base ← addMutationTarget base
        addExprNode loc typeId <| .operation
          (.data (.selectVariants (← addQualifiedRef owner) fields.toArray))
            instantiations #[base] (surfaceSyntax surface)
    | .mk _ _ (.call .deref _ [reference] _) => addExpr reference
    | source@(.mk ty sourceLoc (.«local» _))
    | source@(.mk ty sourceLoc (.param _)) => do
        match ty with
        | .reference .. => addExpr source
        | _ =>
            let loc ← addLocation sourceLoc
            let typeId ← addType loc (.reference true ty)
            let value ← addExpr source
            addExprNode loc typeId <| .operation
              (.reference (.borrow .mutable)) #[] #[value]
    | source => addExpr source

  private partial def addPattern : Pattern → BuildM LeanerIR.PatternId
    | .mk ty sourceLoc node => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc ty
        let kind ← match node with
          | .var name => .variable <$> localId name ty loc
          | .wildcard => pure .wildcard
          | .tuple elements => .tuple <$> elements.toArray.mapM addPattern
          | .struct name inst variant fields => do
              let name ← addName name
              let instantiations ← inst.toArray.mapM fun ty =>
                return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
              pure <| .constructor name instantiations variant (← fields.toArray.mapM addPattern)
          | .literal value => pure <| .literal (constValue value)
          | .range lower upper inclusive =>
              pure <| .range (lower.map constValue) (upper.map constValue) inclusive
        addPatternNode loc typeId kind

  private partial def addCondition (condition : Xast.Condition) : BuildM LeanerIR.Condition :=
    match condition with
    | .mk kind sourceLoc properties expression abortCode additionalCodes emitsHandle emitsCondition updateTarget => do
        let loc ← addLocation sourceLoc
        let mut auxiliary := #[]
        if let some value := abortCode then auxiliary := auxiliary.push ("abortCode", ← addExpr value)
        for value in additionalCodes do auxiliary := auxiliary.push ("additionalCode", ← addExpr value)
        if let some value := emitsHandle then auxiliary := auxiliary.push ("emitsHandle", ← addExpr value)
        if let some value := emitsCondition then auxiliary := auxiliary.push ("emitsCondition", ← addExpr value)
        if let some value := updateTarget then auxiliary := auxiliary.push ("updateTarget", ← addExpr value)
        return {
          loc
          kind := Codec.lirConditionKind kind
          properties := properties.toArray.map toPragma
          expression := ← addExpr expression
          auxiliary }

  private partial def addFrame (frame : Xast.Frame) (loc : LeanerIR.LocId) : BuildM LeanerIR.Frame :=
    match frame with
    | .mk modifies reads modifiesAll readsAll =>
        return {
          modifies := ← modifies.toArray.mapM addExpr
          reads := ← reads.toArray.mapM (generatedTypeUse loc)
          modifiesAll
          readsAll }

  private partial def addSpecBlock (spec : Xast.Spec) (fallback : LeanerIR.LocId) : BuildM LeanerIR.SpecBlock :=
    match spec with
    | .mk sourceLoc pragmas conditions frame => logically do
        let sourceLoc ← sourceLoc.mapM addLocation
        let loc := sourceLoc.getD fallback
        let pragmas := pragmas.toArray.map toPragma
        return {
          loc
          sourceLoc
          pragmas := if (← get).nonTarget then nonTargetPragmas pragmas false else pragmas
          conditions := ← conditions.toArray.mapM addCondition
          frame := ← frame.mapM (addFrame · loc) }
end

private partial def addAttribute : Xast.Attribute → BuildM LeanerIR.Attribute
  | .apply name arguments =>
      return .call name (← arguments.toArray.mapM addAttribute)
  | .assign name (.value value) =>
      return .assign name (.constant (constValue value))
  | .assign name (.name module nameValue) =>
      return .assign name (.name (← module.mapM addModuleRef) nameValue)

/-- These Move compiler attributes decide which source declarations enter a
build mode. Once compiler-v2 has included the declaration in XAST they have no
remaining runtime or specification meaning in LIR. -/
private def isResolvedBuildSelectionAttribute : Xast.Attribute → Bool
  | .apply name [] => name == "test" || name == "test_only" || name == "verify_only"
  | _ => false

private def addGenericBinder (parent : LeanerIR.LocId)
    (parameter : TypeParam) : BuildM LeanerIR.GenericBinder := do
  let loc ← addGeneratedLocation parent
  return {
    name := parameter.name
    kind := .typeArg
    abilities := parameter.abilities.toArray.map lirAbility
    predicates := if parameter.isPhantom then
      #[.profile (LeanerIR.Move.propertyValue "typeParameter.phantom")] else #[]
    loc }

private def functionProperty (tag : String) : LeanerIR.ProfileValue :=
  LeanerIR.Move.propertyValue tag

private def visibilityProperty : Visibility → LeanerIR.ProfileValue
  | .«private» => functionProperty "visibility.private"
  | .public => functionProperty "visibility.public"
  | .friend => functionProperty "visibility.friend"
  | .package => functionProperty "visibility.package"

private def functionKindProperty : FunctionKind → LeanerIR.ProfileValue
  | .regular => functionProperty "function.regular"
  | .inlineRetained => functionProperty "function.inlineRetained"
  | .native => functionProperty "function.native"

/-- A source declaration which can own ordinary comments. Retained inline
functions are valid XAST anchors even though compiler-v2 has already expanded
them and the LIR adapter intentionally omits their declarations. -/
private structure CommentAnchor where
  loc : Loc
  retained : Bool

private def commentAnchors (module : Module) (inlineFunctions : List String) :
    Array CommentAnchor :=
  let anchors :=
    module.constants.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.structs.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.functions.toArray.map (fun declaration =>
      { loc := declaration.loc, retained := declaration.kind != .inlineRetained }) ++
    module.specFuns.toArray.map (fun declaration =>
      { loc := declaration.loc,
        retained := !(declaration.isMoveFun && inlineFunctions.contains declaration.name) }) ++
    module.specVars.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.invariants.toArray.map (fun declaration => { loc := declaration.loc, retained := true })
  anchors.qsort fun left right =>
    left.loc.file < right.loc.file ||
      (left.loc.file == right.loc.file && left.loc.start < right.loc.start)

private def commentOwner? (anchors : Array CommentAnchor) (comment : Comment) :
    Option CommentAnchor :=
  let containing := anchors.filter fun anchor =>
    anchor.loc.file == comment.loc.file && anchor.loc.start <= comment.loc.start &&
      comment.loc.stop <= anchor.loc.stop
  match containing[0]? with
  | some first => some <| containing.foldl (init := first) fun best candidate =>
      let bestSize := best.loc.stop - best.loc.start
      let candidateSize := candidate.loc.stop - candidate.loc.start
      if candidateSize < bestSize ||
          (candidateSize == bestSize && !candidate.retained) then candidate else best
  | none => anchors.find? fun anchor =>
      anchor.loc.file == comment.loc.file && comment.loc.stop <= anchor.loc.start

/-- Keep comments only when the declaration they belong to also survives the
XAST-to-LIR boundary. Without this ownership pass, comments from several
expanded inline functions accumulate before the next retained declaration and
look like duplicated module-level comments. Unanchored comments remain free
standing and are preserved. -/
private def retainedComments (module : Module) (inlineFunctions : List String) :
    List Comment :=
  let anchors := commentAnchors module inlineFunctions
  module.comments.filter fun comment =>
    (commentOwner? anchors comment).all (·.retained)

private def addContract (spec : Xast.Spec) (fallback : LeanerIR.LocId) : BuildM LeanerIR.FunctionContract := do
  let block ← addSpecBlock spec fallback
  return {
    loc := block.sourceLoc
    conditions := block.conditions
    modifies := block.frame.map (·.modifies) |>.getD #[]
    reads := block.frame.map (·.reads) |>.getD #[]
    hasFrame := block.frame.isSome
    modifiesAll := block.frame.any (·.modifiesAll)
    readsAll := block.frame.any (·.readsAll)
    pragmas := block.pragmas }

private def ownName (module : Xast.Module) (name : String) : QualifiedName :=
  { module := module.ref, name }

/-- Encode one nominal declaration: its binders, abilities, fields, variants,
and its own specification contract. The result is what both an owned namespace
and a dependency interface record, the latter without contract or locals. -/
private def buildStructDecl (module : Module) (structDecl : Xast.Struct) :
    BuildM LeanerIR.StructDecl := do
  let structLoc ← addLocation structDecl.loc
  let fields ← structDecl.fields.toArray.mapM fun field => do
    let fieldLoc ← addGeneratedLocation structLoc
    let name ← addName (ownName module field.name)
    let fieldType ← typeUse fieldLoc field.ty
    return { loc := fieldLoc, name, type := fieldType, doc := field.doc }
  let variants ← (structDecl.variants.getD []).toArray.mapM fun variant => do
    let variantLoc ← addLocation variant.loc
    let fields ← variant.fields.toArray.mapM fun field => do
      let fieldLoc ← addGeneratedLocation variantLoc
      let name ← addName (ownName module field.name)
      let fieldType ← typeUse fieldLoc field.ty
      return { loc := fieldLoc, name, type := fieldType, doc := field.doc }
    let name ← addName (ownName module variant.name)
    return { loc := variantLoc, name, fields }
  let owner ← addName (ownName module structDecl.name)
  let structContract ← addContract structDecl.spec structLoc
  let locals := (← get).locals
  let attributes ← structDecl.attributes.toArray
    |>.filter (!isResolvedBuildSelectionAttribute ·) |>.mapM addAttribute
  return {
    loc := structLoc
    name := owner
    doc := structDecl.doc
    generics := ← structDecl.typeParams.toArray.mapM (addGenericBinder structLoc)
    fields
    variants
    abilities := structDecl.abilities.toArray.map lirAbility
    properties := (if structDecl.isNative then #[functionProperty "struct.native"] else #[]) ++
      (if structDecl.variants.isSome then #[functionProperty "struct.variants"] else #[])
    locals
    contract := structContract
    attributes }

/-- Provenance for declarations imported from another package: the location
they were declared at, recorded like any other Move source origin. -/
private def addImportedOrigin (loc : LeanerIR.LocId) (sourceIdentity : Option String) :
    BuildM (LeanerIR.OriginId × LeanerIR.AlignmentId) := do
  let state ← get
  let origin : LeanerIR.OriginId := ⟨state.origins.size⟩
  let alignment : LeanerIR.AlignmentId := ⟨state.alignments.size⟩
  set { state with
    origins := state.origins.push {
      kind := .moveSource, location := loc, sourceIdentity }
    alignments := state.alignments.push {
      source := origin, trust := .checked, description := "compiler-v2 XAST v3" } }
  return (origin, alignment)

/-- A dependency's public declaration shape. Constructing, selecting, and
matching a value of an imported type needs its fields and variants; its bodies,
contracts, and locals stay with the package that declares it. -/
private def buildInterface (module : Module) :
    BuildM LeanerIR.Import.RawNamespaceInterface := do
  let namespaceId ← addModuleRef module.ref
  let mut structs := #[]
  for structDecl in module.structs do
    resetLocals
    let declaration ← buildStructDecl module structDecl
    structs := structs.push { declaration with
      contract := {}, locals := #[], attributes := #[] }
  let inlineFunctions := module.functions.filterMap fun function =>
    if function.kind == .inlineRetained then some function.name else none
  let mut functions := #[]
  for function in module.functions do
    if function.kind == .inlineRetained then continue
    resetLocals
    let functionLoc ← addLocation function.loc
    let parameters ← function.params.toArray.mapM (addParameter · functionLoc)
    let resultLoc ← addGeneratedLocation functionLoc
    let result ← typeUse resultLoc function.result
    let (origin, alignment) ← addImportedOrigin functionLoc module.sources.head?
    let mut profileData := #[visibilityProperty function.visibility,
      functionKindProperty function.kind]
    if function.isEntry then profileData := profileData.push (functionProperty "function.entry")
    if function.isReceiver then
      profileData := profileData.push (functionProperty "function.receiver")
    functions := functions.push {
      loc := functionLoc
      name := ← addName (ownName module function.name)
      doc := function.doc
      profile := .move
      signature := {
        generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
        parameters
        results := #[result] }
      body := LeanerIR.Import.RawBody.absent
      origin
      alignment
      profileData }
  let mut specFunctions := #[]
  for function in module.specFuns do
    if function.isMoveFun && inlineFunctions.contains function.name then continue
    resetLocals
    let functionLoc ← addLocation function.loc
    let parameters ← function.params.toArray.mapM (addSpecificationParameter · functionLoc)
    let resultLoc ← addGeneratedLocation functionLoc
    let result ← typeUse resultLoc (specificationType function.result)
    let (origin, _) ← addImportedOrigin functionLoc module.sources.head?
    let mut profileData := #[]
    if function.uninterpreted then
      profileData := profileData.push (functionProperty "specFunction.uninterpreted")
    if function.isNative then
      profileData := profileData.push (functionProperty "specFunction.native")
    if function.isMoveFun then
      profileData := profileData.push (functionProperty "specFunction.moveFunction")
    if function.usesOld then
      profileData := profileData.push (functionProperty "specFunction.usesOld")
    specFunctions := specFunctions.push {
      loc := functionLoc
      name := ← addName (ownName module function.name)
      doc := function.doc
      profile := .move
      signature := {
        generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
        parameters
        results := #[result] }
      body := none
      origin
      profileData }
  -- An empty export list keeps every name of the namespace resolvable, which
  -- is what an unlisted interface already meant. Narrowing it to the declared
  -- exports is separate work.
  return { namespaceId, profile := some .move, structs, functions, specFunctions }

private def buildNamespace (unitIndex : Nat) (module : Xast.Module) :
    BuildM LeanerIR.Import.RawNamespace := do
    let state ← get
    let moduleFiles := if module.sources.isEmpty then #[{ name := "?" }] else
      module.sources.toArray.map fun source => { name := source }
    let fileOffset := state.files.size
    set { state with
      files := state.files ++ moduleFiles
      fileOffset
      nonTarget := !module.isTarget
      expressions := #[]
      patterns := #[]
      places := #[]
      locals := #[] }
    let _ ← addModuleRef module.ref
    let loc ← addLocation module.loc
    let state ← get
    let origin : LeanerIR.OriginId := ⟨state.origins.size⟩
    let alignment : LeanerIR.AlignmentId := ⟨state.alignments.size⟩
    set { state with
      origins := state.origins.push {
        kind := .moveSource
        location := loc
        sourceIdentity := module.sources.head? }
      alignments := state.alignments.push {
        source := origin
        trust := .checked
        description := "compiler-v2 XAST v3" } }
    let mut constants : Array LeanerIR.ConstantDecl := #[]
    for constant in module.constants do
      let constantLoc ← addLocation constant.loc
      let typeUseValue ← generatedTypeUse constantLoc constant.ty
      let valueLoc ← addGeneratedLocation constantLoc
      let value ← addExprNode valueLoc typeUseValue.typeId (.value (constValue constant.value))
      constants := constants.push {
        loc := constantLoc
        name := ← addName (ownName module constant.name)
        type := typeUseValue
        value
        doc := constant.doc }
    let mut structs : Array LeanerIR.StructDecl := #[]
    let mut intrinsics : Array LeanerIR.IntrinsicDecl := #[]
    for structDecl in module.structs do
      resetLocals
      let declaration ← buildStructDecl module structDecl
      let structLoc := declaration.loc
      let owner := declaration.name
      structs := structs.push declaration
      if let some intrinsic := structDecl.intrinsic then
        let intrinsicLoc ← addGeneratedLocation structLoc
        let executableBindings ← intrinsic.moveFunctions.toArray.mapM fun binding => do
          let bindingLoc ← addGeneratedLocation intrinsicLoc
          let target ← addQualifiedRef binding.target
          return { loc := bindingLoc, role := binding.role, target }
        let specBindings ← intrinsic.specFunctions.toArray.mapM fun binding => do
          let bindingLoc ← addGeneratedLocation intrinsicLoc
          let target ← addQualifiedRef binding.target
          return { loc := bindingLoc, role := binding.role, target }
        intrinsics := intrinsics.push {
          loc := intrinsicLoc, model := intrinsic.name, owner, profile := .move,
          executableBindings, specBindings }
    let inlineFunctions := module.functions.filterMap fun function =>
      if function.kind == .inlineRetained then some function.name else none
    let mut functions : Array (LeanerIR.FunctionDecl LeanerIR.Import.RawBody) := #[]
    for function in module.functions do
      if function.kind == .inlineRetained then continue
      resetLocals
      let functionLoc ← addLocation function.loc
      let parameters ← function.params.toArray.mapM (addParameter · functionLoc)
      let resultLoc ← addGeneratedLocation functionLoc
      let result ← typeUse resultLoc function.result
      let expressionsBefore := (← get).expressions.size
      modify fun state => { state with
        typeParameterCopy := function.typeParams.toArray.map (·.abilities.contains .copy) }
      let body ← function.body.mapM addExpr
      let parameters ← withMutablyBorrowedLocals expressionsBefore parameters
      let functionContract := Frames.explicit (← addContract function.spec functionLoc)
        ((← get).globalWriters.contains (ownName module function.name))
        (pragmaTrue function.pragmas "opaque")
      let locals := (← get).locals
      let attributes ← function.attributes.toArray
        |>.filter (!isResolvedBuildSelectionAttribute ·) |>.mapM addAttribute
      let mut profileData := #[visibilityProperty function.visibility, functionKindProperty function.kind]
      if function.isEntry then profileData := profileData.push (functionProperty "function.entry")
      if function.isReceiver then profileData := profileData.push (functionProperty "function.receiver")
      functions := functions.push {
        loc := functionLoc
        name := ← addName (ownName module function.name)
        doc := function.doc
        profile := .move
        signature := {
          generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
          parameters
          results := #[result] }
        body := body.map LeanerIR.Import.RawBody.structured |>.getD .absent
        origin
        alignment
        locals
        contract := functionContract
        -- A function's pragmas resolve its contract's, the explicit frame's
        -- included; a dependency's resolve `verify = false` from its namespace.
        pragmas := if module.isTarget then
            function.pragmas.toArray.map toPragma ++
              (if LeanerLang.hasLooseFrame functionContract then #[Frames.loosePragma] else #[]) ++
              proofFilePragma module
          else
            let contractNames := functionContract.pragmas.map pragmaName
            functionContract.pragmas ++ (nonTargetPragmas (modulePragmas module) true
              |>.filter fun pragma => !contractNames.contains (pragmaName pragma))
        profileData
        attributes }
    let mut specFunctions : Array LeanerIR.SpecFunctionDecl := #[]
    for function in module.specFuns do
      if function.isMoveFun && inlineFunctions.contains function.name then continue
      resetLocals
      let functionLoc ← addLocation function.loc
      let parameters ← function.params.toArray.mapM (addSpecificationParameter · functionLoc)
      let resultLoc ← addGeneratedLocation functionLoc
      let result ← typeUse resultLoc (specificationType function.result)
      let expressionsBefore := (← get).expressions.size
      let body ← logically (function.body.mapM addExpr)
      let parameters ← withMutablyBorrowedLocals expressionsBefore parameters
      let functionContract ← addContract function.spec functionLoc
      let locals := (← get).locals
      let mut profileData := #[]
      if function.uninterpreted then profileData := profileData.push (functionProperty "specFunction.uninterpreted")
      if function.isNative then profileData := profileData.push (functionProperty "specFunction.native")
      if function.isMoveFun then profileData := profileData.push (functionProperty "specFunction.moveFunction")
      if function.usesOld then profileData := profileData.push (functionProperty "specFunction.usesOld")
      specFunctions := specFunctions.push {
        loc := functionLoc
        name := ← addName (ownName module function.name)
        doc := function.doc
        profile := .move
        signature := {
          generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
          parameters
          results := #[result] }
        body
        origin
        locals
        contract := functionContract
        profileData }
    -- A spec variable is carried by the ghost resource the model backs it
    -- with (the exported `Ghost$x` struct) and the axiom below; it declares
    -- nothing of its own.
    let mut invariants : Array LeanerIR.NamespaceInvariant := #[]
    for invariant in module.invariants do
      resetLocals
      let invariantLoc ← addLocation invariant.loc
      let kind := match invariant.kind with
        | .global => LeanerIR.ConditionKind.globalInvariant invariant.typeParams.toArray
        | .globalUpdate => .globalInvariantUpdate invariant.typeParams.toArray
        | .«axiom» => .axiom_ invariant.typeParams.toArray
      let expression ← logically (addExpr invariant.exp)
      let locals := (← get).locals
      invariants := invariants.push {
        loc := invariantLoc
        condition := {
          loc := invariantLoc
          kind
          properties := invariant.properties.toArray.map toPragma
          expression }
        locals }
    -- The ghost resource backing a spec variable exists throughout, at
    -- address 0: the assumption the Move Prover's well-formedness
    -- instrumentation makes, stated as an axiom of the namespace.
    for specVar in module.specVars do
      if specVar.init.isSome then
        throw s!"spec variable `{specVar.name}` has an initializer, which is not supported"
      unless specVar.typeParams.isEmpty do
        throw s!"spec variable `{specVar.name}` is generic, which is not supported"
      resetLocals
      let variableLoc ← addLocation specVar.loc
      let axiomLoc ← addGeneratedLocation variableLoc
      let ghostType : Xast.Ty := .struct (ownName module s!"Ghost${specVar.name}") []
      let boolType ← addType axiomLoc .bool
      let addressType ← addType axiomLoc .address
      let address ← addExprNode axiomLoc addressType (.value (.address "0x0"))
      let ghostArgument := LeanerIR.GenericArgument.typeArg (← generatedTypeUse axiomLoc ghostType)
      let expression ← addExprNode axiomLoc boolType <|
        .operation (.global .contains) #[ghostArgument] #[address]
      invariants := invariants.push {
        loc := axiomLoc
        condition := { loc := axiomLoc, kind := .axiom_ #[], expression }
        locals := (← get).locals }
    let mut profileMetadata := #[]
    if let some alias := module.addressAlias then
      profileMetadata := profileMetadata.push (LeanerIR.Move.propertyValue "metadata.addressAlias" alias)
    for address in module.namedAddresses do
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.namedAddress" (Codec.encodeNamedAddress address))
    for friend in module.friends do
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.friend" (Codec.encodeModuleRef friend))
    for skipped in module.skipped do
      -- Compiler-v2 has already expanded retained inline functions at their
      -- call sites. Their retained declarations and companion diagnostics are
      -- producer bookkeeping, not unsupported declarations in target LIR.
      let expandedInlineDiagnostic :=
        skipped.reason.startsWith s!"in function `{module.name}::{skipped.name}`:"
      if inlineFunctions.contains skipped.name || expandedInlineDiagnostic then continue
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.skipped" (Codec.pack #[skipped.name, skipped.reason]))
    let comments : Array LeanerIR.Comment ←
        (retainedComments module inlineFunctions).toArray.mapM fun comment => do
      let commentLoc ← addLocation comment.loc
      return { loc := commentLoc, text := comment.text, ownLine := comment.ownLine }
    let state ← get
    return {
      loc
      identity := ⟨unitIndex⟩
      profile := some .move
      doc := module.doc
      expressions := state.expressions
      patterns := state.patterns
      places := state.places
      profileMetadata
      pragmas := if module.isTarget then modulePragmas module
        else nonTargetPragmas (modulePragmas module) true
      constants
      structs
      functions
      specFunctions
      specVars := #[]
      invariants
      intrinsics
      comments }

/-- Translate a complete compiler-v2 package into the only public LIR input
boundary. -/
def package (package : Package) : Except String LeanerIR.Import.RawUnit := do
  let ownedNamespaces := package.modules.toArray.map (·.ref)
  let copyableStructs := (package.modules ++ package.dependencies).flatMap fun module =>
    module.structs.filterMap fun structDecl =>
      if structDecl.abilities.contains .copy then
        some ({ module := module.ref, name := structDecl.name },
          structDecl.typeParams.map fun parameter => !parameter.isPhantom)
      else none
  let initial : BuildState := {
    sourceNamespaces := ownedNamespaces
    globalWriters := Frames.globalWriters package
    copyableStructs }
  -- Dependency interfaces are built after the owned namespaces, so their
  -- namespace identities follow the ones this unit declares.
  let build : BuildM (Array LeanerIR.Import.RawNamespace ×
      Array LeanerIR.Import.RawNamespaceInterface) := do
    let namespaces ← package.modules.toArray.mapIdxM fun index module =>
      buildNamespace index module
    let dependencies ← package.dependencies.toArray.mapM buildInterface
    return (namespaces, dependencies)
  let ((namespaces, dependencies), finalState) ← build.run initial
  let namespaceRefs := finalState.sourceNamespaces.map fun ref =>
    ({ segments := #[ref.address, ref.name], alias := ref.addressAlias } : LeanerIR.NamespaceRef)
  let evidence : Array LeanerIR.Import.ImportEvidence := #[{
    producer := "move exchange --format ast"
    description := "typed XAST v3 imported into neutral LIR" }]
  return {
    tables := {
      files := finalState.files
      locations := finalState.locations
      origins := finalState.origins
      alignments := finalState.alignments
      lifetimes := finalState.lifetimes
      types := finalState.types
      namespaces := namespaceRefs
      names := finalState.names }
    profiles := #[LeanerIR.Move.config]
    namespaces
    dependencies
    evidence }

end LeanerMove.Frontend.LIR.Encode

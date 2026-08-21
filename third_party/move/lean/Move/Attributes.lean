-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Lean.Compiler.InlineAttrs
import Lean.Elab.Declaration
import Lean.Elab.Deriving.Basic
import MoveModel.IR.Module

/-!
# Leaner declaration attributes

Move abilities use Lean's standard `deriving` surface, backed by persistent
declaration metadata. They are not inferred through open-world typeclass
search during compilation.

Source attributes written as `@[name arg ...]` before a `struct`, `enum`, or
`fun` keyword come in two kinds: the closed set of well-known internal
declaration markers (`move_struct`, `move_public`, ...), which desugar to the
tag attributes below, and open-ended user-provided attributes, which are
recorded structurally as `MoveModel.IR.Attribute` metadata. The argument
grammar (`moveAttrArg`) is shared with specification pragmas.
-/

/-- Argument of a user-provided Move attribute or pragma: a name path, a
numeric or boolean constant, or a parenthesized instantiated type. -/
declare_syntax_cat moveAttrArg

namespace Move

open Lean Elab Command

syntax (name := moveAttrArgIdent) ident : moveAttrArg
syntax (name := moveAttrArgNum) num : moveAttrArg
syntax (name := moveAttrArgApply) "(" ident moveAttrArg* ")" : moveAttrArg

private def decodeAttributeName (path : Name) : MoveModel.IR.AttributeArg :=
  match path with
  | `true => .bool true
  | `false => .bool false
  | _ => .name path.toString []

private def decodeAttributeNum (stx : Syntax) :
    Except String MoveModel.IR.AttributeArg := do
  let some value := stx.isNatLit?
    | throw "expected a numeric attribute argument"
  unless value < 2 ^ 64 do
    throw s!"attribute argument `{value}` does not fit in 64 bits"
  return .num value

private partial def decodeAttributeArg (stx : Syntax) :
    Except String MoveModel.IR.AttributeArg := do
  if stx.isOfKind ``moveAttrArgIdent then
    return decodeAttributeName stx[0].getId
  if stx.isOfKind ``moveAttrArgNum then
    return ← decodeAttributeNum stx[0]
  if stx.isOfKind ``moveAttrArgApply then
    let args ← stx[2].getArgs.toList.mapM decodeAttributeArg
    return .name stx[1].getId.toString args
  throw "unsupported attribute argument"

/-- Decode one parsed attribute or pragma instance (`ident moveAttrArg*`)
into structured metadata. -/
def decodeAttributeInstance (stx : Syntax) :
    Except String MoveModel.IR.Attribute := do
  let args ← stx[1].getArgs.toList.mapM decodeAttributeArg
  return { name := stx[0].getId.toString, args }

private def insertUserAttributes
    (map : NameMap (List MoveModel.IR.Attribute))
    (entry : Name × List MoveModel.IR.Attribute) :
    NameMap (List MoveModel.IR.Attribute) :=
  map.insert entry.1 entry.2

private initialize moveUserAttributeExt :
    SimplePersistentEnvExtension (Name × List MoveModel.IR.Attribute)
      (NameMap (List MoveModel.IR.Attribute)) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := insertUserAttributes
    addImportedFn := fun entries =>
      mkStateFromImportedEntries insertUserAttributes {} entries
  }

/-- Record the user-provided attributes of one Move declaration. -/
def registerUserAttributes (env : Environment) (declaration : Name)
    (attributes : List MoveModel.IR.Attribute) : Environment :=
  moveUserAttributeExt.addEntry env (declaration, attributes)

/-- The user-provided attributes recorded for a Move declaration. -/
def userAttributes (env : Environment) (declaration : Name) :
    List MoveModel.IR.Attribute :=
  ((moveUserAttributeExt.getState env).find? declaration).getD []

private def insertDataInvariant (map : NameMap Name) (entry : Name × Name) :
    NameMap Name :=
  map.insert entry.1 entry.2

private initialize moveDataInvariantExt :
    SimplePersistentEnvExtension (Name × Name) (NameMap Name) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := insertDataInvariant
    addImportedFn := fun entries =>
      mkStateFromImportedEntries insertDataInvariant {} entries
  }

private def insertGlobalInvariant (map : NameMap Name) (entry : Name × Name) :
    NameMap Name :=
  map.insert entry.1 entry.2

private initialize moveGlobalInvariantExt :
    SimplePersistentEnvExtension (Name × Name) (NameMap Name) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := insertGlobalInvariant
    addImportedFn := fun entries =>
      mkStateFromImportedEntries insertGlobalInvariant {} entries
  }

/-- Record that a resource family carries a global invariant, whose body (a
predicate over one stored value) is the named declaration.  The store-level
invariant is `Move.Semantics.forallStored body`. -/
def registerGlobalInvariant (env : Environment) (family : Name)
    (body : Name) : Environment :=
  moveGlobalInvariantExt.addEntry env (family, body)

/-- The global-invariant body a resource family declares, if any. -/
def globalInvariant? (env : Environment) (family : Name) : Option Name :=
  (moveGlobalInvariantExt.getState env).find? family

/-- Record that values of a Move type certify a data invariant. -/
def registerDataInvariant (env : Environment) (type : Name)
    (invariant : Name) : Environment :=
  moveDataInvariantExt.addEntry env (type, invariant)

/-- The data invariant a Move type certifies, if it declares one. -/
def dataInvariant? (env : Environment) (type : Name) : Option Name :=
  (moveDataInvariantExt.getState env).find? type

/-- The on-chain identity assigned to declarations enclosed by a
`move_module`. This metadata is persisted in `.olean` files, so an imported
Lean module retains the Move identity needed by cross-module lowering. -/
structure ModuleRef where
  address : String := "0x0"
  name : String
  deriving Inhabited, BEq, Repr

private structure ModuleRegistration where
  leanNamespace : Name
  module : ModuleRef
  deriving Inhabited

private def addModuleRegistration (registrations : List ModuleRegistration)
    (registration : ModuleRegistration) : List ModuleRegistration :=
  registration :: registrations.filter (·.leanNamespace != registration.leanNamespace)

private initialize moveModuleExt :
    SimplePersistentEnvExtension ModuleRegistration (List ModuleRegistration) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := addModuleRegistration
    addImportedFn := fun entries =>
      mkStateFromImportedEntries addModuleRegistration [] entries
  }

/-- Record that declarations below `namespace` belong to one Move module. -/
def registerModuleNamespace (env : Environment) (leanNamespace : Name)
    (module : ModuleRef) : Environment :=
  moveModuleExt.addEntry env { leanNamespace, module }

/-- Find the most closely enclosing registered Move module for a Lean
declaration. Longest-prefix selection also makes nested ordinary Lean
namespaces inside a `move_module` behave as expected. -/
def moduleForDeclaration? (env : Environment) (declaration : Name) : Option ModuleRef :=
  let best : Option (Name × ModuleRef) :=
    (moveModuleExt.getState env).foldl (init := none) fun best registration =>
    if registration.leanNamespace.isPrefixOf declaration then
      match best with
      | none => some (registration.leanNamespace, registration.module)
      | some (leanNamespace, _) =>
          if leanNamespace.toString.length < registration.leanNamespace.toString.length then
            some (registration.leanNamespace, registration.module)
          else
            best
    else
      best
  best.map (·.2)

/-- Move's `copy` ability, used as a `deriving` marker. -/
class Copy (T : Type u) : Prop where
  private marker : True

/-- Move's `drop` ability, used as a `deriving` marker. -/
class Drop (T : Type u) : Prop where
  private marker : True

/-- Move's `store` ability, used as a `deriving` marker. -/
class Store (T : Type u) : Prop where
  private marker : True

/-- Move's `key` ability, used as a `deriving` marker. -/
class Key (T : Type u) : Prop where
  private marker : True

private def preserveMoveCall (declName : Lean.Name) : Lean.AttrM Unit := do
  let env ← getEnv
  match Lean.Compiler.setInlineAttribute env declName .noinline with
  | .ok env => modifyEnv fun _ => env
  | .error message => throwError message

initialize moveStructAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_struct "Move-compatible structure"

initialize moveEnumAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_enum "Move-compatible enum"

initialize moveCopyAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_copy "Move type deriving the copy ability"

initialize moveDropAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_drop "Move type deriving the drop ability"

initialize moveStoreAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_store "Move type deriving the store ability"

initialize moveKeyAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_key "Move type deriving the key ability"

initialize moveFunAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_fun "private Move function" preserveMoveCall

initialize movePublicAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_public "public Move function" preserveMoveCall

initialize moveFriendAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_friend
    "friend-visible Move function" preserveMoveCall

initialize moveEntryAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_entry "public Move entry function" preserveMoveCall

initialize moveNativeAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_native "Move native function declaration"

/-- Whether a declaration is a Move function of any visibility. -/
def isMoveFunction (env : Environment) (name : Name) : Bool :=
  moveFunAttr.hasTag env name || movePublicAttr.hasTag env name ||
    moveFriendAttr.hasTag env name || moveEntryAttr.hasTag env name

private def deriveAbility (tag : Lean.TagAttribute) : Lean.Elab.DerivingHandler :=
  fun typeNames => do
    for typeName in typeNames do
      tag.setTag typeName
    return true

initialize
  Lean.Elab.registerDerivingHandler ``Copy (deriveAbility moveCopyAttr)
  Lean.Elab.registerDerivingHandler ``Drop (deriveAbility moveDropAttr)
  Lean.Elab.registerDerivingHandler ``Store (deriveAbility moveStoreAttr)
  Lean.Elab.registerDerivingHandler ``Key (deriveAbility moveKeyAttr)

private partial def hasMoveTypeAttribute (stx : Syntax) : Bool :=
  if stx.isIdent then
    let name := stx.getId
    name == `move_struct || name == `move_enum
  else
    stx.getArgs.any hasMoveTypeAttribute

private partial def containsInhabited (stx : Syntax) : Bool :=
  if stx.isIdent then
    stx.getId.getString! == "Inhabited"
  else
    stx.getArgs.any containsInhabited

private def explicitlyDerivesInhabited (decl : Syntax) : Bool :=
  if decl.isOfKind ``Lean.Parser.Command.structure then
    containsInhabited decl[5]
  else if decl.isOfKind ``Lean.Parser.Command.inductive then
    containsInhabited decl[6]
  else
    false

private def moveTypeDeclarationName (stx : Syntax) : CommandElabM Name := do
  let (name, _) := Lean.Elab.expandDeclIdCore stx[1][1]
  if (`_root_).isPrefixOf name then
    return name.replacePrefix `_root_ .anonymous
  return (← getCurrNamespace) ++ name

private def canDeriveInhabited (name : Name) : CommandElabM Bool := do
  let some (.inductInfo info) := (← getEnv).find? name | return true
  unless !info.isRec && !info.ctors.isEmpty do return false
  -- A certified type carries a proof in its constructors, so an inhabitant
  -- needs that proof as well; its instance is generated where the invariant
  -- is known.
  let certified ← liftTermElabM do
    info.ctors.anyM fun ctorName => do
      let some (.ctorInfo ctor) := (← getEnv).find? ctorName | return false
      Lean.Meta.forallTelescopeReducing ctor.type fun binders _ =>
        binders.anyM fun binder => do
          Lean.Meta.isProp (← Lean.Meta.inferType binder)
  return !certified

/-- Move values are always inhabited. Generate the corresponding host-side
instance for every Move structure and enum, without requiring a
source-level `deriving Inhabited` clause. -/
@[command_elab Lean.Parser.Command.declaration]
def elabMoveTypeDeclaration : CommandElab := fun stx => do
  let decl := stx[1]
  unless hasMoveTypeAttribute stx[0] &&
      (decl.isOfKind ``Lean.Parser.Command.structure ||
        decl.isOfKind ``Lean.Parser.Command.inductive) do
    throwUnsupportedSyntax
  Lean.Elab.Command.elabDeclaration stx
  let typeName ← moveTypeDeclarationName stx
  -- Empty and recursive types are rejected by the Move frontend. Let those
  -- diagnostics remain authoritative instead of producing a deriving failure.
  unless explicitlyDerivesInhabited decl || !(← canDeriveInhabited typeName) do
    Lean.Elab.applyDerivingHandlers ``Inhabited
      #[typeName]

end Move

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Lean.Compiler.InlineAttrs
import Lean.Elab.Declaration
import Lean.Elab.Deriving.Basic

/-!
# Leaner declaration attributes

Move abilities use Lean's standard `deriving` surface, backed by persistent
declaration metadata. They are not inferred through open-world typeclass
search during compilation.
-/

namespace Move

open Lean Elab Command

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

initialize moveEntryAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_entry "public Move entry function" preserveMoveCall

initialize moveNativeAttr : Lean.TagAttribute ←
  Lean.registerTagAttribute `move_native "Move native function declaration"

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
  return !info.isRec && !info.ctors.isEmpty

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

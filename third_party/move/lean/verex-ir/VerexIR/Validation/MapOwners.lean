-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.TableRoles

/-!
# Intrinsic map owners

Every structure marked an intrinsic map has one abstract layout: its
entries, its ghost slots, and its allocation handle where its Move type
holds table storage (`doc/designs/intrinsic-maps.md`, "Abstract
representation"). Its concrete value holds them in one of a few shapes,
which this module names (`ownerLayout?`); the runtime semantics reads the
abstract value off the concrete one by the shape (`normalize`).
-/

namespace VerexIR.Validation

/-- The declaration a nominal type names: its namespace and index there, the
namespace, and the declaration. -/
def nominalDeclaration? (unit : ValidatedUnit) (ty : Ty) :
    Option (NamespaceId × Nat × ValidatedNamespace × StructDecl) := do
  let .nominal name _ := ty | none
  let qualified ← unit.tables.names[name.index]?
  let ns ← unit.namespaces[qualified.namespaceId.index]?
  let structId ← ns.structs.findIdx? (·.name == name)
  let declaration ← ns.structs[structId]?
  some (qualified.namespaceId, structId, ns, declaration)

/-- Whether type arguments are the type parameters of their owner, in order. -/
def ownTypeParameters (unit : ValidatedUnit) (arguments : Array GenericArgument) : Bool :=
  arguments.zipIdx.all fun (argument, index) =>
    match argument with
    | .typeArg type => unit.tables.types[type.typeId.index]? == some (Ty.typeParameter index)
    | _ => false

/-- How an intrinsic map owner's concrete value holds its entries. -/
inductive OwnerShape where
  /-- A table: the abstract layout itself, which the runtime holds. -/
  | table (layout : TableLayout)
  /-- One vector of two-field entry values, at the field before the ghost
  slots (`SimpleMap`, `OrderedMap`). -/
  | entries
  /-- A table of the owner's key and value types in the first field, its
  size cached in the second (`TableWithLength`). -/
  | wrapped
  /-- Nodes of a tree (`BigOrderedMap`): a root node at field `root`, an
  allocator of nodes at field `nodes`, each node's children at its field
  `children`, an entries map whose values of the variant `leaf` hold the
  map's values; the allocator's table handle at `handlePath`. -/
  | tree (nodeNamespace : NamespaceId) (nodeStruct : Nat) (root nodes children : Nat)
      (leaf : String) (handlePath : Option (List Nat))
  deriving Repr, Inhabited

/-- An intrinsic map owner: the variant its values carry, the shape of its
concrete value, its declared fields' count (its ghost slots follow them), its
ghost slots, and whether it enumerates in key order. -/
structure OwnerLayout where
  namespaceId : NamespaceId
  structId : Nat
  variant : Option String
  shape : OwnerShape
  declared : Nat
  ghosts : Nat
  ordered : Bool
  deriving Repr, Inhabited

/-- Whether a declaration is an intrinsic map owner of its namespace. -/
def isMapOwner (ns : ValidatedNamespace) (declaration : StructDecl) : Bool :=
  ns.intrinsics.any fun intrinsic => intrinsic.model == "map" && intrinsic.owner == declaration.name

/-- The fields of a struct, or of the one variant of an enum. -/
private def soleFields? (declaration : StructDecl) : Option (Array FieldDecl) :=
  match declaration.variants.toList with
  | [] => some declaration.fields
  | [variant] => some variant.fields
  | _ => none

/-- A tree node at its owner's key and value types: its handle, the index of
its field holding its children, an intrinsic map from the key type to an
enum at the value type, and the name of that enum's variant holding a
value. -/
private def treeNode? (unit : ValidatedUnit) (ty : Ty) :
    Option (NamespaceId × Nat × Nat × String) := do
  let .nominal _ arguments := ty | none
  guard (arguments.size == 2 && ownTypeParameters unit arguments)
  let (nodeNamespace, nodeStruct, _, declaration) ← nominalDeclaration? unit ty
  let fields ← soleFields? declaration
  let leafOf (field : FieldDecl) : Option String := do
    let childrenType ← unit.tables.types[field.type.typeId.index]?
    let (_, _, childrenNs, childrenDeclaration) ← nominalDeclaration? unit childrenType
    guard (isMapOwner childrenNs childrenDeclaration)
    let .nominal _ #[.typeArg key, .typeArg child] := childrenType | none
    guard (unit.tables.types[key.typeId.index]? == some (Ty.typeParameter 0))
    let childType ← unit.tables.types[child.typeId.index]?
    let .nominal _ #[.typeArg value] := childType | none
    guard (unit.tables.types[value.typeId.index]? == some (Ty.typeParameter 1))
    let (_, _, _, childDeclaration) ← nominalDeclaration? unit childType
    let variant ← childDeclaration.variants.find? fun variant =>
      match variant.fields.toList with
      | [field] => unit.tables.types[field.type.typeId.index]? == some (Ty.typeParameter 0)
      | _ => false
    (unit.tables.names[variant.name.index]?).map (·.name)
  let children ← fields.findIdx? fun field => (leafOf field).isSome
  let leaf ← leafOf (← fields[children]?)
  some (nodeNamespace, nodeStruct, children, leaf)

/-- The path, from a value of a type, to the handle of the first table it
holds in a field: the field's index, then the table's handle field. -/
private def handlePath? (unit : ValidatedUnit) (ty : Ty) : Option (List Nat) := do
  let (_, _, _, declaration) ← nominalDeclaration? unit ty
  let fields ← soleFields? declaration
  fields.toList.zipIdx.findSome? fun (field, index) => do
    let fieldType ← unit.tables.types[field.type.typeId.index]?
    let (namespaceId, structId, _, _) ← nominalDeclaration? unit fieldType
    let layout ← tableLayout? unit namespaceId structId
    guard layout.handle
    some [index, 0]

/-- The layout of an intrinsic map owner, by the shape of its declaration;
none for a declaration that is no owner, or one holding its entries in a
shape not carried (`SmartTable`). -/
def ownerLayout? (unit : ValidatedUnit) (namespaceId : NamespaceId) (structId : Nat) :
    Option OwnerLayout := do
  let ns ← unit.namespaces[namespaceId.index]?
  let declaration ← ns.structs[structId]?
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name
  guard (declaration.generics.size == 2)
  let ordered := (intrinsic.executableBindings ++ intrinsic.specBindings).any fun binding =>
    (MapRole.ofName? binding.role).any (·.readsInOrder)
  let (variant, fields) ← match declaration.variants.toList with
    | [] => some (none, declaration.fields)
    | [variant] => some ((unit.tables.names[variant.name.index]?).map (·.name), variant.fields)
    | _ => none
  let typeOf (field : FieldDecl) : Option Ty := unit.tables.types[field.type.typeId.index]?
  let table : Option OwnerShape := .table <$> tableLayout? unit namespaceId structId
  let entries : Option OwnerShape := do
    let [field] := fields.toList | none
    let .vector element _ ← typeOf field | none
    let (_, _, _, entryDeclaration) ← nominalDeclaration? unit (← unit.tables.types[element.index]?)
    guard (entryDeclaration.variants.isEmpty && entryDeclaration.fields.size == 2)
    some .entries
  let wrapped : Option OwnerShape := do
    let [held, cached] := fields.toList | none
    guard (typeOf cached == some (.integer (.bits 64) false))
    let heldType ← typeOf held
    let .nominal _ arguments := heldType | none
    guard (arguments.size == 2 && ownTypeParameters unit arguments)
    let (innerNamespace, innerStruct, _, _) ← nominalDeclaration? unit heldType
    let innerLayout ← tableLayout? unit innerNamespace innerStruct
    guard innerLayout.handle
    some .wrapped
  let tree : Option OwnerShape := do
    let root ← fields.findIdx? fun field => ((typeOf field).bind (treeNode? unit)).isSome
    let rootType ← typeOf (← fields[root]?)
    let (nodeNamespace, nodeStruct, children, leaf) ← treeNode? unit rootType
    let nodes ← fields.findIdx? fun field => match typeOf field with
      | some (.nominal _ #[.typeArg held]) => unit.tables.types[held.typeId.index]? == some rootType
      | _ => false
    let path := (typeOf (← fields[nodes]?)).bind (handlePath? unit) |>.map (nodes :: ·)
    some (.tree nodeNamespace nodeStruct root nodes children leaf path)
  let shape ← table <|> entries <|> wrapped <|> tree
  some {
    namespaceId, structId, variant, shape
    declared := fields.size
    ghosts := declaration.ghosts.size
    ordered }

/-- The layout of an intrinsic map owner whose values the denotation holds
abstractly, which the runtime's values are read through
(`SemanticOperations.normalize`): every owner (`doc/designs/intrinsic-maps.md`,
"Abstract representation"). -/
def abstractLayout? (unit : ValidatedUnit) (namespaceId : NamespaceId) (structId : Nat) :
    Option OwnerLayout :=
  ownerLayout? unit namespaceId structId

end VerexIR.Validation

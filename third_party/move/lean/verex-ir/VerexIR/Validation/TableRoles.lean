-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Validated

/-!
# Table owners and their roles

An intrinsic map whose owner holds its entries in a hidden last field is a
table: its value holds its identity (a handle, or the placeholder field of
an owner declared without fields) and its entries, and the runtime provides
the roles its module binds on that value, not running their bodies
(`doc/designs/intrinsic-maps.md`, "Tables in the runtime semantics"). This
module names the owners (`tableLayout?`) and the role functions
(`tableRoleOf?`), which preparation, the runtime, and the denotation share.
-/

namespace VerexIR.Validation

end VerexIR.Validation

namespace VerexIR

/-- The executable roles of the Move `map` intrinsic
(`doc/designs/intrinsic-maps.md`, "Roles"): the functions a map owner binds. The
table runtime dispatches exhaustively over this type and the role contracts
match on it, so a role added here is given a semantics and a contract. -/
inductive MapOperation where
  | addAll | addNoOverride | addOverrideIfExists | append | appendDisjoint
  | backKey | borrow | borrowBack | borrowFront | borrowMut
  | borrowMutWithDefault | borrowWithDefault | delMustExist | delReturnKey
  | destroyEmpty | destroyUnchecked | frontKey | get | hasKey | isEmpty
  | iterBorrowMut | keys | len | new_ | newFrom | newWithConfig | nextKey
  | popBack | popFront | prevKey | removeOrNone | replaceKeyInplace
  | toOrderedMap | toVecPair | trim | upsert | upsertAll | values
  deriving Repr, BEq, DecidableEq, Inhabited

/-- The specification roles of the Move `map` intrinsic: the specification
functions a map owner binds, which the contracts read as the map model. -/
inductive MapSpecRole where
  | abortsAdd | abortsAddAll | abortsAppendDisjoint | abortsBorrow | abortsDel
  | abortsDestroyEmpty | abortsEmpty | abortsIterBorrowMut | abortsNewFrom
  | abortsNewWithConfig | abortsReplaceKeyInplace | abortsTrim | abortsUpsertAll
  | del | get | hasKey | insertionKeyAt | insertionRank | isEmpty | iterPreserved
  | iterValid | keyAt | leafIterValid | leafOffset | len | new_ | rank | set
  deriving Repr, BEq, DecidableEq, Inhabited

/-- A role of the Move `map` intrinsic, named in a binding by its source name. -/
inductive MapRole where
  | executable (operation : MapOperation)
  | specification (role : MapSpecRole)
  deriving Repr, BEq, DecidableEq, Inhabited

namespace MapOperation

/-- The source name of an executable role. -/
def name : MapOperation → String
  | .addAll => "map_add_all"
  | .addNoOverride => "map_add_no_override"
  | .addOverrideIfExists => "map_add_override_if_exists"
  | .append => "map_append"
  | .appendDisjoint => "map_append_disjoint"
  | .backKey => "map_back_key"
  | .borrow => "map_borrow"
  | .borrowBack => "map_borrow_back"
  | .borrowFront => "map_borrow_front"
  | .borrowMut => "map_borrow_mut"
  | .borrowMutWithDefault => "map_borrow_mut_with_default"
  | .borrowWithDefault => "map_borrow_with_default"
  | .delMustExist => "map_del_must_exist"
  | .delReturnKey => "map_del_return_key"
  | .destroyEmpty => "map_destroy_empty"
  | .destroyUnchecked => "map_destroy_unchecked"
  | .frontKey => "map_front_key"
  | .get => "map_get"
  | .hasKey => "map_has_key"
  | .isEmpty => "map_is_empty"
  | .iterBorrowMut => "map_iter_borrow_mut"
  | .keys => "map_keys"
  | .len => "map_len"
  | .new_ => "map_new"
  | .newFrom => "map_new_from"
  | .newWithConfig => "map_new_with_config"
  | .nextKey => "map_next_key"
  | .popBack => "map_pop_back"
  | .popFront => "map_pop_front"
  | .prevKey => "map_prev_key"
  | .removeOrNone => "map_remove_or_none"
  | .replaceKeyInplace => "map_replace_key_inplace"
  | .toOrderedMap => "map_to_ordered_map"
  | .toVecPair => "map_to_vec_pair"
  | .trim => "map_trim"
  | .upsert => "map_upsert"
  | .upsertAll => "map_upsert_all"
  | .values => "map_values"

/-- Every executable role. -/
def all : Array MapOperation := #[
  .addAll, .addNoOverride, .addOverrideIfExists, .append, .appendDisjoint,
  .backKey, .borrow, .borrowBack, .borrowFront, .borrowMut,
  .borrowMutWithDefault, .borrowWithDefault, .delMustExist, .delReturnKey,
  .destroyEmpty, .destroyUnchecked, .frontKey, .get, .hasKey, .isEmpty,
  .iterBorrowMut, .keys, .len, .new_, .newFrom, .newWithConfig, .nextKey,
  .popBack, .popFront, .prevKey, .removeOrNone, .replaceKeyInplace,
  .toOrderedMap, .toVecPair, .trim, .upsert, .upsertAll, .values]

def ofName? (name : String) : Option MapOperation := all.find? (·.name == name)

/-- The position of the map among a role's parameters: an iterator precedes
the map it indexes. -/
def mapArgument : MapOperation → Nat
  | .iterBorrowMut => 1
  | _ => 0

end MapOperation

namespace MapSpecRole

/-- The source name of a specification role. -/
def name : MapSpecRole → String
  | .abortsAdd => "map_spec_aborts_add"
  | .abortsAddAll => "map_spec_aborts_add_all"
  | .abortsAppendDisjoint => "map_spec_aborts_append_disjoint"
  | .abortsBorrow => "map_spec_aborts_borrow"
  | .abortsDel => "map_spec_aborts_del"
  | .abortsDestroyEmpty => "map_spec_aborts_destroy_empty"
  | .abortsEmpty => "map_spec_aborts_empty"
  | .abortsIterBorrowMut => "map_spec_aborts_iter_borrow_mut"
  | .abortsNewFrom => "map_spec_aborts_new_from"
  | .abortsNewWithConfig => "map_spec_aborts_new_with_config"
  | .abortsReplaceKeyInplace => "map_spec_aborts_replace_key_inplace"
  | .abortsTrim => "map_spec_aborts_trim"
  | .abortsUpsertAll => "map_spec_aborts_upsert_all"
  | .del => "map_spec_del"
  | .get => "map_spec_get"
  | .hasKey => "map_spec_has_key"
  | .insertionKeyAt => "map_spec_insertion_key_at"
  | .insertionRank => "map_spec_insertion_rank"
  | .isEmpty => "map_spec_is_empty"
  | .iterPreserved => "map_spec_iter_preserved"
  | .iterValid => "map_spec_iter_valid"
  | .keyAt => "map_spec_key_at"
  | .leafIterValid => "map_spec_leaf_iter_valid"
  | .leafOffset => "map_spec_leaf_offset"
  | .len => "map_spec_len"
  | .new_ => "map_spec_new"
  | .rank => "map_spec_rank"
  | .set => "map_spec_set"

/-- Every specification role. -/
def all : Array MapSpecRole := #[
  .abortsAdd, .abortsAddAll, .abortsAppendDisjoint, .abortsBorrow, .abortsDel,
  .abortsDestroyEmpty, .abortsEmpty, .abortsIterBorrowMut, .abortsNewFrom,
  .abortsNewWithConfig, .abortsReplaceKeyInplace, .abortsTrim, .abortsUpsertAll,
  .del, .get, .hasKey, .insertionKeyAt, .insertionRank, .isEmpty, .iterPreserved,
  .iterValid, .keyAt, .leafIterValid, .leafOffset, .len, .new_, .rank, .set]

def ofName? (name : String) : Option MapSpecRole := all.find? (·.name == name)

end MapSpecRole

namespace MapRole

def name : MapRole → String
  | .executable operation => operation.name
  | .specification role => role.name

/-- Every role, the executable ones first. -/
def all : Array MapRole :=
  MapOperation.all.map .executable ++ MapSpecRole.all.map .specification

def ofName? (name : String) : Option MapRole :=
  (MapOperation.ofName? name).map .executable <|>
    (MapSpecRole.ofName? name).map .specification

/-- The roles that read a map's keys in order: an owner binding one
enumerates its entries in key order. -/
def readsInOrder : MapRole → Bool
  | .specification .keyAt | .specification .rank
  | .executable .borrowFront | .executable .borrowBack | .executable .frontKey
  | .executable .backKey | .executable .popFront | .executable .popBack
  | .executable .prevKey | .executable .nextKey => true
  | _ => false

end MapRole

end VerexIR

namespace VerexIR.Validation

/-- The namespace and index of the declaration an interned nominal name
denotes. -/
def nominalDeclarationId? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (name : NameId) : Option (NamespaceId × Nat) := do
  let qualified ← ns.tables.names[name.index]?
  let typeId ← unit.resolution.nominal? name
  let targetNamespace ← unit.namespaces[qualified.namespaceId.index]?
  let _ ← targetNamespace.structs[typeId.index]?
  return (qualified.namespaceId, typeId.index)

/-- The layout of a table owner: its declaration, whether it holds a handle,
whether it caches its length, whether it holds the Boolean placeholder of a
struct declared without fields, and whether it keeps its keys in order
(`doc/designs/intrinsic-maps.md`, "Views"). -/
structure TableLayout where
  namespaceId : NamespaceId
  structId : Nat
  handle : Bool
  length : Bool
  dummy : Bool
  ordered : Bool
  /-- The owner's ghost slots (`doc/designs/ghost-fields.md`). -/
  ghosts : Nat
  deriving Repr, Inhabited

/-- The table layout of a declaration: an intrinsic map owner with a key and
a value parameter, no variants, and fields that are none, the Boolean
placeholder of an empty struct, a handle, or a handle and a cached length. -/
def tableLayout? (unit : ValidatedUnit) (namespaceId : NamespaceId) (structId : Nat) :
    Option TableLayout := do
  let ns ← unit.namespaces[namespaceId.index]?
  let declaration ← ns.structs[structId]?
  let intrinsic ← ns.intrinsics.find? fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name
  guard (declaration.variants.isEmpty && declaration.generics.size == 2)
  let fieldTypes : List Ty ← declaration.fields.toList.mapM fun field =>
    unit.tables.types[field.type.typeId.index]?
  let (handle, length, dummy) ← match fieldTypes with
    | [] => some (false, false, false)
    -- The Boolean placeholder Move's compilers give an empty struct.
    | [Ty.bool] => some (false, false, true)
    | [Ty.address] => some (true, false, false)
    | [Ty.address, Ty.integer (.bits 64) false] => some (true, true, false)
    | _ => none
  let ordered := (intrinsic.executableBindings ++ intrinsic.specBindings).any fun binding =>
    (MapRole.ofName? binding.role).any (·.readsInOrder)
  some ⟨namespaceId, structId, handle, length, dummy, ordered, declaration.ghosts.size⟩

/-- The table layout of a function's result, where it is a table owner: the
map a conversion role (`map_to_ordered_map`) builds. -/
def resultLayout? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) : Option TableLayout := do
  let result ← declaration.signature.results[0]?
  let .nominal name _ ← ns.tables.types[result.typeId.index]? | none
  let (namespaceId, structId) ← nominalDeclarationId? unit ns name
  tableLayout? unit namespaceId structId

/-- The executable map role a function plays for a table owner of its
namespace, with the owner's layout: a function the runtime provides. -/
def tableRoleOf? (unit : ValidatedUnit) (namespaceId : NamespaceId) (functionId : FunctionId) :
    Option (String × TableLayout) := do
  let ns ← unit.namespaces[namespaceId.index]?
  let declaration ← ns.functions[functionId.index]?
  let target : QualifiedRef := { namespaceId, name := declaration.name }
  ns.intrinsics.findSome? fun intrinsic => do
    guard (intrinsic.model == "map")
    let binding ← intrinsic.executableBindings.find? (·.target == target)
    let structId ← ns.structs.findIdx? (·.name == intrinsic.owner)
    let layout ← tableLayout? unit namespaceId structId
    some (binding.role, layout)

end VerexIR.Validation

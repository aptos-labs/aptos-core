-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program

/-!
# The modeled natives

TRUSTED. The natives of the standard library whose calls are in the supported
subset although they take or return references, each with its shape: the one
list to review (`modeledNatives`). Every other native with a reference
parameter or result stays outside the subset.

Language-owner rulings of 2026-11-03:

1. **Shared contract shapes, not per-native contracts.** The natives are
   described by three shapes, not one contract each. The natives of
   `std::vector` marked `#[bytecode_instruction]` in `vector.move` are *not*
   defined as their instructions (ruling of 2026-11-03, replacing the first
   ruling that they were): `#[bytecode_instruction]` tells the compiler to emit
   the instruction for a call, and neither the declarations nor their docs
   promise that a direct bytecode `Call` of these natives behaves as the
   instruction. They are ordinary modeled natives of the matching shape:
   `length` read; `push_back`, `pop_back` and `swap` update; `borrow` and
   `borrow_mut` borrow. `empty` and `destroy_empty` take and return values
   only, and are natives like any other such (not in the list). The spec
   assumes nothing of them beyond their shape: any behavior that meets it,
   failing included, is theirs.
2. **A signer contains an address that can be borrowed.** "`borrow_address`
   borrows this inner field" (`signer.move`), so `signer::borrow_address(self:
   &signer): &address` returns a reference to the address inside the signer
   its argument designates (`PathStep.signerAddress`).
3. **Scope.** Exactly the natives below, with the shapes given:
   - *read* (`NativeShape.read`): immutable reference parameters and value
     parameters, value results; the native writes nothing;
   - *update* (`NativeShape.update`): mutable reference parameters; the native
     leaves new values in the places they designate, and returns values;
   - *borrow* (`NativeShape.borrow`): the native returns references, each to a
     place inside what one of its reference parameters designates, never more
     mutable than that parameter.

A shape native is still a relation (`NativeRel`), which sees what its
reference arguments designate (`State.nativeView`), not the references; the
semantics writes back and makes references (`resumeNative`).

**Identification** (language-owner ruling, 2026-11-04: by exact
declaration). A native is modeled when its module is `std::m` (address
`0x1`), its name is the entry's, and its declared signature is the entry's:
the same type parameters with exactly the same constraints, and parameter and
return tokens matching the entry's patterns (`TokenPattern`), struct types by
module and name. A native that matches by name but not by signature is not
modeled, nor is one that declares other constraints, as `borrow<Element:
copy>` would. Visibility and `entry` are not part of the identification (the
`string` natives are private). The `#[bytecode_instruction]` attribute is a
source attribute; the binary format carries no such attribute
(`FunctionAttribute`), so the identification does not use it.

The declarations, from `move-stdlib/sources`:

- `vector.move`: `length<Element>(self: &vector<Element>): u64`,
  `borrow<Element>(self: &vector<Element>, i: u64): &Element`,
  `push_back<Element>(self: &mut vector<Element>, e: Element)`,
  `borrow_mut<Element>(self: &mut vector<Element>, i: u64): &mut Element`,
  `pop_back<Element>(self: &mut vector<Element>): Element`, `swap<Element>(self:
  &mut vector<Element>, i: u64, j: u64)`, all `#[bytecode_instruction]`; and
  `move_range<T>(from: &mut vector<T>, removal_position: u64, length: u64, to:
  &mut vector<T>, insert_position: u64)`;
- `signer.move`: `borrow_address(self: &signer): &address`;
- `string.move`: `internal_check_utf8(v: &vector<u8>): bool`,
  `internal_is_char_boundary(v: &vector<u8>, i: u64): bool`,
  `internal_sub_string(v: &vector<u8>, i: u64, j: u64): vector<u8>`,
  `internal_index_of(v: &vector<u8>, r: &vector<u8>): u64`;
- `bcs.move`: `to_bytes<MoveValue>(v: &MoveValue): vector<u8>`,
  `serialized_size<MoveValue>(v: &MoveValue): u64`;
- `cmp.move`: `compare<T>(first: &T, second: &T): Ordering`, where `Ordering`
  is the enum `std::cmp::Ordering`;
- `mem.move`: `swap<T>(left: &mut T, right: &mut T)`.

No type parameter of these declares a constraint.

**Table natives** (language-owner rulings, 2026-11-04). The six natives of
`aptos_std::table` (address `0x1`, module `table`) with reference parameters
or results, as `aptos-stdlib/sources/table.move` declares them, where `Table`
and `Box` are the generic structs of `0x1::table`:

- `add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>)`;
- `borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>`;
- `borrow_box_mut<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>`;
- `contains_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): bool`;
- `remove_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): Box<V>`;
- `destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>)`.

1. *A table's entries are inside the table* (`Value.table`): `contains_box` and
   `destroy_empty_box` read; `add_box` and `remove_box` update, writing back the
   table; `borrow_box` and `borrow_box_mut` borrow, returning a place inside
   the table (`PathStep.tableEntry`).
2. *A condition on the table module.* They are modeled only in a program
   whose module `0x1::table`, where they are declared, keeps the table rules
   (`CompiledModule.followsTableRules`), checked on that module's bytecode. The
   condition is part of the identification (`ModeledNative.admittedIn`): the
   module that declares the natives is the one it is about, and a native that
   is not modeled stays outside the subset, so a program breaking the rules is
   not `Supported` and its calls of these natives are `unsupported`.
3. *Destroying tables.* `destroy_empty_box` only reads: it promises nothing
   about emptiness ("Table cannot know if it is empty or not", `table.move`).
   `drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>)` takes a
   value and returns none, so it is a native like any other such (not in the
   list); it consumes the table with its entries, as the native policy allows
   (`NativePolicy.destroys`).
4. *Contracts promise shapes only*, as for the other modeled natives: any call
   may fail, and the spec does not say which entry a key selects, nor that
   `remove_box` returns what `add_box` stored.

`new_table_handle<K, V>(): address` takes and returns values only, and is a
native like any other such.
-/

namespace MoveBytecode

/-- The shape of a modeled native (module docstring). -/
inductive NativeShape where
  | read
  | update
  | borrow
  deriving DecidableEq, Repr

/-- A pattern for a signature token of a modeled native's declaration. A
struct pattern names a non-generic struct or enum type of the standard
library, `std::module::name`; a struct instantiation pattern a generic one,
`std::module::name<args>`, at type arguments matching `args`. -/
inductive TokenPattern where
  | bool
  | u8
  | u64
  | address
  | signer
  | vector (elem : TokenPattern)
  | typeParameter (index : TypeParameterIndex)
  | struct (module name : String)
  | structInstantiation (module name : String) (args : List TokenPattern)
  | reference (inner : TokenPattern)
  | mutableReference (inner : TokenPattern)
  deriving Repr

mutual
/-- Token `t` of module `m` matches the pattern. A struct token matches when
its handle names the pattern's type, by module id and name, and is not
generic; a struct instantiation token when its handle names the pattern's
type, with one type parameter per type argument, and its type arguments match
the pattern's. -/
def TokenPattern.matches (m : CompiledModule) : TokenPattern → SignatureToken → Bool
  | .bool, .bool | .u8, .u8 | .u64, .u64 | .address, .address | .signer, .signer => true
  | .vector p, .vector t => p.matches m t
  | .typeParameter i, .typeParameter j => i == j
  | .struct module name, .struct h =>
    (m.structHandles[h]?).any (·.typeParameters.isEmpty) &&
      m.structIdAt h == some { module := ⟨stdAddress, module⟩, name }
  | .structInstantiation module name ps, .structInstantiation h ts =>
    (m.structHandles[h]?).any (·.typeParameters.length == ps.length) &&
      m.structIdAt h == some { module := ⟨stdAddress, module⟩, name } &&
      TokenPattern.matchAll m ps ts
  | .reference p, .reference t => p.matches m t
  | .mutableReference p, .mutableReference t => p.matches m t
  | _, _ => false

/-- The tokens `ts` of module `m` match the patterns `ps`, one for one. -/
def TokenPattern.matchAll (m : CompiledModule) : List TokenPattern → List SignatureToken → Bool
  | [], [] => true
  | p :: ps, t :: ts => p.matches m t && TokenPattern.matchAll m ps ts
  | [], _ :: _ => false
  | _ :: _, [] => false
end

/-- A modeled native: `std::module::name`, its declared type parameters'
constraints, parameter and return types, and its shape. -/
structure ModeledNative where
  module : String
  name : String
  typeParameters : List AbilitySet
  params : List TokenPattern
  returns : List TokenPattern
  shape : NativeShape
  deriving Repr

open TokenPattern in
/-- **The modeled natives** (module docstring), with their declarations as
`move-stdlib/sources` and `aptos-stdlib/sources` write them. `T` is the single
type parameter of a generic native of `std` (`Element`, `T` or `MoveValue` in
the sources), without constraints. A table native has the type parameters `K:
copy + drop`, `V` and `B`, and takes a `Table<K, V>` and gives a `Box<V>`, the
structs of `0x1::table`. -/
def modeledNatives : List ModeledNative :=
  let T := typeParameter 0
  let generic : List AbilitySet := [{}]
  let K := typeParameter 0
  let V := typeParameter 1
  let box : List AbilitySet := [{ copy := true, drop := true }, {}, {}]
  let table := structInstantiation "table" "Table" [K, V]
  let entry := structInstantiation "table" "Box" [V]
  [ -- Update shape.
    ⟨"vector", "push_back", generic, [mutableReference (.vector T), T], [], .update⟩,
    ⟨"vector", "pop_back", generic, [mutableReference (.vector T)], [T], .update⟩,
    ⟨"vector", "swap", generic, [mutableReference (.vector T), .u64, .u64], [], .update⟩,
    ⟨"vector", "move_range", generic,
      [mutableReference (.vector T), .u64, .u64, mutableReference (.vector T), .u64], [], .update⟩,
    ⟨"mem", "swap", generic, [mutableReference T, mutableReference T], [], .update⟩,
    -- Borrow shape.
    ⟨"vector", "borrow", generic, [reference (.vector T), .u64], [reference T], .borrow⟩,
    ⟨"vector", "borrow_mut", generic, [mutableReference (.vector T), .u64], [mutableReference T],
      .borrow⟩,
    ⟨"signer", "borrow_address", [], [reference .signer], [reference .address], .borrow⟩,
    -- Read shape.
    ⟨"vector", "length", generic, [reference (.vector T)], [.u64], .read⟩,
    ⟨"string", "internal_check_utf8", [], [reference (.vector .u8)], [.bool], .read⟩,
    ⟨"string", "internal_is_char_boundary", [], [reference (.vector .u8), .u64], [.bool], .read⟩,
    ⟨"string", "internal_sub_string", [], [reference (.vector .u8), .u64, .u64], [.vector .u8],
      .read⟩,
    ⟨"string", "internal_index_of", [], [reference (.vector .u8), reference (.vector .u8)], [.u64],
      .read⟩,
    ⟨"bcs", "to_bytes", generic, [reference T], [.vector .u8], .read⟩,
    ⟨"bcs", "serialized_size", generic, [reference T], [.u64], .read⟩,
    ⟨"cmp", "compare", generic, [reference T, reference T], [.struct "cmp" "Ordering"], .read⟩,
    -- The table natives (2026-11-04), with `K`, `V` and `B` the type parameters
    -- 0, 1 and 2, `K` constrained by `copy + drop`.
    ⟨"table", "add_box", box, [mutableReference table, K, entry], [], .update⟩,
    ⟨"table", "remove_box", box, [mutableReference table, K], [entry], .update⟩,
    ⟨"table", "borrow_box", box, [reference table, K], [reference entry], .borrow⟩,
    ⟨"table", "borrow_box_mut", box, [mutableReference table, K], [mutableReference entry],
      .borrow⟩,
    ⟨"table", "contains_box", box, [reference table, K], [.bool], .read⟩,
    ⟨"table", "destroy_empty_box", box, [reference table], [], .read⟩ ]

/-- The condition on module `m`, the module declaring it, under which entry `e`
is modeled: for a table native, that `m` keeps the table rules (language-owner
ruling 2, 2026-11-04, `CompiledModule.followsTableRules`); none for the
others. -/
def ModeledNative.admittedIn (e : ModeledNative) (m : CompiledModule) : Bool :=
  e.module != "table" || m.followsTableRules

/-- The shape of function definition `d` of module `m`, if it is a modeled
native: it has no code, `m` is `std::module` and `d` is named `name` for an
entry of `modeledNatives`, `d`'s handle declares the entry's type parameters,
with their constraints, and parameter and return types matching the entry's,
and `m` meets the entry's condition (`ModeledNative.admittedIn`). -/
def CompiledModule.modeledNative? (m : CompiledModule) (d : FunctionDefinition) :
    Option NativeShape := do
  if d.code.isSome then none else
  let id ← m.selfId
  let name ← m.functionDefName d
  let fh ← m.functionDefHandle d
  let params ← m.signatures[fh.parameters]?
  let returns ← m.signatures[fh.returns]?
  let entry ← modeledNatives.find? fun e =>
    id == ⟨stdAddress, e.module⟩ && name == e.name && fh.typeParameters == e.typeParameters &&
      TokenPattern.matchAll m e.params params && TokenPattern.matchAll m e.returns returns
  if !entry.admittedIn m then none else
  pure entry.shape

/-- The shape of function `f` of the program, if it is a modeled native
(`CompiledModule.modeledNative?`), whatever its type arguments. -/
def Program.nativeShape? (P : Program) (f : FunctionRef) : Option NativeShape := do
  let info ← P.function? f
  info.module.modeledNative? info.definition

end MoveBytecode

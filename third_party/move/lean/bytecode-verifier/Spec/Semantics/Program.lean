-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Values

/-!
# Linked programs, name resolution, and typing

TRUSTED. A program is a set of modules linked by name. A function handle in
one module names a function by module id (address and name) and function
name; a struct handle names a struct or enum type by module id and type name.
Resolution finds the definition in the program. Resolution failure is a
linking failure, which the semantics reports as `wrong`; property K1 states
that it cannot happen in a well-formed program.

Struct types are declared in the program, so the things that depend on
declarations are defined here:

- the translation of signature tokens into runtime types, which needs the
  module whose struct handles a token indexes, and the closed type arguments
  that the type parameters in scope stand for;
- abilities, which for a struct type are its declared abilities, granted at
  an instance only as its type arguments allow;
- which closed types are types of the program (`RuntimeType.wellFormed`):
  every generic type has one type argument per type parameter, satisfying its
  constraints;
- conformance of a value to a type, which checks struct and enum values
  against their declared fields at the value's instance.

**Generics** (plan Phase 4, stage A). A generic function runs as an
*instance*: a `FunctionRef` carries closed type arguments, and the function's
signature tokens are translated under them (`RuntimeType.ofTokenWith`):
"`fun id<T>(x: T): T`", "once defined, the type parameter `T` can be used in
parameter types, return types, and inside the function body", and calling it
supplies "the type arguments for the function's type parameters" (book,
`generics-and-abilities.md`, "Declaring Type Parameters", "Type Arguments").
A struct type is a definition's name with closed type arguments (`StructId`),
and its field types are the declaration's tokens translated under them. A
`FunctionRef` or `StructId` without type arguments is the non-generic entity
of Phases 1–3, so every non-generic program means what it meant before.

**Function values** (plan Phase 4, stage B). A function type token translates
to a function type whose parameters and results are slot types
(`RuntimeType.ofTokenWith`). Its abilities are those it carries. A closure's
abilities are those of its target, `copy` and `drop`, and `store` if the
target is storable, `public` or `persistent` (`Program.storable`; being `entry`
does not count), intersected with those of the types
of the captured values (`Program.closureValueAbilities`). A value conforms to a
type it is assignable to (`RuntimeType.assignable`): a closure with more
abilities conforms to a function type with fewer.

**Tables** (language-owner rulings, 2026-11-04). A table's entries are inside
it (`Value.table`): they count for its abilities and its conformance. The
table rules on the module `0x1::table` (`CompiledModule.followsTableRules`)
are here, with the abilities of type tokens they use
(`SignatureToken.abilitiesIn`).

Status: plan Phase 4, stage B. Reference types are the types of locals,
parameters and return values, and of the parameters and results of function
types, only (`RuntimeType.ofSlotToken`).
-/

namespace MoveBytecode

namespace CompiledModule

/-- The module id named by a module handle of this module. -/
def moduleIdOf (m : CompiledModule) (mh : ModuleHandle) : Option ModuleId := do
  let address ← m.addressIdentifiers[mh.address]?
  let name ← m.identifiers[mh.name]?
  pure ⟨address, name⟩

/-- The module id named by a module handle index of this module. -/
def moduleIdAt (m : CompiledModule) (h : ModuleHandleIndex) : Option ModuleId := do
  m.moduleIdOf (← m.moduleHandles[h]?)

/-- This module's own id. -/
def selfId (m : CompiledModule) : Option ModuleId :=
  m.moduleIdAt m.selfModuleHandleIdx

/-- The name of a function definition of this module. -/
def functionDefName (m : CompiledModule) (d : FunctionDefinition) : Option String := do
  let h ← m.functionHandles[d.function]?
  m.identifiers[h.name]?

/-- The handle of a function definition of this module. -/
def functionDefHandle (m : CompiledModule) (d : FunctionDefinition) : Option FunctionHandle :=
  m.functionHandles[d.function]?

/-- The handle of a struct definition of this module. It carries the type's
name, declared abilities and type parameters (`file_format.rs`,
`StructDefinition`: "This has the name and the abilities for the type"). -/
def structDefHandle (m : CompiledModule) (d : StructDefinition) : Option StructHandle :=
  m.structHandles[d.structHandle]?

/-- The name of a struct definition of this module. -/
def structDefName (m : CompiledModule) (d : StructDefinition) : Option String := do
  m.identifiers[(← m.structDefHandle d).name]?

/-- The type a struct handle of this module names, without type arguments: the
module id of the handle's module and the handle's name. A generic type's
instances add their type arguments (`RuntimeType.ofTokenWith`). -/
def structIdAt (m : CompiledModule) (h : StructHandleIndex) : Option StructId := do
  let sh ← m.structHandles[h]?
  pure { module := ← m.moduleIdAt sh.module, name := ← m.identifiers[sh.name]? }

end CompiledModule

/-! ## From signature tokens to runtime types -/

mutual
/-- Translate a signature token of module `m` into a runtime *value* type: the
type of a field, a vector element, a type argument, a constant, or the target
of a reference. Type parameter `i` stands for the closed type `tys[i]`: the
type arguments of the function instance or struct instance whose tokens these
are. A struct token names the type of its struct handle (`structIdAt`),
whether the type is defined in `m` or imported; it is not resolved here. A
struct instantiation names that type at its translated type arguments, one
per type parameter of the handle ("instantiations point to a generic handle
and its instantiation", `file_format.rs`); a generic handle named without type
arguments, or with too few or too many, is an arity error and not a type. A
reference is not a value type ("references to references are not allowed";
references "cannot be stored as a field value of structs", book,
`references.md`), so a reference token is `none` here, at any depth: reference
types are translated only at the top of a slot type (`ofSlotTokenWith`), and
at the parameters and results of a function type, which are slot types
(`ofSlotTokensWith`): "the actual parameters of lambdas can be references"
(book, `functions.md`). A function type keeps the abilities it carries. -/
def RuntimeType.ofTokenWith (m : CompiledModule) (tys : List RuntimeType) :
    SignatureToken → Option RuntimeType
  | .bool => some .bool
  | .u8 => some (.int .u8)
  | .u16 => some (.int .u16)
  | .u32 => some (.int .u32)
  | .u64 => some (.int .u64)
  | .u128 => some (.int .u128)
  | .u256 => some (.int .u256)
  | .i8 => some (.int .i8)
  | .i16 => some (.int .i16)
  | .i32 => some (.int .i32)
  | .i64 => some (.int .i64)
  | .i128 => some (.int .i128)
  | .i256 => some (.int .i256)
  | .address => some .address
  | .signer => some .signer
  | .vector elem => (RuntimeType.ofTokenWith m tys elem).map .vector
  | .struct h => do
    let sh ← m.structHandles[h]?
    if sh.typeParameters.isEmpty then pure (.struct (← m.structIdAt h)) else none
  | .structInstantiation h args => do
    let sh ← m.structHandles[h]?
    let sid ← m.structIdAt h
    let targs ← RuntimeType.ofTokenListWith m tys args
    if targs.length = sh.typeParameters.length then pure (.struct { sid with typeArgs := targs })
    else none
  | .typeParameter i => tys[i]?
  | .function ps rs abilities => do
    pure (.function (← RuntimeType.ofSlotTokensWith m tys ps) (← RuntimeType.ofSlotTokensWith m tys rs)
      abilities)
  | .reference _ => none
  | .mutableReference _ => none

/-- Translate a list of value tokens of module `m` under `tys`
(`ofTokenWith`). -/
def RuntimeType.ofTokenListWith (m : CompiledModule) (tys : List RuntimeType) :
    List SignatureToken → Option (List RuntimeType)
  | [] => some []
  | t :: ts => do
    pure ((← RuntimeType.ofTokenWith m tys t) :: (← RuntimeType.ofTokenListWith m tys ts))

/-- Translate a list of slot tokens of module `m` under `tys`: each a value type
or a reference to one, as `ofSlotTokenWith` translates one. -/
def RuntimeType.ofSlotTokensWith (m : CompiledModule) (tys : List RuntimeType) :
    List SignatureToken → Option (List RuntimeType)
  | [] => some []
  | .reference t :: ts => do
    pure (.ref false (← RuntimeType.ofTokenWith m tys t) :: (← RuntimeType.ofSlotTokensWith m tys ts))
  | .mutableReference t :: ts => do
    pure (.ref true (← RuntimeType.ofTokenWith m tys t) :: (← RuntimeType.ofSlotTokensWith m tys ts))
  | t :: ts => do
    pure ((← RuntimeType.ofTokenWith m tys t) :: (← RuntimeType.ofSlotTokensWith m tys ts))
end

/-- Translate a closed signature token of module `m`, one in which no type
parameter is in scope: a constant's type, or a token of a non-generic entity
(`ofTokenWith` with no type arguments). -/
abbrev RuntimeType.ofToken (m : CompiledModule) : SignatureToken → Option RuntimeType :=
  RuntimeType.ofTokenWith m []

/-- Translate a slot token of module `m` under `tys`: the type of a parameter,
a return value or a local, which may be a reference to a value type (`&T`
"where `T` is a non-reference type", book, `references.md`, "Reference
Operators"; L5), or a value type. -/
def RuntimeType.ofSlotTokenWith (m : CompiledModule) (tys : List RuntimeType) :
    SignatureToken → Option RuntimeType
  | .reference t => (RuntimeType.ofTokenWith m tys t).map (.ref false)
  | .mutableReference t => (RuntimeType.ofTokenWith m tys t).map (.ref true)
  | t => RuntimeType.ofTokenWith m tys t

/-- Translate a closed slot token of module `m` (`ofSlotTokenWith` with no type
arguments). -/
abbrev RuntimeType.ofSlotToken (m : CompiledModule) : SignatureToken → Option RuntimeType :=
  RuntimeType.ofSlotTokenWith m []

/-- Translate a signature of parameter, return or local types of module `m`
under `tys` (`ofSlotTokenWith`); `none` if any token is outside the
subset. -/
def RuntimeType.ofSignatureWith (m : CompiledModule) (tys : List RuntimeType) (sig : Signature) :
    Option (List RuntimeType) :=
  sig.mapM (RuntimeType.ofSlotTokenWith m tys)

/-- Translate a closed signature of module `m` (`ofSignatureWith` with no type
arguments). -/
abbrev RuntimeType.ofSignature (m : CompiledModule) (sig : Signature) : Option (List RuntimeType) :=
  RuntimeType.ofSignatureWith m [] sig

/-- The element type named by the signature operand of a vector instruction
of module `m`, under the type arguments `tys` of the running instance: a
single type (`elem_ty_idx` in `file_format.rs`). -/
def CompiledModule.vectorElemTypeWith (m : CompiledModule) (tys : List RuntimeType)
    (s : SignatureIndex) : Option RuntimeType :=
  match m.signatures[s]? with
  | some [t] => RuntimeType.ofTokenWith m tys t
  | some _ => none
  | none => none

/-- The element type named by a vector instruction of a non-generic function
(`vectorElemTypeWith` with no type arguments). -/
abbrev CompiledModule.vectorElemType (m : CompiledModule) (s : SignatureIndex) :
    Option RuntimeType :=
  m.vectorElemTypeWith [] s

/-- The type arguments that instantiation signature `s` of module `m` supplies,
translated under the type arguments `tys` of the running instance. An
instantiation "can be partial": its tokens may mention the enclosing
function's type parameters, "`S<T, W>`, `S<u8, bool>`, `S<T, u8>`,
`S<X<T>, address>`" (`file_format.rs`, "Instantiations"). -/
def CompiledModule.typeArgsWith (m : CompiledModule) (tys : List RuntimeType)
    (s : SignatureIndex) : Option (List RuntimeType) := do
  RuntimeType.ofTokenListWith m tys (← m.signatures[s]?)

/-! ## Programs and resolution -/

structure Program where
  modules : Array CompiledModule
  deriving Inhabited

/-- The pieces of a resolved function. -/
structure FunctionInfo where
  module : CompiledModule
  moduleId : ModuleId
  definition : FunctionDefinition
  handle : FunctionHandle

/-- A struct or enum definition in a program: a module index and a definition
index within that module. -/
structure StructRef where
  module : Nat
  index : StructDefinitionIndex
  deriving DecidableEq, Repr, Inhabited

/-- The pieces of a resolved struct or enum definition. `handle` is the
definition's own handle in its module, which declares its abilities and type
parameters. -/
structure StructInfo where
  module : CompiledModule
  moduleId : ModuleId
  definition : StructDefinition
  handle : StructHandle

namespace Program

/-- The index of the element satisfying `p`, if exactly one does. Resolution is
defensive: an ambiguous name resolves to nothing, which the semantics reports
as `wrong`, rather than to an arbitrary match. -/
def uniqueIdx {α : Type} (a : Array α) (p : α → Bool) : Option Nat :=
  match (List.range a.size).filter (fun i => match a[i]? with
    | some x => p x
    | none => false) with
  | [i] => some i
  | _ => none

/-- The module with id `id`, if exactly one module of the program has it. -/
def findModule (P : Program) (id : ModuleId) : Option Nat :=
  uniqueIdx P.modules (fun m => m.selfId == some id)

/-- The definition of function instance `f`, whatever its type arguments. -/
def function? (P : Program) (f : FunctionRef) : Option FunctionInfo := do
  let m ← P.modules[f.module]?
  let id ← m.selfId
  let d ← m.functionDefs[f.index]?
  let h ← m.functionDefHandle d
  pure ⟨m, id, d, h⟩

/-- Resolve function handle `h` of module `caller` to a definition, named
without type arguments: a generic call adds those it supplies. -/
def resolve (P : Program) (caller : Nat) (h : FunctionHandleIndex) : Option FunctionRef := do
  let m ← P.modules[caller]?
  let fh ← m.functionHandles[h]?
  let targetId ← m.moduleIdAt fh.module
  let name ← m.identifiers[fh.name]?
  let mi ← P.findModule targetId
  let target ← P.modules[mi]?
  let di ← uniqueIdx target.functionDefs (fun d => target.functionDefName d == some name)
  pure { module := mi, index := di }

def struct? (P : Program) (r : StructRef) : Option StructInfo := do
  let m ← P.modules[r.module]?
  let id ← m.selfId
  let d ← m.structDefs[r.index]?
  let h ← m.structDefHandle d
  pure ⟨m, id, d, h⟩

/-- Resolve a struct type's name to its definition: the unique module with the
type's module id, and in it the unique definition with the type's name. Like
function resolution, it is defensive about ambiguity. The type arguments play
no part: every instance of a generic type has the one definition. -/
def resolveStruct (P : Program) (s : StructId) : Option StructRef := do
  let mi ← P.findModule s.module
  let m ← P.modules[mi]?
  let di ← uniqueIdx m.structDefs (fun d => m.structDefName d == some s.name)
  pure ⟨mi, di⟩

/-- The declaration of the closed type `s`: its resolved definition, which
must have one type parameter per type argument of `s` ("type parameters",
"type arguments": a generic type is used at an instance, book,
`generics-and-abilities.md`, "Using Generic Structs"), and whose type
arguments are value types (`RuntimeType.isValue`). A generic definition named
without type arguments, or with too few or too many, is not a type. A
reference is never a type argument: "references to references are not
allowed", references "cannot be stored as a field value of structs" (book,
`references.md`), and a type parameter may stand for a field's type, so a
struct type with a reference among its type arguments is not a type either,
and no value has it. Every struct type the semantics builds has value type
arguments: they are translated from value tokens (`RuntimeType.ofTokenWith`)
or checked (`RuntimeType.argumentsWellFormed`). A non-generic type has none.
Constraints are not checked here (`RuntimeType.wellFormed`). -/
def structType? (P : Program) (s : StructId) : Option StructInfo := do
  let info ← P.struct? (← P.resolveStruct s)
  if s.typeArgs.length = info.handle.typeParameters.length ∧ s.typeArgs.all (·.isValue) then
    some info
  else none

/-- The type that definition `d` of module `mi` defines, named without type
arguments, provided its name resolves back to that definition. Code creates
and destroys values of the types its module defines (`Pack`, `Unpack`, and the
variant forms name a definition of the current module); the round trip makes
the value's type name denote the definition the code used, even in a program
whose names are ambiguous. A generic form adds the instance's type arguments
(`Step.localStruct`). -/
def definedStruct? (P : Program) (mi : Nat) (d : StructDefinitionIndex) : Option StructId := do
  let info ← P.struct? ⟨mi, d⟩
  let name ← info.module.structDefName info.definition
  let s : StructId := { module := info.moduleId, name }
  if P.resolveStruct s = some ⟨mi, d⟩ then some s else none

/-- The field types of struct type `s`, in declaration order: `none` unless
`s` is a struct with declared fields (not an enum, not a native struct) at an
instance with one type argument per type parameter, whose field types are all
in the subset. Field tokens are translated in the defining module, under the
instance's type arguments: "`struct Foo<T> has copy, drop { x: T }`" has a
field of type `bool` at `Foo<bool>` (book, `generics-and-abilities.md`,
"Generic Structs", "Using Generic Structs"). -/
def fieldTypes (P : Program) (s : StructId) : Option (List RuntimeType) := do
  let info ← P.structType? s
  match info.definition.fieldInformation with
  | .declared fields => fields.mapM (fun f => RuntimeType.ofTokenWith info.module s.typeArgs f.signature)
  | .declaredVariants _ => none
  | .native => none

/-- The field types of variant `tag` of enum type `s`, in declaration order,
at `s`'s instance: `none` unless `s` is an enum with a variant `tag` whose
field types are all in the subset. "Similar to structs, enum types can be
generic" (book, `structs-and-enums.md`). -/
def variantFieldTypes (P : Program) (s : StructId) (tag : VariantIndex) :
    Option (List RuntimeType) := do
  let info ← P.structType? s
  match info.definition.fieldInformation with
  | .declaredVariants variants =>
    (← variants[tag]?).fields.mapM
      (fun f => RuntimeType.ofTokenWith info.module s.typeArgs f.signature)
  | .declared _ => none
  | .native => none

/-- The parameter types of function instance `f`: its handle's parameter
tokens under `f`'s type arguments. -/
def paramTypes (P : Program) (f : FunctionRef) : Option (List RuntimeType) := do
  let info ← P.function? f
  RuntimeType.ofSignatureWith info.module f.typeArgs (← info.module.signatures[info.handle.parameters]?)

/-- The return types of function instance `f`. -/
def returnTypes (P : Program) (f : FunctionRef) : Option (List RuntimeType) := do
  let info ← P.function? f
  RuntimeType.ofSignatureWith info.module f.typeArgs (← info.module.signatures[info.handle.returns]?)

/-- Function `f` is storable: a closure over it may be stored. "A _persistent_
function is required to build a storable function value" (book,
`functions.md`, "Denoting Function Values"), and a function is persistent
exactly when it is `public` or carries the `persistent` attribute
(`#[persistent]` in source). Being `entry` does not by itself make a function
persistent: the language owners ruled so on 2026-11-03, which supersedes the
book's "while `public` and `entry` functions are persistent by default". -/
def storable (P : Program) (f : FunctionRef) : Bool :=
  match P.function? f with
  | some info =>
    info.definition.visibility == .public_ || info.handle.attributes.contains .persistent
  | none => false

/-- The abilities a closure gets from its target: "a function handle has `drop`
and `copy`, never has `key`, and only `store` if the underlying function" is
persistent (`file_format.rs`, `PackClosure`; book, `functions.md`, "Denoting
Function Values": "with abilities `copy+drop`", and `store` for a persistent
function). -/
def closureAbilities (P : Program) (f : FunctionRef) : AbilitySet :=
  ⟨true, true, P.storable f, false⟩

end Program

/-! ## Abilities -/

/-- The abilities a container may have when it holds a value with abilities
`a`. Declaring a struct with "`copy`, all fields must have `copy`; `drop`, all
fields must have `drop`; `store`, all fields must have `store`; `key`, all
fields must have `store`" (book, `generics-and-abilities.md`, "Annotating
Structs"). The same rule gives a vector's abilities from its element's
("Conditional Abilities and Generic Types"). -/
def AbilitySet.forContainer (a : AbilitySet) : AbilitySet :=
  ⟨a.copy, a.drop, a.store, a.store⟩

mutual
/-- The abilities of a runtime type (book, `generics-and-abilities.md`):

- booleans, integers and addresses have `copy`, `drop` and `store`;
- `signer` "has `drop`" only: it "cannot be copied and cannot be put into
  global storage";
- `vector<T>` behaves as if declared `has copy, drop, store`, each granted
  only when `T` has what the field rule requires; it never has `key`;
- a struct or enum type has the abilities its definition declares, not
  those a handle importing it declares, which K1 allows to be fewer. At an
  instance, each is granted only when every type argument has what the field
  rule requires ("type parameters are pessimistically assumed to be used
  inside the struct, so the abilities are only granted if the type parameters
  meet the requirements described above for fields", "Conditional Abilities
  and Generic Types"), except the arguments of phantom parameters: "the
  arguments to phantom parameters are excluded when deriving the struct
  abilities" ("Phantom Type Parameters", "Instantiation"). A type that does
  not resolve to a definition with its number of type parameters has none;
- a function type has the abilities it carries ("Function types can have
  associated abilities", book, `functions.md`, "Function Types");
- "immutable references `&` and mutable references `&mut` both have `copy`
  and `drop`. This refers to copying and dropping the reference itself, not
  what they refer to. References cannot appear in global storage, hence they
  do not have `store`." -/
def RuntimeType.abilities (P : Program) : RuntimeType → AbilitySet
  | .bool => AbilitySet.primitives
  | .int _ => AbilitySet.primitives
  | .address => AbilitySet.primitives
  | .signer => { drop := true }
  | .vector elem => AbilitySet.primitives.inter (elem.abilities P).forContainer
  | .struct s =>
    match P.structType? s with
    | some info =>
      (RuntimeType.argumentAbilities P s.typeArgs info.handle.typeParameters).inter
        info.handle.abilities
    | none => AbilitySet.empty
  | .function _ _ abilities => abilities
  | .ref _ _ => { copy := true, drop := true }

/-- What type arguments `args` permit an instance to have: the field rule
applied to each argument whose parameter in `params` is not phantom. -/
def RuntimeType.argumentAbilities (P : Program) :
    List RuntimeType → List StructTypeParameter → AbilitySet
  | arg :: args, p :: ps =>
    (if p.isPhantom then AbilitySet.all else (arg.abilities P).forContainer).inter
      (RuntimeType.argumentAbilities P args ps)
  | [], _ => AbilitySet.all
  | _ :: _, [] => AbilitySet.all
end

/-! ## Types of the program -/

mutual
/-- A closed runtime type is a type of program `P`: every struct type in it,
type arguments included, resolves to a definition with one type parameter per
type argument (`Program.structType?`), and every type argument is a value type
of `P` with the abilities its parameter's constraints require. "Constraints
are checked at call sites": "`struct Foo<T: key> { x: T }`", and "`Foo<u8>`"
is an "error! u8 does not have 'key'" (book, `generics-and-abilities.md`,
"Verifying Constraints"); a phantom parameter's argument too: "the type
argument has to satisfy that constraint, even though the parameter is
phantom" ("Phantom Type Parameters with Ability Constraints"). Vector elements
and the targets of references are value types of `P`. A function type's
parameters and results are slot types of `P`, and it has no `key`: "a function
handle has `drop` and `copy`, never has `key`" (`file_format.rs`,
`PackClosure`), so no function value has `key` (L5). -/
def RuntimeType.wellFormed (P : Program) : RuntimeType → Bool
  | .bool => true
  | .int _ => true
  | .address => true
  | .signer => true
  | .vector elem => elem.isValue && elem.wellFormed P
  | .struct s =>
    match P.structType? s with
    | some info =>
      RuntimeType.argumentsWellFormed P s.typeArgs (info.handle.typeParameters.map (·.constraints))
    | none => false
  | .function ps rs abilities =>
    !abilities.key && RuntimeType.slotsWellFormed P ps && RuntimeType.slotsWellFormed P rs
  | .ref _ target => target.isValue && target.wellFormed P

/-- Slot types `ts` of the program: each a value type of `P`, or a reference to
one. -/
def RuntimeType.slotsWellFormed (P : Program) : List RuntimeType → Bool
  | [] => true
  | .ref _ target :: ts => (target.isValue && target.wellFormed P) && RuntimeType.slotsWellFormed P ts
  | t :: ts => (t.isValue && t.wellFormed P) && RuntimeType.slotsWellFormed P ts

/-- Type arguments `args` instantiate type parameters with constraints
`constraints`: one argument per parameter, each a value type of the program
with every ability its parameter's constraint requires. A constraint is a
set of abilities the argument must have ("`T: copy + drop`", book,
`generics-and-abilities.md`, "Declaring Constraints"). -/
def RuntimeType.argumentsWellFormed (P : Program) :
    List RuntimeType → List AbilitySet → Bool
  | [], [] => true
  | arg :: args, c :: cs =>
    (arg.isValue && arg.wellFormed P && c.subset (arg.abilities P)) &&
      RuntimeType.argumentsWellFormed P args cs
  | [], _ :: _ => false
  | _ :: _, [] => false
end

/-- The type arguments of function instance `f` satisfy its definition's type
parameters (`RuntimeType.argumentsWellFormed`): "constraints are checked at
call sites", "`consume<R>(r)`" with "`fun consume<T: drop>`" is an "error! R
does not have 'drop'" (book, `generics-and-abilities.md`, "Verifying
Constraints"). A non-generic function takes none. -/
def Program.typeArgsOk (P : Program) (f : FunctionRef) : Bool :=
  match P.function? f with
  | some info => RuntimeType.argumentsWellFormed P f.typeArgs info.handle.typeParameters
  | none => false

/-- The abilities that captured values of types `tys` permit a closure: the
intersection of theirs ("take the intersection of the abilities of all
captured arguments", `file_format.rs`, `PackClosure`). -/
def RuntimeType.capturedAbilities (P : Program) : List RuntimeType → AbilitySet
  | [] => AbilitySet.all
  | t :: ts => (t.abilities P).inter (RuntimeType.capturedAbilities P ts)

/-- The abilities of a closure over function instance `f` that captured the
values `captured`: the target's (`Program.closureAbilities`) intersected with
those of the captured values' types. "The function underlying a closure is a
private function, so the function itself is `copy+drop` (and not `store`).
This is intersected with the abilities of all the captured context
variables" (book, `functions.md`, "Lambda Expressions and Closures"); "take
the intersection of the abilities of all captured arguments"
(`file_format.rs`, `PackClosure`). It is the captured values' types, not the
captured parameters' declared ones, that count: a captured value may have
more abilities than its parameter's type, a closure assignable to a function
type with fewer (`RuntimeType.assignable`), and what the closure may do is
what its contents allow, as for every container (`Value.abilities`). -/
def Program.closureValueAbilities (P : Program) (f : FunctionRef) (captured : List Value) :
    AbilitySet :=
  (P.closureAbilities f).inter (RuntimeType.capturedAbilities P (captured.map Value.type))

/-- The types of a closure over function instance `f` capturing the parameters
`mask` selects: the types of the captured parameters, the remaining
parameters, and the results. "If the function `fun` has type `|t1..tn|r`,
then ... the number of bits set in the mask [is at most `n`], ... type ti [of
a captured parameter] is not a reference. ... The type of the resulting value
on the stack is derived from the types `|t1..tn|` for which the bit is not
set, which build the arguments of a function type with `fun`'s result types"
(`file_format.rs`, `PackClosure`). Its abilities depend on the captured values
(`Program.closureValueAbilities`). `none` if `f` does not resolve to a
function with code at type arguments satisfying its constraints, if `mask`
names a parameter `f` does not have, or if it captures a reference. A closure
over a native function is outside the subset: its call would run a native
with arguments some of which come from the closure, which the native
contracts do not describe. -/
def Program.closureType? (P : Program) (f : FunctionRef) (mask : BitVec 64) :
    Option (List RuntimeType × List RuntimeType × List RuntimeType) := do
  let info ← P.function? f
  let _ ← info.definition.code
  if !P.typeArgsOk f then none else
  let params ← P.paramTypes f
  let results ← P.returnTypes f
  if !maskWithin mask params.length then none else
  let (captured, remaining) := maskSplit mask params
  if !captured.all RuntimeType.isValue then none else
  pure (captured, remaining, results)

/-! ## Abilities of type tokens -/

mutual
/-- The abilities of type token `t` of module `m`, where type parameter `i`
has abilities `tparams[i]` (book, `generics-and-abilities.md`, "Builtin
Types" and "Conditional Abilities and Generic Types"):

- primitive types have `copy`, `drop` and `store`; `signer` has `drop`;
- `vector<T>` is as if declared `has copy, drop, store`, each granted when `T`
  has what the field rule requires (`AbilitySet.forContainer`);
- a function type has the abilities it carries;
- references have `copy` and `drop`;
- a struct type has the abilities this module's handle declares, each
  granted only when every type argument has what the field rule requires,
  where "the arguments to phantom parameters are excluded when deriving the
  struct abilities". For an imported type these are at most the definition's
  (K1);
- a type parameter has the abilities `tparams` gives it, none if out of
  scope. -/
def SignatureToken.abilitiesIn (m : CompiledModule) (tparams : List AbilitySet) :
    SignatureToken → AbilitySet
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => AbilitySet.primitives
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => AbilitySet.primitives
  | .address => AbilitySet.primitives
  | .signer => { drop := true }
  | .vector elem => AbilitySet.primitives.inter (elem.abilitiesIn m tparams).forContainer
  | .function _ _ abilities => abilities
  | .struct h =>
    match m.structHandles[h]? with
    | some sh => sh.abilities
    | none => AbilitySet.empty
  | .structInstantiation h args =>
    match m.structHandles[h]? with
    | some sh => sh.abilities.inter (SignatureToken.argumentAbilities m tparams args sh.typeParameters)
    | none => AbilitySet.empty
  | .reference _ | .mutableReference _ => ⟨true, true, false, false⟩
  | .typeParameter i => tparams.getD i AbilitySet.empty

/-- What type arguments `args` permit an instance to have: the field rule
applied to each argument whose parameter in `params` is not phantom. -/
def SignatureToken.argumentAbilities (m : CompiledModule) (tparams : List AbilitySet) :
    List SignatureToken → List StructTypeParameter → AbilitySet
  | arg :: args, p :: ps =>
    (if p.isPhantom then AbilitySet.all else (arg.abilitiesIn m tparams).forContainer).inter
      (SignatureToken.argumentAbilities m tparams args ps)
  | [], _ => AbilitySet.all
  | _ :: _, [] => AbilitySet.all
end

/-! ## Tables

Language-owner rulings of 2026-11-04 (`Spec.Semantics.Values`, "Tables"): a
table holds its entries (`Value.table`), and the natives of `aptos_std::table`
that read, change and borrow them are modeled natives
(`Spec.Semantics.ModeledNatives`), but only in a program whose module
`0x1::table` keeps the table rules, checked on that module's bytecode
(`CompiledModule.followsTableRules`, ruling 2). `table.move` represents table
items as "separate global state items" tracked by "a unique handle"; modeling
them inside the table value agrees with that as long as no two tables share a
handle, which the rules are there to ensure. With the framework's own
`table.move` they always hold; they exist for programs that ship a different
`0x1::table`. Their intent (language owners, 2026-11-04): the model agrees with
the implementation for every program they admit. -/

/-- What an instruction does that the table rules constrain: a direct call, a
`Pack`, an `Unpack` or a mutable field borrow, non-generic or generic, or a
jump, conditional or not, to `target`. -/
inductive TableRuleUse where
  | call (h : FunctionHandleIndex)
  | callGeneric (i : FunctionInstantiationIndex)
  | pack (d : StructDefinitionIndex)
  | packGeneric (i : StructDefInstantiationIndex)
  | unpack (d : StructDefinitionIndex)
  | unpackGeneric (i : StructDefInstantiationIndex)
  | mutBorrowField (h : FieldHandleIndex)
  | mutBorrowFieldGeneric (i : FieldInstantiationIndex)
  | jump (target : CodeOffset)
  | other
  deriving DecidableEq, Repr

def Bytecode.tableRuleUse : Bytecode → TableRuleUse
  | .call h => .call h
  | .callGeneric i => .callGeneric i
  | .pack d => .pack d
  | .packGeneric i => .packGeneric i
  | .unpack d => .unpack d
  | .unpackGeneric i => .unpackGeneric i
  | .mutBorrowField h => .mutBorrowField h
  | .mutBorrowFieldGeneric i => .mutBorrowFieldGeneric i
  | .brTrue o | .brFalse o | .branch o => .jump o
  | .pop | .ret | .readRef | .writeRef | .freezeRef | .abort | .abortMsg | .nop => .other
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => .other
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => .other
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => .other
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => .other
  | .ldConst _ | .ldTrue | .ldFalse => .other
  | .copyLoc _ | .moveLoc _ | .stLoc _ | .mutBorrowLoc _ | .immBorrowLoc _ => .other
  | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => .other
  | .packVariant _ | .packVariantGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => .other
  | .testVariant _ | .testVariantGeneric _ => .other
  | .mutBorrowVariantField _ | .mutBorrowVariantFieldGeneric _ => .other
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => .other
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => .other
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => .other
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => .other
  | .eq | .neq | .lt | .gt | .le | .ge | .shl | .shr | .negate => .other
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => .other

/-- The natives of `table.move` that take the type of a table's entries as
their type parameter `B` ("Primitives which take as an additional type
parameter `Box<V>`, so the implementation can use this to determine
serialization layout"): the box natives. -/
def tableBoxNatives : List String :=
  ["add_box", "borrow_box", "borrow_box_mut", "contains_box", "remove_box", "destroy_empty_box",
   "drop_unchecked_box"]

/-- **The table rules** (language-owner ruling 2, 2026-11-04), on the bytecode
of module `m`, a program's module `0x1::table`. Every function of `m` with
code

1. creates a `Table` only by packing the result of a call to its native
   `new_table_handle`, as `table::new` does: every `PackGeneric` of `Table`
   directly follows a `Call` or `CallGeneric` of `m`'s own `new_table_handle`,
   a native of `m`, at an offset no jump targets, so that what it packs is
   that call's result; and no non-generic `Pack` names `Table`;
2. never unpacks a `Table` (`Unpack`, `UnpackGeneric`);
3. packs a `Table` only at a value type argument `V` that has `store`, as
   `new<K: copy + drop, V: store>` requires: the second type argument of each
   `PackGeneric` of `Table` has `store` under the constraints of the
   function's type parameters (`SignatureToken.abilitiesIn`);
4. calls each box native (`tableBoxNatives`) with `B = Box<V>`, as every call
   in `table.move` does: every call of one of `m`'s own box natives is a
   `CallGeneric` whose third type argument is `m`'s `Box` at its second;

and, refining ruling 2 under its stated intent, that the model agree with the
implementation for every program the rules admit (language owners,
2026-11-04):

5. `m` declares `Table` as `table.move` does: it defines a struct named
   `Table`, and every such definition declares exactly the ability `store` and
   a single field `handle: address`;
6. no function of `m` borrows a field of a `Table` mutably (`MutBorrowField`,
   `MutBorrowFieldGeneric`);
7. `new_table_handle` and the box natives (`tableBoxNatives`) are private
   (neither public nor friend), as `table.move` declares them, so that only
   `m`'s own code calls them, under rules 1 and 4.

Rules 5 and 6 keep a table from being copied and its handle from being
rewritten, either of which would let two tables share a handle; with rule 5 a
table's type has neither `copy`, `drop` nor `key`. Rule 7 makes rules 1 and 4,
checked on `m`'s code, cover every call of these natives.

The six table natives with references are modeled only in a program whose
`0x1::table` keeps these rules (`ModeledNative.admittedIn`), and only there may
a table hold entries (`Program.tablesHoldEntries`). -/
def CompiledModule.followsTableRules (m : CompiledModule) : Bool :=
  let own (name : String) : Option StructId := m.selfId.map fun id => { module := id, name }
  let isTable (d : StructDefinitionIndex) : Bool :=
    ((m.structDefs[d]?).bind m.structDefName) == some "Table"
  let ownFunction (h : FunctionHandleIndex) (names : List String) : Bool :=
    match m.functionHandles[h]? with
    | some fh => m.moduleIdAt fh.module == m.selfId && (m.identifiers[fh.name]?).any names.contains
    | none => false
  let newHandleNative : Bool := m.functionDefs.toList.any fun d =>
    d.code.isNone && m.functionDefName d == some "new_table_handle"
  let callsNewHandle (b : Bytecode) : Bool :=
    newHandleNative &&
      match b.tableRuleUse with
      | .call h => ownFunction h ["new_table_handle"]
      | .callGeneric i =>
        (m.functionInstantiations[i]?).any fun x => ownFunction x.handle ["new_table_handle"]
      | .pack _ | .packGeneric _ | .unpack _ | .unpackGeneric _ | .mutBorrowField _
      | .mutBorrowFieldGeneric _ | .jump _ | .other => false
  let tableDefs := m.structDefs.toList.filter (m.structDefName · == some "Table")
  let declaredAsTableMove : Bool :=
    !tableDefs.isEmpty && tableDefs.all fun sd =>
      (m.structDefHandle sd).any (·.abilities == { store := true }) &&
        match sd.fieldInformation with
        | .declared [f] => m.identifiers[f.name]? == some "handle" && f.signature == .address
        | .declared _ | .declaredVariants _ | .native => false
  let borrowsTable (h : FieldHandleIndex) : Bool := (m.fieldHandles[h]?).any (isTable ·.owner)
  let nativesPrivate : Bool := m.functionDefs.toList.all fun d =>
    d.visibility == .private_ ||
      (m.functionDefName d).all fun name =>
        name != "new_table_handle" && !tableBoxNatives.contains name
  let boxAtV (sig : SignatureIndex) : Bool :=
    match m.signatures[sig]? with
    | some [_, v, .structInstantiation h [v']] => v' == v && m.structIdAt h == own "Box"
    | _ => false
  declaredAsTableMove && nativesPrivate && m.functionDefs.toList.all fun d =>
    match d.code, m.functionDefHandle d with
    | none, _ => true
    | some _, none => false
    | some code, some fh =>
      let instrs := code.code.toList
      let targeted (i : Nat) : Bool := instrs.any (·.tableRuleUse == .jump i)
      instrs.zipIdx.all fun (b, i) =>
        match b.tableRuleUse with
        | .pack d => !isTable d
        | .packGeneric k =>
          (m.structDefInstantiations[k]?).all fun x =>
            !isTable x.def_ ||
              (0 < i && (instrs[i - 1]?).any callsNewHandle && !targeted i &&
                match m.signatures[x.typeParameters]? with
                | some [_, v] => (v.abilitiesIn m fh.typeParameters).store
                | _ => false)
        | .unpack d => !isTable d
        | .unpackGeneric k => (m.structDefInstantiations[k]?).all fun x => !isTable x.def_
        | .mutBorrowField h => !borrowsTable h
        | .mutBorrowFieldGeneric k => (m.fieldInstantiations[k]?).all fun x => !borrowsTable x.handle
        | .call h => !ownFunction h tableBoxNatives
        | .callGeneric k =>
          (m.functionInstantiations[k]?).all fun x =>
            !ownFunction x.handle tableBoxNatives || boxAtV x.typeParameters
        | .jump _ | .other => true

/-- Tables of program `P` may hold entries: its module `0x1::table` keeps the
table rules (`CompiledModule.followsTableRules`). Only the table natives add
entries, and they are modeled only then (`ModeledNative.admittedIn`). In a
program whose table module breaks the rules no table has any
(`Value.wellFormed`): such a module may unpack a table, which would lose the
entries with it. -/
def Program.tablesHoldEntries (P : Program) : Bool :=
  match P.findModule tableModuleId with
  | some mi => (P.modules[mi]?).any (·.followsTableRules)
  | none => false

mutual
/-- The abilities a value actually has: those of its type, restricted by the
field rule applied to every value it contains. "If a value has `copy`, all
values contained inside of that value have `copy`", and likewise for `drop`
and `store` (book, "The Four Abilities"), so copying or dropping a value
copies or drops everything inside it. A table's entries are inside it, and
count for the abilities it has (language-owner ruling 1, 2026-11-04); its
keys do not: they are values of a type with `copy` and `drop`, which the
table's declaration (`Table<phantom K: copy + drop, phantom V>`) does not ask
to have `store`. The semantics checks this deep
ability, not only the type's declared one: a struct declared `has copy`
around a field without `copy` breaks L6, and copying it would copy that
field. Under L6 in every module and K1 the two agree on conforming values:
L6 justifies a definition's declared abilities by its fields' types as the
defining module's handles see them, and K1 makes those at most the abilities
the fields' types have. -/
def Value.abilities (P : Program) : Value → AbilitySet
  | .bool _ => RuntimeType.bool.abilities P
  | .int t _ => (RuntimeType.int t).abilities P
  | .address _ => RuntimeType.address.abilities P
  | .signer _ => RuntimeType.signer.abilities P
  | .vector elem elems =>
    ((RuntimeType.vector elem).abilities P).inter (Value.contentAbilities P elems)
  | .struct s fields => ((RuntimeType.struct s).abilities P).inter (Value.contentAbilities P fields)
  | .variant s _ fields =>
    ((RuntimeType.struct s).abilities P).inter (Value.contentAbilities P fields)
  | .ref r => (RuntimeType.ref r.mutable r.target).abilities P
  | .closure _ _ captured ps rs a =>
    ((RuntimeType.function ps rs a).abilities P).inter (Value.contentAbilities P captured)
  | .table s fields _ entries =>
    (((RuntimeType.struct s).abilities P).inter (Value.contentAbilities P fields)).inter
      (Value.contentAbilities P entries)

/-- What contained values `vs` permit their container to have: the field rule
(`AbilitySet.forContainer`) applied to each. -/
def Value.contentAbilities (P : Program) : List Value → AbilitySet
  | [] => AbilitySet.all
  | v :: vs => (v.abilities P).forContainer.inter (Value.contentAbilities P vs)
end

/-! ## Conformance -/

mutual
/-- A value is well formed when it inhabits its own intrinsic type in program
`P`, all the way down: an integer is in its type's range; every element of a
vector conforms to the vector's element type; a struct value's fields conform,
in number and in order, to its type's declared fields at its instance; an
enum value's tag names a variant of its type and its fields conform to that
variant's. A value of a type that does not resolve, or is not in the subset,
is not well formed. A reference is well formed when its target is a value
type; whether its location is valid depends on the state, and is checked
where the reference is used (`State.deref?`). A closure is well formed when it
records the remaining parameters and the results of a closure over its
function instance with its mask (`Program.closureType?`), its captured values
conform to the captured parameters' types, and it records the abilities its
target and its captured values give it (`Program.closureValueAbilities`).
A value of a table type is a table, never a plain struct value (`Value.table`).

A table is well formed when its fields conform to its type's declared fields,
it has one key per entry, and, if it has entries (language-owner rulings,
2026-11-04):

- its program's table module keeps the table rules
  (`Program.tablesHoldEntries`), the only programs where entries are added;
- its keys conform to `K` and are distinct ("a finite map from keys (values of
  `K`)"), and its entries conform to `Box<V>` (`tableEntryType`);
- its type has no `copy`: modeling the entries inside the table "agrees with
  [`table.move`] as long as no two `Table` values share a handle" (ruling 1),
  and a copy would share it;
- the entries' type, as a container's content, has every ability the table's
  type has (`AbilitySet.forContainer`): entries never take away an ability of
  the table's type, whose parameter `V` is phantom.

Under the table rules `Table` declares exactly `store` (rule 5), so the last
two reduce to the entries' type having `store`, which rule 3 gives every table
the module creates ("packs a `Table` only at a value type argument `V` that
has `store`"). They are stated on the value all the same, so that a table with
entries keeps the abilities of its type whatever the module declares: D1 does
not rest on the rules.

Whether a struct value's type arguments satisfy their constraints is not
checked here: the semantics checks it where a generic entity is instantiated
(the generic instructions and the entry's type arguments,
`RuntimeType.wellFormed`). That the other types a frame names, its locals and
vector element types, are then types of the program too is L5's type
formation under those type arguments; an abilities check never depends on it,
since abilities are computed from the declaration and the type arguments
whatever the constraints. -/
def Value.wellFormed (P : Program) : Value → Bool
  | .bool _ => true
  | .int t x => t.inRange x
  | .address _ => true
  | .signer _ => true
  | .vector elem elems => Value.allConform P elems elem
  | .struct s fields =>
    !s.isTable &&
      match P.fieldTypes s with
      | some tys => conformsAll P fields tys
      | none => false
  | .variant s tag fields =>
    match P.variantFieldTypes s tag with
    | some tys => conformsAll P fields tys
    | none => false
  | .ref r => r.target.isValue
  | .closure f mask captured ps rs a =>
    match P.closureType? f mask with
    | some (capturedTys, remaining, results) =>
      (ps == remaining && rs == results && a == P.closureValueAbilities f captured) &&
        conformsAll P captured capturedTys
    | none => false
  | .table s fields keys entries =>
    s.isTable && keys.length == entries.length &&
      (match P.fieldTypes s with
       | some tys => conformsAll P fields tys
       | none => false) &&
      (entries.isEmpty || P.tablesHoldEntries &&
        match s.typeArgs with
        | [k, v] =>
          Value.allConform P keys k && Value.allConform P entries (tableEntryType v) &&
            decide keys.Nodup && !((RuntimeType.struct s).abilities P).copy &&
            ((RuntimeType.struct s).abilities P).subset
              ((tableEntryType v).abilities P).forContainer
        | _ => false)

/-- Every value of `vs` conforms to `t`. -/
def Value.allConform (P : Program) : List Value → RuntimeType → Bool
  | [], _ => true
  | v :: vs, t => (v.type.assignable t && v.wellFormed P) && Value.allConform P vs t

/-- Pointwise conformance of a list of values to a list of types, of the same
length. -/
def conformsAll (P : Program) : List Value → List RuntimeType → Bool
  | [], [] => true
  | v :: vs, t :: ts => (v.type.assignable t && v.wellFormed P) && conformsAll P vs ts
  | [], _ :: _ => false
  | _ :: _, [] => false
end

/-- A value conforms to a declared type: its intrinsic type is assignable to
that type (`RuntimeType.assignable`), and it is well formed. Every type of the
subset is closed, and the only subtyping is that of function types: "a
function value with more abilities may be used where a function type with
fewer abilities is expected" (plan §5.2); "otherwise types must be equal". For
a type without function types, conformance is type equality plus the
structural invariants.

For references, plan §5.2 makes an immutable reference covariant in its
target. A mutable reference does not conform to an immutable reference type:
the book's
"`&mut T` as a subtype of `&T`" is a rule of source programs, where "the
compiler inserts `freeze` instructions where they are needed" (book,
`references.md`, "`freeze` inference"); bytecode has `FreezeRef` for it, and
`StLoc` and `Call` require the value's type to equal the declared one
(`file_format.rs`: "assert ty == val_ty", "assert ty == locals[...]"). -/
def Value.conforms (P : Program) (v : Value) (t : RuntimeType) : Bool :=
  v.type.assignable t && v.wellFormed P

namespace Program

/-- The types of the locals of function instance `f`: its parameters, then the
locals of its code unit, under `f`'s type arguments. `none` for a native
function. -/
def localTypes (P : Program) (f : FunctionRef) : Option (List RuntimeType) := do
  let info ← P.function? f
  let code ← info.definition.code
  let params ← RuntimeType.ofSignatureWith info.module f.typeArgs
    (← info.module.signatures[info.handle.parameters]?)
  let locals ← RuntimeType.ofSignatureWith info.module f.typeArgs
    (← info.module.signatures[code.locals]?)
  pure (params ++ locals)

/-- The parameter types a caller's function handle declares, at the type
arguments `tys` the call supplies. Struct types are named through the caller's
own struct handles, so comparing them with the callee's compares resolved
names. -/
def handleParamTypesAt (P : Program) (caller : Nat) (h : FunctionHandleIndex)
    (tys : List RuntimeType) : Option (List RuntimeType) := do
  let m ← P.modules[caller]?
  let fh ← m.functionHandles[h]?
  RuntimeType.ofSignatureWith m tys (← m.signatures[fh.parameters]?)

/-- The return types a caller's function handle declares, at the type
arguments `tys` the call supplies. -/
def handleReturnTypesAt (P : Program) (caller : Nat) (h : FunctionHandleIndex)
    (tys : List RuntimeType) : Option (List RuntimeType) := do
  let m ← P.modules[caller]?
  let fh ← m.functionHandles[h]?
  RuntimeType.ofSignatureWith m tys (← m.signatures[fh.returns]?)

/-- The parameter types a caller's handle of a non-generic function declares
(`handleParamTypesAt` with no type arguments). -/
abbrev handleParamTypes (P : Program) (caller : Nat) (h : FunctionHandleIndex) :
    Option (List RuntimeType) :=
  P.handleParamTypesAt caller h []

/-- The return types a caller's handle of a non-generic function declares. -/
abbrev handleReturnTypes (P : Program) (caller : Nat) (h : FunctionHandleIndex) :
    Option (List RuntimeType) :=
  P.handleReturnTypesAt caller h []

/-- Whether code of module `caller` may call function `f` directly or create a
closure over it: a private function only from its own module, a friend
function also from its declared friends, a public function from anywhere.
Visibility is the definition's, whatever the type arguments. -/
def visibleFrom (P : Program) (caller : ModuleId) (f : FunctionRef) : Bool :=
  match P.function? f with
  | none => false
  | some info =>
    match info.definition.visibility with
    | .public_ => true
    | .private_ => info.moduleId == caller
    | .friend =>
      info.moduleId == caller ||
        info.module.friendDecls.any (fun mh => info.module.moduleIdOf mh == some caller)

end Program

end MoveBytecode

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Structure

/-!
# Type formation (L5), struct definitions (L6), friend declarations (L10)

TRUSTED. Properties of one module's text, established by local verification
alone (plan §7, group L). They mention no execution.

- L5 `TypeFormation`: every type the module writes down is formed by the
  language's recursive rules. References appear only at the top level of
  parameter, return and local types, and at the parameter and result
  positions of function types, at any depth of function types; never as
  vector elements, type arguments or field types except inside a function
  type's signature. Type constructors get one argument per type parameter,
  each with the abilities that parameter's constraints require; type
  parameters are in scope; function types carry no `key`.
- L6 `StructDefinitionsWellFormed`: declared abilities are justified by the
  fields; phantom type parameters appear only in phantom positions;
  containment among the module's own struct definitions is acyclic; every
  variant field borrow names a field its listed variants share at the
  instance it borrows.
- L10 `FriendsWellFormed`: a friend is at the module's address and is not the
  module itself.

Status: L5 and L6 are stated in general, generics and function types
included, because that is what local verification establishes; the semantics
of Phase 4 uses them for generic code (type parameters, instantiations and
their constraints, stage A) and function types (slot-typed parameters and
results, abilities without `key`, stage B).

Abilities of tokens are computed from the module's own struct handles
(`SignatureToken.abilitiesIn`). The semantics uses the definitions'. K1
(`StructHandle.ConservativeFor`) lets an imported handle declare fewer
abilities than its definition, treat fewer type parameters as phantom, and
require more constraints; it does not make the two equal (triage 2026-10-02,
see `Spec.Properties.Linking`). Nothing here assumes they are equal: every
rule below requires abilities to be present and never absent, so a token's
abilities computed with a module's handles are at most those computed with
the definitions, and L5 and L6 established with a module's handles hold with
the definitions' too. In particular, with L6 in every module and K1, the
abilities a value actually has (`Value.abilities`) are those of its type.
-/

namespace MoveBytecode

/-! ## Abilities of type tokens

`SignatureToken.abilitiesIn`, the abilities of a module's type tokens, is in
`Spec.Semantics.Program`: the table rules check one there
(`CompiledModule.followsTableRules`). -/

/-! ## Type formation (L5) -/

mutual
/-- `t` is a well-formed value type of module `m`, where the type parameters
in scope have abilities `tparams`: a type that may be a vector element, a type
argument, a field, a constant, or a parameter, return or local type that is
not a reference. -/
def SignatureToken.ValueTypeWellFormed (m : CompiledModule) (tparams : List AbilitySet) :
    SignatureToken → Prop
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => True
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => True
  | .address | .signer => True
  | .vector elem => elem.ValueTypeWellFormed m tparams
  | .function params results abilities =>
    SignatureToken.SlotsWellFormed m tparams params ∧
      SignatureToken.SlotsWellFormed m tparams results ∧ abilities.key = false
  | .struct h => ∃ sh, m.structHandles[h]? = some sh ∧ sh.typeParameters = []
  | .structInstantiation h args =>
    ∃ sh, m.structHandles[h]? = some sh ∧
      SignatureToken.ArgumentsWellFormed m tparams args (sh.typeParameters.map (·.constraints))
  | .reference _ | .mutableReference _ => False
  | .typeParameter i => i < tparams.length

/-- Every token of `ts` is well formed at a position that admits a reference:
a top-level parameter, return or local type, or a parameter or result of a
function type. There a token is a value type, or a reference to one. -/
def SignatureToken.SlotsWellFormed (m : CompiledModule) (tparams : List AbilitySet) :
    List SignatureToken → Prop
  | [] => True
  | .reference t :: ts =>
    t.ValueTypeWellFormed m tparams ∧ SignatureToken.SlotsWellFormed m tparams ts
  | .mutableReference t :: ts =>
    t.ValueTypeWellFormed m tparams ∧ SignatureToken.SlotsWellFormed m tparams ts
  | t :: ts => t.ValueTypeWellFormed m tparams ∧ SignatureToken.SlotsWellFormed m tparams ts

/-- Type arguments `args` instantiate type parameters with constraints
`constraints`: one argument per parameter, each a well-formed value type with
the abilities its parameter's constraints require ("the type argument has to
satisfy that constraint, even though the parameter is phantom", book,
"Phantom Type Parameters with Ability Constraints"). -/
def SignatureToken.ArgumentsWellFormed (m : CompiledModule) (tparams : List AbilitySet) :
    List SignatureToken → List AbilitySet → Prop
  | [], [] => True
  | arg :: args, c :: cs =>
    arg.ValueTypeWellFormed m tparams ∧ c.subset (arg.abilitiesIn m tparams) = true ∧
      SignatureToken.ArgumentsWellFormed m tparams args cs
  | [], _ :: _ => False
  | _ :: _, [] => False
end

namespace CompiledModule

/-- The constraints of the type parameters of struct definition `d`. -/
def structConstraints (m : CompiledModule) (d : StructDefinitionIndex) :
    Option (List AbilitySet) := do
  let sd ← m.structDefs[d]?
  pure ((← m.structDefHandle sd).typeParameters.map (·.constraints))

/-- Signature `s` is a list of well-formed value types. -/
def ValueSignatureWellFormed (m : CompiledModule) (tparams : List AbilitySet)
    (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ ∀ t ∈ sig, t.ValueTypeWellFormed m tparams

/-- Signature `s` is well formed as a list of parameter, return or local
types. -/
def SlotSignatureWellFormed (m : CompiledModule) (tparams : List AbilitySet)
    (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ SignatureToken.SlotsWellFormed m tparams sig

/-- Signature `s` instantiates an entity whose type parameters have
constraints `constraints`. -/
def InstantiationWellFormed (m : CompiledModule) (tparams : List AbilitySet)
    (s : SignatureIndex) (constraints : List AbilitySet) : Prop :=
  ∃ args, m.signatures[s]? = some args ∧ SignatureToken.ArgumentsWellFormed m tparams args constraints

end CompiledModule

/-- The type operands of an instruction of module `m` are well formed, in a
function whose type parameters have constraints `tparams`: the type arguments
of a generic form instantiate the named entity, and the element type of a
vector instruction and the function type of `CallClosure` are value types. -/
def Bytecode.TypeOperandsWellFormed (m : CompiledModule) (tparams : List AbilitySet) :
    Bytecode → Prop
  -- Generic functions.
  | .callGeneric i | .packClosureGeneric i _ =>
    ∃ x fh, m.functionInstantiations[i]? = some x ∧ m.functionHandles[x.handle]? = some fh ∧
      m.InstantiationWellFormed tparams x.typeParameters fh.typeParameters
  -- Generic structs and enums.
  | .packGeneric i | .unpackGeneric i | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i
  | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i =>
    ∃ x cs, m.structDefInstantiations[i]? = some x ∧ m.structConstraints x.def_ = some cs ∧
      m.InstantiationWellFormed tparams x.typeParameters cs
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    ∃ x vh cs, m.structVariantInstantiations[i]? = some x ∧
      m.structVariantHandles[x.handle]? = some vh ∧ m.structConstraints vh.structIndex = some cs ∧
      m.InstantiationWellFormed tparams x.typeParameters cs
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i =>
    ∃ x fdh cs, m.fieldInstantiations[i]? = some x ∧ m.fieldHandles[x.handle]? = some fdh ∧
      m.structConstraints fdh.owner = some cs ∧
      m.InstantiationWellFormed tparams x.typeParameters cs
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    ∃ x vfh cs, m.variantFieldInstantiations[i]? = some x ∧
      m.variantFieldHandles[x.handle]? = some vfh ∧ m.structConstraints vfh.structIndex = some cs ∧
      m.InstantiationWellFormed tparams x.typeParameters cs
  -- Element types and closure types.
  | .vecPack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s | .vecPushBack s
  | .vecPopBack s | .vecUnpack s _ | .vecSwap s | .callClosure s =>
    m.ValueSignatureWellFormed tparams s
  -- No type operand.
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => True
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => True
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => True
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => True
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => True
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => True
  | .packClosure _ _ => True
  | .pack _ | .unpack _ | .packVariant _ | .unpackVariant _ | .testVariant _ => True
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => True
  | .mutBorrowField _ | .immBorrowField _ | .mutBorrowVariantField _
  | .immBorrowVariantField _ => True
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .exists_ _ | .moveFrom _ | .moveTo _ => True
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => True
  | .eq | .neq | .lt | .gt | .le | .ge | .shl | .shr | .negate => True
  | .abort | .abortMsg | .nop => True

/-- **L5.** Every type the module writes down is well formed: the parameter
and return types of function handles and the local types of code, which may
be references; field types and constant types, which may not; and the type
operands of instructions. Type parameters in scope are a function's, with
their constraints, or a struct's, with theirs; a constant's type is closed.
Script signatures are outside modules and not covered. -/
def CompiledModule.TypeFormation (m : CompiledModule) : Prop :=
  (∀ fh ∈ m.functionHandles.toList,
    m.SlotSignatureWellFormed fh.typeParameters fh.parameters ∧
    m.SlotSignatureWellFormed fh.typeParameters fh.returns) ∧
  (∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
    ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
      f.signature.ValueTypeWellFormed m (sh.typeParameters.map (·.constraints))) ∧
  (∀ c ∈ m.constantPool.toList, c.type.ValueTypeWellFormed m []) ∧
  (∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      m.SlotSignatureWellFormed fh.typeParameters code.locals ∧
      ∀ i ∈ code.code.toList, i.TypeOperandsWellFormed m fh.typeParameters)

/-! ## Struct definitions (L6) -/

mutual
/-- No phantom type parameter of `params` occurs in `t` outside a phantom
position. "If a type is used as an argument to a phantom type parameter we
say the type appears in phantom position", and "a phantom type parameter can
only appear in phantom position" (book, "Phantom Type Parameters"). -/
def SignatureToken.PhantomRespected (m : CompiledModule) (params : List StructTypeParameter) :
    SignatureToken → Prop
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => True
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => True
  | .address | .signer | .struct _ => True
  | .vector elem => elem.PhantomRespected m params
  | .function ps rs _ =>
    SignatureToken.PhantomRespectedAll m params ps ∧ SignatureToken.PhantomRespectedAll m params rs
  | .structInstantiation h args =>
    ∃ sh, m.structHandles[h]? = some sh ∧
      SignatureToken.PhantomRespectedArgs m params args sh.typeParameters
  | .reference t | .mutableReference t => t.PhantomRespected m params
  | .typeParameter i => ∃ p, params[i]? = some p ∧ p.isPhantom = false

def SignatureToken.PhantomRespectedAll (m : CompiledModule) (params : List StructTypeParameter) :
    List SignatureToken → Prop
  | [] => True
  | t :: ts => t.PhantomRespected m params ∧ SignatureToken.PhantomRespectedAll m params ts

/-- An argument for a phantom parameter of the instantiated struct is in
phantom position and unconstrained; any other argument is not. -/
def SignatureToken.PhantomRespectedArgs (m : CompiledModule) (params : List StructTypeParameter) :
    List SignatureToken → List StructTypeParameter → Prop
  | [], _ => True
  | arg :: args, q :: qs =>
    (q.isPhantom = true ∨ arg.PhantomRespected m params) ∧
      SignatureToken.PhantomRespectedArgs m params args qs
  | arg :: args, [] =>
    arg.PhantomRespected m params ∧ SignatureToken.PhantomRespectedArgs m params args []
end

mutual
/-- The struct handles whose values a value of type `t` may contain: those
`t` names, through vector elements and type arguments (type parameters are
"pessimistically assumed to be used inside the struct", book, "Conditional
Abilities and Generic Types"). Not through function types: a function value
holds no value of its parameter or result types. -/
def SignatureToken.containedHandles : SignatureToken → List StructHandleIndex
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => []
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => []
  | .address | .signer | .typeParameter _ => []
  | .vector elem => elem.containedHandles
  | .function .. => []
  | .struct h => [h]
  | .structInstantiation h args => h :: SignatureToken.containedHandlesList args
  | .reference t | .mutableReference t => t.containedHandles

def SignatureToken.containedHandlesList : List SignatureToken → List StructHandleIndex
  | [] => []
  | t :: ts => t.containedHandles ++ SignatureToken.containedHandlesList ts
end

mutual
/-- Token `t` at the instance whose type arguments are `args`: each type
parameter `i` replaced by `args[i]`; `none` if `t` names a type parameter
that `args` does not cover. An instantiation gives "the instantiation of the
owner type", "e.g. for `S<u8, bool>.f` where `f` is a field of any type"
(`file_format.rs`, `FieldInstantiation`), and the field's type at that
instance is its declared type with the arguments in place of the
parameters. The arguments of a "partial" instantiation mention the enclosing
function's type parameters, which are left as they are. -/
def SignatureToken.instantiate (args : List SignatureToken) :
    SignatureToken → Option SignatureToken
  | t@.bool | t@.u8 | t@.u16 | t@.u32 | t@.u64 | t@.u128 | t@.u256 => some t
  | t@.i8 | t@.i16 | t@.i32 | t@.i64 | t@.i128 | t@.i256 => some t
  | t@.address | t@.signer | t@(.struct _) => some t
  | .vector elem => return .vector (← elem.instantiate args)
  | .function ps rs abilities =>
    return .function (← SignatureToken.instantiateList args ps)
      (← SignatureToken.instantiateList args rs) abilities
  | .structInstantiation h targs =>
    return .structInstantiation h (← SignatureToken.instantiateList args targs)
  | .reference t => return .reference (← t.instantiate args)
  | .mutableReference t => return .mutableReference (← t.instantiate args)
  | .typeParameter i => args[i]?

def SignatureToken.instantiateList (args : List SignatureToken) :
    List SignatureToken → Option (List SignatureToken)
  | [] => some []
  | t :: ts => return (← t.instantiate args) :: (← SignatureToken.instantiateList args ts)
end

namespace CompiledModule

/-- Struct definition `d₁` of the module has a field, in any variant, whose
type may contain a value of definition `d₂`'s type. -/
def FieldContains (m : CompiledModule) (d₁ d₂ : StructDefinitionIndex) : Prop :=
  ∃ sd₁ sd₂, m.structDefs[d₁]? = some sd₁ ∧ m.structDefs[d₂]? = some sd₂ ∧
    ∃ f ∈ StructFieldInformation.allFields sd₁.fieldInformation,
      sd₂.structHandle ∈ f.signature.containedHandles

/-- Containment among the module's own struct and enum definitions is acyclic:
"Structs cannot be recursive", and "generic structs cannot contain fields of
the same type, either directly or indirectly, even with different type
arguments" (book, `structs-and-enums.md`; `generics-and-abilities.md`,
"Recursive Structs"). Across modules this is K3, which needs acyclic module
dependencies (plan §4.3). -/
def LocalStructsAcyclic (m : CompiledModule) : Prop :=
  ∀ d, ¬ Plus m.FieldContains d d

/-- The variants that variant field handle `h` lists all have, at its
offset, a field of one type at the instance whose type arguments are
`args`: they are "the sequence of variants which share the field at the
given field offset" (`file_format.rs`, `VariantFieldHandle`) at that
instance. -/
def VariantFieldSharedAt (m : CompiledModule) (h : VariantFieldHandleIndex)
    (args : List SignatureToken) : Prop :=
  ∃ vfh vs t, m.variantFieldHandles[h]? = some vfh ∧ m.variants vfh.structIndex = some vs ∧
    ∀ v ∈ vfh.variants, ∃ vd fd, vs[v]? = some vd ∧ vd.fields[vfh.field]? = some fd ∧
      fd.signature.instantiate args = some t

/-- Every variant field borrow of the module, reachable or not, names a field
that the variants it lists share at the instance it borrows. A borrow
consumes a reference to an enum value and, "provided that the struct is of
the given variant", loads a reference to "the field of the variant", whose
type is the one type `field_ty` (`file_format.rs`, `ImmBorrowVariantField`
and its mutable and generic forms). Which listed variant the value has is
known only at run time, so every listed variant must give the field that
type. The non-generic form names an enum without type parameters (L9), whose
field types are closed and need no substitution. The generic form borrows at
the instance its instantiation gives; the field types are compared after
substituting its type arguments, with the enclosing function's type
parameters left as they are, so that tokens equal here stay equal under
every instantiation of the function.

Triage (2026-10-02, plan §10.1): this property first required every variant
field handle in the table to give its listed variants' fields one type
token. A handle has no meaning apart from the borrows that name it, directly
or through an instantiation, and a generic enum's handle is used only at the
instances those borrows give. A module the language admits falsified the
stronger statement without any ill-typed borrow: its handle's variants
declare the field with different tokens, a type parameter of the enum in one
and a closed type in another, which agree once the borrow's type arguments
are substituted. The stronger statement also constrained handles that no
instruction uses. It was a specification defect and is corrected here. That
each listed variant exists and has a field at the handle's offset is L1, for
every handle. -/
def VariantFieldsShared (m : CompiledModule) : Prop :=
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    (∀ h, (Bytecode.immBorrowVariantField h ∈ code.code.toList ∨
        Bytecode.mutBorrowVariantField h ∈ code.code.toList) →
      m.VariantFieldSharedAt h []) ∧
    (∀ i, (Bytecode.immBorrowVariantFieldGeneric i ∈ code.code.toList ∨
        Bytecode.mutBorrowVariantFieldGeneric i ∈ code.code.toList) →
      ∃ x args, m.variantFieldInstantiations[i]? = some x ∧
        m.signatures[x.typeParameters]? = some args ∧ m.VariantFieldSharedAt x.handle args)

/-- **L6.** Local struct facts.

- Every declared ability is justified by every field of every variant. A
  struct declared with "`copy`, all fields must have `copy`; `drop`, all fields
  must have `drop`; `store`, all fields must have `store`; `key`, all fields
  must have `store`" (book, "Annotating Structs"). The struct's own type
  parameters count as having every ability: an instance only has an ability
  when its type arguments have what the field rule requires ("Conditional
  Abilities and Generic Types").
- Phantom type parameters appear only in phantom positions.
- Containment among the module's definitions is acyclic.
- Every variant field borrow names a field its listed variants share at the
  instance it borrows. -/
def StructDefinitionsWellFormed (m : CompiledModule) : Prop :=
  (∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
    ∀ f ∈ StructFieldInformation.allFields d.fieldInformation,
      sh.abilities.subset
          (f.signature.abilitiesIn m (sh.typeParameters.map fun _ => AbilitySet.all)).forContainer
        = true ∧
      f.signature.PhantomRespected m sh.typeParameters) ∧
  m.LocalStructsAcyclic ∧
  m.VariantFieldsShared

/-! ## Friend declarations (L10) -/

/-- **L10.** Every friend declaration names a module at this module's address
other than this module: "A module cannot declare itself as a friend", and
"Friend modules must be within the same account address" (book,
`modules-and-packages.md`, "Friend declaration rules"). That the friend exists
is K5; that a friend list has no duplicates is L2. -/
def FriendsWellFormed (m : CompiledModule) : Prop :=
  ∀ mh ∈ m.friendDecls.toList, ∃ self friend, m.selfId = some self ∧
    m.moduleIdOf mh = some friend ∧ friend.address = self.address ∧ friend ≠ self

end CompiledModule

end MoveBytecode

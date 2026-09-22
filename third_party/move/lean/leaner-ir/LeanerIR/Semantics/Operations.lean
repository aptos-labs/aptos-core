-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Runtime
import LeanerIR.Validation.PlaceIndex

/-!
# Pure operations used by structured-LIR semantics

These total/partial functions give the deterministic meaning of M1 leaf
operations.  Both the relational semantics and executable interpreter use
them; control-flow evaluation itself is defined separately in each layer.
-/

namespace LeanerIR
namespace SemanticOperations

open Validation

/-- Apply the compact type-id substitution carried by a function frame.
The row is deliberately sparse: ids not affected by the invocation are
identity-mapped without an allocation or table lookup. -/
@[irreducible] def instantiatedTypeId (instantiation : Array (TypeId × TypeId))
    (typeId : TypeId) : TypeId :=
  (instantiation.find? fun entry => entry.1 == typeId).map (·.2) |>.getD typeId

@[simp] theorem instantiatedTypeId_empty (typeId : TypeId) :
    instantiatedTypeId #[] typeId = typeId := by
  simp only [instantiatedTypeId, Array.find?_empty, Option.map_none,
    Option.getD_none]

/-- Rewrite type arguments inherited from an outer generic invocation before
they are used to instantiate a direct callee. -/
def instantiateGenericArguments (outer : Array (TypeId × TypeId))
    (arguments : Array GenericArgument) : Array GenericArgument :=
  arguments.map fun argument => match argument with
    | .typeArg value => .typeArg
        { value with typeId := instantiatedTypeId outer value.typeId }
    | argument => argument

/-- Compute the sparse declaration-type substitution for one generic call.
The shared type arena already contains every structurally instantiated node;
the scan happens once at the call boundary, while storage operations perform
only a lookup in the resulting compact array. -/
def invocationTypeInstantiation (ns : ValidatedNamespace)
    (outer : Array (TypeId × TypeId))
    (arguments : Array GenericArgument) : Array (TypeId × TypeId) :=
  let arguments := instantiateGenericArguments outer arguments
  (Array.range ns.tables.types.size).foldl (init := #[]) fun result index =>
    let symbolic : TypeId := ⟨index⟩
    match Validation.instantiatePlaceFieldType? ns arguments symbolic with
    | some concrete =>
        if concrete == symbolic then result else result.push (symbolic, concrete)
    | none => result

/-- Resolve the target namespace and build its invocation substitution. A
validated direct-call handle always selects a namespace; the empty fallback
keeps this helper total for defensive consumers. Monomorphic callees do not
inherit the caller's substitution and need no scan of the type arena. -/
def callTypeInstantiation (unit : ValidatedUnit) (handle : FunctionHandle)
    (outer : Array (TypeId × TypeId))
    (arguments : Array GenericArgument) : Array (TypeId × TypeId) :=
  if arguments.isEmpty then #[] else
    match unit.namespaces[handle.namespaceId.index]? with
    | some ns => invocationTypeInstantiation ns outer arguments
    | none => #[]

def referencedName? (ns : ValidatedNamespace) (reference : QualifiedRef) :
    Option (NamespaceRef × String) := do
  let name ← ns.tables.names[reference.name.index]?
  if name.namespaceId != reference.namespaceId then none else
    let targetNamespace ← ns.tables.namespaces[reference.namespaceId.index]?
    return (targetNamespace, name.name)

def declaredName? (ns : ValidatedNamespace) (nameId : NameId) :
    Option (NamespaceRef × String) := do
  let name ← ns.tables.names[nameId.index]?
  let targetNamespace ← ns.tables.namespaces[name.namespaceId.index]?
  return (targetNamespace, name.name)

/-- Resolve a namespace-local qualified reference to a direct function handle.
Validation has already assigned the reference's canonical declaration identity;
semantic consumers use that index instead of rescanning declarations by name. -/
def resolveFunction? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) : Option FunctionHandle := do
  let source ← unit.namespaces[sourceNamespace.index]?
  let qualified ← source.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let functionId ← unit.resolution.function? reference.name
    let targetNamespace ← unit.namespaces[reference.namespaceId.index]?
    let _ ← targetNamespace.functions[functionId.index]?
    return { namespaceId := reference.namespaceId, functionId }

/-- Resolve a namespace-local qualified reference to a constant handle. -/
def resolveConstant? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) : Option ConstantHandle := do
  let source ← unit.namespaces[sourceNamespace.index]?
  let qualified ← source.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let constantId ← unit.resolution.constant? reference.name
    let targetNamespace ← unit.namespaces[reference.namespaceId.index]?
    let _ ← targetNamespace.constants[constantId.index]?
    return { namespaceId := reference.namespaceId, constantId := constantId.index }

/-- Resolve a namespace-local qualified reference to a nominal declaration. -/
def resolveStruct? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) : Option StructHandle := do
  let source ← unit.namespaces[sourceNamespace.index]?
  let qualified ← source.tables.names[reference.name.index]?
  if qualified.namespaceId != reference.namespaceId then none else
    let typeId ← unit.resolution.nominal? reference.name
    let targetNamespace ← unit.namespaces[reference.namespaceId.index]?
    let _ ← targetNamespace.structs[typeId.index]?
    return { namespaceId := reference.namespaceId, structId := typeId.index }

/-- Runtime identity of the nominal declaration an interned name denotes. -/
def resolveNominal? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (name : NameId) : Option StructHandle := do
  let qualified ← ns.tables.names[name.index]?
  let typeId ← unit.resolution.nominal? name
  let targetNamespace ← unit.namespaces[qualified.namespaceId.index]?
  let _ ← targetNamespace.structs[typeId.index]?
  return { namespaceId := qualified.namespaceId, structId := typeId.index }

/-- Runtime identity of the declaration a spelling names, located by search.
Interpreter entry points and tests construct input values with it; resolved
execution paths use `resolveStruct?`/`resolveNominal?` instead. -/
def findStructHandle? (unit : ValidatedUnit) (name : String) : Option StructHandle :=
  unit.namespaces.zipIdx.findSome? fun (ns, namespaceIndex) =>
    ns.structs.zipIdx.findSome? fun (declaration, structIndex) =>
      if (ns.tables.names[declaration.name.index]?).any (·.name == name) then
        some { namespaceId := ⟨namespaceIndex⟩, structId := structIndex }
      else none

/-- Resolved qualified spelling of a nominal declaration. -/
def structName? (unit : ValidatedUnit) (handle : StructHandle) : Option QualifiedName := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.structs[handle.structId]?
  let (_, name) ← declaredName? ns declaration.name
  let qualified ← ns.tables.names[declaration.name.index]?
  if qualified.name == name then some qualified else none

/-- Ordered fields selected by a structure or enum constructor. -/
def constructorFields? (unit : ValidatedUnit) (handle : StructHandle)
    (variant : Option String) : Option (Array FieldDecl) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.structs[handle.structId]?
  match variant with
  | none => if declaration.variants.isEmpty then some declaration.fields else none
  | some variantName =>
      let variantIndex ← declaration.variants.findIdx? fun candidate =>
        (ns.tables.names[candidate.name.index]?).any (fun name => name.name == variantName)
      return declaration.variants[variantIndex]!.fields

/-- Interpret an M1 constant literal as a runtime value. -/
def constValue? : ConstValue → Option RuntimeValue
  | .unit => some .unit
  | .bool value => some (.bool value)
  | .character value => if isUnicodeScalar value then some (.character value) else none
  | .integer value => some (.integer value)
  | .address value => some (.address value)
  | .string value => some (.string value)
  | .bytes value => some (.bytes value)
  | .vector elements => .vector <$> elements.attach.mapM fun ⟨element, _⟩ =>
      constValue? element
  | .tuple elements => .tuple <$> elements.attach.mapM fun ⟨element, _⟩ =>
      constValue? element
  | .profile _ => none

/-- Reify a closed runtime value for a pure profile operation. References,
nominal data, and closures are intentionally excluded from this boundary. -/
def RuntimeValue.toConst? : RuntimeValue → Option ConstValue
  | .unit => some .unit
  | .bool value => some (.bool value)
  | .character value => if isUnicodeScalar value then some (.character value) else none
  | .integer value => some (.integer value)
  | .address value => some (.address value)
  | .string value => some (.string value)
  | .bytes value => some (.bytes value)
  | .vector elements => .vector <$> elements.attach.mapM fun ⟨element, _⟩ =>
      RuntimeValue.toConst? element
  | .tuple elements => .tuple <$> elements.attach.mapM fun ⟨element, _⟩ =>
      RuntimeValue.toConst? element
  | .signer .. | .nominal .. | .closure .. | .borrow .. | .loanHole .. => none

/-- Construct a resolved nominal runtime value with exact constructor arity. -/
def constructNominal? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (fields : Array RuntimeValue) :
    Option RuntimeValue := do
  let handle ← resolveStruct? unit sourceNamespace reference
  let expected ← constructorFields? unit handle variant
  if expected.size != fields.size then none else
    return .nominal handle variant fields

/-- Destructure a matching resolved nominal value. -/
def destructNominal? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (value : RuntimeValue) :
    Option (Array RuntimeValue) := do
  let .nominal actualSource actualVariant fields := value | none
  let handle ← resolveStruct? unit sourceNamespace reference
  let expected ← constructorFields? unit handle variant
  if actualSource == handle && actualVariant == variant && fields.size == expected.size then
    some fields
  else none

/-- Apply the exact pure-operation implementation selected during semantic
preparation. `none` means that the prepared registry and interpreter disagree,
which is reported as an internal capability failure by the interpreter. -/
def evaluateProfileOperation? (executable : ExecutableUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (operation : ProfileValue) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let semantics ← semanticProfile? executable.semantics operation.profile
  let ty ← ns.tables.types[resultType.index]?
  let closed ← arguments.mapM RuntimeValue.toConst?
  match semantics.evaluatePure operation ty closed with
  | none => none
  | some (.value value) =>
      let runtimeValue ← constValue? value
      some (.ok runtimeValue)
  | some (.throw_ kind values) =>
      let runtimeValues ← values.mapM constValue?
      some (.error (kind, runtimeValues))

def checkedInteger (failure : ThrowKind) (resultType : Ty) (value : Int) :
    Except (ThrowKind × Array RuntimeValue) RuntimeValue :=
  match resultType.integerBounds? with
  | some (lower, upper) =>
      if lower <= value && value <= upper then .ok (.integer value)
      else .error (failure, #[.integer value])
  | none => .error (failure, #[])

def resolveTargetIntegerType? (targetPointerWidth : Option Nat) : Ty → Option Ty
  | .integer .pointer signed => do
      let width ← targetPointerWidth
      if supportedTargetPointerWidth width then some (.integer (.bits width) signed) else none
  | type => some type

def modularInteger (resultType : Ty) (value : Int) : Option RuntimeValue := do
  let .integer (.bits width) signed := resultType | none
  if width == 0 then none else
  let modulus : Int := (2 : Int) ^ width
  let residue := ((value % modulus) + modulus) % modulus
  let normalized := if signed && residue >= (2 : Int) ^ (width - 1) then
    residue - modulus
  else residue
  some (.integer normalized)

/-- Interpret an integer value as the unsigned bit pattern of the result width. -/
def integerBitPattern? (resultType : Ty) (value : Int) : Option Nat := do
  let .integer (.bits width) _ := resultType | none
  if width == 0 then none else
  let modulus : Int := (2 : Int) ^ width
  let residue := ((value % modulus) + modulus) % modulus
  some residue.toNat

def checkedUnaryInteger (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue)
    (operation : Int → Int) : Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer value] => some <| checkedInteger failure resultType (operation value)
  | _ => none

def checkedBinaryInteger (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue)
    (operation : Int → Int → Int) : Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer left, .integer right] =>
      some <| checkedInteger failure resultType (operation left right)
  | _ => none

def modularUnaryInteger (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Int → Int) : Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer value] => .ok <$> modularInteger resultType (operation value)
  | _ => none

def modularBinaryInteger (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Int → Int → Int) : Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer left, .integer right] => .ok <$> modularInteger resultType (operation left right)
  | _ => none

/-- Integer quotient rounded toward zero, matching fixed-width Move and Rust
semantics rather than Lean's Euclidean integer division. -/
def truncatingQuotient? (left right : Int) : Option Int :=
  if right == 0 then none else
    let magnitude := left.natAbs / right.natAbs
    let quotient := Int.ofNat magnitude
    if (left < 0 && 0 < right) || (right < 0 && 0 < left) then
      some (-quotient)
    else
      some quotient

/-- Integer remainder paired with `truncatingQuotient?`; its sign follows the
dividend, as required by Move and Rust. -/
def truncatingRemainder? (left right : Int) : Option Int := do
  let quotient ← truncatingQuotient? left right
  return left - quotient * right

/-! Symbolic execution consumes the truncating evaluators only through their
`Int.tdiv`/`Int.tmod` characterizations in `LeanerIR.Proofs`; sealing them
here keeps whnf from unfolding the sign analysis into a shape those
equations no longer match.  Compiled evaluation is unaffected. -/

attribute [irreducible] truncatingQuotient? truncatingRemainder?

/-- Rust-style overflowing arithmetic returns the fixed-width result together
with a flag indicating whether the mathematical result was out of range. -/
def overflowingBinaryInteger (ns : ValidatedNamespace)
    (targetPointerWidth : Option Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int → Int) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let .tuple resultElements := resultType | none
  let [valueTypeId, overflowTypeId] := resultElements.toList | none
  let valueType ← ns.tables.types[valueTypeId.index]? >>=
    resolveTargetIntegerType? targetPointerWidth
  let overflowType ← ns.tables.types[overflowTypeId.index]?
  if overflowType != .bool then none else
  let [.integer left, .integer right] := arguments.toList | none
  let (lower, upper) ← valueType.integerBounds?
  let exact := operation left right
  let value ← modularInteger valueType exact
  let overflowed := exact < lower || upper < exact
  some (.ok (.tuple #[value, .bool overflowed]))

/-- Apply a bitwise operator to fixed-width integer bit patterns, then restore
the signed or unsigned runtime interpretation selected by `resultType`. -/
def bitwiseBinaryInteger (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Nat → Nat → Nat) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let [.integer left, .integer right] := arguments.toList | none
  let leftBits ← integerBitPattern? resultType left
  let rightBits ← integerBitPattern? resultType right
  let result ← modularInteger resultType (Int.ofNat (operation leftBits rightBits))
  some (.ok result)

def bitwiseBinary (resultType : Ty) (arguments : Array RuntimeValue)
    (integerOperation : Nat → Nat → Nat) (booleanOperation : Bool → Bool → Bool) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.bool left, .bool right] => some (.ok (.bool (booleanOperation left right)))
  | _ => bitwiseBinaryInteger resultType arguments integerOperation

def bitwiseNotInteger (resultType : Ty) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let .integer (.bits width) _ := resultType | none
  if width == 0 then none else
  let [.integer value] := arguments.toList | none
  let bits ← integerBitPattern? resultType value
  let mask := (2 : Nat) ^ width - 1
  let result ← modularInteger resultType (Int.ofNat (bits ^^^ mask))
  some (.ok result)

def booleanOr (left right : Bool) : Bool := left || right
def booleanAnd (left right : Bool) : Bool := left && right
def booleanXor (left right : Bool) : Bool := left != right
def booleanLess (left right : Bool) : Bool := !left && right
def booleanGreater (left right : Bool) : Bool := left && !right
def booleanLessEqual (left right : Bool) : Bool := !left || right
def booleanGreaterEqual (left right : Bool) : Bool := left || !right

/-- Shift a fixed-width bit pattern after checking the language-selected
failure boundary. Right shift sign-extends signed values and zero-extends
unsigned values. -/
def checkedShiftInteger (failure : ThrowKind) (left : Bool)
    (resultType : Ty) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let .integer (.bits width) signed := resultType | none
  if width == 0 then none else
  let [.integer value, .integer distance] := arguments.toList | none
  if distance < 0 || width <= distance.toNat then
    some (.error (failure, #[.integer distance]))
  else
    let amount := distance.toNat
    let bits ← integerBitPattern? resultType value
    let shifted := if left then
      bits <<< amount
    else if signed && (2 : Nat) ^ (width - 1) <= bits && amount != 0 then
      (bits >>> amount) ||| (((2 : Nat) ^ amount - 1) <<< (width - amount))
    else
      bits >>> amount
    let result ← modularInteger resultType (Int.ofNat shifted)
    some (.ok result)

/-- The unchecked shift form is defined only when its distance is in range.
Frontends such as Rust preserve their source-language failure assertion as a
separate control node before emitting this operation. -/
def shiftInteger (left : Bool) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match checkedShiftInteger .panic left resultType arguments with
  | some (.ok value) => some (.ok value)
  | _ => none

def compareOrdered (arguments : Array RuntimeValue)
    (integerRelation : Int → Int → Bool) (booleanRelation : Bool → Bool → Bool)
    (characterRelation : Nat → Nat → Bool) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer left, .integer right] => some (.ok (.bool (integerRelation left right)))
  | [.bool left, .bool right] => some (.ok (.bool (booleanRelation left right)))
  | [.character left, .character right] =>
      some (.ok (.bool (characterRelation left right)))
  | _ => none

/-- Structural equality of two values, as the `==` primitive. -/
def equalValues? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [left, right] => some (.ok (.bool (left == right)))
  | _ => none

/-- Structural inequality of two values, as the `!=` primitive. -/
def notEqualValues? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [left, right] => some (.ok (.bool (left != right)))
  | _ => none

/-- Truncating division at the result type, modular on overflow. -/
def divideIntegers? (resultType : Ty) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer left, .integer right] => do
      let quotient ← truncatingQuotient? left right
      return .ok (← modularInteger resultType quotient)
  | _ => none

/-- Checked truncating division: a zero divisor throws `failure` with no
payload, an out-of-range quotient throws as any checked result. -/
def checkedDivideIntegers? (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer _, .integer 0] => some (.error (failure, #[]))
  | [.integer left, .integer right] => do
      let quotient ← truncatingQuotient? left right
      return checkedInteger failure resultType quotient
  | _ => none

/-- Truncating remainder at the result type, modular on overflow. -/
def moduloIntegers? (resultType : Ty) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer left, .integer right] => do
      let remainder ← truncatingRemainder? left right
      return .ok (← modularInteger resultType remainder)
  | _ => none

/-- Checked truncating remainder: a zero divisor throws `failure`, and the
quotient's range is checked before the remainder's. -/
def checkedModuloIntegers? (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.integer _, .integer 0] => some (.error (failure, #[]))
  | [.integer left, .integer right] => do
      let quotient ← truncatingQuotient? left right
      match checkedInteger failure resultType quotient with
      | .error error => return .error error
      | .ok _ =>
          return checkedInteger failure resultType (left - quotient * right)
  | _ => none

/-- Value-level insertion shared by execution and normalized proofs. The
end position is valid; negative and beyond-end indexes preserve the abort. -/
def insertVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer index, value] =>
      if index < 0 || elements.size < index.toNat then
        some (.error (.abort, #[.integer index]))
      else some (.ok (.vector (elements.insertIdxIfInBounds index.toNat value)))
  | _ => none

/-- Removal transfers the removed element along with the updated vector;
unlike insertion, the end position is out of bounds. -/
def removeVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer index] =>
      if index < 0 then some (.error (.abort, #[.integer index]))
      else match elements[index.toNat]? with
        | some value => some (.ok (.tuple #[value,
            .vector (elements.eraseIdxIfInBounds index.toNat)]))
        | none => some (.error (.abort, #[.integer index]))
  | _ => none

/-- Swap two vector elements without allocating an intermediate list. -/
def swapVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer left, .integer right] =>
      if left < 0 || right < 0 || elements.size ≤ left.toNat ||
          elements.size ≤ right.toNat then
        some (.error (.abort, #[.integer left, .integer right]))
      else
        match elements[left.toNat]?, elements[right.toNat]? with
        | some leftValue, some rightValue =>
            some (.ok (.vector ((elements.set! left.toNat rightValue).set!
              right.toNat leftValue)))
        | _, _ => some (.error (.abort, #[.integer left, .integer right]))
  | _ => none

/-- A structural swap schedule without an intermediate index list or rebuilt
prefix/suffix. The caller checks the complete range once. -/
private def reverseVectorRange : Nat → Nat → Nat → Array RuntimeValue → Array RuntimeValue
  | 0, _, _, elements => elements
  | count + 1, left, right, elements =>
      reverseVectorRange count (left + 1) (right - 1)
        (elements.swapIfInBounds left right)

/-- Raw range reversal. Move library range/error policy is represented by
its guards before invoking this kernel, just as for insertion and removal. -/
def reverseSliceVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer start, .integer stop] =>
      if start < 0 || stop < start || elements.size < stop.toNat then
        some (.error (.abort, #[.integer start, .integer stop]))
      else some (.ok (.vector (reverseVectorRange
        ((stop.toNat - start.toNat) / 2) start.toNat (stop.toNat - 1) elements)))
  | _ => none

/-- Search without allocating a list of elements or indexes; the first match
wins. The structural count bounds the traversal even for malformed callers. -/
private def findVectorIndex (elements : Array RuntimeValue) (needle : RuntimeValue) :
    Nat → Nat → Option Nat
  | 0, _ => none
  | count + 1, index =>
      if elements[index]?.any (· == needle) then some index
      else findVectorIndex elements needle count (index + 1)

def checkVectorIndex? (failure : ThrowKind) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer index] =>
      if 0 ≤ index ∧ index < Int.ofNat elements.size then some (.ok .unit)
      else some (.error (failure, #[.integer 1]))
  | _ => none

def containsVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, needle] =>
      some (.ok (.bool (findVectorIndex elements needle elements.size 0).isSome))
  | _ => none

def indexOfVector? (indexType : Ty) (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let [.vector elements, needle] := arguments.toList | none
  let found := findVectorIndex elements needle elements.size 0
  let position := Int.ofNat (found.getD 0)
  let index ← match indexType with
    | .integer .unbounded _ => some (.integer position)
    | _ => modularInteger indexType position
  some (.ok (.tuple #[.bool found.isSome, index]))

/-- Consume an empty vector without requiring Drop on its element type.
Move's native failure policy is checked before this raw operation. -/
def destroyEmptyVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements] =>
      if elements.isEmpty then some (.ok .unit) else some (.error (.abort, #[]))
  | _ => none

/-- Value-level concatenation shared by the interpreter and proof denotation. -/
def concatVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector left, .vector right] => some (.ok (.vector (left ++ right)))
  | _ => none

/-- Copy a half-open slice. Bounds checks are explicit; `extract` must not
silently clamp an invalid range. -/
def sliceVector? (arguments : Array RuntimeValue) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) :=
  match arguments.toList with
  | [.vector elements, .integer start, .integer stop] =>
      if start < 0 || stop < start || elements.size < stop.toNat then
        some (.error (.abort, #[.integer start, .integer stop]))
      else some (.ok (.vector (elements.extract start.toNat stop.toNat)))
  | _ => none

/-- Deterministic meaning of the shared Move/Rust pure primitive vocabulary. -/
def evaluatePrimitiveOperation? (ns : ValidatedNamespace) (resultType : TypeId)
    (operation : PrimitiveOperation) (arguments : Array RuntimeValue)
    (targetPointerWidth : Option Nat := none) :
    Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue) := do
  let resultType ← ns.tables.types[resultType.index]? >>=
    resolveTargetIntegerType? targetPointerWidth
  match operation with
  | .tuple => some (.ok (.tuple arguments))
  | .vector => some (.ok (.vector arguments))
  | .repeatVector => match resultType, arguments.toList with
      | .vector _ (some (.integer length)), [value] =>
          if length < 0 then none
          else some (.ok (.vector (Array.replicate length.toNat value)))
      | _, _ => none
  | .pushVector => match arguments.toList with
      | [.vector elements, value] => some (.ok (.vector (elements.push value)))
      | _ => none
  | .insertVector => insertVector? arguments
  | .concatVector => concatVector? arguments
  | .removeVector => removeVector? arguments
  | .swapVector => swapVector? arguments
  | .reverseSliceVector => reverseSliceVector? arguments
  | .containsVector => containsVector? arguments
  | .checkVectorIndex failure => checkVectorIndex? failure arguments
  | .indexOfVector =>
      let .tuple elements := resultType | none
      let [_, index] := elements.toList | none
      let indexType ← ns.tables.types[index.index]? >>=
        resolveTargetIntegerType? targetPointerWidth
      indexOfVector? indexType arguments
  | .destroyEmptyVector => destroyEmptyVector? arguments
  | .length =>
      let lengthValue (length : Nat) := match resultType with
        | .integer .unbounded _ => some (.integer (Int.ofNat length))
        | _ => modularInteger resultType length
      match arguments.toList with
      | [.vector elements] => .ok <$> lengthValue elements.size
      | [.string value] => .ok <$> lengthValue value.utf8ByteSize
      | [.bytes values] => .ok <$> lengthValue values.size
      | _ => none
  | .index => match arguments.toList with
      | [.vector elements, .integer index] =>
          if index < 0 then some (.error (.abort, #[.integer index]))
          else match elements[index.toNat]? with
            | some element => some (.ok element)
            | none => some (.error (.abort, #[.integer index]))
      | _ => none
  | .slice => sliceVector? arguments
  | .add => modularBinaryInteger resultType arguments (fun left right => left + right)
  | .checkedAdd failure =>
      checkedBinaryInteger failure resultType arguments (fun left right => left + right)
  | .overflowingAdd =>
      overflowingBinaryInteger ns targetPointerWidth resultType arguments
        (fun left right => left + right)
  | .subtract => modularBinaryInteger resultType arguments (fun left right => left - right)
  | .checkedSubtract failure =>
      checkedBinaryInteger failure resultType arguments (fun left right => left - right)
  | .overflowingSubtract =>
      overflowingBinaryInteger ns targetPointerWidth resultType arguments
        (fun left right => left - right)
  | .multiply => modularBinaryInteger resultType arguments (fun left right => left * right)
  | .checkedMultiply failure =>
      checkedBinaryInteger failure resultType arguments (fun left right => left * right)
  | .overflowingMultiply =>
      overflowingBinaryInteger ns targetPointerWidth resultType arguments
        (fun left right => left * right)
  | .divide => divideIntegers? resultType arguments
  | .checkedDivide failure => checkedDivideIntegers? failure resultType arguments
  | .modulo => moduloIntegers? resultType arguments
  | .checkedModulo failure => checkedModuloIntegers? failure resultType arguments
  | .negate => modularUnaryInteger resultType arguments (-·)
  | .checkedNegate failure => checkedUnaryInteger failure resultType arguments (-·)
  | .bitwiseOr => bitwiseBinary resultType arguments (fun left right => left ||| right) booleanOr
  | .bitwiseAnd => bitwiseBinary resultType arguments (fun left right => left &&& right) booleanAnd
  | .bitwiseXor => bitwiseBinary resultType arguments (fun left right => left ^^^ right) booleanXor
  | .bitwiseNot => bitwiseNotInteger resultType arguments
  | .checkedShiftLeft failure => checkedShiftInteger failure true resultType arguments
  | .checkedShiftRight failure => checkedShiftInteger failure false resultType arguments
  | .shiftLeft => shiftInteger true resultType arguments
  | .shiftRight => shiftInteger false resultType arguments
  | .checkedCast failure => match arguments.toList with
      | [.integer value] => match resultType with
          | .character =>
            if value < 0 || !isUnicodeScalar value.toNat then
              some (.error (failure, #[.integer value]))
            else
              some (.ok (.character value.toNat))
          | _ => some <| checkedInteger failure resultType value
      | [.character value] => some <| checkedInteger failure resultType (Int.ofNat value)
      | _ => none
  | .cast => match arguments.toList with
      | [.integer value] => match resultType with
          | .character =>
            if value < 0 || !isUnicodeScalar value.toNat then none
            else some (.ok (.character value.toNat))
          | _ => .ok <$> modularInteger resultType value
      | [.character value] => .ok <$> modularInteger resultType (Int.ofNat value)
      | _ => none
  | .copyValue | .moveValue => match arguments.toList with
      | [value] => some (.ok value)
      | _ => none
  | .range | .implies |
      .equivalent | .identical => none
  | .logicalAnd => match arguments.toList with
      | [.bool left, .bool right] => some (.ok (.bool (left && right)))
      | _ => none
  | .logicalOr => match arguments.toList with
      | [.bool left, .bool right] => some (.ok (.bool (left || right)))
      | _ => none
  | .logicalNot => match arguments.toList with
      | [.bool operand] => some (.ok (.bool (!operand)))
      | _ => none
  | .equal => equalValues? arguments
  | .notEqual => notEqualValues? arguments
  | .less =>
      compareOrdered arguments (fun left right => left < right) booleanLess
        (fun left right => left < right)
  | .greater =>
      compareOrdered arguments (fun left right => left > right) booleanGreater
        (fun left right => left > right)
  | .lessEqual =>
      compareOrdered arguments (fun left right => left <= right) booleanLessEqual
        (fun left right => left <= right)
  | .greaterEqual =>
      compareOrdered arguments (fun left right => left >= right) booleanGreaterEqual
        (fun left right => left >= right)

/-! ## Concrete places and references -/

private def subsliceBounds? (length start stop : Nat) (fromEnd : Bool) : Option (Nat × Nat) :=
  if fromEnd then
    if start + stop <= length then some (start, length - stop) else none
  else if start <= stop && stop <= length then some (start, stop) else none

/-- Read a value through a resolved projection path. -/
def readProjections? (value : RuntimeValue) :
    List RuntimeProjection → Option RuntimeValue
  | [] => some value
  | .field index :: rest => do
      let .nominal _ _ fields := value | none
      let field ← fields[index]?
      readProjections? field rest
  | .index index :: rest => match value with
      | .vector elements | .tuple elements => do
          let element ← elements[index]?
          readProjections? element rest
      | _ => none
  | .subslice start stop fromEnd :: rest => do
      let .vector elements := value | none
      let (first, last) ← subsliceBounds? elements.size start stop fromEnd
      readProjections? (.vector (elements.extract first last)) rest
  | .downcast variant :: rest => do
      let .nominal _ (some actual) _ := value | none
      if actual == variant then readProjections? value rest else none
  | .deref :: rest => do
      let .borrow _ current := value | none
      readProjections? current rest

/-- Replace the value selected by a resolved projection path. -/
def writeProjections? (value : RuntimeValue)
    (projections : List RuntimeProjection) (replacement : RuntimeValue) : Option RuntimeValue := do
  match projections with
  | [] => some replacement
  | .field index :: rest =>
      let .nominal name variant fields := value | none
      let field ← fields[index]?
      let updated ← writeProjections? field rest replacement
      return .nominal name variant (fields.set! index updated)
  | .index index :: rest => match value with
      | .vector elements =>
          let element ← elements[index]?
          let updated ← writeProjections? element rest replacement
          return .vector (elements.set! index updated)
      | .tuple elements =>
          let element ← elements[index]?
          let updated ← writeProjections? element rest replacement
          return .tuple (elements.set! index updated)
      | _ => none
  | .subslice start stop fromEnd :: rest => do
      let .vector elements := value | none
      let (first, last) ← subsliceBounds? elements.size start stop fromEnd
      let updated ← writeProjections? (.vector (elements.extract first last)) rest replacement
      let .vector replacement := updated | none
      if replacement.size != last - first then none else
        return .vector (elements.extract 0 first ++ replacement ++
          elements.extract last elements.size)
  | .downcast variant :: rest =>
      let .nominal _ (some actual) _ := value | none
      if actual == variant then writeProjections? value rest replacement else none
  | .deref :: rest => do
      let .borrow loan current := value | none
      let updated ← writeProjections? current rest replacement
      return .borrow loan updated

def sourceFieldName? (ns : ValidatedNamespace) (field : NameId) : Option String :=
  (ns.tables.names[field.index]?).map (·.name)

/-- Position of a named field in an ordered payload.  The search is written
as a structural recursion rather than an array search because reduction can
execute one and not the other: a field selection over a literal declaration
must compute, not stall half way through a search loop. -/
private def fieldIndexIn (ns : ValidatedNamespace) (fieldName : String) :
    List FieldDecl → Nat → Option Nat
  | [], _ => none
  | field :: rest, index =>
      if (ns.tables.names[field.name.index]?).any (·.name == fieldName) then some index
      else fieldIndexIn ns fieldName rest (index + 1)

/-- Fields of a handle-identified declaration, indexed directly. -/
def handleFields? (unit : ValidatedUnit) (handle : StructHandle)
    (variant : Option String) :
    Option (ValidatedNamespace × Array FieldDecl) := do
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.structs[handle.structId]?
  match variant with
  | none =>
      if declaration.variants.isEmpty then some (targetNs, declaration.fields)
      else none
  | some variantName => do
      let declared ← declaration.variants.toList.find? fun candidate =>
        (targetNs.tables.names[candidate.name.index]?).any (·.name == variantName)
      some (targetNs, declared.fields)

/-- Position of a named field in a handle-identified payload. -/
def handleFieldIndex? (unit : ValidatedUnit) (handle : StructHandle)
    (variant : Option String) (fieldName : String) : Option Nat := do
  let (targetNs, fields) ← handleFields? unit handle variant
  fieldIndexIn targetNs fieldName fields.toList 0

/-- Resolve a variant-field name set to its closed per-variant payload
offsets.  Variants without any requested field are omitted. -/
def variantFieldChoices? (unit : ValidatedUnit) (handle : StructHandle)
    (fields : Array String) : Option (Array (String × Nat)) := do
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.structs[handle.structId]?
  return declaration.variants.filterMap fun variant => do
    let variantName ← targetNs.tables.names[variant.name.index]?
    let field ← fields.find? fun field =>
      (fieldIndexIn targetNs field variant.fields.toList 0).isSome
    let index ← fieldIndexIn targetNs field variant.fields.toList 0
    some (variantName.name, index)

/-- Select through a closed per-variant payload map. -/
def selectNominalVariantFieldAt? (owner : StructHandle)
    (choices : Array (String × Nat))
    (arguments : Array RuntimeValue) : Option RuntimeValue :=
  match arguments.toList with
  | [.nominal actualSource (some actualVariant) values] => do
      if actualSource != owner then none else
      let (_, index) ← choices.find? fun choice => choice.1 == actualVariant
      values[index]?
  | _ => none

/-- Position of a named field in a referenced nominal declaration, which a
generated contract needs to project the same field the program selects. -/
def referencedFieldIndex? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (field : String) :
    Option Nat := do
  let handle ← resolveStruct? unit sourceNamespace reference
  handleFieldIndex? unit handle variant field

def variantIndex? (unit : ValidatedUnit) (handle : StructHandle)
    (variant : String) : Option Nat := do
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.structs[handle.structId]?
  declaration.variants.findIdx? fun candidate =>
    (targetNs.tables.names[candidate.name.index]?).any (·.name == variant)

def variantDiscriminant? (unit : ValidatedUnit) (handle : StructHandle)
    (variant : String) : Option Int := do
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.structs[handle.structId]?
  let selected ← declaration.variants.find? fun candidate =>
    (targetNs.tables.names[candidate.name.index]?).any (·.name == variant)
  selected.discriminant

/-- Deterministic meaning of typed nominal field and variant operations. -/
def evaluateDataOperation? (unit : ValidatedUnit) (sourceNamespace : NamespaceId)
    (operation : DataOperation) (arguments : Array RuntimeValue) : Option RuntimeValue := do
  match operation, arguments.toList with
  | .select reference field, [.nominal actualSource variant values] =>
      let handle ← resolveStruct? unit sourceNamespace reference
      if actualSource != handle then none else
      let index ← handleFieldIndex? unit handle variant field
      values[index]?
  | .selectVariants reference fields, _ =>
      let handle ← resolveStruct? unit sourceNamespace reference
      let choices ← variantFieldChoices? unit handle fields
      selectNominalVariantFieldAt? handle choices arguments
  | .testVariants reference variants, [.nominal actualSource actualVariant _] =>
      let handle ← resolveStruct? unit sourceNamespace reference
      if actualSource != handle then none else
      some (.bool (actualVariant.any variants.contains))
  | .discriminant reference, [.nominal actualSource (some variant) _] =>
      let handle ← resolveStruct? unit sourceNamespace reference
      if actualSource != handle then none else
      .integer <$> variantDiscriminant? unit handle variant
  | .updateField reference field, [.nominal actualSource variant values, replacement] =>
      let handle ← resolveStruct? unit sourceNamespace reference
      if actualSource != handle then none else
      let index ← handleFieldIndex? unit handle variant field
      if index < values.size then
        some (.nominal actualSource variant (values.set! index replacement))
      else none
  | _, _ => none

/-- Native selection of one field whose owner, payload variant, and position
were fixed by lowering. -/
def selectNominalFieldAt? (owner : StructHandle) (variant : Option String)
    (index : Nat) (arguments : Array RuntimeValue) : Option RuntimeValue :=
  match arguments.toList with
  | [.nominal actualSource actualVariant values] =>
      if actualSource != owner || actualVariant != variant then none
      else values[index]?
  | _ => none

/-- Test a closed nominal handle against a closed variant set. -/
def testNominalVariants? (owner : StructHandle) (variants : Array String)
    (arguments : Array RuntimeValue) : Option RuntimeValue :=
  match arguments.toList with
  | [.nominal actualSource actualVariant _] =>
      if actualSource != owner then none
      else some (.bool (actualVariant.any variants.contains))
  | _ => none

theorem evaluateDataOperation?_testVariants_at
    {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {reference : QualifiedRef} {variants : Array String} {handle : StructHandle}
    (resolved_eq : resolveStruct? unit sourceNamespace reference = some handle)
    (arguments : Array RuntimeValue) :
    evaluateDataOperation? unit sourceNamespace (.testVariants reference variants)
        arguments = testNominalVariants? handle variants arguments := by
  generalize list_eq : arguments.toList = values
  cases values with
  | nil => simp [evaluateDataOperation?, testNominalVariants?, list_eq]
  | cons value rest =>
      cases rest with
      | nil =>
          cases value <;>
            simp [evaluateDataOperation?, testNominalVariants?, list_eq,
              resolved_eq]
      | cons second tail =>
          simp [evaluateDataOperation?, testNominalVariants?, list_eq]

/-- Lower a nominal selection once its resolution and per-variant field map
have been certified. -/
theorem evaluateDataOperation?_select_at
    {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {reference : QualifiedRef} {field : String} {handle : StructHandle}
    {variant : Option String} {index : Nat}
    (resolved_eq : resolveStruct? unit sourceNamespace reference = some handle)
    (index_eq : ∀ actualVariant,
      handleFieldIndex? unit handle actualVariant field =
        if actualVariant == variant then some index else none)
    (arguments : Array RuntimeValue) :
    evaluateDataOperation? unit sourceNamespace (.select reference field) arguments =
      selectNominalFieldAt? handle variant index arguments := by
  generalize list_eq : arguments.toList = values
  cases values with
  | nil => simp [evaluateDataOperation?, selectNominalFieldAt?, list_eq]
  | cons value rest =>
      cases rest with
      | nil =>
          cases value <;>
            try simp [evaluateDataOperation?, selectNominalFieldAt?, list_eq,
              resolved_eq]
          case nominal actualSource actualVariant fields =>
            by_cases source_eq : actualSource = handle
            · subst actualSource
              by_cases variant_eq : actualVariant = variant <;>
                simp [evaluateDataOperation?, selectNominalFieldAt?, list_eq,
                  resolved_eq, index_eq, variant_eq]
            · simp [evaluateDataOperation?, selectNominalFieldAt?, list_eq,
                resolved_eq, bne_iff_ne, source_eq]
      | cons second tail =>
          simp [evaluateDataOperation?, selectNominalFieldAt?, list_eq]

/-- Lower a single-name variant selection once lowering has certified the
unique variant and payload offset containing that name. -/
theorem evaluateDataOperation?_selectVariants_at
    {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {reference : QualifiedRef} {fields : Array String} {handle : StructHandle}
    {choices : Array (String × Nat)}
    (resolved_eq : resolveStruct? unit sourceNamespace reference = some handle)
    (choices_eq : variantFieldChoices? unit handle fields = some choices)
    (arguments : Array RuntimeValue) :
    evaluateDataOperation? unit sourceNamespace
        (.selectVariants reference fields) arguments =
      selectNominalVariantFieldAt? handle choices arguments := by
  simp [evaluateDataOperation?, resolved_eq, choices_eq]


/-! Data operations resolve field and variant names against the declaration
tables, a search reduction cannot execute.  Sealing the operation keeps
reduction out of it — whnf reduces a `match` scrutinee whatever an inventory
says, and would leave the search half executed — and hands it to the simp
phase through the equations below.  The evaluator matches its argument row
against an array literal, whose `match` equations Lean cannot generate; at a
literal row the match reduces definitionally, so each is `rfl`. -/

theorem evaluateDataOperation?_select {unit : ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {field : String}
    {source : StructHandle} {variant : Option String} {values : Array RuntimeValue} :
    evaluateDataOperation? unit sourceNamespace (.select reference field)
        #[.nominal source variant values]
      = (do
          let handle ← resolveStruct? unit sourceNamespace reference
          if source != handle then none else
          let index ← handleFieldIndex? unit handle variant field
          values[index]?) := rfl

theorem evaluateDataOperation?_selectVariants {unit : ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {fields : Array String}
    {source : StructHandle} {variant : String} {values : Array RuntimeValue} :
    evaluateDataOperation? unit sourceNamespace (.selectVariants reference fields)
        #[.nominal source (some variant) values]
      = (do
          let handle ← resolveStruct? unit sourceNamespace reference
          let choices ← variantFieldChoices? unit handle fields
          selectNominalVariantFieldAt? handle choices
            #[.nominal source (some variant) values]) := rfl

theorem evaluateDataOperation?_testVariants {unit : ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variants : Array String}
    {source : StructHandle} {variant : Option String} {values : Array RuntimeValue} :
    evaluateDataOperation? unit sourceNamespace (.testVariants reference variants)
        #[.nominal source variant values]
      = (do
          let handle ← resolveStruct? unit sourceNamespace reference
          if source != handle then none else
          some (.bool (variant.any variants.contains))) := rfl

theorem evaluateDataOperation?_discriminant {unit : ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef}
    {source : StructHandle} {variant : String} {values : Array RuntimeValue} :
    evaluateDataOperation? unit sourceNamespace (.discriminant reference)
        #[.nominal source (some variant) values]
      = (do
          let handle ← resolveStruct? unit sourceNamespace reference
          if source != handle then none else
          .integer <$> variantDiscriminant? unit handle variant) := rfl

theorem evaluateDataOperation?_updateField {unit : ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {field : String}
    {source : StructHandle} {variant : Option String}
    {values : Array RuntimeValue} {replacement : RuntimeValue} :
    evaluateDataOperation? unit sourceNamespace (.updateField reference field)
        #[.nominal source variant values, replacement]
      = (do
          let handle ← resolveStruct? unit sourceNamespace reference
          if source != handle then none else
          let index ← handleFieldIndex? unit handle variant field
          if index < values.size then
            some (.nominal source variant (values.set! index replacement))
          else none) := rfl

attribute [irreducible] evaluateDataOperation?


/-- Read a frame local. -/
def readLocal? (frame : RuntimeFrame) (localId : LocalId) : Option RuntimeValue :=
  frame.locals[localId.index]?.join

/-- Read the value stored at a resolved place root. -/
def readRoot? (frame : RuntimeFrame) (state : RuntimeState) :
    RuntimePlaceRoot → Option RuntimeValue
  | .local localId => readLocal? frame localId
  | .global key => state.globals.lookup key

/-- Read a resolved place, including all projections. -/
def readRuntimePlace? (frame : RuntimeFrame) (state : RuntimeState)
    (place : RuntimePlace) : Option RuntimeValue := do
  let root ← readRoot? frame state place.root
  readProjections? root place.projections.toList

/-- Write a resolved local or global place root. -/
def writeRoot? (frame : RuntimeFrame) (state : RuntimeState)
    (root : RuntimePlaceRoot) (value : RuntimeValue) : Option (RuntimeFrame × RuntimeState) := do
  match root with
  | .local localId =>
      if localId.index >= frame.locals.size then none else
      some ({ frame with locals := frame.locals.set! localId.index (some value) }, state)
  | .global key =>
      -- A write reaches global memory only through a live borrow, so the key
      -- must still be published: writing an absent key would publish it.
      let _ ← state.globals.lookup key
      some (frame, { state with globals := state.globals.insert key value })

/-- Write a resolved place. Writes through an immutable reference are
rejected even if malformed LIR bypasses the static borrow checker. -/
def writeRuntimePlace? (frame : RuntimeFrame) (state : RuntimeState)
    (place : RuntimePlace) (value : RuntimeValue) : Option (RuntimeFrame × RuntimeState) := do
  if !place.writable then none else
    if place.projections.isEmpty then
      -- A root assignment initializes declared storage and therefore does not
      -- require the local's previous value to exist. Projected writes still
      -- need an initialized aggregate/reference root to update.
      writeRoot? frame state place.root value
    else
      let root ← readRoot? frame state place.root
      let updated ← writeProjections? root place.projections.toList value
      writeRoot? frame state place.root updated

/-! ## Loan bookkeeping

The prophetic ownership model reunites a mutable loan's hole and current
value at the loan's recorded death. Holes are position-independent: they are
found by loanInstance wherever they sit, including inside another borrow's
current value. -/

/-! ### Total value traversal

The ownership walkers share one descent: node before children, children of
vectors, tuples, nominal fields, closure captures, and a borrow's current
value.  Three total combinators cover rewriting, querying, and pruned
collection, so symbolic execution can reduce through every walker. -/

private theorem sizeOf_toList_lt (elements : Array RuntimeValue) :
    sizeOf elements.toList < sizeOf elements := by
  cases elements with
  | mk toList => simp

mutual
  /-- Rewrite the first node `f` matches, in preorder. `none` when nothing
  matches. -/
  def rewriteFirst (f : RuntimeValue → Option RuntimeValue)
      (value : RuntimeValue) : Option RuntimeValue :=
    match f value with
    | some rewritten => some rewritten
    | none =>
        match value with
        | .vector elements =>
            (rewriteFirstList f elements.toList).map fun rewritten =>
              .vector rewritten.toArray
        | .tuple elements =>
            (rewriteFirstList f elements.toList).map fun rewritten =>
              .tuple rewritten.toArray
        | .nominal name variant fields =>
            (rewriteFirstList f fields.toList).map fun rewritten =>
              .nominal name variant rewritten.toArray
        | .closure function captures =>
            (rewriteFirstList f captures.toList).map fun rewritten =>
              .closure function rewritten.toArray
        | .borrow loanInstance current =>
            (rewriteFirst f current).map (.borrow loanInstance)
        | _ => none
  termination_by sizeOf value
  decreasing_by
    all_goals simp_wf
    all_goals try (first
      | (have := sizeOf_toList_lt elements)
      | (have := sizeOf_toList_lt fields)
      | (have := sizeOf_toList_lt captures))
    all_goals omega

  /-- `rewriteFirst` over the elements of one aggregate. -/
  def rewriteFirstList (f : RuntimeValue → Option RuntimeValue) :
      List RuntimeValue → Option (List RuntimeValue)
    | [] => none
    | element :: rest =>
        match rewriteFirst f element with
        | some rewritten => some (rewritten :: rest)
        | none => (rewriteFirstList f rest).map (element :: ·)
  termination_by elements => sizeOf elements
  decreasing_by all_goals (simp_wf; omega)
end

mutual
  /-- First query result of `f`, in preorder. -/
  def findFirst {α : Type} (f : RuntimeValue → Option α)
      (value : RuntimeValue) : Option α :=
    match f value with
    | some found => some found
    | none =>
        match value with
        | .vector elements | .tuple elements => findFirstList f elements.toList
        | .nominal _ _ fields => findFirstList f fields.toList
        | .closure _ captures => findFirstList f captures.toList
        | .borrow _ current => findFirst f current
        | _ => none
  termination_by sizeOf value
  decreasing_by
    all_goals simp_wf
    all_goals try (first
      | (have := sizeOf_toList_lt elements)
      | (have := sizeOf_toList_lt fields)
      | (have := sizeOf_toList_lt captures))
    all_goals omega

  /-- `findFirst` over the elements of one aggregate. -/
  def findFirstList {α : Type} (f : RuntimeValue → Option α) :
      List RuntimeValue → Option α
    | [] => none
    | element :: rest =>
        match findFirst f element with
        | some found => some found
        | none => findFirstList f rest
  termination_by elements => sizeOf elements
  decreasing_by all_goals (simp_wf; omega)
end

mutual
  /-- All matches of `f`, pruning descent below a match. -/
  def collectPruned {α : Type} (f : RuntimeValue → Option α)
      (value : RuntimeValue) : Array α :=
    match f value with
    | some found => #[found]
    | none =>
        match value with
        | .vector elements | .tuple elements => collectPrunedList f elements.toList
        | .nominal _ _ fields => collectPrunedList f fields.toList
        | .closure _ captures => collectPrunedList f captures.toList
        | .borrow _ current => collectPruned f current
        | _ => #[]
  termination_by sizeOf value
  decreasing_by
    all_goals simp_wf
    all_goals try (first
      | (have := sizeOf_toList_lt elements)
      | (have := sizeOf_toList_lt fields)
      | (have := sizeOf_toList_lt captures))
    all_goals omega

  /-- `collectPruned` over the elements of one aggregate. -/
  def collectPrunedList {α : Type} (f : RuntimeValue → Option α) :
      List RuntimeValue → Array α
    | [] => #[]
    | element :: rest => collectPruned f element ++ collectPrunedList f rest
  termination_by elements => sizeOf elements
  decreasing_by all_goals (simp_wf; omega)
end

/-! ### Node matchers

Every walk over a value looks for one kind of node — the hole of a loan, or
the borrow resting on it — and does one thing there.  The matchers are
named so that each walk is one constant applied to one constant: a lemma
about a walk through a symbolic value can then be stated for the matcher
by name, where a lambda would have no rewrite identity. -/

/-- The hole of `loan` at a node, filled with `replacement`. -/
@[simp] def holeFill? (loan : Nat) (replacement : RuntimeValue) :
    RuntimeValue → Option RuntimeValue
  | .loanHole hole => if hole == loan then some replacement else none
  | _ => none

/-- The hole of `loan` at a node. -/
@[simp] def holeMark? (loan : Nat) : RuntimeValue → Option Unit
  | .loanHole hole => if hole == loan then some () else none
  | _ => none

/-- Any hole at a node, by its loan. -/
@[simp] def anyHole? : RuntimeValue → Option Nat
  | .loanHole loan => some loan
  | _ => none

/-- The current of the borrow `loan` at a node. -/
@[simp] def borrowCurrent? (loan : Nat) : RuntimeValue → Option RuntimeValue
  | .borrow instance_ current => if instance_ == loan then some current else none
  | _ => none

/-- The borrow `loan` at a node, its current replaced. -/
@[simp] def borrowRewrite? (loan : Nat) (replacement : RuntimeValue) :
    RuntimeValue → Option RuntimeValue
  | .borrow instance_ _ =>
      if instance_ == loan then some (.borrow instance_ replacement) else none
  | _ => none

/-- The borrow `loan` at a node, retired to a unit. -/
@[simp] def borrowClear? (loan : Nat) : RuntimeValue → Option RuntimeValue
  | .borrow instance_ _ => if instance_ == loan then some .unit else none
  | _ => none

/-- Replace the hole of `loan` by `replacement`; `none` when the hole is not
in this value. A live loan has exactly one hole. -/
def fillHole? (loan : Nat) (replacement : RuntimeValue)
    (value : RuntimeValue) : Option RuntimeValue :=
  rewriteFirst (holeFill? loan replacement) value

/-- Whether the hole of `loan` sits inside this value. The search is the
preorder walk every hole operation uses, phrased without a replacement so
scans and visibility predicates share one shape. -/
def holeWithin (loan : Nat) (value : RuntimeValue) : Bool :=
  (findFirst (holeMark? loan) value).isSome

/-- The key registered for `loan` in a global-loan registry.  Keeping the
registry lookup separate from `RuntimeState` lets proof normalization expose
the one state field that matters without unfolding the lookup itself. -/
def globalLoanKeyIn? (loans : List (Nat × GlobalKey))
    (loan : Nat) : Option GlobalKey :=
  (loans.find? (·.1 == loan)).map (·.2)

/-- A registry headed by the loan's own registration resolves to that key
without scanning the symbolic tail. -/
theorem globalLoanKeyIn?_head (loan : Nat) (key : GlobalKey)
    (rest : List (Nat × GlobalKey)) :
    globalLoanKeyIn? ((loan, key) :: rest) loan = some key := by
  simp [globalLoanKeyIn?]

/-- A distinct registration cannot capture an unregistered identity. -/
theorem globalLoanKeyIn?_cons_none {registered loan : Nat} {key : GlobalKey}
    {rest : List (Nat × GlobalKey)} (different : registered ≠ loan)
    (absent : globalLoanKeyIn? rest loan = none) :
    globalLoanKeyIn? ((registered, key) :: rest) loan = none := by
  simpa [globalLoanKeyIn?, different] using absent

/-- The key whose global slot holds the hole of `loan`, per the state's
loan registry.  Registration at the borrow is what keys the write-back:
certified exclusivity keeps the recorded key the hole's location for the
loan's whole life, so nothing ever searches global memory for a hole. -/
def globalLoanKey? (state : RuntimeState) (loan : Nat) : Option GlobalKey :=
  globalLoanKeyIn? state.globalLoans loan

/-- A global-loan query depends only on the native registry, not on any of
the other runtime-state fields.  Generated proofs use this equation to pull
record updates out of routing decisions. -/
theorem globalLoanKey?_registry (state : RuntimeState) (loan : Nat) :
    globalLoanKey? state loan = globalLoanKeyIn? state.globalLoans loan :=
  rfl

/-- Every identifier at or beyond `nextLoan` is absent from the global-loan
registry.  This is the reachable-state invariant that makes a freshly minted
local loan route to an ancestor frame rather than aliasing a global loan. -/
def FreshGlobalLoanIds (state : RuntimeState) : Prop :=
  ∀ loan, state.nextLoan ≤ loan →
    globalLoanKeyIn? state.globalLoans loan = none

/-- Any identifier at or beyond the allocation frontier is globally
unregistered.  This named form lets generated closers consume the invariant
without unfolding it or searching the whole local context. -/
theorem FreshGlobalLoanIds.lookup_of_le {state : RuntimeState} {loan : Nat}
    (fresh : FreshGlobalLoanIds state) (bound : state.nextLoan ≤ loan) :
    globalLoanKeyIn? state.globalLoans loan = none :=
  fresh loan bound

/-- The next identifier itself is globally unregistered. -/
theorem FreshGlobalLoanIds.lookup_next {state : RuntimeState}
    (fresh : FreshGlobalLoanIds state) :
    globalLoanKeyIn? state.globalLoans state.nextLoan = none :=
  fresh state.nextLoan (Nat.le_refl _)

/-- Direct lookup form of freshness for an identifier minted at an offset
from `nextLoan`. -/
theorem FreshGlobalLoanIds.lookup_add {state : RuntimeState}
    (fresh : FreshGlobalLoanIds state) (offset : Nat) :
    globalLoanKeyIn? state.globalLoans (state.nextLoan + offset) = none := by
  apply fresh
  omega

/-- The loan discipline a function keeps across its whole execution, stated
as the interface a modular caller consumes: freshness of unminted ids is
preserved, the registration of every id minted before the call reads the
same after it, and ids only grow.  Registry equality would be wrong — a
callee returning a global `&mut` legitimately exits with its returned loan
registered. -/
def LoanDiscipline (initial final : RuntimeState) : Prop :=
  (FreshGlobalLoanIds initial → FreshGlobalLoanIds final) ∧
  (∀ loan, loan < initial.nextLoan →
    globalLoanKeyIn? final.globalLoans loan =
      globalLoanKeyIn? initial.globalLoans loan) ∧
  initial.nextLoan ≤ final.nextLoan

/-- A body that leaves the registry untouched keeps the discipline. -/
theorem LoanDiscipline.of_eq {initial final : RuntimeState}
    (loans_eq : final.globalLoans = initial.globalLoans)
    (monotone : initial.nextLoan ≤ final.nextLoan) :
    LoanDiscipline initial final := by
  refine ⟨fun fresh loan h => ?_, fun loan _ => by rw [loans_eq], monotone⟩
  rw [loans_eq]
  exact fresh loan (Nat.le_trans monotone h)

/-- The discipline composes across sequential calls. -/
theorem LoanDiscipline.trans {first second third : RuntimeState}
    (early : LoanDiscipline first second)
    (late : LoanDiscipline second third) :
    LoanDiscipline first third := by
  obtain ⟨earlyFresh, earlyStable, earlyMonotone⟩ := early
  obtain ⟨lateFresh, lateStable, lateMonotone⟩ := late
  refine ⟨fun fresh => lateFresh (earlyFresh fresh), fun loan h => ?_,
    Nat.le_trans earlyMonotone lateMonotone⟩
  rw [lateStable loan (by omega), earlyStable loan h]

/-- Reflexive discipline, in simp form: an exit whose state normalization
already restored the entry state leaves the clause closable inside any
surrounding connective, where only a rewrite can reach it. -/
@[simp] theorem LoanDiscipline.self (state : RuntimeState) :
    LoanDiscipline state state :=
  LoanDiscipline.of_eq rfl (Nat.le_refl _)

/-- Registry-preserving literal exit state, in simp form. -/
@[simp] theorem LoanDiscipline.mk_self (state : RuntimeState)
    (globals : GlobalMap) (pending : Array (Nat × RuntimeValue)) :
    LoanDiscipline state
      { globals, globalLoans := state.globalLoans,
        nextLoan := state.nextLoan, pending } :=
  LoanDiscipline.of_eq rfl (Nat.le_refl _)

/-- Registry-preserving literal exit state whose loan frontier advanced,
in simp form. -/
@[simp] theorem LoanDiscipline.mk_self_add (state : RuntimeState)
    (globals : GlobalMap) (pending : Array (Nat × RuntimeValue))
    (offset : Nat) :
    LoanDiscipline state
      { globals, globalLoans := state.globalLoans,
        nextLoan := state.nextLoan + offset, pending } :=
  LoanDiscipline.of_eq rfl (Nat.le_add_right _ _)

/-- A body whose one surviving loan was minted inside it keeps the
discipline: the registration is invisible below the entry `nextLoan` and
inside the final frontier. -/
theorem LoanDiscipline.of_registered {initial final : RuntimeState}
    {registered : Nat} {key : GlobalKey}
    (loans_eq : final.globalLoans =
      (registered, key) :: initial.globalLoans)
    (minted : initial.nextLoan ≤ registered)
    (live : registered < final.nextLoan) :
    LoanDiscipline initial final := by
  refine ⟨fun fresh loan h => ?_, fun loan h => ?_, by omega⟩
  · rw [loans_eq]
    have different : (registered == loan) = false := by
      simp only [beq_eq_false_iff_ne]
      omega
    simp only [globalLoanKeyIn?, List.find?_cons, different]
    exact fresh loan (by omega)
  · rw [loans_eq]
    have different : (registered == loan) = false := by
      simp only [beq_eq_false_iff_ne]
      omega
    simp only [globalLoanKeyIn?, List.find?_cons, different]

/-- Exact-next form of prior/future loan separation. -/
theorem priorLoan_beq_next_false {loan nextLoan : Nat}
    (prior : loan < nextLoan) :
    (loan == nextLoan) = false := by
  simp
  omega

/-- Symmetric exact-next form of prior/future loan separation. -/
theorem nextLoan_beq_prior_false {loan nextLoan : Nat}
    (prior : loan < nextLoan) :
    (nextLoan == loan) = false := by
  simp
  omega

/-- Propositional exact-next separation, used by `bne`-based filters. -/
theorem priorLoan_ne_next {loan nextLoan : Nat}
    (prior : loan < nextLoan) : loan ≠ nextLoan := by
  omega

/-- Symmetric propositional exact-next separation. -/
theorem nextLoan_ne_prior {loan nextLoan : Nat}
    (prior : loan < nextLoan) : nextLoan ≠ loan := by
  omega

/-- A loan that was already live before `nextLoan` cannot equal any dynamic
identifier minted from `nextLoan` onward. -/
theorem priorLoan_beq_future_false {loan nextLoan offset : Nat}
    (prior : loan < nextLoan) :
    (loan == nextLoan + offset) = false := by
  simp
  omega

/-- Symmetric comparison form of `priorLoan_beq_future_false`. -/
theorem futureLoan_beq_prior_false {loan nextLoan offset : Nat}
    (prior : loan < nextLoan) :
    (nextLoan + offset == loan) = false := by
  simp
  omega

/-- Propositional separation from every future minted identifier. -/
theorem priorLoan_ne_future {loan nextLoan offset : Nat}
    (prior : loan < nextLoan) : loan ≠ nextLoan + offset := by
  omega

/-- Symmetric propositional separation from every future identifier. -/
theorem futureLoan_ne_prior {loan nextLoan offset : Nat}
    (prior : loan < nextLoan) : nextLoan + offset ≠ loan := by
  omega

/-- A newly registered global loan resolves to its recorded slot directly.
This is the native routing certificate used when a whole-resource borrow is
finalized; no search through either global memory or the registry survives. -/
theorem globalLoanKey?_registered (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan : Nat)
    (pending : Array (Nat × RuntimeValue)) (loan : Nat) (key : GlobalKey) :
    globalLoanKey?
      { globals
        globalLoans := (loan, key) :: rest
        nextLoan
        pending }
      loan = some key := by
  simp [globalLoanKey?, globalLoanKeyIn?]

/-- Drop one loan's registry entry. -/
def removeGlobalLoan : List (Nat × GlobalKey) → Nat → List (Nat × GlobalKey)
  | [], _ => []
  | entry :: rest, loan =>
      if entry.1 == loan then rest
      else entry :: removeGlobalLoan rest loan

/-- Removing a loan the registry does not hold leaves it unchanged. -/
theorem removeGlobalLoan_of_free (loans : List (Nat × GlobalKey)) (loan : Nat)
    (free : globalLoanKeyIn? loans loan = none) :
    removeGlobalLoan loans loan = loans := by
  induction loans with
  | nil => rfl
  | cons entry rest ih =>
      by_cases h : (entry.1 == loan) = true
      · exfalso
        simp [globalLoanKeyIn?, List.find?_cons, h] at free
      · simp only [Bool.not_eq_true] at h
        have restFree : globalLoanKeyIn? rest loan = none := by
          simpa only [globalLoanKeyIn?, List.find?_cons, h, Bool.false_eq_true,
            if_false] using free
        simp only [removeGlobalLoan, h, Bool.false_eq_true, if_false, ih restFree]

/-- Retirement changes only the selected identity's lookup. No registry
shape or resource payload is exposed to consumers of this law. -/
theorem globalLoanKeyIn?_remove_other (loans : List (Nat × GlobalKey))
    (retired loan : Nat) (different : retired ≠ loan) :
    globalLoanKeyIn? (removeGlobalLoan loans retired) loan =
      globalLoanKeyIn? loans loan := by
  induction loans with
  | nil => rfl
  | cons entry rest ih =>
      by_cases removed : entry.1 = retired
      · simp [removeGlobalLoan, globalLoanKeyIn?, removed, different]
      · by_cases matched : entry.1 = loan
        · simp [removeGlobalLoan, globalLoanKeyIn?, matched, Ne.symm different]
        · simpa [removeGlobalLoan, globalLoanKeyIn?, removed, matched] using ih

/-- Whether the hole of `loan` sits in a global slot. A contract over a
borrow-taking function assumes this is false for its argument loans: the
lender's hole lives in a caller frame, so the loan's death exports through
the pending set rather than writing a global. -/
def holeInGlobals (state : RuntimeState) (loan : Nat) : Bool :=
  (globalLoanKey? state loan).isSome

/-- Index of the first row a predicate accepts, counting from `index`.
Structural recursion keeps searches over lowered literal rows transparent;
`Array.findIdx?`/`List.findIdx?` use well-founded recursion and are opaque to
the proof-facing reduction path. -/
@[simp] private def indexOfFrom {α : Type} (accepts : α → Bool) :
    List α → Nat → Option Nat
  | [], _ => none
  | entry :: rest, index =>
      if accepts entry then some index else indexOfFrom accepts rest (index + 1)

/-- Relate the evaluator's structural search to a public list certificate.
The executable search itself remains unchanged. -/
private theorem indexOfFrom_eq_findIdx? {α : Type} (accepts : α → Bool)
    (xs : List α) (offset : Nat) :
    indexOfFrom accepts xs offset = (xs.findIdx? accepts).map (· + offset) := by
  induction xs generalizing offset with
  | nil => simp [indexOfFrom]
  | cons x xs ih =>
    simp only [indexOfFrom, List.findIdx?_cons]
    split <;> simp_all [Option.map_map, Function.comp_def, Nat.add_comm, Nat.add_left_comm]

/-- Whether the hole of `loan` sits inside this frame's locals. -/
def holeInFrame (frame : RuntimeFrame) (loan : Nat) : Bool :=
  (indexOfFrom (fun slot => (slot.map (holeWithin loan)).getD false)
    frame.locals.toList 0).isSome

/-- The evaluation form of `holeInFrame`, keyed on the frame constructor:
a symbolic frame stays folded — so no search over it is ever exposed —
while a built frame computes. -/
theorem holeInFrame_mk {locals : Array (Option RuntimeValue)}
    {activeLoans : Array (ExprId × Nat)}
    {loanLocations : Array (Nat × RuntimePlace)}
    {typeInstantiation : Array (TypeId × TypeId)} {loan : Nat} :
    holeInFrame ⟨locals, activeLoans, loanLocations, typeInstantiation⟩ loan =
      (indexOfFrom (fun slot => (slot.map (holeWithin loan)).getD false)
        locals.toList 0).isSome :=
  rfl

/-- The already resolved local place associated with a dynamic loan.  This
cache is an execution index, not ownership state: every use validates the
value at the recorded place and falls back to the semantic search if the
entry became stale after a move. -/
def localLoanPlace? (frame : RuntimeFrame) (loan : Nat) : Option RuntimePlace := do
  let (_, place) ← frame.loanLocations.findRev? (·.1 == loan)
  let .local _ := place.root | none
  some place

/-- A callee that returns a reborrow exports its lender current with the
returned dynamic loan's hole in it. Recovering that identifier from the
value is the path-free transfer certificate: the hole itself, not an owner
path, says which dynamic loan now occupies the caller's lexical lifetime. -/
def transferredLoan? (replacement : RuntimeValue) : Option Nat :=
  findFirst anyHole? replacement

/-- Retarget a keyed global loan when write-back installs a value carrying a
new prophecy hole.  The registry retains only the owning storage key; the
hole itself identifies the returned dynamic loan and its position inside the
resource, so no projection path is transferred to the reference. -/
def transferGlobalLoan (globalLoans : List (Nat × GlobalKey))
    (completed : Nat) (key : GlobalKey) (replacement : RuntimeValue) :
    List (Nat × GlobalKey) :=
  let retained := removeGlobalLoan globalLoans completed
  match transferredLoan? replacement with
  | some transferred => (transferred, key) :: retained
  | none => retained

/-- Retiring a loan minted inside the body keeps the discipline: its
registration was invisible below the entry `nextLoan`, and removing it
restores freshness of every id at or beyond the final frontier. -/
theorem LoanDiscipline.of_retired {initial final : RuntimeState}
    {retired : Nat} {key : GlobalKey} {rest : List (Nat × GlobalKey)}
    (discipline : LoanDiscipline initial final)
    (loans_eq : final.globalLoans = (retired, key) :: rest)
    (minted : initial.nextLoan ≤ retired) :
    LoanDiscipline initial { final with globalLoans := rest } := by
  obtain ⟨fresh, stable, monotone⟩ := discipline
  refine ⟨fun freshInitial loan h => ?_, fun loan h => ?_, monotone⟩
  · have := fresh freshInitial loan h
    rw [loans_eq] at this
    simp only [globalLoanKeyIn?, List.find?_cons] at this ⊢
    split at this
    · simp at this
    · exact this
  · have := stable loan h
    rw [loans_eq] at this
    have different : (retired == loan) = false := by
      simp only [beq_eq_false_iff_ne]
      omega
    simp only [globalLoanKeyIn?, List.find?_cons, different] at this
    simpa only [globalLoanKeyIn?] using this

/-- A body that hands its one minted registry entry to a returned reborrow
keeps the discipline.  This is the exit spelling of a returned global
`&mut`: the completed loan's own registration is removed and the surviving
hole re-registers under the same storage key — both identifiers minted
inside the body. -/
theorem LoanDiscipline.of_transfer {initial final : RuntimeState}
    {completed transferred : Nat} {entry : GlobalKey} {key : GlobalKey}
    {replacement : RuntimeValue}
    (loans_eq : final.globalLoans =
      transferGlobalLoan ((completed, entry) :: initial.globalLoans)
        completed key replacement)
    (transfer_eq : transferredLoan? replacement = some transferred)
    (transferred_minted : initial.nextLoan ≤ transferred)
    (live : transferred < final.nextLoan) :
    LoanDiscipline initial final := by
  have spelled : final.globalLoans =
      (transferred, key) :: initial.globalLoans := by
    rw [loans_eq]
    simp only [transferGlobalLoan, removeGlobalLoan, beq_self_eq_true,
      if_pos, transfer_eq]
  exact LoanDiscipline.of_registered spelled transferred_minted live

/-- Retarget the caller's explicit lexical death marker when a callee turns
one live loan into a returned reborrow. Ordinary write-backs contain no hole
and simply retire the completed dynamic loan. -/
def transferActiveLoan (activeLoans : Array (ExprId × Nat))
    (loan : Nat) (replacement : RuntimeValue) : Array (ExprId × Nat) :=
  let retained := activeLoans.filter (·.2 != loan)
  match transferredLoan? replacement,
      activeLoans.findRev? (·.2 == loan) with
  | some transferred, some (lexical, _) => retained.push (lexical, transferred)
  | _, _ => retained

/-- Retarget the optional native address index in parallel with the lexical
death marker. This index remains an optimization: the semantic identity of
the transferred loan came from the hole carried in `replacement`. -/
def transferLoanLocation (loanLocations : Array (Nat × RuntimePlace))
    (loan : Nat) (replacement : RuntimeValue) : Array (Nat × RuntimePlace) :=
  let retained := loanLocations.filter (·.1 != loan)
  match transferredLoan? replacement,
      loanLocations.findRev? (·.1 == loan) with
  | some transferred, some (_, place) => retained.push (transferred, place)
  | _, _ => retained

/-- Fill a registered local hole directly.  The location was recorded when
the hole was created, so no frame-wide search is involved.  Reconciliation
also retires the dynamic loan from both native indexes: retaining its
lexical-site row would make a later `endLoans?` try to rediscover a borrow
that the callee has already returned. -/
def fillLocalLoanHole? (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) :
    Option (RuntimeFrame × RuntimeState) := do
  let place ← localLoanPlace? frame loan
  let value ← readRuntimePlace? frame state place
  let filled ← fillHole? loan replacement value
  let (frame, _) ← writeRuntimePlace? frame state place filled
  some ({ frame with
    activeLoans := transferActiveLoan frame.activeLoans loan replacement
    loanLocations := transferLoanLocation frame.loanLocations loan replacement }, state)

/-- Fill the hole of `loan` wherever it is visible from this frame: the
locals first, then the global slots. Each scan sits behind its visibility
predicate, so a contract hypothesis refuting `holeInFrame` or
`holeInGlobals` decides the branch without the scan ever computing. -/
def fillVisibleHole (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) :
    RuntimeFrame × RuntimeState × Bool :=
  if holeInFrame frame loan then
    match indexOfFrom (fun slot =>
        (slot.map (holeWithin loan)).getD false) frame.locals.toList 0 with
    | some index =>
        match frame.locals[index]?.bind (fun slot =>
            slot.bind (fillHole? loan replacement)) with
        | some filled =>
            ({ frame with locals := frame.locals.set! index (some filled) }, state, true)
        | none => (frame, state, false)
    | none => (frame, state, false)
  else
    -- A global loan's hole is its whole slot, so the keyed write replaces
    -- the slot and retires the registry entry.  A contract states the
    -- absence of a global loan as this lookup being `none`, which rewrites
    -- the scrutinee: the branch is decided without a case analysis.
    match globalLoanKey? state loan with
    | some key =>
        let filled :=
          match state.globals.lookup key with
          | some stored => (fillHole? loan replacement stored).getD replacement
          | none => replacement
        (frame,
          { state with
            globals := state.globals.insert key filled
            globalLoans := transferGlobalLoan state.globalLoans loan key replacement },
          true)
    | none => (frame, state, false)

/-- Reunite one loan's hole with its final value, or export the write-back
to an ancestor frame through the pending set. -/
def applyWriteBack (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (current : RuntimeValue) : RuntimeFrame × RuntimeState :=
  let (frame, state, found) := fillVisibleHole frame state loan current
  if found then (frame, state)
  else (frame, { state with pending := state.pending.push (loan, current) })

/-- A write-back with no visible hole is exactly a pending export. This is
the equation a caller-side contract uses: with `holeInFrame` computed away
and `holeInGlobals` refuted by the contract's precondition, a dying loan's
reconciliation is the observable `pending.push`. -/
theorem applyWriteBack_export {frame : RuntimeFrame} {state : RuntimeState}
    {loan : Nat} {value : RuntimeValue}
    (noLocalHole : holeInFrame frame loan = false)
    (noGlobalHole : globalLoanKey? state loan = none) :
    applyWriteBack frame state loan value =
      (frame, { state with pending := state.pending.push (loan, value) }) := by
  unfold applyWriteBack fillVisibleHole
  simp [noLocalHole, noGlobalHole]

/-- Function finalization writes an escaping loan back with an empty local
frame.  At that boundary the only remaining routing fact is the native
global-loan key lookup; when it is absent, the value is exported to the
pending array directly. -/
theorem applyWriteBack_empty_export {state : RuntimeState} {loan : Nat}
    {value : RuntimeValue}
    (noGlobalHole : globalLoanKey? state loan = none) :
    applyWriteBack ({} : RuntimeFrame) state loan value =
      (({} : RuntimeFrame),
        { state with pending := state.pending.push (loan, value) }) := by
  apply applyWriteBack_export
  · simp [holeInFrame]
  · exact noGlobalHole

/-- Closed routing equation for a write-back emitted by a dying function
frame.  There is no local search at this boundary: the native loan registry
either names the global slot directly or the value is exported to an
ancestor frame. -/
theorem applyWriteBack_empty (state : RuntimeState) (loan : Nat)
    (value : RuntimeValue) :
    applyWriteBack ({} : RuntimeFrame) state loan value =
      match globalLoanKey? state loan with
      | some key =>
          let filled :=
            match state.globals.lookup key with
            | some stored => (fillHole? loan value stored).getD value
            | none => value
          (({} : RuntimeFrame),
            { state with
              globals := state.globals.insert key filled
              globalLoans := transferGlobalLoan state.globalLoans loan key value })
      | none =>
          (({} : RuntimeFrame),
            { state with pending := state.pending.push (loan, value) }) := by
  cases lookup_eq : globalLoanKey? state loan <;>
    simp [applyWriteBack, fillVisibleHole, holeInFrame, lookup_eq]

/-- Reconcile one write-back which has already crossed a function boundary.
Such an entry can only target an ancestor frame: global loans are keyed and
are reconciled by `exportFrameLoans` before they ever enter `pending`.
Restricting this second-stage operation to the caller frame makes that
semantic invariant native instead of relying on arbitrary `RuntimeState`
values to satisfy it. -/
def applyPendingWriteBack (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (current : RuntimeValue) : RuntimeFrame × RuntimeState :=
  match fillLocalLoanHole? frame state loan current with
  | some resolved => resolved
  | none =>
      if holeInFrame frame loan then
        match indexOfFrom (fun slot =>
            (slot.map (holeWithin loan)).getD false) frame.locals.toList 0 with
        | some index =>
            match frame.locals[index]?.bind (fun slot =>
                slot.bind (fillHole? loan current)) with
            | some filled =>
                ({ frame with
                    locals := frame.locals.set! index (some filled)
                    activeLoans := transferActiveLoan frame.activeLoans loan current
                    loanLocations := transferLoanLocation frame.loanLocations loan current },
                  state)
            | none => (frame,
                { state with pending := state.pending.push (loan, current) })
        | none => (frame,
            { state with pending := state.pending.push (loan, current) })
      else
        (frame, { state with pending := state.pending.push (loan, current) })

/-- Apply the write-backs a callee appended after `inherited`.  Entries that
were already pending when the call began belong to an older ancestor and are
preserved without being replayed at this boundary. -/
def applyPendingFrom (inherited : Array (Nat × RuntimeValue))
    (frame : RuntimeFrame) (state : RuntimeState) :
    RuntimeFrame × RuntimeState :=
  (state.pending.extract inherited.size state.pending.size).foldl
    (init := (frame, { state with pending := inherited }))
    fun (frame, state) (loan, current) =>
      applyPendingWriteBack frame state loan current

/-- A callee exporting exactly one new write-back is reconciled in one
native step.  This equation keeps the caller frame concrete; unfolding the
generic fold would make its accumulator symbolic and hide the registered
loan location from reduction. -/
theorem applyPendingFrom_single {inherited : Array (Nat × RuntimeValue)}
    {frame : RuntimeFrame} {state : RuntimeState} {loan : Nat}
    {current : RuntimeValue}
    (pending_eq : state.pending = inherited.push (loan, current)) :
    applyPendingFrom inherited frame state =
      applyPendingWriteBack frame { state with pending := inherited }
        loan current := by
  unfold applyPendingFrom
  rw [pending_eq]
  rw [Array.extract_push]
  have too_large : ¬ inherited.size + 1 ≤ inherited.size := by omega
  simp [too_large]

/-- A callee exporting nothing leaves the caller's frame and pending as
they were. -/
theorem applyPendingFrom_none {inherited : Array (Nat × RuntimeValue)}
    {frame : RuntimeFrame} {state : RuntimeState}
    (pending_eq : state.pending = inherited) :
    applyPendingFrom inherited frame state =
      (frame, { state with pending := inherited }) := by
  unfold applyPendingFrom
  rw [pending_eq]
  simp

/-- Constructor-shaped form of `applyPendingFrom_single`.  Finalization
builds a fresh runtime-state record, so this equation exposes the one-entry
write-back without asking proof normalization to reconstruct a record-field
equality first. -/
theorem applyPendingFrom_push (inherited : Array (Nat × RuntimeValue))
    (frame : RuntimeFrame) (globals : GlobalMap)
    (globalLoans : List (Nat × GlobalKey)) (nextLoan : Nat)
    (loan : Nat) (current : RuntimeValue) :
    applyPendingFrom inherited frame
        { globals, globalLoans, nextLoan
          pending := inherited.push (loan, current) } =
      applyPendingWriteBack frame
        { globals, globalLoans, nextLoan, pending := inherited }
        loan current := by
  apply applyPendingFrom_single
  rfl

/-- Two callee exports are reconciled in append order without exposing the
generic pending-array fold.  Lowering uses this constructor equation as the
small structural bridge before selecting native caller locations. -/
theorem applyPendingFrom_two_push
    (inherited : Array (Nat × RuntimeValue))
    (frame : RuntimeFrame) (globals : GlobalMap)
    (globalLoans : List (Nat × GlobalKey))
    (nextLoan firstLoan secondLoan : Nat)
    (firstValue secondValue : RuntimeValue) :
    applyPendingFrom inherited frame
        { globals, globalLoans, nextLoan
          pending := inherited.push (firstLoan, firstValue)
            |>.push (secondLoan, secondValue) } =
      let first := applyPendingWriteBack frame
        { globals, globalLoans, nextLoan, pending := inherited }
        firstLoan firstValue
      applyPendingWriteBack first.1 first.2 secondLoan secondValue := by
  have tooLarge : ¬ inherited.size + 1 ≤ inherited.size := by omega
  simp [applyPendingFrom, Array.extract_push, tooLarge]

/-- Native reconciliation certificate for a mutable reborrow of local zero.
The caller frame contains the lender borrow and the freshly minted hole; the
location cache names the dereference directly.  A one-entry callee export
therefore updates that exact current value and retires only the fresh cache
entry, without any search through locals or values. -/
theorem applyPendingFrom_derefLocalZero
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat) (replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow outerLoan (.loanHole loan))]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement)]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  rw [applyPendingFrom_push]
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?,
    writeProjections?, separate]

/-- Fresh-loan form of `applyPendingFrom_derefLocalZero`.  Function-call
verification already carries the ordered allocation invariant, so this
version discharges by assumption and keeps the write-back constructor-shaped
before its continuation is opened. -/
theorem applyPendingFrom_derefLocalZero_of_lt
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat) (replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (fresh : outerLoan < loan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow outerLoan (.loanHole loan))]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement)]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  exact applyPendingFrom_derefLocalZero inherited globals globalLoans
    runtimeNextLoan outerLoan loan replacement activeLoans
      (Nat.ne_of_lt fresh)

/-- Apply a complete pending array, used at a boundary with no inherited
write-backs.  Calls use `applyPendingFrom` with their operand state's pending
prefix so only callee exports are reconciled. -/
def applyPending (frame : RuntimeFrame) (state : RuntimeState) :
    RuntimeFrame × RuntimeState :=
  applyPendingFrom #[] frame state

theorem applyPendingWriteBack_globals (frame : RuntimeFrame)
    (state : RuntimeState) (loan : Nat) (current : RuntimeValue) :
    (applyPendingWriteBack frame state loan current).2.globals = state.globals := by
  unfold applyPendingWriteBack
  split
  · rename_i resolved result_eq
    simp only [fillLocalLoanHole?] at result_eq
    rcases Option.bind_eq_some_iff.mp result_eq with ⟨place, -, result_eq⟩
    rcases Option.bind_eq_some_iff.mp result_eq with ⟨value, -, result_eq⟩
    rcases Option.bind_eq_some_iff.mp result_eq with ⟨filled, -, result_eq⟩
    rcases Option.bind_eq_some_iff.mp result_eq with ⟨written, -, result_eq⟩
    simp only [Option.some.injEq] at result_eq
    subst resolved
    rfl
  · split
    · split
      · split <;> rfl
      · rfl
    · rfl

/-- Applying callee exports can update the caller's native local locations,
but never global storage.  The global write-back, when one exists, was
already performed during callee finalization through its keyed slot. -/
theorem applyPendingFrom_globals (inherited : Array (Nat × RuntimeValue))
    (frame : RuntimeFrame) (state : RuntimeState) :
    (applyPendingFrom inherited frame state).2.globals = state.globals := by
  let step : (RuntimeFrame × RuntimeState) → (Nat × RuntimeValue) →
      RuntimeFrame × RuntimeState :=
    fun (frame, state) (loan, current) =>
      applyPendingWriteBack frame state loan current
  have folded := Array.foldl_hom
    (xs := state.pending.extract inherited.size state.pending.size)
    (init := (frame, { state with pending := inherited }))
    (g₁ := step) (g₂ := fun globals _ => globals)
    (fun pair => pair.2.globals)
    (fun pair entry => by
      rcases pair with ⟨frame, state⟩
      rcases entry with ⟨loan, current⟩
      exact (applyPendingWriteBack_globals frame state loan current).symm)
  have unchanged :
      (state.pending.extract inherited.size state.pending.size).foldl
          (fun globals _ => globals) state.globals =
        state.globals := by
    apply Array.foldl_induction
      (motive := fun _ globals => globals = state.globals)
    · rfl
    · intro _ globals established
      exact established
  rw [unchanged] at folded
  simpa [applyPendingFrom, step] using folded.symm

@[simp] theorem applyPendingFrom_self (frame : RuntimeFrame)
    (state : RuntimeState) :
    applyPendingFrom state.pending frame state = (frame, state) := by
  simp [applyPendingFrom]

/-- Constructor-shaped no-export reconciliation.  Native call lowering often
retains the state fields explicitly, so exposing the unchanged pending prefix
avoids a proof-time record projection without weakening the fail-closed
boundary. -/
theorem applyPendingFrom_samePending
    (inherited : Array (Nat × RuntimeValue)) (frame : RuntimeFrame)
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (nextLoan : Nat) :
    applyPendingFrom inherited frame
        { globals, globalLoans, nextLoan, pending := inherited } =
      (frame, { globals, globalLoans, nextLoan, pending := inherited }) := by
  simp [applyPendingFrom]

theorem applyPending_globals (frame : RuntimeFrame) (state : RuntimeState) :
    (applyPending frame state).2.globals = state.globals := by
  exact applyPendingFrom_globals #[] frame state

/-- Update a borrow at its registered local place, validating the cached
address before writing.  A stale cache entry returns `none`, preserving the
search-based semantic fallback. -/
def updateLocalBorrowValue? (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) :
    Option (RuntimeFrame × RuntimeState) := do
  let place ← localLoanPlace? frame loan
  let value ← readRuntimePlace? frame state place
  let updated ← rewriteFirst (borrowRewrite? loan replacement) value
  writeRuntimePlace? frame state place updated

/-- No cached address requires no traversal of a mutation's write path. -/
theorem updateLocalBorrowValue_no_location (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue)
    (location : localLoanPlace? frame loan = none) :
    updateLocalBorrowValue? frame state loan replacement = none := by
  simp [updateLocalBorrowValue?, location]

/-- A stale address can designate a local that has already been consumed. -/
theorem updateLocalBorrowValue_no_read (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) (place : RuntimePlace)
    (location : localLoanPlace? frame loan = some place)
    (read : readRuntimePlace? frame state place = none) :
    updateLocalBorrowValue? frame state loan replacement = none := by
  simp [updateLocalBorrowValue?, location, read]

/-- A cached hole or unrelated value cannot be rewritten as this borrow.
Keep the unreachable write operation out of each generated certificate. -/
theorem updateLocalBorrowValue_no_rewrite (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement value : RuntimeValue) (place : RuntimePlace)
    (location : localLoanPlace? frame loan = some place)
    (read : readRuntimePlace? frame state place = some value)
    (absent : rewriteFirst (borrowRewrite? loan replacement) value = none) :
    updateLocalBorrowValue? frame state loan replacement = none := by
  simp [updateLocalBorrowValue?, location, read, absent]

/-- Update the current value of the live borrow `loan` wherever it rests in
the frame locals or global slots, descending into borrow currents. `none`
when the borrow is not at rest — a consumed temporary. -/
def updateBorrowValue? (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) : Option (RuntimeFrame × RuntimeState) :=
  match updateLocalBorrowValue? frame state loan replacement with
  | some updated => some updated
  | none =>
      let update (value : RuntimeValue) : Option RuntimeValue :=
        rewriteFirst (borrowRewrite? loan replacement) value
      match indexOfFrom (fun slot => (slot.bind update).isSome)
          frame.locals.toList 0 with
      | some index =>
          (frame.locals[index]?.bind (fun slot => slot.bind update)).map fun updated =>
            ({ frame with locals := frame.locals.set! index (some updated) }, state)
      | none =>
          match indexOfFrom (fun slot => (update slot.value).isSome)
              state.globals.entries.toList 0 with
          | some index =>
              state.globals.entries[index]?.bind fun slot =>
                (update slot.value).map fun value =>
                  (frame, { state with
                    globals := ⟨state.globals.entries.set! index { slot with value }⟩ })
          | none => none

/-- A stale cache does not prevent a resting borrow from being mutated.
The certificate identifies the first matching local in an arbitrary frame;
unrelated suffix locals and global storage remain untouched. -/
theorem updateBorrowValue_uncached_local (frame : RuntimeFrame) (state : RuntimeState)
    (slot : LocalId) (loan : Nat) (current replacement : RuntimeValue)
    (cached : updateLocalBorrowValue? frame state loan replacement = none)
    (found : frame.locals.toList.findIdx? (fun value =>
      (value.bind (rewriteFirst (borrowRewrite? loan replacement))).isSome) = some slot.index)
    (read : frame.locals[slot.index]? = some (some (.borrow loan current))) :
    updateBorrowValue? frame state loan replacement =
      some ({ frame with
        locals := frame.locals.set! slot.index (some (.borrow loan replacement)) }, state) := by
  simp [updateBorrowValue?, cached, indexOfFrom_eq_findIdx?, found, read,
    rewriteFirst, borrowRewrite?]

/-- Remove the resting occurrence of the ended borrow `loan`: a reconciled
loan's stale borrow value must not be exported again when its holder's
frame dies. -/
def clearBorrowValue (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) : RuntimeFrame × RuntimeState :=
  let clear (value : RuntimeValue) : Option RuntimeValue :=
    rewriteFirst (borrowClear? loan) value
  match indexOfFrom (fun slot => (slot.bind clear).isSome)
      frame.locals.toList 0 with
  | some index =>
      match frame.locals[index]?.bind (fun slot => slot.bind clear) with
      | some updated =>
          ({ frame with locals := frame.locals.set! index (some updated) }, state)
      | none => (frame, state)
  | none =>
      match indexOfFrom (fun slot => (clear slot.value).isSome)
          state.globals.entries.toList 0 with
      | some index =>
          match state.globals.entries[index]?.bind (fun slot =>
              (clear slot.value).map fun value => { slot with value }) with
          | some updated =>
              (frame, { state with globals := ⟨state.globals.entries.set! index updated⟩ })
          | none => (frame, state)
      | none => (frame, state)

/-- A certified first local holder can be cleared without revisiting the
global fallback. The certificate preserves the semantic search order. -/
theorem clearBorrowValue_local (frame : RuntimeFrame) (state : RuntimeState)
    (slot : LocalId) (loan : Nat) (current : RuntimeValue)
    (found : frame.locals.toList.findIdx? (fun value =>
      (value.bind (rewriteFirst (borrowClear? loan))).isSome) = some slot.index)
    (read : frame.locals[slot.index]? = some (some (.borrow loan current))) :
    clearBorrowValue frame state loan =
      ({ frame with locals := frame.locals.set! slot.index (some .unit) }, state) := by
  simp [clearBorrowValue, indexOfFrom_eq_findIdx?, found, read,
    rewriteFirst, borrowClear?]

/-- The current value of the live borrow `loan`, searched through the frame
locals and global slots, descending into borrow currents. -/
def findBorrowValue? (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) : Option RuntimeValue :=
  let find (value : RuntimeValue) : Option RuntimeValue :=
    findFirst (borrowCurrent? loan) value
  match frame.locals.findSome? (fun slot => slot.bind find) with
  | some current => some current
  | none => state.globals.entries.findSome? fun slot => find slot.value

/-- Pruned-collection entry of an outermost borrow.  Named so every
occurrence of the collector is one constant with a stable rewrite identity. -/
def borrowEntry? : RuntimeValue → Option (Option (Nat × RuntimeValue))
  | .borrow loan current => some (some (loan, current))
  | _ => none

/-- Outermost mutable borrows at rest in a value: the loans a dying frame
must export. Loans nested inside another borrow's current travel with it. -/
def outermostBorrows (value : RuntimeValue) : Array (Nat × RuntimeValue) :=
  collectPruned borrowEntry? value |>.filterMap id

/-- Reconstruct the logical value visible after all mutable references in a
returned row die.  Each returned borrow contributes only its dynamic loan id
and current value; filling that id's prophecy hole in a parameter export does
not require (or retain) an owner root or projection path.  A returned global
borrow simply has no hole in a parameter export and leaves it unchanged. -/
def resolveReturnedBorrows (results : Array RuntimeValue)
    (replacement : RuntimeValue) : RuntimeValue :=
  results.foldl (init := replacement) fun resolved result =>
    (outermostBorrows result).foldl (init := resolved) fun resolved returned =>
      (fillHole? returned.1 returned.2 resolved).getD resolved

/-- A callee with no returned references exports its write-back as-is. -/
theorem resolveReturnedBorrows_empty (replacement : RuntimeValue) :
    resolveReturnedBorrows #[] replacement = replacement := rfl

/-- A callee returning one plain integer exports its write-back as-is. -/
theorem resolveReturnedBorrows_integer (value : Int)
    (replacement : RuntimeValue) :
    resolveReturnedBorrows #[.integer value] replacement = replacement := by
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, borrowEntry?]

/-- A callee returning one borrow fills that loan's hole in the export,
whatever the export's spelling: the general row the literal spellings
evaluate through. -/
theorem resolveReturnedBorrows_singleBorrow (loan : Nat) (current replacement : RuntimeValue) :
    resolveReturnedBorrows #[.borrow loan current] replacement =
      (fillHole? loan current replacement).getD replacement := by
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, borrowEntry?]

/-- A callee returning a reborrow fills its lender's hole with the returned
current: the prophecy the lender exported is the returned loan's. -/
theorem resolveReturnedBorrows_returnedBorrow (loan : Nat) (value : Int) :
    resolveReturnedBorrows #[.borrow loan (.integer value)] (.loanHole loan) =
      .integer value := by
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, borrowEntry?,
    fillHole?, rewriteFirst]

/-- A callee returning a pair of reborrows fills each lender's hole with its
own returned current, the first lender's from the first component. -/
theorem resolveReturnedBorrows_returnedPair_left (leftLoan rightLoan : Nat)
    (left right : Int) (separate : leftLoan ≠ rightLoan) :
    resolveReturnedBorrows
        #[.tuple #[.borrow leftLoan (.integer left), .borrow rightLoan (.integer right)]]
        (.loanHole leftLoan) =
      .integer left := by
  have separate' : rightLoan ≠ leftLoan := Ne.symm separate
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList,
    borrowEntry?, fillHole?, rewriteFirst, separate']

/-- The second lender's hole is filled from the second component. -/
theorem resolveReturnedBorrows_returnedPair_right (leftLoan rightLoan : Nat)
    (left right : Int) (separate : leftLoan ≠ rightLoan) :
    resolveReturnedBorrows
        #[.tuple #[.borrow leftLoan (.integer left), .borrow rightLoan (.integer right)]]
        (.loanHole rightLoan) =
      .integer right := by
  have separate' : rightLoan ≠ leftLoan := Ne.symm separate
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, collectPrunedList,
    borrowEntry?, fillHole?, rewriteFirst, separate, separate']

/-- A returned reborrow leaves a lender that exported a plain value as it
is: there is no hole to fill. -/
theorem resolveReturnedBorrows_returnedBorrow_integer (loan : Nat)
    (value exported : Int) :
    resolveReturnedBorrows #[.borrow loan (.integer value)] (.integer exported) =
      .integer exported := by
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, borrowEntry?,
    fillHole?, rewriteFirst]

/-- The pruned borrow collection is all-`none` exactly when the value has no
outermost borrow.  This routes a nested aggregate's collection residual to
the member's own `outermostBorrows` fact. -/
theorem collectPruned_borrows_all_none_iff (value : RuntimeValue) :
    (∀ entry ∈ collectPruned borrowEntry? value, entry = none) ↔
      outermostBorrows value = #[] := by
  simp [outermostBorrows, Array.filterMap_eq_empty_iff]

/-- Outermost borrows the dying frame still holds, in slot order. -/
def frameBorrows (frame : RuntimeFrame) : Array (Nat × RuntimeValue) :=
  frame.locals.foldl (init := #[]) fun borrows slot =>
    match slot with
    | some value => borrows ++ outermostBorrows value
    | none => borrows

/-- Settle the loans whose holes die with the frame: each round moves one
resting current into its in-frame hole, so a value that escapes through an
outer loan carries the mutations its focused holes received.  Every round
consumes one resting borrow, so the initial borrow count bounds the
iteration. -/
def settleFrameLoans (frame : RuntimeFrame) (state : RuntimeState) :
    Nat → RuntimeFrame × RuntimeState
  | 0 => (frame, state)
  | fuel + 1 =>
      match (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
      | none => (frame, state)
      | some (loan, current) =>
          let (frame, state) := clearBorrowValue frame state loan
          let (frame, state, _) := fillVisibleHole frame state loan current
          settleFrameLoans frame state fuel

/-- Write back the loans of a frame whose holes have been settled: a loan
of a global writes back now, and one loaning an ancestor frame joins the
pending set. -/
def exportSettledLoans (frame : RuntimeFrame) (state : RuntimeState) : RuntimeState :=
  (frameBorrows frame).foldl (init := state) fun state (loan, current) =>
    if holeInFrame frame loan then state else
      let (_, state) := applyWriteBack { locals := #[] } state loan current
      state

/-- Export the loans a dying frame still holds.  A frame holding no hole of
its own exports directly; when a hole dies with the frame it is settled
first, so the holder that escapes through an outer loan carries the focused
value. -/
def exportFrameLoans (frame : RuntimeFrame) (state : RuntimeState) : RuntimeState :=
  match (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
  | none => exportSettledLoans frame state
  | some _ =>
      let (frame, state) := settleFrameLoans frame state (frameBorrows frame).size
      exportSettledLoans frame state

/-- A frame holding no borrow exports nothing.  Stated over the borrow
collection rather than a fixed slot layout, this covers every borrow-free
frame at once.  It is deliberately not in the automatic inventory: its side
condition walks the frame, and paying for that walk on every frame that
does hold a borrow costs more than the closed rows it would retire. -/
theorem exportFrameLoans_borrowFree (frame : RuntimeFrame) (state : RuntimeState)
    (borrowFree : frameBorrows frame = #[]) :
    exportFrameLoans frame state = state := by
  simp [exportFrameLoans, exportSettledLoans, borrowFree]

mutual
  /-- Clear the dying frame's copies of returned handles. Do not descend
  into another borrow's current: it is owned by that borrow and must travel
  with its export. The returned values themselves are never changed. -/
  def maskReturnedBorrows (loans : Array Nat) (value : RuntimeValue) : RuntimeValue :=
    match value with
    | .borrow loan _ => if loans.contains loan then .unit else value
    | .vector elements => .vector (maskReturnedBorrowList loans elements.toList).toArray
    | .tuple elements => .tuple (maskReturnedBorrowList loans elements.toList).toArray
    | .nominal source variant fields =>
        .nominal source variant (maskReturnedBorrowList loans fields.toList).toArray
    | .closure function captures =>
        .closure function (maskReturnedBorrowList loans captures.toList).toArray
    | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ |
        .string _ | .bytes _ | .loanHole _ => value
  termination_by sizeOf value
  decreasing_by
    all_goals simp_wf
    all_goals first
      | (have := sizeOf_toList_lt elements; omega)
      | (have := sizeOf_toList_lt fields; omega)
      | (have := sizeOf_toList_lt captures; omega)

  def maskReturnedBorrowList (loans : Array Nat) : List RuntimeValue → List RuntimeValue
    | [] => []
    | value :: rest => maskReturnedBorrows loans value :: maskReturnedBorrowList loans rest
  termination_by values => sizeOf values
  decreasing_by all_goals (simp_wf; omega)
end

/-- Dynamic identities that leave the invocation with its result, not loans
that die with the invocation's frame. This uses the same outermost ownership
boundary as ordinary frame export. -/
def returnedBorrowIds (results : Array RuntimeValue) : Array Nat :=
  results.foldl (init := #[]) fun loans result =>
    loans ++ (outermostBorrows result).map (·.1)

/-- Recognize the common singleton scalar result without inspecting its
payload or the dying frame. Aggregate results retain the loan-aware path. -/
def scalarFunctionResult (results : Array RuntimeValue) : Bool :=
  match results.toList with
  | [.unit] | [.bool _] | [.character _] | [.integer _] | [.address _]
  | [.signer _] | [.string _] | [.bytes _] => true
  | _ => false

theorem scalarFunctionResult_noBorrows (results : Array RuntimeValue)
    (scalar : scalarFunctionResult results = true) : returnedBorrowIds results = #[] := by
  rcases results with ⟨values⟩
  cases values with
  | nil => simp [scalarFunctionResult] at scalar
  | cons value rest =>
      cases rest with
      | cons next rest => simp [scalarFunctionResult] at scalar
      | nil =>
          cases value <;>
            simp [scalarFunctionResult, returnedBorrowIds, outermostBorrows,
              collectPruned, borrowEntry?] at scalar ⊢

/-- A result may copy a reference still present in a local. Such a local is
not a dying loan: its identity has escaped in the result. Mask those local
copies before settling/exporting the other loans, preserving holes in lenders
for the caller to fill. In particular, a global lender transfers its key to
the returned field loan instead of prematurely restoring the old value.

Empty results use the existing export path immediately. A borrow-free frame
returns without traversing a potentially large result. Other results with no
references reuse the existing export path. -/
def exportReturnedFrameLoans (results : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) : RuntimeState :=
  if results.isEmpty then exportFrameLoans frame state else
  if scalarFunctionResult results then exportFrameLoans frame state else
  if (frameBorrows frame).isEmpty then state else
    let loans := returnedBorrowIds results
    if loans.isEmpty then exportFrameLoans frame state else
      exportFrameLoans { frame with
        locals := frame.locals.map (Option.map (maskReturnedBorrows loans)) } state

theorem exportReturnedFrameLoans_borrowFree (results : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) (borrowFree : frameBorrows frame = #[]) :
    exportReturnedFrameLoans results frame state = state := by
  simp [exportReturnedFrameLoans, borrowFree, exportFrameLoans_borrowFree _ _ borrowFree]

theorem exportReturnedFrameLoans_noReturnedBorrows (results : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) (noReturned : returnedBorrowIds results = #[]) :
    exportReturnedFrameLoans results frame state = exportFrameLoans frame state := by
  by_cases h : (frameBorrows frame).isEmpty
  · have empty : frameBorrows frame = #[] := Array.isEmpty_iff.mp h
    simp [exportReturnedFrameLoans, noReturned, h, exportFrameLoans_borrowFree _ _ empty]
  · simp [exportReturnedFrameLoans, noReturned, h]

/-- A frame with no resting aliases of the returned handles uses ordinary
export, independently of its shape or the returned referents' payloads. -/
theorem exportReturnedFrameLoans_noAliases (results : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState)
    (unchanged : frame.locals.map (Option.map (maskReturnedBorrows (returnedBorrowIds results))) =
      frame.locals) :
    exportReturnedFrameLoans results frame state = exportFrameLoans frame state := by
  by_cases h : (frameBorrows frame).isEmpty
  · have empty : frameBorrows frame = #[] := Array.isEmpty_iff.mp h
    simp [exportReturnedFrameLoans, h, exportFrameLoans_borrowFree _ _ empty]
  · simp [exportReturnedFrameLoans, h, unchanged]

/-- Closed finalization rule for the common scalar mutable-reference case.
The parameter location is already native in the frame, and the dying frame
contains exactly one outer borrow.  Consequently finalization exports one
pending write-back directly; neither a local scan nor a fold survives into
the caller's proof. -/
theorem exportFrameLoans_singleInteger
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (inherited : Array (Nat × RuntimeValue))
    (value : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? globalLoans loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer value))]
          activeLoans
          loanLocations }
        { globals, globalLoans, nextLoan, pending := inherited } =
      { globals, globalLoans, nextLoan
        pending := inherited.push (loan, .integer value) } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst, applyWriteBack_empty_export, globalLoanKey?,
    noGlobal]

/-- State-polymorphic form of the scalar finalization certificate.  Direct
functions often retain their incoming state as one opaque value, while a
call constructs the same fields explicitly.  Both shapes reduce through
this equation using the native global-loan registry fact. -/
theorem exportFrameLoans_singleInteger_state
    (state : RuntimeState) (loan : Nat) (value : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? state loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer value))]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (loan, .integer value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  exact exportFrameLoans_singleInteger globals globalLoans nextLoan loan
    inherited value activeLoans loanLocations noGlobal

/-- Finalize the scalar shape produced when a callee returns a reborrow.
The returned reference itself is not stored in the dying frame; its dynamic
loan is visible only as the hole now occupying the outer parameter loan.
Consequently finalization exports that hole to the caller without recording
an owner path. -/
theorem exportFrameLoans_returnedReborrow_state
    (state : RuntimeState) (outerLoan returnedLoan : Nat)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ returnedLoan)
    (noGlobal : globalLoanKey? state outerLoan = none) :
    exportFrameLoans
        { locals := #[some (.borrow outerLoan (.loanHole returnedLoan))]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (outerLoan, .loanHole returnedLoan) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noGlobalIn : globalLoanKeyIn? globalLoans outerLoan = none := noGlobal
  have returnedSeparate : returnedLoan ≠ outerLoan := Ne.symm separate
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    applyWriteBack_empty, globalLoanKey?, noGlobalIn, returnedSeparate]

/-- Finalize a returned reborrow projected from a nominal field.  The
projected loan is represented solely by its prophetic hole in the enclosing
value; no root or projection path is exported to the caller. -/
theorem exportFrameLoans_returnedProjectedReborrow_state
    (state : RuntimeState) (outerLoan returnedLoan : Nat)
    (name : StructHandle) (right : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ returnedLoan)
    (noGlobal : globalLoanKey? state outerLoan = none) :
    exportFrameLoans
        { locals := #[some (.borrow outerLoan
              (.nominal name none
                #[.loanHole returnedLoan, .integer right]))]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push
          (outerLoan,
            .nominal name none
              #[.loanHole returnedLoan, .integer right]) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noGlobalIn : globalLoanKeyIn? globalLoans outerLoan = none := noGlobal
  have returnedSeparate : returnedLoan ≠ outerLoan := Ne.symm separate
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, collectPrunedList, holeInFrame, holeWithin,
    findFirst, findFirstList, applyWriteBack_empty, globalLoanKey?, noGlobalIn,
    returnedSeparate]

/-- Finalize the packed two-result analogue of a returned reborrow.  The
dying callee exports two prophecy holes in parameter order; the result loans
themselves remain live across the boundary and carry no owner path. -/
theorem exportFrameLoans_twoReturnedReborrows_state
    (initial : RuntimeState) (runtimeNextLoan : Nat)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (freshGlobal : FreshGlobalLoanIds initial) :
    exportFrameLoans
        { locals := #[some (.borrow initial.nextLoan
              (.loanHole (initial.nextLoan + 1 + 1))),
            some (.borrow (initial.nextLoan + 1)
              (.loanHole (initial.nextLoan + 1 + 1 + 1)))]
          activeLoans
          loanLocations }
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := initial.pending } =
      { globals := initial.globals
        globalLoans := initial.globalLoans
        nextLoan := runtimeNextLoan
        pending := initial.pending.push
            (initial.nextLoan, .loanHole (initial.nextLoan + 1 + 1))
          |>.push (initial.nextLoan + 1,
            .loanHole (initial.nextLoan + 1 + 1 + 1)) } := by
  have noFirst : globalLoanKeyIn? initial.globalLoans initial.nextLoan = none :=
    freshGlobal.lookup_next
  have noSecond :
      globalLoanKeyIn? initial.globalLoans (initial.nextLoan + 1) = none :=
    freshGlobal.lookup_add 1
  have returnedFirstOuterFirst :
      initial.nextLoan + 1 + 1 ≠ initial.nextLoan := by omega
  have returnedFirstOuterSecond :
      initial.nextLoan + 1 + 1 ≠ initial.nextLoan + 1 := by omega
  have returnedSecondOuterFirst :
      initial.nextLoan + 1 + 1 + 1 ≠ initial.nextLoan := by omega
  have returnedSecondOuterSecond :
      initial.nextLoan + 1 + 1 + 1 ≠ initial.nextLoan + 1 := by omega
  simp [exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, collectPrunedList,
    holeInFrame, holeWithin, findFirst, findFirstList,
    applyWriteBack_empty, globalLoanKey?, noFirst, noSecond,
    returnedFirstOuterFirst, returnedFirstOuterSecond,
    returnedSecondOuterFirst, returnedSecondOuterSecond]

/-- A scalar-only frame has no loan to export.  Finalization is therefore
the identity, independently of its native address bookkeeping. -/
theorem exportFrameLoans_plainInteger_state
    (state : RuntimeState) (value : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.integer value)]
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned]

/-- A moved singleton local leaves no value whose loans could escape.  This
constructor rule is deliberately polymorphic in the moved value: it is the
native finalization certificate used for a storage-parametric generic carrier, so
the proof never has to inspect that carrier's runtime representation. -/
theorem exportFrameLoans_clearedSingleton_state
    (state : RuntimeState) (value : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some value].set! 0 none
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows]

/-- Boolean and address parameters are scalar native values, hence contain
no loans to export when their frame dies. -/
theorem exportFrameLoans_plainBool_state
    (state : RuntimeState) (value : Bool)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.bool value)]
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned]

theorem exportFrameLoans_plainAddress_state
    (state : RuntimeState) (value : String)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.address value)]
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned]

/-- Two scalar parameters finalize without running the generic local fold. -/
theorem exportFrameLoans_plainTwoIntegers_state
    (state : RuntimeState) (first second : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.integer first), some (.integer second)]
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned]

/-- Destructuring a resource into an address plus a one-field scalar value
leaves a borrow-free frame.  This closed constructor certificate is used by
the native `move_from` denotation and hides the generic aggregate walk. -/
theorem exportFrameLoans_addressNominalInteger_state
    (state : RuntimeState) (address : String) (name : StructHandle)
    (variant : Option String) (value : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.address address),
            some (.nominal name variant #[.integer value])]
          activeLoans
          loanLocations }
        state = state := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned,
    collectPrunedList]

/-- Finalize the V1 whole-resource global-borrow shape.  Lowering has fixed
both the local holder and keyed global hole; finalization writes the nested
scalar resource directly to that key and retires its registry row. -/
theorem exportFrameLoans_globalNominalThirdLocal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (coinName amountName : StructHandle)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    exportFrameLoans
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan
              (.nominal coinName none
                #[.nominal amountName none #[.integer value]]))]
          activeLoans
          loanLocations := loanLocations.push
            (loan, { root := .global key }) }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal coinName none
            #[.nominal amountName none #[.integer value]])
        globalLoans := rest
        nextLoan
        pending } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned,
    holeInFrame, holeWithin, findFirst, findFirstList,
    applyWriteBack_empty, globalLoanKey?, globalLoanKeyIn?,
    removeGlobalLoan, transferGlobalLoan, transferredLoan?, fillHole?,
    rewriteFirst, rewriteFirstList]

/-- One-level form of the whole-resource finalization: the written value is
a scalar-field nominal rather than a nested aggregate. -/
theorem exportFrameLoans_globalScalarNominalSingletonLocation
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (coinName : StructHandle)
    (activeLoans : Array (ExprId × Nat)) :
    exportFrameLoans
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan (.nominal coinName none #[.integer value]))]
          activeLoans
          loanLocations := #[(loan, { root := .global key })] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal coinName none #[.integer value])
        globalLoans := rest
        nextLoan
        pending } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned,
    holeInFrame, holeWithin, findFirst, findFirstList,
    applyWriteBack_empty, globalLoanKey?, globalLoanKeyIn?,
    removeGlobalLoan, transferGlobalLoan, transferredLoan?, fillHole?,
    rewriteFirst, rewriteFirstList]

/-- Literal-registry form of `exportFrameLoans_globalNominalThirdLocal`:
push normalization turns the freshly registered global location into a
singleton row before finalization is reconciled. -/
theorem exportFrameLoans_globalNominalSingletonLocation
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (coinName amountName : StructHandle)
    (activeLoans : Array (ExprId × Nat)) :
    exportFrameLoans
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan
              (.nominal coinName none
                #[.nominal amountName none #[.integer value]]))]
          activeLoans
          loanLocations := #[(loan, { root := .global key })] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal coinName none
            #[.nominal amountName none #[.integer value]])
        globalLoans := rest
        nextLoan
        pending } := by
  simpa using exportFrameLoans_globalNominalThirdLocal globals rest nextLoan
    loan pending key address amount value coinName amountName activeLoans #[]

/-- Finalize a field-focused storage borrow.  The frame holds the focused
loan resting on its written value and the holder of the whole resource,
whose value still carries the focused hole.  Settlement moves the written
value into that hole, and the reconciled resource is then written back to
the borrowed key. -/
theorem exportFrameLoans_focusedGlobalNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (outerName innerName : StructHandle)
    (activeLoans : Array (ExprId × Nat)) :
    exportFrameLoans
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨3⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal outerName none
            #[.nominal innerName none #[.integer value]])
        globalLoans := rest
        nextLoan
        pending } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, settleFrameLoans, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    findFirstList, clearBorrowValue, fillVisibleHole, fillHole?, rewriteFirst,
    rewriteFirstList, applyWriteBack_empty, globalLoanKey?, globalLoanKeyIn?,
    removeGlobalLoan, transferGlobalLoan, transferredLoan?]

/-- Finalize the enclosing keyed resource while a projected field reborrow
escapes in the result.  The resource is written back with the returned
loan's prophecy hole; neither the reference nor the call boundary carries a
path to the key. -/
theorem exportFrameLoans_returnedFocusedGlobalNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (right : Int)
    (name : StructHandle) (activeLoans : Array (ExprId × Nat)) :
    exportFrameLoans
        { locals := #[some (.address address),
            some (.borrow loan
              (.nominal name none
                #[.loanHole (loan + 1), .integer right]))]
          activeLoans
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨1⟩ : LocalId), #[.deref, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal name none #[.loanHole (loan + 1), .integer right])
        globalLoans := (loan + 1, key) :: rest
        nextLoan
        pending } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, collectPrunedList, holeInFrame, holeWithin,
    findFirst, findFirstList, applyWriteBack_empty, globalLoanKey?,
    globalLoanKeyIn?, removeGlobalLoan, transferGlobalLoan, transferredLoan?,
    fillHole?, rewriteFirst, rewriteFirstList]

/-- Closed call-boundary reconciliation for a returned global projection.
The callee has already transferred the owning key to the returned prophecy
hole, and there are no newly exported pending writes to replay in the caller. -/
theorem applyPendingFrom_returnedFocusedGlobalNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (right : Int)
    (name : StructHandle) (activeLoans : Array (ExprId × Nat))
    (callerFrame : RuntimeFrame) :
    applyPendingFrom pending callerFrame
        (exportFrameLoans
          { locals := #[some (.address address),
              some (.borrow loan
                (.nominal name none
                  #[.loanHole (loan + 1), .integer right]))]
            activeLoans
            loanLocations := #[(loan, { root := .global key }),
              (loan + 1,
                (⟨.local (⟨1⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace))] }
          { globals := globals.insert key (.loanHole loan)
            globalLoans := (loan, key) :: rest
            nextLoan
            pending }) =
      (callerFrame,
        { globals := (globals.insert key (.loanHole loan)).insert key
            (.nominal name none #[.loanHole (loan + 1), .integer right])
          globalLoans := (loan + 1, key) :: rest
          nextLoan
          pending }) := by
  rw [exportFrameLoans_returnedFocusedGlobalNominal]
  exact applyPendingFrom_samePending pending callerFrame _ _ _

/-- Finalize a field-focused storage borrow whose read was saved in a local
before the mutation: the saved value sits between the focused reference and
its holder. -/
theorem exportFrameLoans_focusedGlobalNominalSaved
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount saved value : Int)
    (outerName innerName : StructHandle)
    (activeLoans : Array (ExprId × Nat)) :
    exportFrameLoans
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)), some (.integer saved),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      { globals := (globals.insert key (.loanHole loan)).insert key
          (.nominal outerName none
            #[.nominal innerName none #[.integer value]])
        globalLoans := rest
        nextLoan
        pending } := by
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, settleFrameLoans, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    findFirstList, clearBorrowValue, fillVisibleHole, fillHole?, rewriteFirst,
    rewriteFirstList, applyWriteBack_empty, globalLoanKey?, globalLoanKeyIn?,
    removeGlobalLoan, transferGlobalLoan, transferredLoan?]

/-- Finalize one scalar mutable parameter while preserving one scalar local.
The single native parameter row determines the only export. -/
theorem exportFrameLoans_borrowInteger_state
    (state : RuntimeState) (loan : Nat) (current saved : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? state loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer current)),
            some (.integer saved)]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (loan, .integer current) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noGlobalIn : globalLoanKeyIn? globalLoans loan = none := noGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst, applyWriteBack_empty, globalLoanKey?, noGlobalIn]

/-- The `Bool` twin of `exportFrameLoans_borrowInteger_state`. -/
theorem exportFrameLoans_borrowBool_state
    (state : RuntimeState) (loan : Nat) (current : Int) (saved : Bool)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? state loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer current)),
            some (.bool saved)]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (loan, .integer current) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noGlobalIn : globalLoanKeyIn? globalLoans loan = none := noGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst, applyWriteBack_empty, globalLoanKey?, noGlobalIn]

/-- Finalize the same mutable parameter shape with two scalar locals. -/
theorem exportFrameLoans_borrowTwoIntegers_state
    (state : RuntimeState) (loan : Nat) (current first second : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? state loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer current)),
            some (.integer first), some (.integer second)]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (loan, .integer current) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noGlobalIn : globalLoanKeyIn? globalLoans loan = none := noGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst, applyWriteBack_empty, globalLoanKey?, noGlobalIn]

/-- Finalize after local execution has advanced only the runtime loan
counter and pending suffix.  The registry is still the incoming state's
native registry, so its existing no-global fact routes the export directly. -/
theorem exportFrameLoans_singleInteger_fromState
    (initial : RuntimeState) (runtimeNextLoan loan : Nat)
    (inherited : Array (Nat × RuntimeValue)) (value : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? initial loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer value))]
          activeLoans
          loanLocations }
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := inherited } =
      { globals := initial.globals
        globalLoans := initial.globalLoans
        nextLoan := runtimeNextLoan
        pending := inherited.push (loan, .integer value) } := by
  exact exportFrameLoans_singleInteger initial.globals initial.globalLoans
    runtimeNextLoan loan inherited value activeLoans loanLocations noGlobal

/-- Finalize a two-local frame whose second local is a saved scalar.  The
second slot is borrow-free by construction, so only the leading parameter
loan is exported. -/
theorem exportFrameLoans_borrowInteger_fromState
    (initial : RuntimeState) (runtimeNextLoan loan : Nat)
    (inherited : Array (Nat × RuntimeValue)) (current saved : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? initial loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer current)),
            some (.integer saved)]
          activeLoans
          loanLocations }
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := inherited } =
      { globals := initial.globals
        globalLoans := initial.globalLoans
        nextLoan := runtimeNextLoan
        pending := inherited.push (loan, .integer current) } := by
  have noGlobal' :
      globalLoanKey?
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := inherited }
        loan = none := noGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst]
  exact congrArg Prod.snd (applyWriteBack_empty_export noGlobal')

/-- The `Bool` twin of `exportFrameLoans_borrowInteger_fromState`. -/
theorem exportFrameLoans_borrowBool_fromState
    (initial : RuntimeState) (runtimeNextLoan loan : Nat)
    (inherited : Array (Nat × RuntimeValue)) (current : Int) (saved : Bool)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKey? initial loan = none) :
    exportFrameLoans
        { locals := #[some (.borrow loan (.integer current)),
            some (.bool saved)]
          activeLoans
          loanLocations }
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := inherited } =
      { globals := initial.globals
        globalLoans := initial.globalLoans
        nextLoan := runtimeNextLoan
        pending := inherited.push (loan, .integer current) } := by
  have noGlobal' :
      globalLoanKey?
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := inherited }
        loan = none := noGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst]
  exact congrArg Prod.snd (applyWriteBack_empty_export noGlobal')

/-- Finalize two scalar mutable parameters in local order.  Their native
registry facts decide both routes, yielding two pending appends and no
global-memory traversal. -/
theorem exportFrameLoans_twoIntegers_state
    (state : RuntimeState) (leftLoan rightLoan : Nat)
    (left right : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noLeftGlobal : globalLoanKey? state leftLoan = none)
    (noRightGlobal : globalLoanKey? state rightLoan = none) :
    exportFrameLoans
        { locals := #[some (.borrow leftLoan (.integer left)),
            some (.borrow rightLoan (.integer right))]
          activeLoans
          loanLocations }
        state =
      { state with
        pending := state.pending.push (leftLoan, .integer left)
          |>.push (rightLoan, .integer right) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noLeft : globalLoanKeyIn? globalLoans leftLoan = none :=
    noLeftGlobal
  have noRight : globalLoanKeyIn? globalLoans rightLoan = none :=
    noRightGlobal
  simp [exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows, borrowEntry?, collectPruned, holeInFrame,
    holeWithin, findFirst, applyWriteBack_empty, globalLoanKey?,
    noLeft, noRight]

mutual
  private def simpleIndexFuel? (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (frame : RuntimeFrame) (state : RuntimeState) (fuel : Nat)
      (expressionId : ExprId) : Option Nat := do
    match fuel with
    | 0 => none
    | fuel + 1 =>
      let value ← match Validation.placeIndexForm? ns expressionId with
        | some (.literal value) => some value
        | some (.local localId) | some (.copyLocal localId) =>
            let .integer value ← readLocal? frame localId | none
            some value
        | some (.fromEnd sourcePlace offset) =>
            let resolved ← resolvePlaceFuel? unit ns frame state fuel sourcePlace
            let .vector elements ← readRuntimePlace? frame state resolved | none
            some (Int.ofNat elements.size - Int.ofNat offset)
        | none => none
      if value < 0 then none else some value.toNat
  termination_by structural fuel

  private def resolvePlaceFuel? (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (frame : RuntimeFrame) (state : RuntimeState) (fuel : Nat)
      (placeId : PlaceId) : Option RuntimePlace := do
    match fuel with
    | 0 => none
    | fuel + 1 =>
      let place ← ns.places[placeId.index]?
      match place with
      | .localVar localId =>
          if localId.index < frame.locals.size then some { root := .local localId } else none
      | .deref base =>
          -- Shared dereferences are erased at preparation; a residual
          -- dereference projects into a live mutable borrow.
          let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
          let .borrow _ _ ← readRuntimePlace? frame state basePlace | none
          some { basePlace with
            projections := basePlace.projections.push .deref }
      | .field base owner field =>
          -- The projection names its owner, so the index is decided by the
          -- declaration rather than by whatever value sits at the base; a
          -- value of another shape is a mismatch, exactly as it is for a
          -- value-level selection.
          let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
          let .nominal source variant _ ← readRuntimePlace? frame state basePlace | none
          let declared ← resolveStruct? unit ns.identity owner
          if source != declared then none else
          let fieldName ← sourceFieldName? ns field
          let index ← handleFieldIndex? unit declared variant fieldName
          some { basePlace with projections := basePlace.projections.push (.field index) }
      | .index base indexExpression =>
          let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
          let index ← simpleIndexFuel? unit ns frame state fuel indexExpression
          let value ← readRuntimePlace? frame state basePlace
          match value with
          | .vector elements | .tuple elements =>
              if index < elements.size then
                some { basePlace with projections := basePlace.projections.push (.index index) }
              else none
          | _ => none
      | .subslice base start stop fromEnd => do
          let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
          some { basePlace with
            projections := basePlace.projections.push (.subslice start stop fromEnd) }
      | .downcast base variant =>
          let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
          let .nominal _ (some actual) _ ← readRuntimePlace? frame state basePlace | none
          let expected ← sourceFieldName? ns variant
          if actual == expected then
            some { basePlace with projections := basePlace.projections.push (.downcast expected) }
          else none
  termination_by structural fuel
end

/-- One statically resolved nominal-field projection.  This is the compact
native counterpart of a source `.field` place node: lowering has already
selected the declaration and field position, while execution still checks
the nominal shape of the value it traverses. -/
structure NominalFieldStep where
  source : StructHandle
  variant : Option String
  index : Nat
  deriving Repr, BEq, Inhabited

/-- Traverse a closed row of nominal projections.  This operation contains
no source place, namespace, or unit lookup. -/
def resolveNominalFieldSteps? (frame : RuntimeFrame) (state : RuntimeState) :
    List NominalFieldStep → RuntimePlace → Option RuntimePlace
  | [], place => some place
  | step :: rest, place => do
      let .nominal source variant _ ← readRuntimePlace? frame state place | none
      if source != step.source || variant != step.variant then none else
      resolveNominalFieldSteps? frame state rest
        { place with projections := place.projections.push (.field step.index) }

/-- Resolve a local-reference slot followed by a closed nominal field row.
Only the genuinely dynamic slot, borrow-tag, and nominal-shape checks remain. -/
def resolveDerefLocalFieldPath? (localId : LocalId)
    (fields : List NominalFieldStep) (frame : RuntimeFrame)
    (state : RuntimeState) : Option RuntimePlace := do
  if localId.index < frame.locals.size then
    let referent : RuntimePlace := {
      root := .local localId
      projections := #[.deref] }
    let some (.borrow _ _) := readLocal? frame localId | none
    resolveNominalFieldSteps? frame state fields referent
  else none

/-- A proof-only certificate connecting a source
`deref (localVar localId) / field*` place to its compact native field row.
The certificate is erased from the executable denotation. -/
inductive DerefLocalFieldPath (unit : ValidatedUnit)
    (ns : ValidatedNamespace) :
    PlaceId → List NominalFieldStep → Type where
  | deref {place base : PlaceId} {localId : LocalId}
      (place_eq : ns.places[place.index]? = some (.deref base))
      (base_eq : ns.places[base.index]? = some (.localVar localId)) :
      DerefLocalFieldPath unit ns place []
  | field {place base : PlaceId} {reversed : List NominalFieldStep}
      {owner : QualifiedRef} {field : NameId} {fieldName : String}
      (step : NominalFieldStep)
      (place_eq : ns.places[place.index]? = some (.field base owner field))
      (base_path : DerefLocalFieldPath unit ns base reversed)
      (resolved_eq : resolveStruct? unit ns.identity owner = some step.source)
      (name_eq : sourceFieldName? ns field = some fieldName)
      (index_eq : ∀ actualVariant,
        handleFieldIndex? unit step.source actualVariant fieldName =
          if actualVariant == step.variant then some step.index else none) :
      DerefLocalFieldPath unit ns place (step :: reversed)

def DerefLocalFieldPath.localId :
    {place : PlaceId} → {reversed : List NominalFieldStep} →
      DerefLocalFieldPath unit ns place reversed → LocalId
  | _, _, .deref (localId := localId) .. => localId
  | _, _, .field _ _ basePath .. => basePath.localId

private theorem resolveNominalFieldSteps?_append
    (frame : RuntimeFrame) (state : RuntimeState)
    (headFields tailFields : List NominalFieldStep) (place : RuntimePlace) :
    resolveNominalFieldSteps? frame state (headFields ++ tailFields) place =
      (resolveNominalFieldSteps? frame state headFields place).bind
        (resolveNominalFieldSteps? frame state tailFields) := by
  induction headFields generalizing place with
  | nil => rfl
  | cons step rest ih =>
      simp only [List.cons_append, resolveNominalFieldSteps?]
      cases readRuntimePlace? frame state place <;> simp
      case some value =>
        cases value <;> simp
        case nominal source variant fields =>
          split <;> simp_all [ih]

private theorem resolveDerefLocalFieldPath?_append
    (localId : LocalId) (headFields tailFields : List NominalFieldStep)
    (frame : RuntimeFrame) (state : RuntimeState) :
    resolveDerefLocalFieldPath? localId (headFields ++ tailFields) frame state =
      (resolveDerefLocalFieldPath? localId headFields frame state).bind
        (resolveNominalFieldSteps? frame state tailFields) := by
  by_cases in_bounds : localId.index < frame.locals.size
  · cases local_eq : readLocal? frame localId <;>
      simp [resolveDerefLocalFieldPath?, in_bounds, local_eq,
        resolveNominalFieldSteps?_append]
    case some value => cases value <;>
      simp [resolveDerefLocalFieldPath?, in_bounds, local_eq,
        resolveNominalFieldSteps?_append]
  · simp [resolveDerefLocalFieldPath?, in_bounds]

private theorem resolvePlaceFuel?_of_derefLocalFieldPath
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {place : PlaceId} {reversed : List NominalFieldStep}
    (path : DerefLocalFieldPath unit ns place reversed)
    (frame : RuntimeFrame) (state : RuntimeState) (fuel : Nat)
    (enough : reversed.length + 2 ≤ fuel) :
    resolvePlaceFuel? unit ns frame state fuel place =
      resolveDerefLocalFieldPath? path.localId reversed.reverse frame state := by
  induction path generalizing fuel with
  | @deref place base localId place_eq base_eq =>
      cases fuel with
      | zero => omega
      | succ fuel =>
        cases fuel with
        | zero => omega
        | succ fuel =>
          simp only [DerefLocalFieldPath.localId, List.reverse_nil]
          simp [resolvePlaceFuel?, place_eq, base_eq, readRuntimePlace?,
            readRoot?, readProjections?]
          by_cases in_bounds : localId.index < frame.locals.size
          · simp only [in_bounds, ↓reduceIte, RuntimePlace.writable]
            cases local_eq : readLocal? frame localId with
            | none => simp [local_eq, resolveDerefLocalFieldPath?,
                resolveNominalFieldSteps?]
            | some value =>
              cases value <;> simp [local_eq, resolveDerefLocalFieldPath?,
                resolveNominalFieldSteps?, readProjections?] <;> exact in_bounds
          · simp [in_bounds, resolveDerefLocalFieldPath?]
  | @field place base reversed owner field fieldName step
      place_eq basePath resolved_eq name_eq index_eq ih =>
      cases fuel with
      | zero => omega
      | succ fuel =>
        have base_enough : reversed.length + 2 ≤ fuel := by simpa using enough
        simp only [resolvePlaceFuel?]
        rw [place_eq]
        simp only [bind, Option.bind]
        simp only [DerefLocalFieldPath.localId, List.reverse_cons]
        change
          (do
            let basePlace ← resolvePlaceFuel? unit ns frame state fuel base
            let .nominal actualSource actualVariant _ ←
              readRuntimePlace? frame state basePlace | none
            let declared ← resolveStruct? unit ns.identity owner
            if actualSource != declared then none else
            let actualFieldName ← sourceFieldName? ns field
            let actualIndex ←
              handleFieldIndex? unit declared actualVariant actualFieldName
            some { basePlace with projections :=
              basePlace.projections.push (.field actualIndex) }) =
          resolveDerefLocalFieldPath? basePath.localId
            (reversed.reverse ++ [step]) frame state
        rw [ih fuel base_enough]
        rw [resolveDerefLocalFieldPath?_append]
        apply Option.bind_congr
        intro basePlace
        cases value_eq : readRuntimePlace? frame state basePlace with
        | none => simp [resolveNominalFieldSteps?, value_eq]
        | some value =>
          cases value <;> simp [resolveNominalFieldSteps?, value_eq]
          case nominal actualSource actualVariant values =>
            by_cases source_eq : actualSource = step.source
            · subst actualSource
              by_cases variant_eq : actualVariant = step.variant
              · subst actualVariant
                simp [resolved_eq, name_eq, index_eq]
              · simp [resolved_eq, name_eq, index_eq, variant_eq,
                  bne_iff_ne]
            · simp [resolved_eq, name_eq, index_eq, source_eq, bne_iff_ne]

/-- Resolve the currently executable, side-effect-free place subset. Index
expressions admit integer literals, local reads, copies of local places, and
the exact length-minus-offset form emitted for a from-end Rust slice index.
A dereference is a projection into the borrow value at the base place; its
writability follows the borrow's kind. The place arena is acyclic, so a
fuel of one unit per arena slot is exact rather than an approximation; the
constant beyond it is what reduction peels, one step per level of the
shapes lowering emits, and a field projection of a dereferenced local is
the deepest of them. -/
def resolvePlace? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (state : RuntimeState) (placeId : PlaceId) :
    Option RuntimePlace :=
  resolvePlaceFuel? unit ns frame state (2 * ns.places.size + 3) placeId

/-- Resolve the indexed element of a vector or tuple held directly in a
local.  Lowering has already classified the index expression as a local
read, so execution retains only the two slots and the dynamic bounds check. -/
def resolveLocalIndex? (base index : LocalId) (frame : RuntimeFrame) :
    Option RuntimePlace := do
  if base.index >= frame.locals.size then none else
  let .integer position ← readLocal? frame index | none
  if position < 0 then none else
  let value ← readLocal? frame base
  let size ← match value with
    | .vector elements | .tuple elements => some elements.size
    | _ => none
  if position.toNat < size then
    some { root := .local base, projections := #[.index position.toNat] }
  else none

/-- Resolve a statically indexed vector or tuple rooted at a known local.
`deref` distinguishes an owned local aggregate from an aggregate reached
through a mutable-reference parameter.  The index expression itself has
already been classified and erased by lowering. -/
def resolveLocalLiteralIndex? (base : LocalId) (deref : Bool) (index : Nat)
    (frame : RuntimeFrame) (state : RuntimeState) : Option RuntimePlace := do
  if base.index >= frame.locals.size then none else
  let root : RuntimePlace := { root := .local base }
  let root ← if deref then do
      let .borrow _ _ ← readLocal? frame base | none
      some { root with projections := #[.deref] }
    else some root
  let value ← readRuntimePlace? frame state root
  let size ← match value with
    | .vector elements | .tuple elements => some elements.size
    | _ => none
  if index < size then
    some { root with projections := root.projections.push (.index index) }
  else none

/-- Resolve an index read from a known local, optionally through a borrow.
Only the two local slots survive lowering; the source arena is absent. -/
def resolveLocalDynamicIndex? (base index : LocalId) (deref : Bool)
    (frame : RuntimeFrame) (state : RuntimeState) : Option RuntimePlace := do
  if base.index >= frame.locals.size then none else
  let .integer position ← readLocal? frame index | none
  if position < 0 then none else
  resolveLocalLiteralIndex? base deref position.toNat frame state

/-- Resolve one statically selected nominal field below a literal-indexed
owned local aggregate.  Both indices are fixed by lowering; execution only
checks the vector bound and the nominal value's runtime shape. -/
def resolveLocalLiteralIndexField? (base : LocalId) (index : Nat)
    (field : NominalFieldStep) (frame : RuntimeFrame)
    (state : RuntimeState) : Option RuntimePlace := do
  let element ← resolveLocalLiteralIndex? base false index frame state
  let .nominal source actualVariant _ ←
    readRuntimePlace? frame state element | none
  if source != field.source then none else
  let actualIndex ←
    if actualVariant == field.variant then some field.index else none
  some { element with
    projections := element.projections.push (.field actualIndex) }

/-- A literal index into an aggregate held directly in a local is exactly
the compact static-index resolver. -/
theorem resolvePlace?_localLiteralIndex {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId basePlaceId : PlaceId} {indexExpr : ExprId}
    {base : LocalId} {index : Nat}
    (place_eq : ns.places[placeId.index]? =
      some (.index basePlaceId indexExpr))
    (base_eq : ns.places[basePlaceId.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr =
      some (.literal (index : Int))) :
    resolvePlace? unit ns frame state placeId =
      resolveLocalLiteralIndex? base false index frame state := by
  have nonnegative : ¬ (index : Int) < 0 :=
    Int.not_lt_of_ge (Int.ofNat_nonneg index)
  by_cases in_bounds : base.index < frame.locals.size
  · have not_out_of_bounds : ¬ frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
      index_eq, resolveLocalLiteralIndex?, readRuntimePlace?, readRoot?,
      readProjections?, in_bounds, not_out_of_bounds, nonnegative]
  · have out_of_bounds : frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
      index_eq, resolveLocalLiteralIndex?, in_bounds, out_of_bounds]

/-- A field directly below a literal index into an owned local has the
compact indexed-field resolver shape. -/
theorem resolvePlace?_fieldOfLocalLiteralIndex {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId indexPlaceId basePlaceId : PlaceId} {indexExpr : ExprId}
    {base : LocalId} {index fieldIndex : Nat}
    {owner : QualifiedRef} {fieldNameId : NameId} {fieldName : String}
    {handle : StructHandle}
    (place_eq : ns.places[placeId.index]? =
      some (.field indexPlaceId owner fieldNameId))
    (index_place_eq : ns.places[indexPlaceId.index]? =
      some (.index basePlaceId indexExpr))
    (base_eq : ns.places[basePlaceId.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr =
      some (.literal (index : Int)))
    (resolved_eq : resolveStruct? unit ns.identity owner = some handle)
    (name_eq : sourceFieldName? ns fieldNameId = some fieldName)
    (field_index_eq : ∀ actualVariant,
      handleFieldIndex? unit handle actualVariant fieldName =
        if actualVariant == none then some fieldIndex else none) :
    resolvePlace? unit ns frame state placeId =
      resolveLocalLiteralIndexField? base index
        ⟨handle, none, fieldIndex⟩ frame state := by
  have nonnegative : ¬ (index : Int) < 0 :=
    Int.not_lt_of_ge (Int.natCast_nonneg index)
  by_cases in_bounds : base.index < frame.locals.size
  · have not_out_of_bounds : ¬ frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq,
      index_place_eq, base_eq, index_eq, resolved_eq, name_eq,
      field_index_eq, resolveLocalLiteralIndexField?,
      resolveLocalLiteralIndex?, resolveNominalFieldSteps?,
      readRuntimePlace?, readRoot?, readProjections?, in_bounds,
      not_out_of_bounds, nonnegative]
    cases value_eq : readLocal? frame base with
    | none => simp [value_eq]
    | some value =>
        cases value <;> simp [value_eq, readProjections?, resolved_eq,
          name_eq, field_index_eq, bne_iff_ne]
        case vector elements | tuple elements =>
          apply Option.bind_congr
          intro element _
          apply Option.bind_congr
          intro elementValue _
          cases elementValue <;> simp
          case nominal =>
            rename_i actualSource actualVariant fields elementEq
            by_cases source_eq : actualSource = handle
            · subst actualSource
              by_cases variant_eq : actualVariant = none <;>
                simp [variant_eq]
            · simp [source_eq]
  · have out_of_bounds : frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq,
      index_place_eq, base_eq, index_eq, resolveLocalLiteralIndexField?,
      resolveLocalLiteralIndex?, in_bounds, out_of_bounds, nonnegative]

/-- The corresponding compact equation when the indexed aggregate is the
referent of a mutable-reference local. -/
theorem resolvePlace?_derefLocalLiteralIndex {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId basePlaceId localPlaceId : PlaceId} {indexExpr : ExprId}
    {base : LocalId} {index : Nat}
    (place_eq : ns.places[placeId.index]? =
      some (.index basePlaceId indexExpr))
    (base_eq : ns.places[basePlaceId.index]? = some (.deref localPlaceId))
    (local_eq : ns.places[localPlaceId.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr =
      some (.literal (index : Int))) :
    resolvePlace? unit ns frame state placeId =
      resolveLocalLiteralIndex? base true index frame state := by
  have nonnegative : ¬ (index : Int) < 0 :=
    Int.not_lt_of_ge (Int.ofNat_nonneg index)
  by_cases in_bounds : base.index < frame.locals.size
  · have not_out_of_bounds : ¬ frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
      local_eq, index_eq, resolveLocalLiteralIndex?, readRuntimePlace?,
      readRoot?, readProjections?, in_bounds, not_out_of_bounds, nonnegative]
    cases value_eq : readLocal? frame base with
    | none => simp [value_eq]
    | some value => cases value <;> simp [value_eq, readProjections?]
  · have out_of_bounds : frame.locals.size ≤ base.index := by omega
    simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
      local_eq, index_eq, resolveLocalLiteralIndex?, in_bounds, out_of_bounds]

/-- Dynamic indexing through a borrowed local agrees with the same source
place resolver as literal indexing. -/
theorem resolvePlace?_derefLocalDynamicIndex {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId basePlaceId localPlaceId : PlaceId} {indexExpr : ExprId}
    {base index : LocalId}
    (place_eq : ns.places[placeId.index]? = some (.index basePlaceId indexExpr))
    (base_eq : ns.places[basePlaceId.index]? = some (.deref localPlaceId))
    (local_eq : ns.places[localPlaceId.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr = some (.local index) ∨
      placeIndexForm? ns indexExpr = some (.copyLocal index)) :
    resolvePlace? unit ns frame state placeId =
      resolveLocalDynamicIndex? base index true frame state := by
  rcases index_eq with index_eq | index_eq
  all_goals
    by_cases in_bounds : base.index < frame.locals.size
    · have not_out_of_bounds : ¬ frame.locals.size ≤ base.index := by omega
      simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
        local_eq, index_eq, resolveLocalDynamicIndex?, resolveLocalLiteralIndex?,
        readRuntimePlace?, readRoot?, readProjections?, in_bounds, not_out_of_bounds]
      cases index_value : readLocal? frame index with
      | none => simp [index_value]
      | some value =>
          cases value <;> simp [index_value]
          case integer position =>
            by_cases negative : position < 0 <;> simp [negative]
            cases base_value : readLocal? frame base with
            | none => simp [base_value]
            | some value => cases value <;> simp [base_value, readProjections?]
    · have out_of_bounds : frame.locals.size ≤ base.index := by omega
      simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
        local_eq, index_eq, resolveLocalDynamicIndex?, resolveLocalLiteralIndex?,
        in_bounds, out_of_bounds]

/-- A source index place whose base and index are local reads is exactly the
compact local-index resolver.  The disjunction covers both the direct and
explicit-copy spellings admitted by `placeIndexForm?`. -/
theorem resolvePlace?_localIndex {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId basePlaceId : PlaceId} {indexExpr : ExprId}
    {base index : LocalId}
    (place_eq : ns.places[placeId.index]? =
      some (.index basePlaceId indexExpr))
    (base_eq : ns.places[basePlaceId.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr = some (.local index) ∨
      placeIndexForm? ns indexExpr = some (.copyLocal index)) :
    resolvePlace? unit ns frame state placeId =
      resolveLocalIndex? base index frame := by
  rcases index_eq with index_eq | index_eq
  all_goals
    by_cases in_bounds : base.index < frame.locals.size
    · have not_out_of_bounds : ¬ frame.locals.size ≤ base.index := by omega
      simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
        index_eq, resolveLocalIndex?, readRuntimePlace?, readRoot?,
        readProjections?, Option.bind_assoc, in_bounds, not_out_of_bounds]
      cases index_value : readLocal? frame index with
      | none => simp [index_value]
      | some value =>
          cases value <;> simp [index_value]
          case integer position =>
            by_cases negative : position < 0 <;> simp [negative]
    · have out_of_bounds : frame.locals.size ≤ base.index := by omega
      simp [resolvePlace?, resolvePlaceFuel?, simpleIndexFuel?, place_eq, base_eq,
        index_eq, resolveLocalIndex?, in_bounds, out_of_bounds]

/-- Soundness of a compact local-field path certificate for the public
place resolver.  The numeric side condition is closed by generation and
prevents the theorem from unfolding the surrounding unit. -/
theorem resolvePlace?_of_derefLocalFieldPath
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {place : PlaceId} {reversed : List NominalFieldStep}
    (path : DerefLocalFieldPath unit ns place reversed)
    (enough : reversed.length + 2 ≤ 2 * ns.places.size + 3)
    (frame : RuntimeFrame) (state : RuntimeState) :
    resolvePlace? unit ns frame state place =
      resolveDerefLocalFieldPath? path.localId reversed.reverse frame state :=
  resolvePlaceFuel?_of_derefLocalFieldPath path frame state _ enough

/-- A local root selected by lowering resolves without any residual place
arena walk.  This equation is the certificate consumed by native local-place
denotations. -/
theorem resolvePlace?_localVar {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId : PlaceId} {localId : LocalId}
    (place_eq : ns.places[placeId.index]? = some (.localVar localId)) :
    resolvePlace? unit ns frame state placeId =
      if localId.index < frame.locals.size then
        some { root := .local localId }
      else none := by
  simp [resolvePlace?, resolvePlaceFuel?, place_eq]

/-- A field projection of a dereferenced local slot has a native
runtime-place shape.  The projection names its owner, so the position is
decided by the declaration; only the genuinely dynamic checks remain — that
the slot holds a mutable borrow, and that its referent has the declared
nominal shape. -/
theorem resolvePlace?_fieldOfDerefLocal {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId baseId localPlaceId : PlaceId} {localId : LocalId}
    {owner : QualifiedRef} {field : NameId} {handle : StructHandle}
    {variant : Option String} {index : Nat}
    (place_eq : ns.places[placeId.index]? = some (.field baseId owner field))
    (base_eq : ns.places[baseId.index]? = some (.deref localPlaceId))
    (local_eq : ns.places[localPlaceId.index]? = some (.localVar localId))
    (resolved_eq : resolveStruct? unit ns.identity owner = some handle)
    (name_eq : sourceFieldName? ns field = some fieldName)
    (index_eq : ∀ actualVariant,
      handleFieldIndex? unit handle actualVariant fieldName =
        if actualVariant == variant then some index else none) :
    resolvePlace? unit ns frame state placeId =
      if localId.index < frame.locals.size then
        match readLocal? frame localId with
        | some (.borrow _ current) =>
            match current with
            | .nominal source actualVariant _ =>
                if source != handle || actualVariant != variant then none
                else some {
                  root := .local localId
                  projections := #[.deref, .field index] }
            | _ => none
        | _ => none
      else none := by
  by_cases in_bounds : localId.index < frame.locals.size
  · cases local_value : readLocal? frame localId with
    | none =>
        simp [resolvePlace?, resolvePlaceFuel?, place_eq, base_eq, local_eq,
          readRuntimePlace?, readRoot?, readProjections?, in_bounds, local_value]
    | some value =>
        cases value <;>
          simp [resolvePlace?, resolvePlaceFuel?, place_eq, base_eq, local_eq,
            readRuntimePlace?, readRoot?, readProjections?, in_bounds,
            local_value, resolved_eq, name_eq]
        case borrow loan current =>
          cases current <;>
            simp [readProjections?, resolved_eq, name_eq, index_eq,
              RuntimePlace.writable]
          case nominal source actualVariant fields =>
            by_cases source_eq : source = handle
            · subst source_eq
              by_cases variant_eq : actualVariant = variant <;>
                simp [index_eq, variant_eq, bne_iff_ne]
            · simp [bne_iff_ne, source_eq]
  · simp [resolvePlace?, resolvePlaceFuel?, place_eq, base_eq, local_eq,
      readRuntimePlace?, readRoot?, readProjections?, in_bounds]

/-- A dereference rooted at a direct local slot has a native runtime-place
shape.  Resolution still performs the genuinely dynamic check that the slot
currently contains a mutable borrow, but it performs no arena search once
the two source-place equations have been certified by lowering. -/
theorem resolvePlace?_derefLocal {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {placeId baseId : PlaceId} {localId : LocalId}
    (place_eq : ns.places[placeId.index]? = some (.deref baseId))
    (base_eq : ns.places[baseId.index]? = some (.localVar localId)) :
    resolvePlace? unit ns frame state placeId =
      if localId.index < frame.locals.size then
        match readLocal? frame localId with
        | some (.borrow _ _) => some {
            root := .local localId
            projections := #[.deref] }
        | _ => none
      else none := by
  simp [resolvePlace?, resolvePlaceFuel?, place_eq, base_eq,
    readRuntimePlace?, readRoot?, readProjections?]
  by_cases in_bounds : localId.index < frame.locals.size
  · simp only [in_bounds, ↓reduceIte, RuntimePlace.writable]
    cases local_eq : readLocal? frame localId with
    | none => simp [local_eq]
    | some value =>
        cases value <;> simp [local_eq, readProjections?]
  · simp [in_bounds]

/-- The lexical loan identity of a borrow site, from the owning function's
certificate.  The list traversals are structural so head reduction executes
the search over the quoted unit's literal certificate rows. -/
def certificateLoanId? (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (site : ExprId) : Option Nat :=
  unit.borrowCertificates.toList.findSome? fun certificate =>
    if certificate.namespaceId != namespaceId then none else
      indexOfFrom (·.expression == site) certificate.loans.toList 0

/-- Attach the checked caller-side lexical identity to a borrow produced by
a call result.  Only the dynamic loan id crosses the boundary in the value;
no owner root or projection path is introduced. -/
def registerReturnedLoan (lexical : Option Nat) (results : Array RuntimeValue)
    (frame : RuntimeFrame) : RuntimeFrame :=
  match lexical with
  | none => frame
  | some lexical =>
      let borrows := results.foldl (init := #[]) fun found result =>
        found ++ outermostBorrows result
      match borrows[0]? with
      | none => frame
      | some (loan, _) =>
          { frame with
            activeLoans := (frame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
              (⟨lexical⟩, loan) }

@[simp] theorem registerReturnedLoan_none
    (results : Array RuntimeValue) (frame : RuntimeFrame) :
    registerReturnedLoan none results frame = frame := rfl

theorem registerReturnedLoan_some_singleBorrow
    (lexical loan : Nat) (current : RuntimeValue) (frame : RuntimeFrame) :
    registerReturnedLoan (some lexical) #[.borrow loan current] frame =
      { frame with
        activeLoans := (frame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
          (⟨lexical⟩, loan) } := by
  simp [registerReturnedLoan, outermostBorrows, borrowEntry?, collectPruned]

/-- Borrow a resolved place after lowering has selected the lexical loan
identity.  This is the native location operation: it contains no unit,
namespace, expression, certificate, or arena lookup. -/
def borrowRuntimePlaceAt? (lexical : Nat) (referenceType : ReferenceType)
    (kind : BorrowKind)
    (frame : RuntimeFrame) (state : RuntimeState) (place : RuntimePlace) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) := do
  let expectedKind ← match kind with
    | .immutable => some ReferenceKind.shared
    | .mutable => some ReferenceKind.mutable
    | .profile _ => none
  if referenceType.kind != expectedKind || (!place.writable && expectedKind == .mutable) then none else
  let value ← readRuntimePlace? frame state place
  match expectedKind with
  | .shared =>
      -- A shared reference is the observed value: certified exclusivity
      -- erases the wrapper, and no loan instance is minted.
      some (frame, state, value)
  | .mutable => do
      let loanInstance := state.nextLoan
      let state := { state with nextLoan := loanInstance + 1 }
      let (frame, state) ← writeRuntimePlace? frame state place (.loanHole loanInstance)
      -- A global loan registers the key its hole occupies, so its death
      -- writes back by key instead of searching global memory.
      let state := match place.root with
        | .global key =>
            { state with globalLoans := (loanInstance, key) :: state.globalLoans }
        | .local _ => state
      let activeLoans := frame.activeLoans.filter (·.1 != ⟨lexical⟩) |>.push (⟨lexical⟩, loanInstance)
      let loanLocations := frame.loanLocations.push (loanInstance, place)
      some ({ frame with activeLoans, loanLocations }, state,
        .borrow loanInstance value)

/-- Borrow a resolved place under prophetic ownership.  A shared borrow does
not consult a loan certificate; a mutable borrow resolves its lexical site
at the point where it records the live dynamic loan. -/
def borrowRuntimePlace? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (site : ExprId) (referenceType : ReferenceType) (kind : BorrowKind)
    (frame : RuntimeFrame) (state : RuntimeState) (place : RuntimePlace) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) := do
  let expectedKind ← match kind with
    | .immutable => some ReferenceKind.shared
    | .mutable => some ReferenceKind.mutable
    | .profile _ => none
  if referenceType.kind != expectedKind ||
      (!place.writable && expectedKind == .mutable) then none else
  let value ← readRuntimePlace? frame state place
  match expectedKind with
  | .shared => some (frame, state, value)
  | .mutable => do
      let loanInstance := state.nextLoan
      let state := { state with nextLoan := loanInstance + 1 }
      let lexical ← certificateLoanId? unit ns.identity site
      let (frame, state) ←
        writeRuntimePlace? frame state place (.loanHole loanInstance)
      let state := match place.root with
        | .global key =>
            { state with globalLoans := (loanInstance, key) :: state.globalLoans }
        | .local _ => state
      let activeLoans :=
        (frame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
          (⟨lexical⟩, loanInstance)
      let loanLocations := frame.loanLocations.push (loanInstance, place)
      some ({ frame with activeLoans, loanLocations }, state,
        .borrow loanInstance value)

theorem borrowRuntimePlace?_immutable_at (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (site : ExprId) (referenceType : ReferenceType)
    (lexical : Nat) (frame : RuntimeFrame) (state : RuntimeState)
    (place : RuntimePlace) :
    borrowRuntimePlace? unit ns site referenceType .immutable frame state place =
      borrowRuntimePlaceAt? lexical referenceType .immutable frame state place := rfl

theorem borrowRuntimePlace?_mutable_at (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (site : ExprId) (referenceType : ReferenceType)
    (lexical : Nat) (frame : RuntimeFrame) (state : RuntimeState)
    (place : RuntimePlace)
    (loan_eq : certificateLoanId? unit ns.identity site = some lexical) :
    borrowRuntimePlace? unit ns site referenceType .mutable frame state place =
      borrowRuntimePlaceAt? lexical referenceType .mutable frame state place := by
  simp [borrowRuntimePlace?, borrowRuntimePlaceAt?, loan_eq]

/-- A shared borrow of a native global location, once lowering/proof-side
storage splitting has established the slot contents.  The theorem removes
all residual place reads from the denotation proof. -/
theorem borrowRuntimePlaceAt?_global_immutable_of_lookup
    (lexical : Nat) (referenceType : ReferenceType)
    (frame : RuntimeFrame) (state : RuntimeState) (key : GlobalKey)
    (value : RuntimeValue) (kind_eq : referenceType.kind = .shared)
    (lookup_eq : state.globals.lookup key = some value) :
    borrowRuntimePlaceAt? lexical referenceType .immutable frame state
        { root := .global key } =
      some (frame, state, value) := by
  simp [borrowRuntimePlaceAt?, readRuntimePlace?, readRoot?,
    readProjections?, kind_eq, lookup_eq] <;> rfl

/-- A mutable borrow of a native global location, once the selected slot is
known to be present.  The result is the keyed hole write and loan registry
update directly—not a search through global storage. -/
theorem borrowRuntimePlaceAt?_global_mutable_of_lookup
    (lexical : Nat) (referenceType : ReferenceType)
    (frame : RuntimeFrame) (state : RuntimeState) (key : GlobalKey)
    (value : RuntimeValue) (kind_eq : referenceType.kind = .mutable)
    (lookup_eq : state.globals.lookup key = some value) :
    borrowRuntimePlaceAt? lexical referenceType .mutable frame state
        { root := .global key } =
      some
        ({ frame with
          activeLoans :=
            (frame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
              (⟨lexical⟩, state.nextLoan)
          loanLocations := frame.loanLocations.push
            (state.nextLoan, { root := .global key }) },
         { state with
           globals := state.globals.insert key (.loanHole state.nextLoan)
           globalLoans := (state.nextLoan, key) :: state.globalLoans
           nextLoan := state.nextLoan + 1 },
         .borrow state.nextLoan value) := by
  simp [borrowRuntimePlaceAt?, readRuntimePlace?, readRoot?,
    readProjections?, writeRuntimePlace?, writeRoot?, kind_eq, lookup_eq] <;> rfl

/-- Result of a global-storage primitive. Missing resources and duplicate
publication are language-level aborts rather than interpreter failures. -/
inductive GlobalOperationResult where
  | value (frame : RuntimeFrame) (state : RuntimeState) (value : RuntimeValue)
  | throw_ (frame : RuntimeFrame) (state : RuntimeState) (kind : ThrowKind)
      (arguments : Array RuntimeValue := #[])
  deriving Repr, BEq

/-- The key a resource family and a key value name.  Both a program and a
specification address global memory through this, so their reads of one
resource are the same term. -/
def globalKey (namespaceId : NamespaceId) (typeId : TypeId)
    (key : RuntimeValue) : GlobalKey :=
  { namespaceId, typeId, key := key.storageKey }

/-- The resource published at a key, if any. -/
def globalValue? (state : RuntimeState) (namespaceId : NamespaceId)
    (typeId : TypeId) (key : RuntimeValue) : Option RuntimeValue :=
  state.globals.lookup (globalKey namespaceId typeId key)

/-- The resource published at a key, junk where none is.  A clause is a
proposition and so reads storage totally; the declared abort condition of a
missing resource makes the junk branch unreachable in a proof. -/
def globalValue (state : RuntimeState) (namespaceId : NamespaceId)
    (typeId : TypeId) (key : RuntimeValue) : RuntimeValue :=
  (globalValue? state namespaceId typeId key).getD .unit

/-- Whether a resource is published at a key. -/
def globalExists (state : RuntimeState) (namespaceId : NamespaceId)
    (typeId : TypeId) (key : RuntimeValue) : Bool :=
  (globalValue? state namespaceId typeId key).isSome

/-- Common dynamic portion of a global borrow.  Lowering changes only the
already-resolved `borrow` function supplied here; key-shape checking,
presence testing, and the storage access itself stay authoritative and
shared. -/
abbrev borrowGlobalUsing? (namespaceId : NamespaceId) (typeId : TypeId)
    (borrow : RuntimeFrame → RuntimeState → RuntimePlace →
      Option (RuntimeFrame × RuntimeState × RuntimeValue))
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) : Option GlobalOperationResult := do
  let [key] := arguments.toList | none
  let _ ← key.storageKey?
  match globalValue? state namespaceId typeId key with
  | none => some (.throw_ frame state .abort)
  | some _ => do
      let (frame, state, value) ← borrow frame state
        { root := .global (globalKey namespaceId typeId key) }
      some (.value frame state value)

theorem borrowGlobalUsing?_unfold (namespaceId : NamespaceId) (typeId : TypeId)
    (borrow : RuntimeFrame → RuntimeState → RuntimePlace →
      Option (RuntimeFrame × RuntimeState × RuntimeValue))
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) :
    borrowGlobalUsing? namespaceId typeId borrow arguments frame state = (do
      let [key] := arguments.toList | none
      let _ ← key.storageKey?
      match globalValue? state namespaceId typeId key with
      | none => some (.throw_ frame state .abort)
      | some _ => do
          let (frame, state, value) ← borrow frame state
            { root := .global (globalKey namespaceId typeId key) }
          some (.value frame state value)) := rfl

/-! Keyed global actions after lowering has fixed the resource family and,
for a borrow, its reference type and lexical loan identity. -/

def containsGlobalAt? (namespaceId : NamespaceId) (typeId : TypeId)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) : Option GlobalOperationResult := do
  let [key] := arguments.toList | none
  let _ ← key.storageKey?
  some (.value frame state (.bool (globalExists state namespaceId typeId key)))

def borrowGlobalAt? (namespaceId : NamespaceId) (typeId : TypeId)
    (referenceType : ReferenceType) (lexicalLoan : Nat) (kind : BorrowKind)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) : Option GlobalOperationResult :=
  borrowGlobalUsing? namespaceId typeId
    (fun frame state place =>
      borrowRuntimePlaceAt? lexicalLoan referenceType kind frame state place)
    arguments frame state

def takeGlobalAt? (namespaceId : NamespaceId) (typeId : TypeId)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) : Option GlobalOperationResult := do
  let [key] := arguments.toList | none
  let _ ← key.storageKey?
  match globalValue? state namespaceId typeId key with
  | none => some (.throw_ frame state .abort)
  | some value =>
      let globals := state.globals.erase (globalKey namespaceId typeId key)
      some (.value frame { state with globals } value)

def publishGlobalAt? (namespaceId : NamespaceId) (typeId : TypeId)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) : Option GlobalOperationResult := do
  let [key, value] := arguments.toList | none
  let _ ← key.storageKey?
  match globalValue? state namespaceId typeId key with
  | some _ => some (.throw_ frame state .abort)
  | none =>
      let globals := state.globals.insert (globalKey namespaceId typeId key) value
      some (.value frame { state with globals } .unit)

/-- Deterministic core meaning of keyed global storage. The resource family is
the single operation type instantiation. -/
def evaluateGlobalOperation? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (kind : GlobalKind)
    (instantiations : Array GenericArgument)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    Option GlobalOperationResult := do
  let #[.typeArg resource] := instantiations | none
  let resourceType := instantiatedTypeId frame.typeInstantiation resource.typeId
  match kind with
  | .contains =>
      let [key] := arguments.toList | none
      let _ ← key.storageKey?
      some (.value frame state
        (.bool (globalExists state ns.identity resourceType key)))
  | .borrow kind =>
      let [key] := arguments.toList | none
      let _ ← key.storageKey?
      match globalValue? state ns.identity resourceType key with
      | none => some (.throw_ frame state .abort)
      | some _ => do
          let .reference referenceType ← ns.tables.types[resultType.index]? | none
          let (frame, state, value) ← borrowRuntimePlace? unit ns site
            referenceType kind frame state
            { root := .global (globalKey ns.identity resourceType key) }
          some (.value frame state value)
  | .take =>
      let [key] := arguments.toList | none
      let _ ← key.storageKey?
      match globalValue? state ns.identity resourceType key with
      | none => some (.throw_ frame state .abort)
      | some value =>
          let globals := state.globals.erase
            (globalKey ns.identity resourceType key)
          some (.value frame { state with globals } value)
  | .publish =>
      let [key, value] := arguments.toList | none
      let _ ← key.storageKey?
      match globalValue? state ns.identity resourceType key with
      | some _ => some (.throw_ frame state .abort)
      | none =>
          let globals := state.globals.insert
            (globalKey ns.identity resourceType key) value
          some (.value frame { state with globals } .unit)

/-- Equation after lowering has selected the sole resource instantiation.
Array-literal pattern matches do not receive useful automatically generated
equations, so expose this boundary explicitly. -/
theorem evaluateGlobalOperation?_typeArg (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : GlobalKind) (resource : TypeUse) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    evaluateGlobalOperation? unit ns resultType site kind #[.typeArg resource]
        arguments frame state =
      (let resourceType :=
        instantiatedTypeId frame.typeInstantiation resource.typeId
      (match kind with
      | .contains => do
          let [key] := arguments.toList | none
          let _ ← key.storageKey?
          some (.value frame state
            (.bool (globalExists state ns.identity resourceType key)))
      | .borrow borrowKind => do
          let [key] := arguments.toList | none
          let _ ← key.storageKey?
          match globalValue? state ns.identity resourceType key with
          | none => some (.throw_ frame state .abort)
          | some _ => do
              let .reference referenceType ←
                ns.tables.types[resultType.index]? | none
              let (frame, state, value) ← borrowRuntimePlace? unit ns site
                referenceType borrowKind frame state
                { root := .global
                    (globalKey ns.identity resourceType key) }
              some (.value frame state value)
      | .take => do
          let [key] := arguments.toList | none
          let _ ← key.storageKey?
          match globalValue? state ns.identity resourceType key with
          | none => some (.throw_ frame state .abort)
          | some value =>
              let globals := state.globals.erase
                (globalKey ns.identity resourceType key)
              some (.value frame { state with globals } value)
      | .publish => do
          let [key, value] := arguments.toList | none
          let _ ← key.storageKey?
          match globalValue? state ns.identity resourceType key with
          | some _ => some (.throw_ frame state .abort)
          | none =>
              let globals := state.globals.insert
                (globalKey ns.identity resourceType key) value
              some (.value frame { state with globals } .unit))) := rfl

/-! Native reference-value operations.  These helpers are below the lowering
boundary: result types and loan rows are direct arguments, never table or
arena lookups. -/

def dereferenceBorrow? (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) :=
  match arguments.toList with
  | [.borrow _ current] => some (frame, state, current)
  -- Shared references are represented by the observed value itself.  The
  -- static reference type distinguishes this case; no runtime wrapper or
  -- loan reconciliation is needed.
  | [value] => some (frame, state, value)
  | _ => none

def freezeBorrow? (resultType : ReferenceType) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) := do
  let #[.borrow loan current] := arguments | none
  if resultType.kind != .shared then none else
  let (frame, state) := applyWriteBack frame state loan current
  let frame := { frame with
    activeLoans := frame.activeLoans.filter (·.2 != loan) }
  some (frame, state, current)

def mutateBorrow? (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) :=
  match arguments.toList with
  | [.borrow loan _, value] =>
      match updateBorrowValue? frame state loan value with
      | some (frame, state) => some (frame, state, .unit)
      | none =>
          let (frame, state) := applyWriteBack frame state loan value
          some (frame, state, .unit)
  | _ => none

def endLoans? (loans : Array LoanId) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) :=
  /- The certificate lists a marker's loans in minting order, and a
  reborrow is minted after the loan it projects.  Folding from the right
  therefore reconciles innermost first: a hole settles before the value
  holding it moves, where the forward order would write a lender back
  still carrying it. -/
  let (frame, state) := loans.foldr (init := (frame, state))
    fun lexical (frame, state) =>
      match frame.activeLoans.find? (·.1 == ⟨lexical.index⟩) with
      | none => (frame, state)
      | some (_, loanInstance) =>
          let frame := { frame with
            activeLoans := frame.activeLoans.filter (·.1 != ⟨lexical.index⟩) }
          match findBorrowValue? frame state loanInstance with
          | some current =>
              let (frame, state) := clearBorrowValue frame state loanInstance
              applyWriteBack frame state loanInstance current
          | none => (frame, state)
  match arguments.toList with
  | [value] => some (frame, state, value)
  | [] => some (frame, state, .unit)
  | _ => none

/-- Execute the explicit borrow-analysis death marker for a returned scalar
reborrow.  The marker removes the resting returned borrow, fills its hole in
the lender, and retires the lexical row.  Its cached address is deliberately
irrelevant here: loan identity and death both come from the validated marker
and the prophetic value graph. -/
theorem endLoans?_returnedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ loan) :
    endLoans? #[(⟨0⟩ : LoanId)] #[]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state =
      some
        ({ locals := #[some (.borrow outerLoan current), some .unit]
           activeLoans := #[]
           loanLocations },
         state, .unit) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst, indexOfFrom,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    separate, siteSelf, siteNotDifferent]

/-- Pass-through form of `endLoans?_returnedReborrow_zero`.  Validation can
attach the same explicit death marker to a value-producing expression; the
marker reconciles the loan and returns that value unchanged. -/
theorem endLoans?_returnedReborrow_zero_value
    (state : RuntimeState) (outerLoan loan : Nat) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ loan) :
    endLoans? #[(⟨0⟩ : LoanId)] #[argument]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state =
      some
        ({ locals := #[some (.borrow outerLoan current), some .unit]
           activeLoans := #[]
           loanLocations },
         state, argument) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst, indexOfFrom,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    separate, siteSelf, siteNotDifferent]

/-- End a field-projected returned reborrow.  The explicit analysis marker
selects the dynamic loan; write-back fills the matching prophecy hole inside
the nominal lender, without consulting an owning projection path. -/
theorem endLoans?_returnedProjectedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (name : StructHandle)
    (right : Int) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ loan) :
    endLoans? #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.borrow outerLoan
              (.nominal name none #[.loanHole loan, .integer right])),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state =
      some
        ({ locals := #[some (.borrow outerLoan
              (.nominal name none #[current, .integer right])), some .unit]
           activeLoans := #[]
           loanLocations },
         state, .unit) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp [endLoans?, findBorrowValue?, findFirst, findFirstList,
    clearBorrowValue, rewriteFirst, rewriteFirstList, indexOfFrom,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?,
    Array.filter, separate, siteSelf, siteNotDifferent]

/-- End a reference returned from a global projection after the caller has
updated it.  The caller retains only the dynamic loan in its local and the
lexical-to-dynamic row emitted by borrow analysis.  Settlement discovers the
prophecy hole in the global value itself; no path back to the owning key is
stored on the reference. -/
theorem endLoans?_returnedGlobalProjection_zero
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (name : StructHandle)
    (right current : Int) :
    endLoans? #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.address address), some (.borrow loan (.integer current))]
          activeLoans := #[(⟨0⟩, loan)] }
        { globals := globals.insert key
              (.nominal name none #[.loanHole loan, .integer right])
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some .unit]
           activeLoans := #[] },
         { globals := (globals.insert key
               (.nominal name none #[.loanHole loan, .integer right])).insert key
               (.nominal name none #[.integer current, .integer right])
           globalLoans := rest
           nextLoan
           pending },
         .unit) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp [endLoans?, findBorrowValue?, findFirst, findFirstList,
    clearBorrowValue, rewriteFirst, rewriteFirstList, indexOfFrom,
    applyWriteBack, fillVisibleHole, holeInFrame, holeInGlobals,
    holeWithin, fillHole?, globalLoanKey?, globalLoanKeyIn?,
    transferGlobalLoan, transferredLoan?, removeGlobalLoan, Array.filter,
    siteSelf, siteNotDifferent]

/-- End both members of a packed pair of returned reborrows.  The validated
lexical rows supply the dynamic identities; folding right-to-left settles
each returned current into its lender hole before clearing the resting
borrow local. -/
theorem endLoans?_twoReturnedReborrows
    (state : RuntimeState) (leftOuter rightOuter nextLoan : Nat)
    (left right : Int)
    (leftPrior : leftOuter < nextLoan)
    (rightPrior : rightOuter < nextLoan) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[.unit]
        { locals := #[some (.borrow leftOuter (.loanHole (nextLoan + 1 + 1))),
            some (.borrow rightOuter (.loanHole (nextLoan + 1 + 1 + 1))),
            some (.borrow (nextLoan + 1 + 1) (.integer left)),
            some (.borrow (nextLoan + 1 + 1 + 1) (.integer right))]
          activeLoans := #[(⟨0⟩, nextLoan + 1 + 1),
            (⟨1⟩, nextLoan + 1 + 1 + 1)]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1 + 1,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state =
      some
        ({ locals := #[some (.borrow leftOuter (.integer left)),
              some (.borrow rightOuter (.integer right)),
              some .unit, some .unit]
           activeLoans := #[]
           loanLocations :=
             #[(leftOuter,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (rightOuter,
                 (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (nextLoan + 1 + 1,
                 (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
               (nextLoan + 1 + 1 + 1,
                 (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
         state, .unit) := by
  have leftSeparateFirst : leftOuter ≠ nextLoan + 1 + 1 := by omega
  have leftSeparateSecond : leftOuter ≠ nextLoan + 1 + 1 + 1 := by omega
  have rightSeparateFirst : rightOuter ≠ nextLoan + 1 + 1 := by omega
  have rightSeparateSecond : rightOuter ≠ nextLoan + 1 + 1 + 1 := by omega
  have siteZeroZero : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by decide
  have siteZeroOne : ((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false := by decide
  have siteOneZero : ((⟨1⟩ : ExprId) == (⟨0⟩ : ExprId)) = false := by decide
  have siteOneOne : ((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true := by decide
  have siteZeroNotZero : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by decide
  have siteZeroNotOne : ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true := by decide
  have siteOneNotZero : ((⟨1⟩ : ExprId) != (⟨0⟩ : ExprId)) = true := by decide
  have siteOneNotOne : ((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false := by decide
  simp [endLoans?, Array.find?, List.find?, findBorrowValue?, findFirst, findFirstList,
    clearBorrowValue, rewriteFirst, rewriteFirstList, indexOfFrom,
    applyWriteBack, fillVisibleHole, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    writeRuntimePlace?, writeRoot?, writeProjections?,
    holeInFrame, holeWithin, fillHole?, Array.filter,
    leftSeparateFirst, leftSeparateSecond,
    rightSeparateFirst, rightSeparateSecond,
    siteZeroZero, siteZeroOne, siteOneZero, siteOneOne,
    siteZeroNotZero, siteZeroNotOne, siteOneNotZero, siteOneNotOne]

/-- Execute the explicit function-exit death of a global borrow held in the
third local. The global-loan registry supplies the write-back key directly;
the reference contains only its dynamic loan and prophetic current value. -/
theorem endLoans?_globalBorrowThirdLocal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (argument : Int)
    (current : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.address address), some (.integer argument),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(loan, { root := .global key })] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some (.integer argument),
              some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key })] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               current
           globalLoans := transferGlobalLoan ((loan, key) :: rest) loan key current
           nextLoan
           pending },
         .unit) := by
  have siteSelf :
      (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by
    decide
  have siteNotDifferent :
      (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by
    decide
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst, indexOfFrom, applyWriteBack,
    fillVisibleHole, holeInFrame, holeWithin, globalLoanKey?, globalLoanKeyIn?,
    removeGlobalLoan, transferGlobalLoan, transferredLoan?, fillHole?,
    rewriteFirstList, Array.filter, siteSelf, siteNotDifferent]

/-- Execute the paired deaths of a field-focused global borrow.  The marker
lists the enclosing resource loan before the projected field loan, so
`endLoans?` settles the field first and then writes the reconstructed resource
to the key recorded in the global-loan registry. -/
theorem endLoans?_focusedGlobalNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (outerName innerName : StructHandle) (argument : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨3⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some .unit, some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨3⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (.nominal outerName none
                 #[.nominal innerName none #[.integer value]])
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have outer_ne_inner : loan ≠ loan + 1 := by omega
  have inner_ne_outer : loan + 1 ≠ loan := by omega
  have zero_eq_zero :
      (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one :
      (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one :
      (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have one_eq_zero :
      (((⟨1⟩ : ExprId) == (⟨0⟩ : ExprId)) = false) := by decide
  have zero_ne_zero :
      (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one :
      (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one :
      (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  have one_ne_zero :
      (((⟨1⟩ : ExprId) != (⟨0⟩ : ExprId)) = true) := by decide
  simp [endLoans?, Array.find?, List.find?, findBorrowValue?, findFirst,
    findFirstList,
    clearBorrowValue, rewriteFirst, indexOfFrom, applyWriteBack,
    fillVisibleHole, holeInFrame, holeWithin, fillHole?, rewriteFirstList,
    globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan, transferGlobalLoan,
    transferredLoan?, fillHole?, rewriteFirstList, Array.filter,
    outer_ne_inner, inner_ne_outer, zero_eq_zero, one_eq_one, zero_eq_one,
    one_eq_zero, zero_ne_zero, one_ne_one, zero_ne_one, one_ne_zero]

/-- Saved-value variant of `endLoans?_focusedGlobalNominal`.  Reading the
field before mutation leaves one scalar local between the focused reference
and its enclosing global holder. -/
theorem endLoans?_focusedGlobalNominalSaved
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount saved value : Int)
    (outerName innerName : StructHandle) (argument : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)), some (.integer saved),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some .unit, some (.integer saved), some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (.nominal outerName none
                 #[.nominal innerName none #[.integer value]])
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have outer_ne_inner : loan ≠ loan + 1 := by omega
  have inner_ne_outer : loan + 1 ≠ loan := by omega
  have zero_eq_zero :
      (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one :
      (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one :
      (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have one_eq_zero :
      (((⟨1⟩ : ExprId) == (⟨0⟩ : ExprId)) = false) := by decide
  have zero_ne_zero :
      (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one :
      (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one :
      (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  have one_ne_zero :
      (((⟨1⟩ : ExprId) != (⟨0⟩ : ExprId)) = true) := by decide
  simp [endLoans?, Array.find?, List.find?, findBorrowValue?, findFirst,
    findFirstList,
    clearBorrowValue, rewriteFirst, indexOfFrom, applyWriteBack,
    fillVisibleHole, holeInFrame, holeWithin, fillHole?, rewriteFirstList,
    globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan, transferGlobalLoan,
    transferredLoan?, fillHole?, rewriteFirstList, Array.filter,
    outer_ne_inner, inner_ne_outer, zero_eq_zero, one_eq_one, zero_eq_one,
    one_eq_zero, zero_ne_zero, one_ne_one, zero_ne_one, one_ne_zero]

/-- Deterministic meaning of all currently executable place operations. -/
def evaluatePlaceOperation? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (operation : Operation)
    (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    Option (RuntimeFrame × RuntimeState × RuntimeValue) := do
  match operation with
  | .copy place | .read place =>
      if !arguments.isEmpty then none else
      let resolved ← resolvePlace? unit ns frame state place
      let value ← readRuntimePlace? frame state resolved
      some (frame, state, value)
  | .move place =>
      if !arguments.isEmpty then none else
      let resolved ← resolvePlace? unit ns frame state place
      let value ← readRuntimePlace? frame state resolved
      let .local localId := resolved.root | none
      if resolved.projections.isEmpty then
        let frame := { frame with locals := frame.locals.set! localId.index none }
        some (frame, state, value)
      else
        -- Prepared projected consumption is restricted to owned local indexes,
        -- subslices, and fields, optionally qualified by an enum downcast.
        -- The initialization certificate makes the retained aggregate field
        -- observationally inaccessible until it is rewritten.
        some (frame, state, value)
  | .borrow kind place =>
      if !arguments.isEmpty then none else
      let .reference referenceType ← ns.tables.types[resultType.index]? | none
      let resolved ← resolvePlace? unit ns frame state place
      borrowRuntimePlace? unit ns site referenceType kind frame state resolved
  | .write place =>
      let #[value] := arguments | none
      let resolved ← resolvePlace? unit ns frame state place
      let (frame, state) ← writeRuntimePlace? frame state resolved value
      some (frame, state, .unit)
  | .drop place =>
      if !arguments.isEmpty then none else
      let resolved ← resolvePlace? unit ns frame state place
      let .local localId := resolved.root | none
      if resolved.projections.isEmpty then
        let frame := { frame with locals := frame.locals.set! localId.index none }
        some (frame, state, .unit)
      else some (frame, state, .unit)
  | .reference .dereference =>
      -- Shared dereferences are erased at preparation; a residual
      -- dereference observes a live mutable borrow's current value.
      dereferenceBorrow? arguments frame state
  | .reference (.freeze _) =>
      -- A shared-source freeze is erased at preparation. Freezing a
      -- mutable borrow consumes it: its loan ends here with the current
      -- value, and the result is the bare shared observation.
      let .reference resultReference ← ns.tables.types[resultType.index]? | none
      freezeBorrow? resultReference arguments frame state
  | .reference (.endLoan loans) =>
      -- Reunite each named live loan's hole with its current value. A loan
      -- not created on this path, or already reconciled through a call
      -- boundary, is a no-op.
      endLoans? loans arguments frame state
  | .reference .mutate =>
      -- Mutation through a reference value updates the live borrow wherever
      -- it rests; a consumed temporary has no resting place and reconciles
      -- with its hole immediately.
      mutateBorrow? arguments frame state
  | .data operation => do
      let value ← evaluateDataOperation? unit ns.identity operation arguments
      some (frame, state, value)
  | _ => none

/-- Bind a pattern row pointwise through `bind`.  Structural recursion over
the lists so head reduction executes it — `Array.zip` and `Array.foldlM` are
well-founded recursions symbolic execution cannot unfold. -/
private def bindPatternRow
    (bind : PatternId → RuntimeValue → RuntimeFrame → Option RuntimeFrame) :
    List PatternId → List RuntimeValue → RuntimeFrame → Option RuntimeFrame
  | [], [], frame => some frame
  | pattern :: patterns, value :: values, frame => do
      bindPatternRow bind patterns values (← bind pattern value frame)
  | _, _, _ => none

/-- A pattern after lowering has replaced every arena edge and interned
spelling with its direct semantic payload. -/
inductive NativePattern where
  | wildcard
  | variable (localId : LocalId)
  | tuple (elements : List NativePattern)
  | constructor (source : StructHandle) (variant : Option String)
      (fields : List NativePattern)
  | literal (value : RuntimeValue)
  | range (lower upper : Option RuntimeValue) (inclusive : Bool)
  deriving Repr, BEq, Inhabited

/-- A closed native pattern together with the structural recursion budget
certified while lowering its arena representation. -/
structure NativePatternBinder where
  fuel : Nat
  pattern : NativePattern
  deriving Repr, BEq, Inhabited

def bindNativePatternRow
    (bind : NativePattern → RuntimeValue → RuntimeFrame → Option RuntimeFrame) :
    List NativePattern → List RuntimeValue → RuntimeFrame → Option RuntimeFrame
  | [], [], frame => some frame
  | pattern :: patterns, value :: values, frame => do
      bindNativePatternRow bind patterns values (← bind pattern value frame)
  | _, _, _ => none

/-- Match and bind a lowered pattern.  Recursion is structural in the
lowering-supplied natural-number budget, so reducing a closed binder never
consults an arena or unfolds a well-founded search. -/
def bindNativePatternFuel (frame : RuntimeFrame) :
    Nat → NativePattern → RuntimeValue → Option RuntimeFrame
  | 0, _, _ => none
  | fuel + 1, pattern, value =>
      match pattern with
      | .wildcard => some frame
      | .variable localId =>
          if localId.index < frame.locals.size then
            some { frame with
              locals := frame.locals.set! localId.index (some value) }
          else none
      | .tuple elements =>
          match value with
          | .tuple values =>
              if elements.length != values.size then none else
                bindNativePatternRow (fun pattern value frame =>
                    bindNativePatternFuel frame fuel pattern value)
                  elements values.toList frame
          | _ => none
      | .literal expected => if expected == value then some frame else none
      | .range lower upper inclusive =>
          match value with
          | .integer actual => do
              let lowerOk ← match lower with
                | none => some true
                | some (.integer value) => some (value <= actual)
                | some _ => none
              let upperOk ← match upper with
                | none => some true
                | some (.integer value) =>
                    some (if inclusive then actual <= value else actual < value)
                | some _ => none
              if lowerOk && upperOk then some frame else none
          | .character actual => do
              let lowerOk ← match lower with
                | none => some true
                | some (.character value) => some (value <= actual)
                | some _ => none
              let upperOk ← match upper with
                | none => some true
                | some (.character value) =>
                    some (if inclusive then actual <= value else actual < value)
                | some _ => none
              if lowerOk && upperOk then some frame else none
          | _ => none
      | .constructor expectedName variant fields =>
          match value with
          | .nominal actualName actualVariant values =>
              if expectedName != actualName || variant != actualVariant ||
                  fields.length != values.size then none else
                bindNativePatternRow (fun pattern value frame =>
                    bindNativePatternFuel frame fuel pattern value)
                  fields values.toList frame
          | _ => none

def NativePatternBinder.bind (binder : NativePatternBinder)
    (frame : RuntimeFrame) (value : RuntimeValue) : Option RuntimeFrame :=
  bindNativePatternFuel frame binder.fuel binder.pattern value

private def lowerPatternRow?
    (lower : PatternId → Option NativePattern) :
    List PatternId → Option (List NativePattern)
  | [] => some []
  | id :: ids => do
      let pattern ← lower id
      let patterns ← lowerPatternRow? lower ids
      some (pattern :: patterns)

private def lowerPatternBound? :
    Option ConstValue → Option (Option RuntimeValue)
  | none => some none
  | some value => some <$> constValue? value

/-- Partially evaluate one pattern-arena node into its native shape. -/
def lowerPatternFuel? (unit : ValidatedUnit) (ns : ValidatedNamespace) :
    Nat → PatternId → Option NativePattern
  | 0, _ => none
  | fuel + 1, patternId => do
      let pattern ← ns.patterns[patternId.index]?
      match pattern.kind with
      | .wildcard => some .wildcard
      | .variable localId => some (.variable localId)
      | .tuple elements =>
          .tuple <$> lowerPatternRow? (lowerPatternFuel? unit ns fuel) elements.toList
      | .constructor name _ variant fields => do
          let source ← resolveNominal? unit ns name
          let lowered ← lowerPatternRow? (lowerPatternFuel? unit ns fuel) fields.toList
          some (.constructor source variant lowered)
      | .literal _ | .range .. => none

/-- Lower a checked pattern with the same exact fuel used by `bindPattern`. -/
def lowerPattern? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (patternId : PatternId) : Option NativePatternBinder := do
  let fuel := ns.patterns.size + 1
  let pattern ← lowerPatternFuel? unit ns fuel patternId
  some { fuel, pattern }

private def bindPatternFuel (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (fuel : Nat) (patternId : PatternId)
    (value : RuntimeValue) : Option RuntimeFrame := do
  match fuel with
  | 0 => none
  | fuel + 1 =>
  let pattern ← ns.patterns[patternId.index]?
  match pattern.kind with
  | .wildcard => return frame
  | .variable localId =>
      if localId.index < frame.locals.size then
        return { frame with locals := frame.locals.set! localId.index (some value) }
      else none
  | .tuple elements =>
      let .tuple values := value | none
      if elements.size != values.size then none else
        bindPatternRow (fun pattern value frame =>
            bindPatternFuel unit ns frame fuel pattern value)
          elements.toList values.toList frame
  | .literal literal =>
      let expected ← constValue? literal
      if expected == value then return frame else none
  | .range lower upper inclusive =>
      match value with
      | .integer actual =>
          let lowerOk ← match lower with
            | none => some true
            | some bound => match constValue? bound with
              | some (.integer value) => some (value <= actual)
              | _ => none
          let upperOk ← match upper with
            | none => some true
            | some bound => match constValue? bound with
              | some (.integer value) =>
                  some (if inclusive then actual <= value else actual < value)
              | _ => none
          if lowerOk && upperOk then return frame else none
      | .character actual =>
          let lowerOk ← match lower with
            | none => some true
            | some bound => match constValue? bound with
              | some (.character value) => some (value <= actual)
              | _ => none
          let upperOk ← match upper with
            | none => some true
            | some bound => match constValue? bound with
              | some (.character value) =>
                  some (if inclusive then actual <= value else actual < value)
              | _ => none
          if lowerOk && upperOk then return frame else none
      | _ => none
  | .constructor name _ variant fields =>
      let .nominal actualSource actualVariant values := value | none
      let expected ← resolveNominal? unit ns name
      if expected != actualSource || variant != actualVariant || fields.size != values.size then none else
        bindPatternRow (fun pattern value frame =>
            bindPatternFuel unit ns frame fuel pattern value)
          fields.toList values.toList frame
termination_by structural fuel

/-- Atomically match and bind one M1 pattern.  Failure returns `none` without
exposing a partially updated frame.  The pattern arena is acyclic, so a fuel
of one unit per arena slot is exact rather than an approximation. -/
def bindPattern (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (patternId : PatternId) (value : RuntimeValue) :
    Option RuntimeFrame :=
  bindPatternFuel unit ns frame (ns.patterns.size + 1) patternId value

private theorem lowerPatternRow?_length
    {lower : PatternId → Option NativePattern}
    {ids : List PatternId} {native : List NativePattern}
    (lower_eq : lowerPatternRow? lower ids = some native) :
    native.length = ids.length := by
  induction ids generalizing native with
  | nil =>
      simp only [lowerPatternRow?] at lower_eq
      cases lower_eq
      rfl
  | cons id ids ih =>
      simp only [lowerPatternRow?] at lower_eq
      cases head_eq : lower id with
      | none => simp [head_eq] at lower_eq
      | some head =>
          cases tail_eq : lowerPatternRow? lower ids with
          | none => simp [head_eq, tail_eq] at lower_eq
          | some tail =>
              simp [head_eq, tail_eq] at lower_eq
              cases lower_eq
              simp [ih tail_eq]

private theorem lowerPatternRow?_bind_eq
    {lower : PatternId → Option NativePattern}
    {nativeBind : NativePattern → RuntimeValue → RuntimeFrame →
      Option RuntimeFrame}
    {sourceBind : PatternId → RuntimeValue → RuntimeFrame →
      Option RuntimeFrame}
    (sound : ∀ {id native}, lower id = some native →
      ∀ value frame, nativeBind native value frame =
        sourceBind id value frame)
    {ids : List PatternId} {native : List NativePattern}
    (lower_eq : lowerPatternRow? lower ids = some native) :
    ∀ values frame,
      bindNativePatternRow nativeBind native values frame =
        bindPatternRow sourceBind ids values frame := by
  induction ids generalizing native with
  | nil =>
      simp only [lowerPatternRow?] at lower_eq
      cases lower_eq
      intro values frame
      cases values <;> rfl
  | cons id ids ih =>
      simp only [lowerPatternRow?] at lower_eq
      cases head_eq : lower id with
      | none => simp [head_eq] at lower_eq
      | some head =>
          cases tail_eq : lowerPatternRow? lower ids with
          | none => simp [tail_eq] at lower_eq
          | some tail =>
              simp [head_eq, tail_eq] at lower_eq
              cases lower_eq
              intro values frame
              cases values with
              | nil => rfl
              | cons value values =>
                  simp only [bindNativePatternRow, bindPatternRow]
                  rw [sound head_eq value frame]
                  cases bound_eq : sourceBind id value frame with
                  | none => rfl
                  | some bound =>
                      exact ih tail_eq values bound

theorem lowerPatternFuel?_bind_eq {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {fuel : Nat} {patternId : PatternId} {native : NativePattern}
    (lower_eq : lowerPatternFuel? unit ns fuel patternId = some native) :
    ∀ frame value,
      bindNativePatternFuel frame fuel native value =
        bindPatternFuel unit ns frame fuel patternId value := by
  induction fuel generalizing patternId native with
  | zero => simp [lowerPatternFuel?] at lower_eq
  | succ fuel ih =>
      simp only [lowerPatternFuel?] at lower_eq
      cases pattern_eq : ns.patterns[patternId.index]? with
      | none => simp [pattern_eq] at lower_eq
      | some pattern =>
          simp only [pattern_eq, Option.bind_some] at lower_eq
          simp only [bind, Option.bind] at lower_eq
          cases kind_eq : pattern.kind with
          | wildcard =>
              rw [kind_eq] at lower_eq
              simp only [Option.some.injEq] at lower_eq
              subst native
              intro frame value
              simp [bindNativePatternFuel, bindPatternFuel, pattern_eq, kind_eq]
          | «variable» localId =>
              rw [kind_eq] at lower_eq
              simp only [Option.some.injEq] at lower_eq
              subst native
              intro frame value
              simp [bindNativePatternFuel, bindPatternFuel, pattern_eq, kind_eq]
          | tuple elements =>
              rw [kind_eq] at lower_eq
              cases row_eq : lowerPatternRow?
                  (lowerPatternFuel? unit ns fuel) elements.toList with
              | none => simp [row_eq] at lower_eq
              | some lowered =>
                  simp [row_eq] at lower_eq
                  cases lower_eq
                  intro frame value
                  cases value <;>
                    simp [bindNativePatternFuel, bindPatternFuel, pattern_eq,
                      kind_eq]
                  case tuple values =>
                    have lowered_length : lowered.length = elements.size := by
                      have := lowerPatternRow?_length row_eq
                      simpa using this
                    by_cases source_size_eq : elements.size = values.size
                    · have native_size_eq : lowered.length = values.size :=
                        lowered_length.trans source_size_eq
                      simp [source_size_eq, native_size_eq]
                      apply lowerPatternRow?_bind_eq
                        (lower := lowerPatternFuel? unit ns fuel)
                        (nativeBind := fun pattern value frame =>
                          bindNativePatternFuel frame fuel pattern value)
                        (sourceBind := fun id value frame =>
                          bindPatternFuel unit ns frame fuel id value)
                        (fun child_eq value frame => ih child_eq frame value)
                        row_eq
                    · have native_size_ne : lowered.length ≠ values.size := by
                        simpa [lowered_length] using source_size_eq
                      simp [source_size_eq, native_size_ne]
          | «constructor» name instantiations variant fields =>
              rw [kind_eq] at lower_eq
              cases resolve_eq : resolveNominal? unit ns name with
              | none => simp [resolve_eq] at lower_eq
              | some source =>
                  simp only [resolve_eq, Option.bind_some] at lower_eq
                  cases row_eq : lowerPatternRow?
                      (lowerPatternFuel? unit ns fuel) fields.toList with
                  | none => simp [row_eq] at lower_eq
                  | some lowered =>
                      simp [row_eq, bind, Option.bind] at lower_eq
                      cases lower_eq
                      intro frame value
                      cases value <;>
                        simp [bindNativePatternFuel, bindPatternFuel, pattern_eq,
                          kind_eq, resolve_eq]
                      case nominal actualSource actualVariant values =>
                        by_cases sources_eq : source = actualSource
                        · subst actualSource
                          by_cases variants_eq : variant = actualVariant
                          · subst actualVariant
                            have lowered_length : lowered.length = fields.size := by
                              have := lowerPatternRow?_length row_eq
                              simpa using this
                            by_cases source_size_eq : fields.size = values.size
                            · have native_size_eq : lowered.length = values.size :=
                                lowered_length.trans source_size_eq
                              simp [source_size_eq, native_size_eq]
                              apply lowerPatternRow?_bind_eq
                                (lower := lowerPatternFuel? unit ns fuel)
                                (nativeBind := fun pattern value frame =>
                                  bindNativePatternFuel frame fuel pattern value)
                                (sourceBind := fun id value frame =>
                                  bindPatternFuel unit ns frame fuel id value)
                                (fun child_eq value frame => ih child_eq frame value)
                                row_eq
                            · have native_size_ne : lowered.length ≠ values.size := by
                                simpa [lowered_length] using source_size_eq
                              simp [source_size_eq, native_size_ne]
                          · simp [variants_eq]
                        · simp [sources_eq]
          | literal literal =>
              rw [kind_eq] at lower_eq
              cases lower_eq
          | range lower upper inclusive =>
              rw [kind_eq] at lower_eq
              cases lower_eq

theorem lowerPattern?_bind_eq {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {patternId : PatternId} {binder : NativePatternBinder}
    (lower_eq : lowerPattern? unit ns patternId = some binder) :
    ∀ frame value,
      binder.bind frame value = bindPattern unit ns frame patternId value := by
  simp only [lowerPattern?, NativePatternBinder.bind, bindPattern] at lower_eq ⊢
  cases pattern_eq : lowerPatternFuel? unit ns (ns.patterns.size + 1) patternId with
  | none => simp [pattern_eq] at lower_eq
  | some native =>
      simp [pattern_eq] at lower_eq
      cases lower_eq
      exact lowerPatternFuel?_bind_eq pattern_eq

/-- Write the M1 local-place subset. -/
def writePlace (ns : ValidatedNamespace) (frame : RuntimeFrame)
    (placeId : PlaceId) (value : RuntimeValue) : Option RuntimeFrame := do
  let place ← ns.places[placeId.index]?
  match place with
  | .localVar localId =>
      if localId.index < frame.locals.size then
        return { frame with locals := frame.locals.set! localId.index (some value) }
      else none
  | _ => none

/-- Pack a function's result vector into the value produced at a call site. -/
def packResults (values : Array RuntimeValue) : RuntimeValue :=
  match values with
  | #[] => .unit
  | #[value] => value
  | values => .tuple values

/-- Interpret a fall-through body value as the declared function results.
The unit test matches structurally: the derived `BEq` on `RuntimeValue` is a
`partial` definition nothing can reason about. -/
def unpackFallthrough (expected : Nat) (value : RuntimeValue) : Option (Array RuntimeValue) :=
  match expected with
  | 0 => match value with
    | .unit => some #[]
    | _ => none
  | 1 => some #[value]
  | _ => match value with
    | .tuple values => if values.size == expected then some values else none
    | _ => none

/-- Native local addresses of mutable-reference arguments in a fresh frame.
The argument array and its leading-local placement are already known at the
function boundary, so mutation need not rediscover those loans by scanning. -/
def parameterLoanLocations (arguments : Array RuntimeValue) :
    Array (Nat × RuntimePlace) :=
  arguments.zipIdx.filterMap fun (value, index) =>
    match value with
    | .borrow loan _ => some (loan, { root := .local ⟨index⟩ })
    | _ => none

/-- The native address table for a single mutable-reference parameter is a
single, concrete local location.  Keeping this constructor equation closed
prevents `Array.filterMap` from leaking from frame initialization into a
proof. -/
theorem parameterLoanLocations_singleBorrow (loan : Nat)
    (current : RuntimeValue) :
    parameterLoanLocations #[.borrow loan current] =
      #[(loan,
          (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp [parameterLoanLocations]

/-- Two mutable parameters are lowered to their two leading local slots.
The dynamic loan identifiers remain values, but locating either loan is a
closed lookup over this native row. -/
theorem parameterLoanLocations_twoBorrows (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent : RuntimeValue) :
    parameterLoanLocations
        #[.borrow leftLoan leftCurrent, .borrow rightLoan rightCurrent] =
      #[(leftLoan,
          (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
        (rightLoan,
          (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp [parameterLoanLocations]

/-- A scalar argument following one mutable parameter contributes no loan
location; lowering retains only the parameter's local-zero address. -/
theorem parameterLoanLocations_borrowInteger (loan : Nat)
    (current : RuntimeValue) (value : Int) :
    parameterLoanLocations #[.borrow loan current, .integer value] =
      #[(loan,
          (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp [parameterLoanLocations]

/-- A `Bool` selector in front of two mutable parameters: the two loans
sit in locals 1 and 2. -/
theorem parameterLoanLocations_boolTwoBorrows (flag : Bool) (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent : RuntimeValue) :
    parameterLoanLocations
        #[.bool flag, .borrow leftLoan leftCurrent, .borrow rightLoan rightCurrent] =
      #[(leftLoan, (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
        (rightLoan, (⟨.local (⟨2⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp [parameterLoanLocations]

/-- The `Bool` twin of `parameterLoanLocations_borrowInteger`. -/
theorem parameterLoanLocations_borrowBool (loan : Nat)
    (current : RuntimeValue) (value : Bool) :
    parameterLoanLocations #[.borrow loan current, .bool value] =
      #[(loan,
          (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp [parameterLoanLocations]

/-- Constructor residual of `parameterLoanLocations_borrowInteger` after
`zipIdx` has discarded the scalar parameter.  Keeping this intermediate row
native lets later reborrow reconciliation use the cached local-zero address
without normalizing `Array.filterMap` inside the verification condition. -/
@[simp] theorem parameterLoanLocations_filterMap_singleBorrow (loan : Nat)
    (current : RuntimeValue) :
    Array.filterMap
        (fun row : RuntimeValue × Nat =>
          match row.1 with
          | .borrow instance_ _ =>
              some (instance_, { root := .local ⟨row.2⟩ })
          | _ => none)
        #[((.borrow loan current), 0)] =
      #[(loan,
          (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] := by
  simp

/-- A scalar-only parameter row contains no dynamic loan addresses. -/
theorem parameterLoanLocations_singleInteger (value : Int) :
    parameterLoanLocations #[.integer value] = #[] := by
  simp [parameterLoanLocations]

/-- The `Bool` twin of `parameterLoanLocations_singleInteger`. -/
theorem parameterLoanLocations_singleBool (value : Bool) :
    parameterLoanLocations #[.bool value] = #[] := by
  simp [parameterLoanLocations]

/-- Address and integer parameters are both scalar, so storage entry points
start with an empty native loan-location row. -/
theorem parameterLoanLocations_addressInteger (address : String) (value : Int) :
    parameterLoanLocations #[.address address, .integer value] = #[] := by
  simp [parameterLoanLocations]

/-- Direct selection from the native address row for one mutable
parameter. -/
theorem localLoanPlace?_singleLocal
    (locals : Array (Option RuntimeValue))
    (activeLoans : Array (ExprId × Nat)) (loan : Nat) :
    localLoanPlace?
        { locals
          activeLoans
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        loan =
      some (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace) := by
  simp [localLoanPlace?]

/-- A singleton native address row selects local zero independently of how
many trailing locals the function has allocated.  The two premises are the
closed read/bounds facts lowering can compute for that concrete frame; the
result is a direct array update, with no local or value search. -/
theorem updateBorrowValue?_localZero
    (state : RuntimeState) (loan : Nat)
    (current replacement : RuntimeValue)
    (locals : Array (Option RuntimeValue))
    (activeLoans : Array (ExprId × Nat))
    (atZero : locals[0]? = some (some (.borrow loan current)))
    (nonempty : 0 < locals.size) :
    updateBorrowValue?
        { locals
          activeLoans
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := locals.set! 0 (some (.borrow loan replacement))
           activeLoans
           loanLocations :=
             #[(loan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
         state) := by
  have atZero' : locals[0] = some (.borrow loan current) := by
    have found := Array.getElem?_eq_getElem (xs := locals) (i := 0) nonempty
    rw [found] at atZero
    exact Option.some.inj atZero
  have notEmpty : locals ≠ #[] := by
    intro empty
    subst locals
    simp at nonempty
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?, atZero', notEmpty, nonempty]

/-- Updating the sole mutable parameter uses its native local-zero address;
the generic fallback search is unreachable for this lowered frame shape. -/
theorem updateBorrowValue?_singleLocal
    (state : RuntimeState) (loan : Nat)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.borrow loan current)]
          activeLoans
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.borrow loan replacement)]
           activeLoans
           loanLocations :=
             #[(loan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?]

/-- The same native update while preserving one trailing local.  This is the
shape produced after a function saves a scalar result beside its mutable
parameter. -/
theorem updateBorrowValue?_singleLocal_pair
    (state : RuntimeState) (loan : Nat)
    (current replacement : RuntimeValue) (saved : Option RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.borrow loan current), saved]
          activeLoans
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.borrow loan replacement), saved]
           activeLoans
           loanLocations :=
             #[(loan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?]

/-- Update a returned reborrow after it has been bound beside its lender.
The retargeted address index still names the lender's hole, so the indexed
attempt validates and fails before the semantic local scan finds the resting
returned borrow in slot one.  This constructor equation makes that fallback
native without turning the address index into ownership state. -/
theorem updateBorrowValue?_returnedReborrow_pair
    (state : RuntimeState) (outerLoan loan : Nat)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    updateBorrowValue?
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.borrow outerLoan (.loanHole loan)),
              some (.borrow loan replacement)]
           activeLoans
           loanLocations :=
             #[(outerLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (loan,
                 (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    rewriteFirstList,
    writeRuntimePlace?, writeRoot?, indexOfFrom, separate]

/-- Update a returned field reborrow resting beside its nominal lender.
The native cache names only the caller-local dereference; the semantic
fallback identifies the returned borrow by loan id and does not reconstruct
or retain an ownership path. -/
theorem updateBorrowValue?_returnedProjectedReborrow_pair
    (state : RuntimeState) (outerLoan loan : Nat) (name : StructHandle)
    (right : Int) (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    updateBorrowValue?
        { locals := #[some (.borrow outerLoan
              (.nominal name none #[.loanHole loan, .integer right])),
            some (.borrow loan current)]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.borrow outerLoan
              (.nominal name none #[.loanHole loan, .integer right])),
              some (.borrow loan replacement)]
           activeLoans
           loanLocations :=
             #[(outerLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (loan,
                 (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    rewriteFirstList,
    writeRuntimePlace?, writeRoot?, indexOfFrom, separate]

/-- Update a returned external/global borrow with no owner-location cache in
the caller.  The resting reference is selected by loan identity; its owning
global is represented only by the prophecy hole already in runtime state. -/
theorem updateBorrowValue?_addressReturnedBorrow
    (state : RuntimeState) (address : String) (loan : Nat)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.address address), some (.borrow loan current)]
          activeLoans }
        state loan replacement =
      some
        ({ locals := #[some (.address address), some (.borrow loan replacement)]
           activeLoans },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    rewriteFirst, indexOfFrom]

/-- Update the first member of a packed pair of returned reborrows.  Both
native cache entries name lender holes, so the checked fallback finds the
resting returned borrow in local two. -/
theorem updateBorrowValue?_twoReturnedReborrows_left
    (state : RuntimeState)
    (leftOuter rightOuter leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent : Int) (replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (leftOuterSeparate : leftOuter ≠ leftLoan)
    (rightOuterSeparate : rightOuter ≠ leftLoan)
    (loansSeparate : rightLoan ≠ leftLoan) :
    updateBorrowValue?
        { locals := #[some (.borrow leftOuter (.loanHole leftLoan)),
            some (.borrow rightOuter (.loanHole rightLoan)),
            some (.borrow leftLoan (.integer leftCurrent)),
            some (.borrow rightLoan (.integer rightCurrent))]
          activeLoans
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state leftLoan replacement =
      some
        ({ locals := #[some (.borrow leftOuter (.loanHole leftLoan)),
              some (.borrow rightOuter (.loanHole rightLoan)),
              some (.borrow leftLoan replacement),
              some (.borrow rightLoan (.integer rightCurrent))]
           activeLoans
           loanLocations :=
             #[(leftOuter,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (rightOuter,
                 (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (leftLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
               (rightLoan,
                 (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?, indexOfFrom, leftOuterSeparate,
    rightOuterSeparate, loansSeparate]

/-- Update the second member of a packed pair of returned reborrows. -/
theorem updateBorrowValue?_twoReturnedReborrows_right
    (state : RuntimeState)
    (leftOuter rightOuter leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent : Int) (replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (leftOuterSeparate : leftOuter ≠ rightLoan)
    (rightOuterSeparate : rightOuter ≠ rightLoan)
    (loansSeparate : leftLoan ≠ rightLoan) :
    updateBorrowValue?
        { locals := #[some (.borrow leftOuter (.loanHole leftLoan)),
            some (.borrow rightOuter (.loanHole rightLoan)),
            some (.borrow leftLoan (.integer leftCurrent)),
            some (.borrow rightLoan (.integer rightCurrent))]
          activeLoans
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state rightLoan replacement =
      some
        ({ locals := #[some (.borrow leftOuter (.loanHole leftLoan)),
              some (.borrow rightOuter (.loanHole rightLoan)),
              some (.borrow leftLoan (.integer leftCurrent)),
              some (.borrow rightLoan replacement)]
           activeLoans
           loanLocations :=
             #[(leftOuter,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (rightOuter,
                 (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (leftLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
               (rightLoan,
                 (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?, indexOfFrom, leftOuterSeparate,
    rightOuterSeparate, loansSeparate]

/-- Direct selection from the native address row for two mutable
parameters.  The first lookup uses the contract's parameter-loan
separation; the second is the most recently recorded row. -/
theorem localLoanPlace?_twoLocals_left
    (locals : Array (Option RuntimeValue))
    (activeLoans : Array (ExprId × Nat)) (leftLoan rightLoan : Nat)
    (separate : leftLoan ≠ rightLoan) :
    localLoanPlace?
        { locals
          activeLoans
          loanLocations :=
            #[(leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        leftLoan =
      some (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace) := by
  simp [localLoanPlace?, separate, ne_comm]

theorem localLoanPlace?_twoLocals_right
    (locals : Array (Option RuntimeValue))
    (activeLoans : Array (ExprId × Nat)) (leftLoan rightLoan : Nat) :
    localLoanPlace?
        { locals
          activeLoans
          loanLocations :=
            #[(leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        rightLoan =
      some (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace) := by
  simp [localLoanPlace?]

/-- Updating the first of two mutable parameters follows its native local
address directly.  The separation premise is the parameter-loan invariant;
it decides the reverse lookup without exposing the registry walk to a
generated proof. -/
theorem updateBorrowValue?_twoLocals_left
    (state : RuntimeState) (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : leftLoan ≠ rightLoan) :
    updateBorrowValue?
        { locals := #[some (.borrow leftLoan leftCurrent),
            some (.borrow rightLoan rightCurrent)]
          activeLoans
          loanLocations :=
            #[(leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state leftLoan replacement =
      some
        ({ locals := #[some (.borrow leftLoan replacement),
              some (.borrow rightLoan rightCurrent)]
           activeLoans
           loanLocations :=
             #[(leftLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (rightLoan,
                 (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?, separate, ne_comm]

/-- Updating the second of two mutable parameters selects the newest native
address row, so it requires no dynamic search or separation side condition. -/
theorem updateBorrowValue?_twoLocals_right
    (state : RuntimeState) (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.borrow leftLoan leftCurrent),
            some (.borrow rightLoan rightCurrent)]
          activeLoans
          loanLocations :=
            #[(leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state rightLoan replacement =
      some
        ({ locals := #[some (.borrow leftLoan leftCurrent),
              some (.borrow rightLoan replacement)]
           activeLoans
           loanLocations :=
             #[(leftLoan,
                 (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
               (rightLoan,
                 (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?]

/-- A whole-resource global borrow is held in the third local of the V1
scoped-borrow shape.  Its native registry row names the global hole, while
the lowered frame shape names the resting reference slot.  Mutation updates
that slot directly; the generic local/global search is discharged once in
this certificate and never appears in the generated obligation. -/
theorem updateBorrowValue?_globalBorrowThirdLocal
    (state : RuntimeState) (loan : Nat) (key : GlobalKey)
    (address : String) (amount : Int)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    updateBorrowValue?
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan current)]
          activeLoans
          loanLocations := loanLocations.push
            (loan, { root := .global key }) }
        state loan replacement =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some (.borrow loan replacement)]
           activeLoans
           loanLocations := loanLocations.push
             (loan, { root := .global key }) },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, rewriteFirst]

/-- Mutate through a field-focused reborrow of a global resource.  The
focused reference rests in the frame slot ahead of its holder, whose
current value carries the focus hole; the resting slot updates directly and
the hole is left for reconciliation. -/
theorem updateBorrowValue?_focusedBorrowPair
    (state : RuntimeState) (loan outer : Nat) (key : GlobalKey)
    (address : String) (amount : Int)
    (outerSource innerSource : StructHandle)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan current),
            some (.borrow outer
              (.nominal outerSource none
                #[.nominal innerSource none #[.loanHole loan]]))]
          activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, (⟨.local (⟨3⟩ : LocalId),
              #[.deref, .field 0, .field 0], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some (.borrow loan replacement),
              some (.borrow outer
                (.nominal outerSource none
                  #[.nominal innerSource none #[.loanHole loan]]))]
           activeLoans
           loanLocations := #[(outer, { root := .global key }),
             (loan, (⟨.local (⟨3⟩ : LocalId),
               #[.deref, .field 0, .field 0], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?]

/-- Focused reborrow of a global resource whose read was saved in a local
before the mutation.  The saved value sits between the focused reference and
its holder, so the holder is one slot further out. -/
theorem updateBorrowValue?_focusedBorrowPairSaved
    (state : RuntimeState) (loan outer : Nat) (key : GlobalKey)
    (address : String) (amount saved : Int)
    (outerSource innerSource : StructHandle)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan current), some (.integer saved),
            some (.borrow outer
              (.nominal outerSource none
                #[.nominal innerSource none #[.loanHole loan]]))]
          activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, (⟨.local (⟨4⟩ : LocalId),
              #[.deref, .field 0, .field 0], true⟩ : RuntimePlace))] }
        state loan replacement =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some (.borrow loan replacement), some (.integer saved),
              some (.borrow outer
                (.nominal outerSource none
                  #[.nominal innerSource none #[.loanHole loan]]))]
           activeLoans
           loanLocations := #[(outer, { root := .global key }),
             (loan, (⟨.local (⟨4⟩ : LocalId),
               #[.deref, .field 0, .field 0], true⟩ : RuntimePlace))] },
         state) := by
  simp [updateBorrowValue?, updateLocalBorrowValue?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rewriteFirst,
    writeRuntimePlace?, writeRoot?]

/-- Literal-registry form of `updateBorrowValue?_globalBorrowThirdLocal`:
push normalization turns the freshly registered global location into a
singleton row before the mutation is reconciled. -/
theorem updateBorrowValue?_globalBorrowSingletonLocation
    (state : RuntimeState) (loan : Nat) (key : GlobalKey)
    (address : String) (amount : Int)
    (current replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    updateBorrowValue?
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow loan current)]
          activeLoans
          loanLocations := #[(loan, { root := .global key })] }
        state loan replacement =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some (.borrow loan replacement)]
           activeLoans
           loanLocations := #[(loan, { root := .global key })] },
         state) := by
  simpa using updateBorrowValue?_globalBorrowThirdLocal state loan key
    address amount current replacement activeLoans #[]

/-- Updating the only slot of a one-local frame is a constructor-level
operation, not a residual bounds computation. -/
theorem array_singleton_setIfInBounds_zero {α : Type} (old updated : α) :
    #[old].setIfInBounds 0 updated = #[updated] := by
  rfl

/-- Closed updates for a two-local frame. -/
theorem array_pair_set_zero {α : Type} (first second updated : α) :
    #[first, second].set! 0 updated = #[updated, second] := by
  rfl

theorem array_pair_set_one {α : Type} (first second updated : α) :
    #[first, second].set! 1 updated = #[first, updated] := by
  rfl

theorem array_triple_set_zero {α : Type} (first second third updated : α) :
    #[first, second, third].set! 0 updated = #[updated, second, third] := by
  rfl

theorem array_triple_set_two {α : Type} (first second third updated : α) :
    #[first, second, third].set! 2 updated = #[first, second, updated] := by
  rfl

/-- Constructor equations for the small native index rows emitted by
lowering. -/
theorem array_filter_empty {α : Type} (accepts : α → Bool) :
    (#[] : Array α).filter accepts = #[] := by
  rfl

/-- Canonical spelling of a literal array: the `#[..]` form every closed
frame lemma is stated over. -/
theorem array_mk_eq_toArray {α : Type} (entries : List α) :
    Array.mk entries = entries.toArray := by
  rfl

/-- Canonical spelling of a checked update. -/
theorem array_set!_eq_setIfInBounds {α : Type} (values : Array α)
    (index : Nat) (updated : α) :
    values.set! index updated = values.setIfInBounds index updated := by
  rfl

theorem array_pair_setIfInBounds_zero {α : Type} (first second updated : α) :
    #[first, second].setIfInBounds 0 updated = #[updated, second] := by
  rfl

theorem array_pair_setIfInBounds_one {α : Type} (first second updated : α) :
    #[first, second].setIfInBounds 1 updated = #[first, updated] := by
  rfl

theorem array_triple_setIfInBounds_zero {α : Type}
    (first second third updated : α) :
    #[first, second, third].setIfInBounds 0 updated =
      #[updated, second, third] := by
  rfl

theorem array_triple_setIfInBounds_one {α : Type}
    (first second third updated : α) :
    #[first, second, third].setIfInBounds 1 updated =
      #[first, updated, third] := by
  rfl

theorem array_triple_setIfInBounds_two {α : Type}
    (first second third updated : α) :
    #[first, second, third].setIfInBounds 2 updated =
      #[first, second, updated] := by
  rfl

theorem array_four_setIfInBounds_zero {α : Type}
    (first second third fourth updated : α) :
    #[first, second, third, fourth].setIfInBounds 0 updated =
      #[updated, second, third, fourth] := by
  rfl

theorem array_four_setIfInBounds_one {α : Type}
    (first second third fourth updated : α) :
    #[first, second, third, fourth].setIfInBounds 1 updated =
      #[first, updated, third, fourth] := by
  rfl

/-- Closed updates for the five-local storage frame used by field-focused
global borrows. -/
theorem array_five_setIfInBounds_two {α : Type}
    (first second third fourth fifth updated : α) :
    #[first, second, third, fourth, fifth].setIfInBounds 2 updated =
      #[first, second, updated, fourth, fifth] := by
  rfl

theorem array_five_setIfInBounds_three {α : Type}
    (first second third fourth fifth updated : α) :
    #[first, second, third, fourth, fifth].setIfInBounds 3 updated =
      #[first, second, third, updated, fifth] := by
  rfl

theorem array_five_setIfInBounds_four {α : Type}
    (first second third fourth fifth updated : α) :
    #[first, second, third, fourth, fifth].setIfInBounds 4 updated =
      #[first, second, third, fourth, updated] := by
  rfl

/-- Indexing the singleton field array produced by nominal destruction is a
constructor reduction, not a runtime lookup. -/
theorem array_singleton_getElem_zero {α : Type} (value : α) :
    #[value][0] = value := by
  rfl

theorem array_singleton_push {α : Type} (first second : α) :
    #[first].push second = #[first, second] := by
  rfl

theorem array_pair_push {α : Type} (first second third : α) :
    #[first, second].push third = #[first, second, third] := by
  rfl

theorem array_triple_push {α : Type} (first second third fourth : α) :
    #[first, second, third].push fourth = #[first, second, third, fourth] := by
  rfl

theorem array_empty_push {α : Type} (entry : α) :
    (#[] : Array α).push entry = #[entry] := by
  rfl

/-- Retiring the sole dynamic loan leaves no lexical-site entry.  This
constructor equation keeps `Array.filter` itself out of generated proofs. -/
theorem activeLoans_singleton_retire (site : ExprId) (loan : Nat) :
    (#[((site, loan))] : Array (ExprId × Nat)).filter
        (fun row => row.2 != loan) = #[] := by
  simp [Array.filter]

/-- Borrowing lexical site one after site zero preserves the first native
site row and appends the second. -/
theorem activeLoans_zero_then_one (firstLoan secondLoan : Nat) :
    ((#[(((⟨0⟩ : ExprId), firstLoan))] : Array (ExprId × Nat)).filter
          (fun row => row.1 != (⟨1⟩ : ExprId))).push
        ((⟨1⟩ : ExprId), secondLoan) =
      #[((⟨0⟩ : ExprId), firstLoan), ((⟨1⟩ : ExprId), secondLoan)] := by
  have distinct : ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true := by
    decide
  simp [Array.filter, distinct]

/-- Reducer-normal form of `activeLoans_zero_then_one`.  Head reduction may
expose `Array.filter` and `Array.push` as a list filter and append before the
reconciliation simp pass; this equation restores the same closed native row. -/
theorem activeLoans_zero_then_one_list (firstLoan secondLoan : Nat) :
    (List.filter
          (fun row : ExprId × Nat => row.1 != (⟨1⟩ : ExprId))
          [((⟨0⟩ : ExprId), firstLoan)] ++
        [((⟨1⟩ : ExprId), secondLoan)]).toArray =
      #[((⟨0⟩ : ExprId), firstLoan), ((⟨1⟩ : ExprId), secondLoan)] := by
  have distinct : ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true := by
    decide
  simp [distinct]

/-- Reconcile two fresh dereference loans into the two mutable parameters
that lent them.  Every location is a native row emitted by lowering; the
five separation facts are precisely the freshness invariants needed to
retire the fresh rows while preserving the two enclosing parameter rows. -/
theorem applyPendingFrom_twoDerefLocals
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan leftOuter rightOuter firstLoan secondLoan : Nat)
    (leftReplacement rightReplacement : RuntimeValue)
    (noLeftTransfer : transferredLoan? leftReplacement = none)
    (noRightTransfer : transferredLoan? rightReplacement = none)
    (freshSeparate : firstLoan ≠ secondLoan)
    (leftFirst : leftOuter ≠ firstLoan)
    (leftSecond : leftOuter ≠ secondLoan)
    (rightFirst : rightOuter ≠ firstLoan)
    (rightSecond : rightOuter ≠ secondLoan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow leftOuter (.loanHole firstLoan)),
            some (.borrow rightOuter (.loanHole secondLoan))]
          activeLoans := #[(⟨0⟩, firstLoan), (⟨1⟩, secondLoan)]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (firstLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (secondLoan,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (firstLoan, leftReplacement)
            |>.push (secondLoan, rightReplacement) } =
      ({ locals := #[some (.borrow leftOuter leftReplacement),
            some (.borrow rightOuter rightReplacement)]
         activeLoans := #[]
         loanLocations :=
           #[(leftOuter,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (rightOuter,
               (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  have secondFirst : secondLoan ≠ firstLoan := Ne.symm freshSeparate
  rw [applyPendingFrom_two_push]
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?,
    transferActiveLoan, transferLoanLocation, noLeftTransfer, noRightTransfer,
    Array.filter, secondFirst, leftFirst, leftSecond, rightFirst, rightSecond]

/-- Reconcile a packed pair of returned reborrows into two mutable caller
parameters.  The two returned loan ids replace the two call-argument ids in
the caller's native location cache; no owner root or projection path is
exported by the callee. -/
theorem applyPendingFrom_twoReturnedReborrows_derefLocals
    (initial : RuntimeState) (runtimeNextLoan leftOuter rightOuter : Nat)
    (freshGlobal : FreshGlobalLoanIds initial)
    (leftPrior : leftOuter < initial.nextLoan)
    (rightPrior : rightOuter < initial.nextLoan) :
    applyPendingFrom initial.pending
        { locals := #[some (.borrow leftOuter (.loanHole initial.nextLoan)),
            some (.borrow rightOuter (.loanHole (initial.nextLoan + 1))),
            none, none]
          activeLoans := #[(⟨0⟩, initial.nextLoan),
            (⟨1⟩, initial.nextLoan + 1)]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (initial.nextLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (initial.nextLoan + 1,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        (exportFrameLoans
          { locals := #[some (.borrow initial.nextLoan
                (.loanHole (initial.nextLoan + 1 + 1))),
              some (.borrow (initial.nextLoan + 1)
                (.loanHole (initial.nextLoan + 1 + 1 + 1)))]
            activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1),
              (⟨1⟩, initial.nextLoan + 1 + 1 + 1)]
            loanLocations :=
              #[(initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1,
                  (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1 + 1,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1 + 1 + 1,
                  (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }) =
      ({ locals := #[some (.borrow leftOuter
              (.loanHole (initial.nextLoan + 1 + 1))),
            some (.borrow rightOuter
              (.loanHole (initial.nextLoan + 1 + 1 + 1))),
            none, none]
         activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1),
           (⟨1⟩, initial.nextLoan + 1 + 1 + 1)]
         loanLocations :=
           #[(leftOuter,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (rightOuter,
               (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1 + 1,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1 + 1 + 1,
               (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
       { globals := initial.globals
         globalLoans := initial.globalLoans
         nextLoan := runtimeNextLoan
         pending := initial.pending }) := by
  rw [exportFrameLoans_twoReturnedReborrows_state initial runtimeNextLoan _ _
    freshGlobal]
  rw [applyPendingFrom_two_push]
  have leftFirst : leftOuter ≠ initial.nextLoan := by omega
  have leftSecond : leftOuter ≠ initial.nextLoan + 1 := by omega
  have rightFirst : rightOuter ≠ initial.nextLoan := by omega
  have rightSecond : rightOuter ≠ initial.nextLoan + 1 := by omega
  have firstSecond : initial.nextLoan ≠ initial.nextLoan + 1 := by omega
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?,
    transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, leftFirst, leftSecond, rightFirst, rightSecond, firstSecond]

/-- Reconcile a reborrow rooted at local zero while preserving one trailing
local.  The cached place still selects the first slot directly; no scan of
either local is involved. -/
theorem applyPendingFrom_derefLocalZero_pair
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat)
    (replacement : RuntimeValue) (saved : Option RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow outerLoan (.loanHole loan)), saved]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement), saved]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  rw [applyPendingFrom_push]
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?, separate]

/-- Closed cross-call certificate for returning a scalar reborrow.  The
callee exports the returned loan hole, and the caller uses that hole to
retarget the borrow-analysis death marker from the argument loan to the
returned loan.  The only places below are optional native indexes already
present in the two frames; no ownership path crosses the call boundary. -/
theorem applyPendingFrom_returnedReborrow_derefLocalZero_pair
    (initial : RuntimeState) (runtimeNextLoan parameterLoan : Nat)
    (saved : Option RuntimeValue)
    (freshGlobal : FreshGlobalLoanIds initial)
    (parameterPrior : parameterLoan < initial.nextLoan) :
    applyPendingFrom initial.pending
        { locals := #[some (.borrow parameterLoan
              (.loanHole initial.nextLoan)), saved]
          activeLoans := #[(⟨0⟩, initial.nextLoan)]
          loanLocations :=
            #[(parameterLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (initial.nextLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        (exportFrameLoans
          { locals := #[some (.borrow initial.nextLoan
                (.loanHole (initial.nextLoan + 1)))]
            activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
            loanLocations :=
              #[(initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }) =
      ({ locals := #[some (.borrow parameterLoan
            (.loanHole (initial.nextLoan + 1))), saved]
         activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
         loanLocations :=
           #[(parameterLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
       { globals := initial.globals
         globalLoans := initial.globalLoans
         nextLoan := runtimeNextLoan
         pending := initial.pending }) := by
  have returnedSeparate : initial.nextLoan ≠ initial.nextLoan + 1 := by omega
  have parameterSeparate : parameterLoan ≠ initial.nextLoan :=
    Nat.ne_of_lt parameterPrior
  have noGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }
          initial.nextLoan = none :=
    FreshGlobalLoanIds.lookup_next freshGlobal
  rw [exportFrameLoans_returnedReborrow_state _ _ _ _ _
    returnedSeparate noGlobal]
  simpa [transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, parameterSeparate] using
    (applyPendingFrom_derefLocalZero_pair initial.pending initial.globals
      initial.globalLoans runtimeNextLoan parameterLoan initial.nextLoan
      (.loanHole (initial.nextLoan + 1)) saved
      #[(⟨0⟩, initial.nextLoan)] parameterSeparate)

/-- Closed cross-call certificate for a reborrow returned from a nominal
field.  Reconciliation follows the returned loan hole in the prophetic value
graph.  The cached dereference is only the caller's local analysis index; the
callee does not export an owning root or projection path. -/
theorem applyPendingFrom_returnedProjectedReborrow_derefLocalZero_pair
    (initial : RuntimeState) (runtimeNextLoan parameterLoan : Nat)
    (saved : Option RuntimeValue) (name : StructHandle) (right : Int)
    (freshGlobal : FreshGlobalLoanIds initial)
    (parameterPrior : parameterLoan < initial.nextLoan) :
    applyPendingFrom initial.pending
        { locals := #[some (.borrow parameterLoan
              (.loanHole initial.nextLoan)), saved]
          activeLoans := #[(⟨0⟩, initial.nextLoan)]
          loanLocations :=
            #[(parameterLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (initial.nextLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        (exportFrameLoans
          { locals := #[some (.borrow initial.nextLoan
                (.nominal name none
                  #[.loanHole (initial.nextLoan + 1), .integer right]))]
            activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
            loanLocations :=
              #[(initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref, .field 0], true⟩ :
                    RuntimePlace))] }
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }) =
      ({ locals := #[some (.borrow parameterLoan
            (.nominal name none
              #[.loanHole (initial.nextLoan + 1), .integer right])), saved]
         activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
         loanLocations :=
           #[(parameterLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
       { globals := initial.globals
         globalLoans := initial.globalLoans
         nextLoan := runtimeNextLoan
         pending := initial.pending }) := by
  have returnedSeparate : initial.nextLoan ≠ initial.nextLoan + 1 := by omega
  have parameterSeparate : parameterLoan ≠ initial.nextLoan :=
    Nat.ne_of_lt parameterPrior
  have noGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }
          initial.nextLoan = none :=
    FreshGlobalLoanIds.lookup_next freshGlobal
  rw [exportFrameLoans_returnedProjectedReborrow_state _ _ _ _ _ _ _
    returnedSeparate noGlobal]
  simpa [transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    findFirstList, Array.filter, parameterSeparate] using
    (applyPendingFrom_derefLocalZero_pair initial.pending initial.globals
      initial.globalLoans runtimeNextLoan parameterLoan initial.nextLoan
      (.nominal name none
        #[.loanHole (initial.nextLoan + 1), .integer right]) saved
      #[(⟨0⟩, initial.nextLoan)] parameterSeparate)

/-- Singleton-frame form of the returned-reborrow call certificate.  A
forwarding function has no saved result local after its return value moves
out, so its caller reconciliation acts on just the mutable parameter slot. -/
theorem applyPendingFrom_returnedReborrow_derefLocalZero
    (initial : RuntimeState) (runtimeNextLoan parameterLoan : Nat)
    (freshGlobal : FreshGlobalLoanIds initial)
    (parameterPrior : parameterLoan < initial.nextLoan) :
    applyPendingFrom initial.pending
        { locals := #[some (.borrow parameterLoan
              (.loanHole initial.nextLoan))]
          activeLoans := #[(⟨0⟩, initial.nextLoan)]
          loanLocations :=
            #[(parameterLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (initial.nextLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        (exportFrameLoans
          { locals := #[some (.borrow initial.nextLoan
                (.loanHole (initial.nextLoan + 1)))]
            activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
            loanLocations :=
              #[(initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan + 1,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }) =
      ({ locals := #[some (.borrow parameterLoan
            (.loanHole (initial.nextLoan + 1)))]
         activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
         loanLocations :=
           #[(parameterLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
       { globals := initial.globals
         globalLoans := initial.globalLoans
         nextLoan := runtimeNextLoan
         pending := initial.pending }) := by
  have returnedSeparate : initial.nextLoan ≠ initial.nextLoan + 1 := by omega
  have parameterSeparate : parameterLoan ≠ initial.nextLoan :=
    Nat.ne_of_lt parameterPrior
  have noGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }
          initial.nextLoan = none :=
    FreshGlobalLoanIds.lookup_next freshGlobal
  rw [exportFrameLoans_returnedReborrow_state _ _ _ _ _
    returnedSeparate noGlobal]
  simpa [transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, parameterSeparate] using
    (applyPendingFrom_derefLocalZero initial.pending initial.globals
      initial.globalLoans runtimeNextLoan parameterLoan initial.nextLoan
      (.loanHole (initial.nextLoan + 1)) #[(⟨0⟩, initial.nextLoan)]
      parameterSeparate)

/-- Finalize a scalar forwarding function in one constructor-shaped step.
The inner call transfers its returned dynamic loan to the forwarding frame;
that frame then exports the hole through its own parameter loan.  The loan
identity comes entirely from the hole value, while `loanLocations` remains a
validated local cache that never crosses the boundary. -/
theorem exportFrameLoans_applyPendingFrom_returnedReborrow_derefLocalZero
    (initial : RuntimeState) (runtimeNextLoan parameterLoan : Nat)
    (freshGlobal : FreshGlobalLoanIds initial)
    (parameterPrior : parameterLoan < initial.nextLoan)
    (parameterNoGlobal :
      globalLoanKeyIn? initial.globalLoans parameterLoan = none) :
    exportFrameLoans
        (applyPendingFrom initial.pending
          { locals := #[some (.borrow parameterLoan
                (.loanHole initial.nextLoan))]
            activeLoans := #[(⟨0⟩, initial.nextLoan)]
            loanLocations :=
              #[(parameterLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
          (exportFrameLoans
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            { globals := initial.globals
              globalLoans := initial.globalLoans
              nextLoan := runtimeNextLoan
              pending := initial.pending })).fst
        (applyPendingFrom initial.pending
          { locals := #[some (.borrow parameterLoan
                (.loanHole initial.nextLoan))]
            activeLoans := #[(⟨0⟩, initial.nextLoan)]
            loanLocations :=
              #[(parameterLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                (initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
          (exportFrameLoans
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            { globals := initial.globals
              globalLoans := initial.globalLoans
              nextLoan := runtimeNextLoan
              pending := initial.pending })).snd =
      { globals := initial.globals
        globalLoans := initial.globalLoans
        nextLoan := runtimeNextLoan
        pending := initial.pending.push
          (parameterLoan, .loanHole (initial.nextLoan + 1)) } := by
  rw [applyPendingFrom_returnedReborrow_derefLocalZero initial runtimeNextLoan
    parameterLoan freshGlobal parameterPrior]
  have parameterSeparate : parameterLoan ≠ initial.nextLoan + 1 := by omega
  have noGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := runtimeNextLoan
            pending := initial.pending }
          parameterLoan = none := parameterNoGlobal
  exact exportFrameLoans_returnedReborrow_state _ parameterLoan
    (initial.nextLoan + 1) _ _ parameterSeparate noGlobal

/-- Reconcile a reference forwarded through two scalar call boundaries into
the original caller frame.  This is the native constructor equation needed
before that caller can bind and mutate the returned borrow.  Each boundary
derives the next loan from the preceding `loanHole`; no owner path is part of
the reference or of this certificate. -/
theorem applyPendingFrom_forwardedReborrow_derefLocalZero_pair
    (initial : RuntimeState) (runtimeNextLoan parameterLoan : Nat)
    (saved : Option RuntimeValue)
    (freshGlobal : FreshGlobalLoanIds initial)
    (parameterPrior : parameterLoan < initial.nextLoan) :
    applyPendingFrom initial.pending
        { locals := #[some (.borrow parameterLoan
              (.loanHole initial.nextLoan)), saved]
          activeLoans := #[(⟨0⟩, initial.nextLoan)]
          loanLocations :=
            #[(parameterLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (initial.nextLoan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        (exportFrameLoans
          (applyPendingFrom initial.pending
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            (exportFrameLoans
              { locals := #[some (.borrow (initial.nextLoan + 1)
                    (.loanHole (initial.nextLoan + 1 + 1)))]
                activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1)]
                loanLocations :=
                  #[(initial.nextLoan + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                    (initial.nextLoan + 1 + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
              { globals := initial.globals
                globalLoans := initial.globalLoans
                nextLoan := runtimeNextLoan
                pending := initial.pending })).fst
          (applyPendingFrom initial.pending
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            (exportFrameLoans
              { locals := #[some (.borrow (initial.nextLoan + 1)
                    (.loanHole (initial.nextLoan + 1 + 1)))]
                activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1)]
                loanLocations :=
                  #[(initial.nextLoan + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                    (initial.nextLoan + 1 + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
              { globals := initial.globals
                globalLoans := initial.globalLoans
                nextLoan := runtimeNextLoan
                pending := initial.pending })).snd) =
      ({ locals := #[some (.borrow parameterLoan
            (.loanHole (initial.nextLoan + 1 + 1))), saved]
         activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1)]
         loanLocations :=
           #[(parameterLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (initial.nextLoan + 1 + 1,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] },
       { globals := initial.globals
         globalLoans := initial.globalLoans
         nextLoan := runtimeNextLoan
         pending := initial.pending }) := by
  let shifted : RuntimeState :=
    { globals := initial.globals
      globalLoans := initial.globalLoans
      nextLoan := initial.nextLoan + 1
      pending := initial.pending }
  have shiftedFresh : FreshGlobalLoanIds shifted := by
    intro candidate lower
    apply freshGlobal candidate
    change initial.nextLoan + 1 ≤ candidate at lower
    omega
  have forwardExport :
      exportFrameLoans
          (applyPendingFrom initial.pending
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            (exportFrameLoans
              { locals := #[some (.borrow (initial.nextLoan + 1)
                    (.loanHole (initial.nextLoan + 1 + 1)))]
                activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1)]
                loanLocations :=
                  #[(initial.nextLoan + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                    (initial.nextLoan + 1 + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
              { globals := initial.globals
                globalLoans := initial.globalLoans
                nextLoan := runtimeNextLoan
                pending := initial.pending })).fst
          (applyPendingFrom initial.pending
            { locals := #[some (.borrow initial.nextLoan
                  (.loanHole (initial.nextLoan + 1)))]
              activeLoans := #[(⟨0⟩, initial.nextLoan + 1)]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
            (exportFrameLoans
              { locals := #[some (.borrow (initial.nextLoan + 1)
                    (.loanHole (initial.nextLoan + 1 + 1)))]
                activeLoans := #[(⟨0⟩, initial.nextLoan + 1 + 1)]
                loanLocations :=
                  #[(initial.nextLoan + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                    (initial.nextLoan + 1 + 1,
                      (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
              { globals := initial.globals
                globalLoans := initial.globalLoans
                nextLoan := runtimeNextLoan
                pending := initial.pending })).snd =
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := runtimeNextLoan
          pending := initial.pending.push
            (initial.nextLoan, .loanHole (initial.nextLoan + 1 + 1)) } := by
    have forwardedPrior : initial.nextLoan < shifted.nextLoan := by
      simp [shifted]
    have forwardedNoGlobal :
        globalLoanKeyIn? shifted.globalLoans initial.nextLoan = none := by
      exact FreshGlobalLoanIds.lookup_next freshGlobal
    simpa [shifted] using
      (exportFrameLoans_applyPendingFrom_returnedReborrow_derefLocalZero
        shifted runtimeNextLoan initial.nextLoan shiftedFresh forwardedPrior
        forwardedNoGlobal)
  rw [forwardExport]
  have parameterSeparate : parameterLoan ≠ initial.nextLoan :=
    Nat.ne_of_lt parameterPrior
  simpa [transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, parameterSeparate] using
    (applyPendingFrom_derefLocalZero_pair initial.pending initial.globals
      initial.globalLoans runtimeNextLoan parameterLoan initial.nextLoan
      (.loanHole (initial.nextLoan + 1 + 1)) saved
      #[(⟨0⟩, initial.nextLoan)] parameterSeparate)

/-- Fresh-loan form of `applyPendingFrom_derefLocalZero_pair`.  Native
verification carries freshness as the stronger ordered fact
`outerLoan < loan`; exposing that fact directly lets equation reduction use
the cached-location rewrite by assumption, without synthesizing a separate
disequality proof inside a large expression. -/
theorem applyPendingFrom_derefLocalZero_pair_of_lt
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat)
    (replacement : RuntimeValue) (saved : Option RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (fresh : outerLoan < loan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow outerLoan (.loanHole loan)), saved]
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement), saved]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  exact applyPendingFrom_derefLocalZero_pair inherited globals globalLoans
    runtimeNextLoan outerLoan loan replacement saved activeLoans
      (Nat.ne_of_lt fresh)

/-- Reconcile the exact native row exposed when a mutable parameter is
followed by a saved scalar local.  `nativeInitialFrame?` may have reduced
`parameterLoanLocations` through `zipIdx` before the call boundary is
reached; this theorem keeps that residual array computation out of VCs. -/
theorem applyPendingFrom_derefLocalZero_pair_parameterRow
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat)
    (current replacement : RuntimeValue) (saved : Option RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow outerLoan (.loanHole loan)), saved]
          activeLoans
          loanLocations :=
            (Array.filterMap
                (fun row : RuntimeValue × Nat =>
                  match row.1 with
                  | .borrow instance_ _ =>
                      some (instance_, { root := .local ⟨row.2⟩ })
                  | _ => none)
                #[((.borrow outerLoan current), 0)]).push
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)) }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement), saved]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  rw [parameterLoanLocations_filterMap_singleBorrow]
  exact applyPendingFrom_derefLocalZero_pair inherited globals globalLoans
    runtimeNextLoan outerLoan loan replacement saved activeLoans separate

/-- Reconcile the constructor form emitted immediately after reborrowing
the only local.  This is the lowering-facing companion to
`applyPendingFrom_derefLocalZero`: it absorbs the concrete one-slot update
and cache push before either can become residual proof computation. -/
theorem applyPendingFrom_derefLocalZero_afterBorrow
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan outerLoan loan : Nat)
    (previous replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (separate : outerLoan ≠ loan) :
    applyPendingFrom inherited
        { locals :=
            #[some (.borrow outerLoan previous)].setIfInBounds 0
              (some (.borrow outerLoan (.loanHole loan)))
          activeLoans
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))].push
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)) }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (loan, replacement) } =
      ({ locals := #[some (.borrow outerLoan replacement)]
         activeLoans := transferActiveLoan activeLoans loan replacement
         loanLocations := transferLoanLocation
           #[(outerLoan,
               (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
             (loan,
               (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
           loan replacement },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  rw [array_singleton_setIfInBounds_zero]
  exact applyPendingFrom_derefLocalZero inherited globals globalLoans
    runtimeNextLoan outerLoan loan replacement activeLoans separate

/-- Materialize the local row of a fresh function frame. Parameters occupy
the leading slots and every remaining declared local starts uninitialized.

Keep the established `Array.ofFn` representation here: call reconciliation
proofs rely on this normal form. Native verification specializes the few new
closed frame shapes with constructor equations below, so no dependent index
proof survives in those generated verification conditions. -/
def initialLocals (localCount : Nat) (arguments : Array RuntimeValue) :
    Array (Option RuntimeValue) :=
  Array.ofFn (n := localCount) fun index =>
    if h : index < arguments.size then some arguments[index] else none

/-- Create a fresh function frame and initialize the leading parameter locals
in declaration order. Arity or a missing leading local makes invocation stuck. -/
def initialFrame? (declaration : FunctionDecl FunctionBody)
    (arguments : Array RuntimeValue)
    (typeInstantiation : Array (TypeId × TypeId) := #[]) : Option RuntimeFrame :=
  if arguments.size != declaration.signature.parameters.size then none else
  if declaration.locals.size < arguments.size then none else
    some {
      locals := initialLocals declaration.locals.size arguments
      loanLocations := parameterLoanLocations arguments
      typeInstantiation }

/-- The closed runtime-relevant projection of a function declaration.
Lowering computes this once; native execution and verification do not retain
the signature, local declarations, attributes, or source metadata. -/
structure FunctionShape where
  parameterCount : Nat
  localCount : Nat
  resultCount : Nat
  profile : Profile
  deriving Repr, BEq, DecidableEq, Inhabited

def FunctionShape.ofDeclaration
    (declaration : FunctionDecl FunctionBody) : FunctionShape :=
  { parameterCount := declaration.signature.parameters.size
    localCount := declaration.locals.size
    resultCount := declaration.signature.results.size
    profile := declaration.profile }

/-- Create a frame from a lowered function shape.  All local locations are
materialized by `localCount`, and arguments occupy the leading slots. -/
def nativeInitialFrame? (shape : FunctionShape)
    (arguments : Array RuntimeValue)
    (typeInstantiation : Array (TypeId × TypeId) := #[]) : Option RuntimeFrame :=
  if arguments.size != shape.parameterCount then none else
  if shape.localCount < arguments.size then none else
    some {
      locals := initialLocals shape.localCount arguments
      loanLocations := parameterLoanLocations arguments
      typeInstantiation }

/-- Closed initial frame for a one-parameter mutable function.  Lowered
verification uses this constructor equation instead of re-running arity,
local placement, and parameter-location extraction. -/
theorem nativeInitialFrame?_singleBorrow_oneLocal
    (resultCount : Nat) (profile : Profile) (loan : Nat)
    (current : RuntimeValue) :
    nativeInitialFrame?
        { parameterCount := 1, localCount := 1, resultCount, profile }
        #[.borrow loan current] =
      some
        { locals := #[some (.borrow loan current)]
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] } := by
  simp [nativeInitialFrame?, parameterLoanLocations_singleBorrow,
    initialLocals, Array.ofFn_succ]

/-- Closed initial frame for one mutable parameter plus one lowered local. -/
theorem nativeInitialFrame?_singleBorrow_twoLocals
    (resultCount : Nat) (profile : Profile) (loan : Nat)
    (current : RuntimeValue) :
    nativeInitialFrame?
        { parameterCount := 1, localCount := 2, resultCount, profile }
        #[.borrow loan current] =
      some
        { locals := #[some (.borrow loan current), none]
          loanLocations :=
            #[(loan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] } := by
  simp [nativeInitialFrame?, parameterLoanLocations_singleBorrow,
    initialLocals, Array.ofFn_succ]

/-- Closed initial frame for two mutable parameters. -/
theorem nativeInitialFrame?_twoBorrows_twoLocals
    (resultCount : Nat) (profile : Profile)
    (leftLoan rightLoan : Nat) (leftCurrent rightCurrent : RuntimeValue) :
    nativeInitialFrame?
        { parameterCount := 2, localCount := 2, resultCount, profile }
        #[.borrow leftLoan leftCurrent, .borrow rightLoan rightCurrent] =
      some
        { locals := #[some (.borrow leftLoan leftCurrent),
            some (.borrow rightLoan rightCurrent)]
          loanLocations :=
            #[(leftLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightLoan,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] } := by
  simp [nativeInitialFrame?, parameterLoanLocations_twoBorrows,
    initialLocals, Array.ofFn_succ]

/-- Closed initial frame for two parameters plus two lowered locals. This is
the native account-deposit frame shape. -/
theorem nativeInitialFrame?_twoArguments_fourLocals
    (resultCount : Nat) (profile : Profile)
    (first second : RuntimeValue) :
    nativeInitialFrame?
        { parameterCount := 2, localCount := 4, resultCount, profile }
        #[first, second] =
      some
        { locals := #[some first, some second, none, none]
          loanLocations := parameterLoanLocations #[first, second] } := by
  simp [nativeInitialFrame?, initialLocals, Array.ofFn_succ]

/-- Closed initial frame for two parameters plus three lowered locals. This
is the native account-withdraw frame shape. -/
theorem nativeInitialFrame?_twoArguments_fiveLocals
    (resultCount : Nat) (profile : Profile)
    (first second : RuntimeValue) :
    nativeInitialFrame?
        { parameterCount := 2, localCount := 5, resultCount, profile }
        #[first, second] =
      some
        { locals := #[some first, some second, none, none, none]
          loanLocations := parameterLoanLocations #[first, second] } := by
  simp [nativeInitialFrame?, initialLocals, Array.ofFn_succ]

theorem nativeInitialFrame?_ofDeclaration
    (declaration : FunctionDecl FunctionBody)
    (arguments : Array RuntimeValue) :
    nativeInitialFrame? (.ofDeclaration declaration) arguments =
      initialFrame? declaration arguments := by
  rfl

/-- Finalize function-local control according to the declared result arity. -/
def finishControl? (resultCount : Nat) : Control → Option Outcome
  | .value value => .returned <$> unpackFallthrough resultCount value
  | .return_ values =>
      if values.size == resultCount then some (.returned values) else none
  | .throw_ kind arguments => some (.threw kind arguments)
  | .break_ .. | .continue_ .. => none

/-- Export the dying frame's loans and apply the selected source profile's
transaction boundary to a completed function. Move-style aborts can hide all
mutations made by the invocation; Rust-style panics and ordinary returns
expose the evaluated nonlocal state. Missing semantics cannot occur for an
`ExecutableUnit` and conservatively preserve that state. -/
def finalizeFunctionState (executable : ExecutableUnit) (profile : Profile)
    (initialState evaluatedState : RuntimeState) (frame : RuntimeFrame) : Outcome → RuntimeState
  | .returned results => exportReturnedFrameLoans results frame evaluatedState
  | .threw kind _ =>
      match semanticProfile? executable.semantics profile with
      | some semantics => if semantics.rollbackThrow kind then initialState
          else exportFrameLoans frame evaluatedState
      | none => exportFrameLoans frame evaluatedState

/-! Proof reduction must reach `applyPendingFrom` through the closed native
equations above.  In particular, simplifying the `.1` or `.2` projection of
an unresolved reconciliation must not delta-reduce its generic fold and
reintroduce scans through the caller frame.  The executable definition is
still available to the interpreter and code generator. -/

attribute [irreducible] applyPendingFrom

/- The write-back resolution is rewritten only by its closed rows: an
unfolding reached by a definitional step would bury a lender's resolution
under its fold before the rows could read it. -/
attribute [irreducible] resolveReturnedBorrows

end SemanticOperations
end LeanerIR

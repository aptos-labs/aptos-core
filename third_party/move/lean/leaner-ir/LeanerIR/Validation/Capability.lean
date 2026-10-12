-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Profiles
import LeanerIR.Validation.Unify
import LeanerIR.Validation.Borrowing
import LeanerIR.Validation.Initialization
import LeanerIR.Validation.PlaceIndex
import LeanerIR.Validation.Eliminate
import LeanerIR.Validation.IndexedArena
import LeanerIR.Validation.StaticTyping

/-!
# LIR semantic capability preparation

This module is the checked boundary between structurally validated LIR and
the execution and verification semantics.  Its exhaustive core inventories
make a new syntax constructor a compile error until it is classified.
Profile extensions provide an equally explicit, versioned inventory.
-/

namespace LeanerIR.Validation

open LeanerIR.Import

/-- The semantic role of a syntax feature. -/
inductive SemanticDomain where
  | executable
  | logicalOnly
  | frontendOnly
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Whether the current semantic implementation supports a classified
feature. Unsupported features retain their intended domain and a stable
explanation for diagnostics and coverage reports. -/
inductive SemanticSupport where
  | supported
  | unsupported (reason : String)
  deriving Repr, BEq, Inhabited

/-- Semantic classification of one core constructor or profile feature. -/
structure SemanticClassification where
  domain : SemanticDomain
  support : SemanticSupport
  deriving Repr, BEq, Inhabited

namespace SemanticClassification

def executable : SemanticClassification := { domain := .executable, support := .supported }
def logicalOnly : SemanticClassification := { domain := .logicalOnly, support := .supported }
def frontendOnly : SemanticClassification := { domain := .frontendOnly, support := .supported }

def unsupported (domain : SemanticDomain) (reason : String) : SemanticClassification :=
  { domain, support := .unsupported reason }

end SemanticClassification

/-- A stable feature name paired with its semantic classification. -/
structure SemanticFeature where
  name : String
  classification : SemanticClassification
  deriving Repr, BEq, Inhabited

/-- Result of a deterministic profile-owned operation whose inputs and output
are closed values. Stateful reference/resource handlers are added separately;
keeping this hook closed makes arithmetic and collection semantics available
without allowing a profile to inspect frontend syntax or interpreter state. -/
inductive PureProfileResult where
  | value (value : ConstValue)
  | throw_ (kind : ThrowKind) (arguments : Array ConstValue := #[])
  deriving Repr, BEq, Inhabited

/-- Every context in which a `ProfileValue` can affect semantic preparation. -/
inductive ProfileSemanticSite where
  | type
  | constant
  | operation
  | surface
  | property
  | borrow
  | throw_
  | call
  | quantifier
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Versioned semantic inventory supplied by one extension profile. Returning
`none` means that a tag was structurally accepted but has no semantic
classification, which is always a preparation error. -/
structure SemanticProfile where
  profile : Profile
  name : String
  version : Nat := 1
  classify : ProfileSemanticSite → ProfileValue → Option SemanticClassification
  evaluatePure : ProfileValue → Ty → Array ConstValue → Option PureProfileResult :=
    fun _ _ _ => none
  rollbackThrow : ThrowKind → Bool := fun _ => false

abbrev SemanticsRegistry := Array SemanticProfile

def semanticProfile? (registry : SemanticsRegistry) (profile : Profile) : Option SemanticProfile :=
  registry.find? (·.profile == profile)

/-- Resolve the compilation target's Rust pointer width. Validation of the
Rust profile restricts this option to the widths supported by rustc targets. -/
def targetPointerWidth? (unit : ValidatedUnit) : Option Nat := do
  let config ← profileConfig? unit.profiles .rust
  let option ← config.options.find? (·.1 == "target_pointer_width")
  supportedTargetPointerWidth? option.2

/-- Private-constructor wrapper accepted by the future LIR interpreter,
indexed by the validated unit it executes. -/
structure ExecutableUnit (unit : ValidatedUnit) where
  private mk ::
  semantics : SemanticsRegistry
  targetPointerWidth : Option Nat
  initializationCertificates : Array InitializationCertificate
  borrowCertificates : Array BorrowCertificate
  /-- The unit is statically typed: every node its bodies run satisfies its
  check (`StaticTyping.checkUnit`). -/
  typed : StaticTyping.checkUnit unit targetPointerWidth = true
  /-- The target width is the one the unit's profiles select. -/
  width_eq : targetPointerWidth = targetPointerWidth? unit

/-- Private-constructor wrapper accepted by the LIR verification layer. -/
structure VerifiableUnit where
  private mk ::
  unit : ValidatedUnit
  semantics : SemanticsRegistry
  targetPointerWidth : Option Nat
  initializationCertificates : Array InitializationCertificate
  borrowCertificates : Array BorrowCertificate

private def feature (name : String) (classification : SemanticClassification) : SemanticFeature :=
  { name, classification }

private def unsupportedExecutable (name reason : String) : SemanticFeature :=
  feature name (.unsupported .executable reason)

private def unsupportedLogical (name reason : String) : SemanticFeature :=
  feature name (.unsupported .logicalOnly reason)

/-! ## Exhaustive core semantic inventory -/

/-- Provenance affects reporting and alignment, never semantic behavior. -/
def coreOriginFeature : OriginKind → SemanticFeature
  | .moveSource => feature "origin.move" .frontendOnly
  | .leanerSource => feature "origin.leaner" .frontendOnly
  | .rustMir => feature "origin.rustMir" .frontendOnly
  | .generated _ => feature "origin.generated" .frontendOnly

def coreTrustFeature : Trust → SemanticFeature
  | .authored => feature "trust.authored" .frontendOnly
  | .checked => feature "trust.checked" .frontendOnly
  | .assumed => feature "trust.assumed" .frontendOnly

def coreIntWidthFeature : IntWidth → SemanticFeature
  | .bits _ => feature "integerWidth.bits" .executable
  | .pointer => feature "integerWidth.pointer" .executable
  | .unbounded => unsupportedLogical "integerWidth.unbounded"
      "mathematical integers require M4 logical semantics"

def coreReferenceKindFeature : ReferenceKind → SemanticFeature
  | .shared => feature "reference.shared" .executable
  | .mutable => feature "reference.mutable" .executable

def coreLifetimeKindFeature : LifetimeKind → SemanticFeature
  | .static => feature "lifetime.static" .executable
  | .parameter _ => feature "lifetime.parameter" .executable
  | .inference => feature "lifetime.inference" .executable
  | .local => feature "lifetime.local" .executable

/-- M0 classification of every core type constructor. -/
def coreTypeFeature : Ty → SemanticFeature
  | .unit => feature "type.unit" .executable
  | .never => feature "type.never" .executable
  | .bool => feature "type.bool" .executable
  | .character => feature "type.character" .executable
  | .string => feature "type.string" .executable
  | .bytes => feature "type.bytes" .executable
  | .address => feature "type.address" .executable
  | .signer => feature "type.signer" .executable
  | .integer (.bits _) _ => feature "type.integer.bits" .executable
  | .integer .pointer _ => feature "type.integer.pointer" .executable
  | .integer .unbounded _ =>
      unsupportedLogical "type.integer.unbounded" "mathematical integers require M4 logical semantics"
  | .tuple _ => feature "type.tuple" .executable
  | .vector _ none => feature "type.vector" .executable
  | .vector _ (some _) => feature "type.vector.fixed" .executable
  | .range => unsupportedLogical "type.range" "range types require M4 logical semantics"
  | .eventStore => unsupportedLogical "type.eventStore" "event specifications require M4 logical semantics"
  | .typeDomain _ => unsupportedLogical "type.typeDomain" "type domains require M4 logical semantics"
  | .resourceDomain _ _ => unsupportedLogical "type.resourceDomain" "resource domains require M4 logical semantics"
  | .stateDomain => unsupportedLogical "type.stateDomain" "state domains require M4 logical semantics"
  | .nominal _ _ => feature "type.nominal" .executable
  | .function _ _ _ => feature "type.function" .executable
  | .typeParameter _ => feature "type.parameter" .executable
  | .reference _ => feature "type.reference" .executable
  | .profile _ =>
      unsupportedExecutable "type.profile" "profile type requires a registered semantic classification"

def coreGenericArgumentFeature : GenericArgument → SemanticFeature
  | .typeArg _ => feature "generic.type" .executable
  | .const _ => feature "generic.const" .executable
  | .lifetime _ => feature "generic.lifetime" .executable
  | .evidence _ => feature "generic.evidence" .executable

def coreAbilityFeature : Ability → SemanticFeature
  | .copy => feature "ability.copy" .executable
  | .drop => feature "ability.drop" .executable
  | .store => feature "ability.store" .executable
  | .key => feature "ability.key" .executable

def coreGenericPredicateFeature : GenericPredicate → SemanticFeature
  | .ability _ _ => feature "predicate.ability" .executable
  | .implements _ _ => feature "predicate.implements" .executable
  | .associatedTypeEq .. => feature "predicate.associatedTypeEq" .executable
  | .associatedConstEq .. => feature "predicate.associatedConstEq" .executable
  | .lifetimeOutlives .. => feature "predicate.lifetimeOutlives" .executable
  | .constEq .. => feature "predicate.constEq" .executable
  | .profile _ => unsupportedExecutable "predicate.profile"
      "profile predicate requires a registered semantic classification"

def coreAttributeValueFeature : AttributeValue → SemanticFeature
  | .constant _ => feature "attribute.constant" .frontendOnly
  | .name _ _ => feature "attribute.name" .frontendOnly
  | .qualifiedName _ => feature "attribute.qualifiedName" .frontendOnly

def coreAttributeFeature : Attribute → SemanticFeature
  | .call _ _ _ => feature "attribute.call" .frontendOnly
  | .assign _ _ _ => feature "attribute.assign" .frontendOnly

/-- M0 classification of every constant-value constructor. -/
def coreConstFeature : ConstValue → SemanticFeature
  | .unit => feature "constant.unit" .executable
  | .bool _ => feature "constant.bool" .executable
  | .character _ => feature "constant.character" .executable
  | .integer _ => feature "constant.integer" .executable
  | .address _ => feature "constant.address" .executable
  | .string _ => feature "constant.string" .executable
  | .bytes _ => feature "constant.bytes" .executable
  | .vector _ => feature "constant.vector" .executable
  | .tuple _ => feature "constant.tuple" .executable
  | .profile _ =>
      unsupportedExecutable "constant.profile" "profile constant requires a registered semantic classification"

def coreBorrowFeature : BorrowKind → SemanticFeature
  | .immutable => feature "borrow.immutable" .executable
  | .mutable => feature "borrow.mutable" .executable
  | .profile _ => unsupportedExecutable "borrow.profile"
      "profile borrow requires a registered semantic classification"

/-- M0 classification of every call form. -/
def coreCallFeature : CallKind → SemanticFeature
  | .function _ => feature "call.function" .executable
  | .constructor _ _ => feature "call.constructor" .executable
  | .destructor _ _ => feature "call.destructor" .executable
  | .closure _ _ => feature "call.closure" .executable
  | .invoke => feature "call.invoke" .executable
  | .extension _ _ =>
      unsupportedExecutable "call.extension" "extension call requires a registered semantic classification"

/-- M2 classification of the known keyed global-storage primitives. -/
def coreGlobalFeature : GlobalKind → SemanticFeature
  | .contains => feature "global.contains" .executable
  | .borrow _ => feature "global.borrow" .executable
  | .take => feature "global.take" .executable
  | .publish => feature "global.publish" .executable

/-- M2 classification of the shared pure primitive vocabulary. -/
def corePrimitiveFeature : PrimitiveOperation → SemanticFeature
  | .tuple => feature "primitive.tuple" .executable
  | .vector => feature "primitive.vector" .executable
  | .repeatVector => feature "primitive.repeatVector" .executable
  | .pushVector => feature "primitive.pushVector" .executable
  | .concatVector => feature "primitive.concatVector" .executable
  | .insertVector => feature "primitive.insertVector" .executable
  | .removeVector => feature "primitive.removeVector" .executable
  | .swapVector => feature "primitive.swapVector" .executable
  | .reverseSliceVector => feature "primitive.reverseSliceVector" .executable
  | .destroyEmptyVector => feature "primitive.destroyEmptyVector" .executable
  | .containsVector => feature "primitive.containsVector" .executable
  | .compare => feature "primitive.compare" .executable
  | .signerAddress => feature "primitive.signerAddress" .executable
  | .indexOfVector => feature "primitive.indexOfVector" .executable
  | .checkVectorIndex _ => feature "primitive.checkVectorIndex" .executable
  | .length => feature "primitive.length" .executable
  | .index => feature "primitive.index" .executable
  | .slice => feature "primitive.slice" .executable
  | .add => feature "primitive.add" .executable
  | .checkedAdd _ => feature "primitive.checkedAdd" .executable
  | .overflowingAdd => feature "primitive.overflowingAdd" .executable
  | .subtract => feature "primitive.subtract" .executable
  | .checkedSubtract _ => feature "primitive.checkedSubtract" .executable
  | .overflowingSubtract => feature "primitive.overflowingSubtract" .executable
  | .multiply => feature "primitive.multiply" .executable
  | .checkedMultiply _ => feature "primitive.checkedMultiply" .executable
  | .overflowingMultiply => feature "primitive.overflowingMultiply" .executable
  | .modulo => feature "primitive.modulo" .executable
  | .checkedModulo _ => feature "primitive.checkedModulo" .executable
  | .divide => feature "primitive.divide" .executable
  | .checkedDivide _ => feature "primitive.checkedDivide" .executable
  | .bitwiseOr => feature "primitive.bitwiseOr" .executable
  | .bitwiseAnd => feature "primitive.bitwiseAnd" .executable
  | .bitwiseXor => feature "primitive.bitwiseXor" .executable
  | .bitwiseNot => feature "primitive.bitwiseNot" .executable
  | .shiftLeft => feature "primitive.shiftLeft" .executable
  | .checkedShiftLeft _ => feature "primitive.checkedShiftLeft" .executable
  | .shiftRight => feature "primitive.shiftRight" .executable
  | .checkedShiftRight _ => feature "primitive.checkedShiftRight" .executable
  | .logicalAnd => feature "primitive.logicalAnd" .executable
  | .logicalOr => feature "primitive.logicalOr" .executable
  | .equal => feature "primitive.equal" .executable
  | .notEqual => feature "primitive.notEqual" .executable
  | .less => feature "primitive.less" .executable
  | .greater => feature "primitive.greater" .executable
  | .lessEqual => feature "primitive.lessEqual" .executable
  | .greaterEqual => feature "primitive.greaterEqual" .executable
  | .logicalNot => feature "primitive.logicalNot" .executable
  | .negate => feature "primitive.negate" .executable
  | .checkedNegate _ => feature "primitive.checkedNegate" .executable
  | .copyValue => feature "primitive.copyValue" .executable
  | .moveValue => feature "primitive.moveValue" .executable
  | .cast => feature "primitive.cast" .executable
  | .checkedCast _ => feature "primitive.checkedCast" .executable
  | .range => unsupportedExecutable "primitive.range" "range construction requires the remaining M2 checks"
  | .implies => feature "primitive.implies" .logicalOnly
  | .equivalent => feature "primitive.equivalent" .logicalOnly
  | .identical => feature "primitive.identical" .logicalOnly

def coreReferenceOperationFeature : ReferenceOperation → SemanticFeature
  | .borrow _ => unsupportedExecutable "reference.borrowValue"
      "value borrow requires checked place normalization"
  | .dereference => feature "reference.dereferenceValue" .executable
  | .freeze _ => feature "reference.freeze" .executable
  | .mutate => feature "reference.mutate" .executable

def coreDataOperationFeature : DataOperation → SemanticFeature
  | .select _ _ => feature "data.select" .executable
  | .selectVariants _ _ => feature "data.selectVariants" .executable
  | .testVariants _ _ => feature "data.testVariants" .executable
  | .discriminant _ => feature "data.discriminant" .executable
  | .updateField _ _ => feature "data.updateField" .executable

private def unsupportedSpecification (name : String) : SemanticFeature :=
  unsupportedLogical s!"specification.{name}" "specification operations require M4 logical semantics"

def coreSpecOperationFeature : SpecOperation → SemanticFeature
  | .functionCall _ _ => unsupportedSpecification "functionCall"
  | .lemma _ _ => unsupportedSpecification "lemma"
  | .behavior .requiresOf _ => unsupportedSpecification "behavior.requiresOf"
  | .behavior .abortsOf _ => unsupportedSpecification "behavior.abortsOf"
  | .behavior .ensuresOf _ => unsupportedSpecification "behavior.ensuresOf"
  | .behavior .resultOf _ => unsupportedSpecification "behavior.resultOf"
  | .behavior .unchangedOf _ => unsupportedSpecification "behavior.unchangedOf"
  | .behavior .foldsOf _ => unsupportedSpecification "behavior.foldsOf"
  | .behavior (.writeOf _) _ => unsupportedSpecification "behavior.writeOf"
  | .result _ => unsupportedSpecification "result"
  | .typeValue => unsupportedSpecification "typeValue"
  | .typeDomain => unsupportedSpecification "typeDomain"
  | .resourceDomain => unsupportedSpecification "resourceDomain"
  | .stateDomain => unsupportedSpecification "stateDomain"
  | .global _ => unsupportedSpecification "global"
  | .exists _ => unsupportedSpecification "exists"
  | .canModify => unsupportedSpecification "canModify"
  | .old => unsupportedSpecification "old"
  | .final => unsupportedSpecification "final"
  | .saveStateAnchor _ => unsupportedSpecification "saveStateAnchor"
  | .withStateAnchor _ => unsupportedSpecification "withStateAnchor"
  | .foldsCaptureAnchor _ => unsupportedSpecification "foldsCaptureAnchor"
  | .inlineCallSummary => unsupportedSpecification "inlineCallSummary"
  | .trace _ => unsupportedSpecification "trace"
  | .publish _ => unsupportedSpecification "publish"
  | .remove _ => unsupportedSpecification "remove"
  | .update _ => unsupportedSpecification "update"
  | .emptyVector => unsupportedSpecification "emptyVector"
  | .singletonVector => unsupportedSpecification "singletonVector"
  | .updateVector => unsupportedSpecification "updateVector"
  | .concatVector => unsupportedSpecification "concatVector"
  | .indexOfVector => unsupportedSpecification "indexOfVector"
  | .containsVector => unsupportedSpecification "containsVector"
  | .lengthVector => unsupportedSpecification "lengthVector"
  | .indexVector => unsupportedSpecification "indexVector"
  | .sliceVector => unsupportedSpecification "sliceVector"
  | .inRange => unsupportedSpecification "inRange"
  | .inVectorRange => unsupportedSpecification "inVectorRange"
  | .vectorRange => unsupportedSpecification "vectorRange"
  | .maxValue _ => unsupportedSpecification "maxValue"
  | .bitVectorToInt => unsupportedSpecification "bitVectorToInt"
  | .intToBitVector => unsupportedSpecification "intToBitVector"
  | .abortFlag => unsupportedSpecification "abortFlag"
  | .abortCode => unsupportedSpecification "abortCode"
  | .wellFormed => unsupportedSpecification "wellFormed"
  | .boxValue => unsupportedSpecification "boxValue"
  | .unboxValue => unsupportedSpecification "unboxValue"
  | .emptyEventStore => unsupportedSpecification "emptyEventStore"
  | .extendEventStore => unsupportedSpecification "extendEventStore"
  | .eventStoreIncludes => unsupportedSpecification "eventStoreIncludes"
  | .eventStoreIncludedIn => unsupportedSpecification "eventStoreIncludedIn"
  | .noOp => unsupportedSpecification "noOp"

/-- M0 classification of every operation form. -/
def coreOperationFeature : Operation → SemanticFeature
  | .move _ => feature "operation.move" .executable
  | .copy _ => feature "operation.copy" .executable
  | .borrow _ _ => feature "operation.borrow" .executable
  | .read _ => feature "operation.read" .executable
  | .write _ => feature "operation.write" .executable
  | .call kind => coreCallFeature kind
  | .global kind => coreGlobalFeature kind
  | .primitive kind => corePrimitiveFeature kind
  | .reference kind => coreReferenceOperationFeature kind
  | .data kind => coreDataOperationFeature kind
  | .specification kind => coreSpecOperationFeature kind
  | .assert => feature "operation.assert" .executable
  | .drop _ => feature "operation.drop" .executable
  | .profile _ _ =>
      unsupportedExecutable "operation.profile" "profile operation requires a registered semantic classification"

/-- M0 classification of every preferred surface form. -/
def coreSurfaceFeature : SurfaceSyntax → SemanticFeature
  | .receiverCall => feature "surface.receiverCall" .frontendOnly
  | .indexNotation => feature "surface.indexNotation" .frontendOnly
  | .extension _ =>
      feature "surface.extension" (.unsupported .frontendOnly
        "profile surface form requires a registered semantic classification")

/-- M0 classification of every place form. -/
def corePlaceFeature : Place → SemanticFeature
  | .localVar _ => feature "place.local" .executable
  | .deref _ => feature "place.deref" .executable
  | .field .. => feature "place.field" .executable
  | .index _ _ => feature "place.index" .executable
  | .subslice .. => feature "place.subslice" .executable
  | .downcast _ _ => feature "place.downcast" .executable

/-- M0 classification of every pattern form. -/
def corePatternFeature : PatternKind → SemanticFeature
  | .wildcard => feature "pattern.wildcard" .executable
  | .variable _ => feature "pattern.variable" .executable
  | .tuple _ => feature "pattern.tuple" .executable
  | .constructor _ _ _ _ => feature "pattern.constructor" .executable
  | .literal _ => feature "pattern.literal" .executable
  | .range _ _ _ => feature "pattern.range" .executable

def coreBinderFeature : BinderKind → SemanticFeature
  | .typeArg => feature "binder.type" .executable
  | .const => feature "binder.const" .executable
  | .lifetime => feature "binder.lifetime" .executable
  | .evidence => feature "binder.evidence" .executable

/-- M0 classification of every quantifier form. -/
def coreQuantifierFeature : QuantifierKind → SemanticFeature
  | .forall => unsupportedLogical "quantifier.forall" "quantifiers require M4 logical semantics"
  | .exists => unsupportedLogical "quantifier.exists" "quantifiers require M4 logical semantics"
  | .choose => unsupportedLogical "quantifier.choose" "choice requires M4 logical semantics"
  | .chooseMin => unsupportedLogical "quantifier.chooseMin" "choice requires M4 logical semantics"
  | .profile _ => unsupportedLogical "quantifier.profile"
      "profile quantifier requires a registered semantic classification"

/-- Shared specification-condition inventory. Logical interpretation lands in
M4, but no language profile is needed to identify these roles. -/
def coreConditionFeature : ConditionKind → SemanticFeature
  | .letPost _ => unsupportedLogical "condition.letPost" "conditions require M4 logical semantics"
  | .letPre _ => unsupportedLogical "condition.letPre" "conditions require M4 logical semantics"
  | .assertion => unsupportedLogical "condition.assert" "conditions require M4 logical semantics"
  | .assumption => unsupportedLogical "condition.assume" "conditions require M4 logical semantics"
  | .decreases => unsupportedLogical "condition.decreases" "conditions require M4 logical semantics"
  | .abortsIf => unsupportedLogical "condition.abortsIf" "conditions require M4 logical semantics"
  | .abortsWith => unsupportedLogical "condition.abortsWith" "conditions require M4 logical semantics"
  | .succeedsIf => unsupportedLogical "condition.succeedsIf" "conditions require M4 logical semantics"
  | .emits => unsupportedLogical "condition.emits" "conditions require M4 logical semantics"
  | .ensures => unsupportedLogical "condition.ensures" "conditions require M4 logical semantics"
  | .requires => unsupportedLogical "condition.requires" "conditions require M4 logical semantics"
  | .structInvariant => unsupportedLogical "condition.structInvariant" "conditions require M4 logical semantics"
  | .functionInvariant => unsupportedLogical "condition.functionInvariant" "conditions require M4 logical semantics"
  | .loopInvariant => unsupportedLogical "condition.loopInvariant" "conditions require M4 logical semantics"
  | .globalInvariant _ => unsupportedLogical "condition.globalInvariant" "conditions require M4 logical semantics"
  | .globalInvariantUpdate _ => unsupportedLogical "condition.globalInvariantUpdate" "conditions require M4 logical semantics"
  | .schemaInvariant => unsupportedLogical "condition.schemaInvariant" "conditions require M4 logical semantics"
  | .axiom_ _ => unsupportedLogical "condition.axiom" "conditions require M4 logical semantics"
  | .update => unsupportedLogical "condition.update" "conditions require M4 logical semantics"
  | .apply => unsupportedLogical "condition.apply" "conditions require M4 logical semantics"
  | .split => unsupportedLogical "condition.split" "conditions require M4 logical semantics"

/-- Most condition expressions are propositions, but let bindings, decreases
measures, emitted values, abort-code sets, and update values carry data. -/
private def conditionExpressionIsProposition : ConditionKind → Bool
  | .assertion | .assumption | .abortsIf | .succeedsIf | .ensures | .requires |
      .structInvariant | .functionInvariant | .loopInvariant | .globalInvariant _ |
      .globalInvariantUpdate _ | .schemaInvariant | .axiom_ _ | .apply => true
  | .letPost _ | .letPre _ | .decreases | .abortsWith | .emits | .update | .split => false

private def conditionAuxiliaryAllowed (kind : ConditionKind) (name : String) : Bool :=
  match kind, name with
  | .abortsIf, "abortCode" | .abortsWith, "additionalCode" |
      .emits, "emitsHandle" | .emits, "emitsCondition" |
      .update, "updateTarget" => true
  | _, _ => false

private def conditionAuxiliaryDiagnostics (condition : Condition) : Array Diagnostic :=
  let roleErrors := condition.auxiliary.zipIdx.foldl (init := #[])
    fun diagnostics (auxiliary, index) =>
      let diagnostics := if conditionAuxiliaryAllowed condition.kind auxiliary.1 then diagnostics
        else diagnostics.push <| .at "LIR-SEMANTIC-CONDITION"
          s!"auxiliary role `{auxiliary.1}` is not valid for {repr condition.kind}" condition.loc
      if auxiliary.1 == "additionalCode" ||
          !(condition.auxiliary.take index).any (·.1 == auxiliary.1) then diagnostics
      else diagnostics.push <| .at "LIR-SEMANTIC-CONDITION"
        s!"condition has duplicate auxiliary role `{auxiliary.1}`" condition.loc
  let required := match condition.kind with
    | .emits => some "emitsHandle"
    | .update => some "updateTarget"
    | _ => none
  match required with
  | some name => if condition.auxiliary.any (·.1 == name) then roleErrors else
      roleErrors.push <| .at "LIR-SEMANTIC-CONDITION"
        s!"condition is missing required auxiliary role `{name}`" condition.loc
  | none => roleErrors

/-- M0 classification of every throw form. -/
def coreThrowFeature : ThrowKind → SemanticFeature
  | .abort => feature "throw.abort" .executable
  | .panic => feature "throw.panic" .executable
  | .profile _ => unsupportedExecutable "throw.profile"
      "profile throw requires a registered semantic classification"

/-- M0 classification of every expression constructor. Operations,
quantifiers, throws, profile values, and children receive their more specific
classification during traversal. -/
def coreExprFeature : ExprKind → SemanticFeature
  | .value _ _ => feature "expression.value" .executable
  | .constant _ => feature "expression.constant" .executable
  | .localVar _ => feature "expression.local" .executable
  | .operation _ _ _ _ => feature "expression.operation" .executable
  | .block _ _ => feature "expression.block" .executable
  | .letDecl _ _ _ => feature "expression.let" .executable
  | .ifElse _ _ _ => feature "expression.if" .executable
  | .match_ _ _ => feature "expression.match" .executable
  | .loop _ _ => feature "expression.loop" .executable
  | .break_ _ _ => feature "expression.break" .executable
  | .continue_ _ => feature "expression.continue" .executable
  | .return_ _ => feature "expression.return" .executable
  | .throw_ _ _ => feature "expression.throw" .executable
  | .assign _ _ => feature "expression.assign" .executable
  | .assignPattern _ _ => feature "expression.assignPattern" .executable
  | .quantifier _ _ _ _ _ => unsupportedLogical "expression.quantifier"
      "quantifier expressions require M4 logical semantics"
  | .spec _ => feature "expression.spec" .logicalOnly

/-- Raw control is frontend-only and disappears at validation. These matches
still make additions to the raw boundary require explicit inventory review. -/
def coreRawTerminatorFeature : RawTerminator → SemanticFeature
  | .goto _ => feature "rawTerminator.goto" .frontendOnly
  | .branch _ _ _ => feature "rawTerminator.branch" .frontendOnly
  | .switch _ _ _ => feature "rawTerminator.switch" .frontendOnly
  | .call _ _ _ => feature "rawTerminator.call" .frontendOnly
  | .drop _ _ _ => feature "rawTerminator.drop" .frontendOnly
  | .assert _ _ _ _ _ => feature "rawTerminator.assert" .frontendOnly
  | .return_ _ => feature "rawTerminator.return" .frontendOnly
  | .throw_ _ _ => feature "rawTerminator.throw" .frontendOnly
  | .unreachable => feature "rawTerminator.unreachable" .frontendOnly
  | .resume => feature "rawTerminator.resume" .frontendOnly
  | .abort => feature "rawTerminator.abort" .frontendOnly

/-- Raw statements are likewise inventoried even though administrative MIR
forms are rejected until their state semantics are implemented. -/
def coreRawStatementFeature : RawStatement → SemanticFeature
  | .execute _ => feature "rawStatement.execute" .frontendOnly
  | .storageLive _ => feature "rawStatement.storageLive" .frontendOnly
  | .storageDead _ => feature "rawStatement.storageDead" .frontendOnly
  | .deinit _ => feature "rawStatement.deinit" .frontendOnly
  | .setDiscriminant _ _ => feature "rawStatement.setDiscriminant" .frontendOnly
  | .retag _ => feature "rawStatement.retag" .frontendOnly
  | .placeMention _ => feature "rawStatement.placeMention" .frontendOnly
  | .ascribeUserType _ _ => feature "rawStatement.ascribeUserType" .frontendOnly
  | .profile _ => feature "rawStatement.profile" .frontendOnly

def coreRawBodyFeature : RawBody → SemanticFeature
  | .absent => feature "rawBody.absent" .frontendOnly
  | .structured _ => feature "rawBody.structured" .frontendOnly
  | .cfg _ => feature "rawBody.cfg" .frontendOnly

def coreFunctionBodyFeature : FunctionBody → SemanticFeature
  | .absent => unsupportedExecutable "functionBody.absent"
      "absent function bodies require an external semantic implementation"
  | .structured _ => feature "functionBody.structured" .executable

/-! ## Preparation traversal -/

private inductive PreparationMode where
  | typing
  | execution
  | verification
  deriving BEq

/-- Typing obligations are authoritative at validation (`.typing`) and remain
on the verification path until specification typing moves into validation;
execution preparation trusts the validated unit. -/
private def typingGate (mode : PreparationMode) (ds : Array Diagnostic) : Array Diagnostic :=
  match mode with
  | .execution => #[]
  | _ => ds

private def modeName : PreparationMode → String
  | .typing => "typing"
  | .execution => "execution"
  | .verification => "verification"

private def unsupportedCode : PreparationMode → String
  | .typing => "LIR-SEMANTIC-UNSUPPORTED"
  | .execution => "LIR-EXEC-UNSUPPORTED"
  | .verification => "LIR-VERIFY-UNSUPPORTED"

private def classificationDiagnostics (mode : PreparationMode) (loc : LocId)
    (semanticFeature : SemanticFeature) : Array Diagnostic :=
  if mode matches .typing then #[] else
  match semanticFeature.classification.support with
  | .unsupported reason =>
      #[.at (unsupportedCode mode)
        s!"{semanticFeature.name} is not supported for {modeName mode}: {reason}" loc]
  | .supported =>
      match mode, semanticFeature.classification.domain with
      | .execution, .logicalOnly =>
          #[.at "LIR-EXEC-LOGICAL"
            s!"{semanticFeature.name} is logical-only and cannot execute" loc]
      | _, _ => #[]

private def registryDiagnostics (registry : SemanticsRegistry)
    (unit : ValidatedUnit) : Array Diagnostic :=
  let duplicateErrors := (Array.range registry.size).foldl (fun diagnostics index =>
    match registry[index]? with
    | some semantics =>
        if registry.take index |>.any (·.profile == semantics.profile) then
          diagnostics.push <| .error "LIR-SEMANTICS-DUPLICATE"
            s!"profile {repr semantics.profile} has more than one semantic implementation"
        else diagnostics
    | none => diagnostics) #[]
  unit.profiles.foldl (init := duplicateErrors) fun diagnostics config =>
    match semanticProfile? registry config.profile with
    | none => diagnostics.push <| .error "LIR-SEMANTICS-UNREGISTERED"
        s!"profile {config.name}@{config.version} has no registered semantics"
    | some semantics =>
        if semantics.name == config.name && semantics.version == config.version then diagnostics
        else diagnostics.push <| .error "LIR-SEMANTICS-VERSION"
          s!"profile configuration {config.name}@{config.version} does not match semantic implementation {semantics.name}@{semantics.version}"

private def profileDiagnostics (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (site : ProfileSemanticSite) (loc : LocId)
    (value : ProfileValue) : Array Diagnostic :=
  if mode matches .typing then #[] else
  match semanticProfile? registry value.profile, profileConfig? unit.profiles value.profile with
  | some semantics, some config =>
      if semantics.name != config.name || semantics.version != config.version then
        #[.at "LIR-SEMANTICS-VERSION"
          s!"profile value uses {config.name}@{config.version}, but semantics provides {semantics.name}@{semantics.version}" loc]
      else
        match semantics.classify site value with
        | some classification =>
            classificationDiagnostics mode loc
              (feature s!"profile.{config.name}.{value.tag}" classification)
        | none => #[.at "LIR-SEMANTICS-UNCLASSIFIED"
            s!"profile {config.name}@{config.version} does not classify {repr site} tag `{value.tag}`" loc]
  | _, _ => #[.at "LIR-SEMANTICS-UNREGISTERED"
      s!"profile {repr value.profile} has no matching semantic implementation" loc]

private def genericArgumentsEquivalent? (leftNs : ValidatedNamespace)
    (left : Array GenericArgument) (rightNs : ValidatedNamespace)
    (right : Array GenericArgument) : Bool :=
  left.size == right.size && (left.zip right).all fun pair => match pair with
    | (.typeArg left, .typeArg right) =>
        typeIdsEquivalent? leftNs left.typeId rightNs right.typeId
    | (.const left, .const right) => left == right
    | (.lifetime left, .lifetime right) =>
        lifetimeKindsEquivalent? leftNs left rightNs right
    | (.evidence left, .evidence right) => left == right
    | _ => false

/-- Structural type agreement for typing rules: identical interned nodes, or
structurally equivalent types whose lifetimes agree by kind. Frontends may
intern distinct inference lifetimes for equal reference spellings, so typing
never compares reference types by interned identity alone. -/
private def typesAgree (ns : ValidatedNamespace) (left right : TypeId) : Bool :=
  left == right || typeIdsEquivalent? ns left ns right

private def exprTypeAgrees (ns : ValidatedNamespace) (id : ExprId)
    (expected : TypeId) : Bool :=
  match (ns.expressions[id.index]?).map (·.typeId) with
  | some actual => typesAgree ns actual expected
  | none => false

/-- Agreement between a specification-side type and an executable declaration
type. The specification projection erases one reference layer, and every
integer width lives in the mathematical integer domain, so specification
integers agree regardless of width. Tuples agree componentwise. -/
private def specProjectedAgree (ns : ValidatedNamespace)
    (specType execType : TypeId) (fuel : Nat := ns.tables.types.size) : Bool :=
  typesAgree ns specType execType ||
    (match fuel, ns.tables.types[specType.index]?, ns.tables.types[execType.index]? with
     | fuel + 1, some (.tuple specElements), some (.tuple execElements) =>
         specElements.size == execElements.size &&
           (specElements.zip execElements).all fun pair =>
             specProjectedAgree ns pair.1 pair.2 fuel
     -- A vector reads its elements at projection too.
     | fuel + 1, some (.vector specElement _), some (.vector execElement _) =>
         specProjectedAgree ns specElement execElement fuel
     | _, _, _ => false) ||
    (isAnyIntegerType ns specType && isAnyIntegerType ns execType) ||
    (match ns.tables.types[execType.index]? with
     | some (.reference reference) =>
         typesAgree ns specType reference.referent ||
           (isAnyIntegerType ns specType &&
             isAnyIntegerType ns reference.referent)
     | _ => false) ||
    (match ns.tables.types[specType.index]? with
     | some (.reference reference) =>
         typesAgree ns reference.referent execType ||
           (isAnyIntegerType ns reference.referent &&
             isAnyIntegerType ns execType)
     | _ => false)

private structure ScanContext where
  locals : Array LocalDecl := #[]
  results : Array TypeUse := #[]
  profile : Profile := .move
  generics : Array GenericBinder := #[]
  loopResults : Array TypeId := #[]
  /-- Whether the scanned expression is specification content: the logical
  domain admits mathematical integers and the specification projection of
  executable declarations (one reference layer erased, integers widened). -/
  logical : Bool := false
  /-- Whether lemma instances may occur: in an `apply` step, whose shape
  `applyFormulaDiagnostics` checks. -/
  lemmaInstances : Bool := false

private def arenaGetNative? {α : Type} (values : Array α) (index : Nat) : Option α :=
  values[index]?

/-- Native validation keeps constant-time array access. Kernel reduction uses
the equivalent list lookup, which does not traverse the entire arena to
recompute its length before walking to the requested slot. -/
@[implemented_by arenaGetNative?]
private def arenaGet? {α : Type} (values : Array α) (index : Nat) : Option α :=
  values.toList[index]?

private theorem arenaGet?_eq {α : Type} (values : Array α) (index : Nat) :
    arenaGet? values index = arenaGetNative? values index := by
  simp [arenaGet?, arenaGetNative?]

private def exprType? (ns : ValidatedNamespace) (id : ExprId) : Option TypeId :=
  (arenaGet? ns.expressions id.index).map (·.typeId)

/-- Abrupt expressions are bottom-polymorphic: because they produce no normal
value, their stored type need not equal the surrounding result type. -/
private def exprMatchesOrIsAbrupt (ns : ValidatedNamespace) (id : ExprId)
    (expected : TypeId) : Bool :=
  exprTypeAgrees ns id expected || !expressionCanFallThrough ns id

private def loopResultType? (context : ScanContext) (nest : Nat) : Option TypeId :=
  if nest < context.loopResults.size then
    context.loopResults[context.loopResults.size - nest - 1]?
  else none

private def patternType? (ns : ValidatedNamespace) (id : PatternId) : Option TypeId :=
  (ns.patterns[id.index]?).map (·.typeId)

private def isBoolType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  ns.tables.types[id.index]? == some .bool

private def isLogicalNumType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  ns.tables.types[id.index]? == some (.integer .unbounded true)

/-- Whether a specification reads a value of this type as an integer: the
specification projection erases one reference layer. -/
private def specReadsInteger (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  isAnyIntegerType ns id ||
    match ns.tables.types[id.index]? with
    | some (.reference reference) => isAnyIntegerType ns reference.referent
    | _ => false

private def typeMismatch (loc : LocId) (message : String) : Array Diagnostic :=
  #[.at "LIR-SEMANTIC-TYPE" message loc]

private def arityMismatch (loc : LocId) (operation : String)
    (expected actual : Nat) : Array Diagnostic :=
  #[.at "LIR-SEMANTIC-ARITY"
    s!"{operation} expects {expected} operand(s), but has {actual}" loc]

private def exactArity (loc : LocId) (operation : String) (expected actual : Nat) :
    Array Diagnostic :=
  if expected == actual then #[] else arityMismatch loc operation expected actual

private def genericArgumentMatches : BinderKind → GenericArgument → Bool
  | .typeArg, .typeArg _ | .const, .const _ | .lifetime, .lifetime _ |
      .evidence, .evidence _ => true
  | _, _ => false

private def genericInstantiationDiagnostics (loc : LocId) (description : String)
    (binders : Array GenericBinder) (arguments : Array GenericArgument) : Array Diagnostic :=
  let arityErrors := exactArity loc description binders.size arguments.size
  (binders.zip arguments).zipIdx.foldl (init := arityErrors) fun ds (pair, index) =>
    if genericArgumentMatches pair.1.kind pair.2 then ds else
      ds.push <| .at "LIR-SEMANTIC-GENERIC-KIND"
        s!"{description} argument {index} does not match binder kind {repr pair.1.kind}" loc

private def literalIndex? (ns : ValidatedNamespace) (id : ExprId) : Option Nat := do
  let expression ← ns.expressions[id.index]?
  let .value (.integer value) _ := expression.kind | none
  if value < 0 then none else some value.toNat

private def isSimplePlaceIndex (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  (placeIndexForm? ns id).isSome

private partial def isOwnedLocalPlaceBase (ns : ValidatedNamespace) (id : PlaceId)
    (fuel : Nat := 0) : Bool :=
  let fuel := if fuel == 0 then ns.places.size + 1 else fuel
  match fuel, ns.places[id.index]? with
  | 0, _ | _, none => false
  | _ + 1, some (.localVar _) => true
  | fuel + 1, some (.field base ..) | fuel + 1, some (.subslice base ..) |
      fuel + 1, some (.downcast base _) =>
      isOwnedLocalPlaceBase ns base fuel
  | fuel + 1, some (.index base index) =>
      isSimplePlaceIndex ns index && isOwnedLocalPlaceBase ns base fuel
  | _, some (.deref _) => false

private def isConsumableLocalPlace (ns : ValidatedNamespace) (id : PlaceId) : Bool :=
  match ns.places[id.index]? with
  | some (.localVar _) => true
  | some (.field base ..) => isOwnedLocalPlaceBase ns base
  | some (.index base index) =>
      isSimplePlaceIndex ns index && isOwnedLocalPlaceBase ns base
  | some (.subslice base ..) => isOwnedLocalPlaceBase ns base
  | _ => false

private def referenceResultDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (kind : BorrowKind) : Array Diagnostic :=
  match ns.tables.types[resultType.index]? with
  | some (.reference reference) =>
      match kind with
      | .immutable =>
          if reference.kind == .shared then #[]
          else typeMismatch loc "immutable borrow result is not an immutable reference"
      | .mutable =>
          if reference.kind == .mutable then #[]
          else typeMismatch loc "mutable borrow result is not a mutable reference"
      | .profile _ => #[]
  | _ => typeMismatch loc "borrow result is not a reference type"

private def isFixedIntegerType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  match ns.tables.types[id.index]? with
  | some (.integer (.bits width) _) => width != 0
  | some (.integer .pointer _) => true
  | _ => false

private def isVectorType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  match ns.tables.types[id.index]? with
  | some (.vector _ _) => true
  | _ => false

/-- A vector, or a reference to one: the specification projection treats the
reference transparently. -/
private def specVectorElement? (ns : ValidatedNamespace) (id : TypeId) : Option TypeId :=
  match ns.tables.types[id.index]? with
  | some (.vector element _) => some element
  | some (.reference reference) =>
      match ns.tables.types[reference.referent.index]? with
      | some (.vector element _) => some element
      | _ => none
  | _ => none

private def hasLengthType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  match ns.tables.types[id.index]? with
  | some (.vector _ _) | some .string | some .bytes => true
  | _ => false

private def isUnitType (ns : ValidatedNamespace) (id : TypeId) : Bool :=
  match ns.tables.types[id.index]? with
  | some .unit => true
  | some (.tuple elements) => elements.isEmpty
  | _ => false

private structure StaticPlaceInfo where
  typeId : TypeId
  writable : Bool
  selectedVariant : Option String := none

/-- The owned namespace holding declarations for an interned name. External
names have no owned namespace and resolve to `none`. -/
private def ownedNamespaceForName? (unit : ValidatedUnit) (name : NameId) :
    Option ValidatedNamespace := do
  let qualifiedName ← unit.tables.names[name.index]?
  -- Owned namespaces are a list, not an array keyed by namespace identity.
  unit.namespaces.find? (·.identity == qualifiedName.namespaceId)

/-- The nominal declaration a dependency interface exports under this name.
Every namespace of a unit shares one checked table snapshot, so a declaration
read from an interface is read with the same names and types as an owned one. -/
private def interfaceStructForName? (unit : ValidatedUnit) (name : NameId) :
    Option StructDecl := do
  let qualifiedName ← unit.tables.names[name.index]?
  let interface ← unit.dependencies.find? (·.namespaceId == qualifiedName.namespaceId)
  interface.structs.find? (·.name == name)

/-- Resolve the declaration named by a nominal type through the checked
resolution index. All namespaces in a validated compilation unit share one
checked table snapshot, so `NameId` and `TypeId` remain meaningful across the
returned namespace boundary. -/
private def nominalDeclarationForType? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (typeId : TypeId) (typeAt : Nat → Option Ty := fun index => ns.tables.types[index]?) :
    Option (ValidatedNamespace × StructDecl × Array GenericArgument) := do
  let .nominal name arguments ← typeAt typeId.index | none
  let owned : Option (ValidatedNamespace × StructDecl) := do
    let declarationId ← unit.resolution.nominal? name
    let targetNs ← ownedNamespaceForName? unit name
    let declaration ← targetNs.structs[declarationId.index]?
    some (targetNs, declaration)
  let (targetNs, declaration) ← owned <|>
    (interfaceStructForName? unit name).map fun declaration => (ns, declaration)
  some (targetNs, declaration, arguments)

private def sourceName? (ns : ValidatedNamespace) (name : NameId) : Option String :=
  (ns.tables.names[name.index]?).map (·.name)


private def fieldNamedForPlace? (targetNs : ValidatedNamespace)
    (fields : Array FieldDecl) (name : String) : Option FieldDecl :=
  fields.find? fun field => sourceName? targetNs field.name == some name

/-- Select the possible declarations of a field at a place. Before a
downcast, an enum field is usable only when every variant declares it; after
a downcast, only the selected variant participates. -/
private def selectedPlaceFieldTypes? (targetNs : ValidatedNamespace)
    (declaration : StructDecl) (selectedVariant : Option String) (field : String) :
    Option (Array TypeUse) :=
  if declaration.variants.isEmpty then do
    if selectedVariant.isSome then none else
      let selected ← fieldNamedForPlace? targetNs declaration.fields field
      some #[selected.type]
  else match selectedVariant with
    | some variantName => do
        let variant ← declaration.variants.find? fun variant =>
          sourceName? targetNs variant.name == some variantName
        let selected ← fieldNamedForPlace? targetNs variant.fields field
        some #[selected.type]
    | none =>
        -- Before a downcast, an enum field selects across the variants that
        -- declare it: the place form of a variant-field select, whose
        -- wrong-variant behavior is a runtime abort rather than a typing
        -- error.
        let declared := declaration.variants.filterMap fun variant =>
          fieldNamedForPlace? targetNs variant.fields field
        if declared.isEmpty then none else some (declared.map (fun f => f.type))

/-- Whether a field type still contains a declaration-local type parameter.
Concrete fields of generic nominal declarations remain checkable; only a
field whose own type depends on an instantiation waits for substitution. -/
private def typeNeedsSubstitution (tables : Tables) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, typeId => match tables.types[typeId.index]? with
      | some (.typeParameter _) => true
      | some (.tuple elements) => elements.any (typeNeedsSubstitution tables fuel)
      | some (.vector element _) => typeNeedsSubstitution tables fuel element
      | some (.typeDomain type) => typeNeedsSubstitution tables fuel type
      | some (.resourceDomain _ arguments) => arguments.toArray.flatten.any
          (typeNeedsSubstitution tables fuel)
      | some (.nominal _ arguments) => arguments.any fun argument => match argument with
          | .typeArg value => typeNeedsSubstitution tables fuel value.typeId
          | .const _ | .lifetime _ | .evidence _ => false
      | some (.function arguments result _) =>
          arguments.any (typeNeedsSubstitution tables fuel) ||
            typeNeedsSubstitution tables fuel result
      | some (.reference reference) => typeNeedsSubstitution tables fuel reference.referent
      | _ => false

private def fieldTypeNeedsSubstitution (ns : ValidatedNamespace) (typeId : TypeId) : Bool :=
  typeNeedsSubstitution ns.tables (ns.tables.types.size + 1) typeId

/-- `instantiatePlaceFieldTypeFuel?` reading the types through an index and
searching them as a list: the form a kernel certificate evaluates, where an
array is read through its list and an indexed search over it is quadratic. -/
def instantiatePlaceFieldTypeFuelIn? (ns : ValidatedNamespace) (types : IndexedArena Ty)
    (typeList : List Ty) (instantiations : Array GenericArgument) : Nat → TypeId → Option TypeId
  | 0, _ => none
  | fuel + 1, typeId => do
      let type ← types.get? typeId.index
      match type with
      | .typeParameter index => match instantiations[index]? with
          | some (.typeArg value) => some value.typeId
          | _ => none
      | .tuple elements => do
          let instantiated ← elements.mapM
            (instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel)
          let index ← typeList.findIdx? (fun candidate => candidate == .tuple instantiated)
          some ⟨index⟩
      | .vector element length => do
          let element ← instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel element
          let index ← typeList.findIdx? fun candidate =>
            candidate == .vector element length
          some ⟨index⟩
      | .typeDomain nested => do
          let nested ← instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel nested
          let index ← typeList.findIdx? (fun candidate => candidate == .typeDomain nested)
          some ⟨index⟩
      | .resourceDomain resource arguments => do
          let arguments ← arguments.mapM fun arguments =>
            arguments.mapM (instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel)
          let index ← typeList.findIdx? fun candidate =>
            candidate == .resourceDomain resource arguments
          some ⟨index⟩
      | .nominal name arguments => do
          let arguments ← arguments.mapM fun argument => match argument with
            | .typeArg value => do
                let typeId ← instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel
                  value.typeId
                some (.typeArg { value with typeId })
            | .lifetime value => .lifetime <$> instantiateLifetime? ns instantiations value
            | .const value => some (.const value)
            | .evidence value => some (.evidence value)
          let index ← typeList.findIdx? fun candidate => match candidate with
            | .nominal candidateName candidateArguments =>
                candidateName == name &&
                  sameGenericArgumentValues candidateArguments arguments
            | _ => false
          some ⟨index⟩
      | .function arguments result abilities => do
          let arguments ← arguments.mapM
            (instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel)
          let result ← instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel result
          let index ← typeList.findIdx? fun candidate =>
            candidate == .function arguments result abilities
          some ⟨index⟩
      | .reference reference => do
          let referent ← instantiatePlaceFieldTypeFuelIn? ns types typeList instantiations fuel
            reference.referent
          let lifetime ← instantiateLifetime? ns instantiations reference.lifetime
          let instantiated := .reference { reference with referent, lifetime }
          let index ← typeList.findIdx? (fun candidate => candidate == instantiated)
          some ⟨index⟩
      | _ => some typeId

/-- A search over an array is the search over its list. -/
theorem findIdx?_toList {α : Type} (values : Array α) (p : α → Bool) :
    values.toList.findIdx? p = values.findIdx? p := by
  rw [← List.findIdx?_toArray, Array.toArray_toList]

/-- The indexed form agrees with the native one at the namespace's own tables. -/
theorem instantiatePlaceFieldTypeFuelIn?_eq (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (fuel : Nat) :
    instantiatePlaceFieldTypeFuelIn? ns (.ofArray ns.tables.types) ns.tables.types.toList
        instantiations fuel =
      instantiatePlaceFieldTypeFuel? ns instantiations fuel := by
  induction fuel with
  | zero => rfl
  | succ fuel ih =>
      funext typeId
      simp only [instantiatePlaceFieldTypeFuelIn?, instantiatePlaceFieldTypeFuel?,
        IndexedArena.get?_ofArray, ih, findIdx?_toList]
      cases ns.tables.types[typeId.index]? with
      | none => rfl
      | some type => cases type <;> rfl

/-- Instantiate several same-named field declarations (for example across enum
variants) and return their common concrete type when one exists. -/
def instantiatedCommonPlaceTypeId? (ns : ValidatedNamespace)
    (instantiations : Array GenericArgument) (types : Array TypeUse) : Option TypeId := do
  let instantiated ← types.mapM fun typeUse =>
    instantiatePlaceFieldType? ns instantiations typeUse.typeId
  let first ← instantiated[0]?
  if instantiated.all (· == first) then some first else none

private def subsliceLength? (length : Nat) (start stop : Nat) (fromEnd : Bool) : Option Nat :=
  if fromEnd then
    if start + stop <= length then some (length - start - stop) else none
  else if start <= stop && stop <= length then some (stop - start) else none

/-- Resolve the type and static write permission of places whose field types
do not require generic substitution. Runtime bounds and variant-shape checks
remain in the executable place resolver. The place arena is acyclic, so a
fuel of one unit per arena slot is exact rather than an approximation; the
fuel keeps the resolver total, which the semantic-preparation rewrites need
so a prepared unit stays a computable value. -/
private def staticPlaceInfoFuel? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (fuel : Nat)
    (id : PlaceId) (placeAt : Nat → Option Place := arenaGet? ns.places)
    (typeAt : Nat → Option Ty := fun index => ns.tables.types[index]?) :
    Option StaticPlaceInfo := do
  match fuel with
  | 0 => none
  | fuel + 1 =>
  let place ← placeAt id.index
  match place with
  | .localVar localId => do
      let declaration ← context.locals[localId.index]?
      some { typeId := declaration.type.typeId, writable := declaration.mutable }
  | .deref base => do
      let baseInfo ← staticPlaceInfoFuel? unit ns context fuel base placeAt typeAt
      let .reference reference ← typeAt baseInfo.typeId.index | none
      some { typeId := reference.referent, writable := reference.kind == .mutable }
  | .index base index => do
      let baseInfo ← staticPlaceInfoFuel? unit ns context fuel base placeAt typeAt
      match typeAt baseInfo.typeId.index with
      | some (.vector element _) => some { typeId := element, writable := baseInfo.writable }
      | some (.tuple elements) => do
          let index ← literalIndex? ns index
          let element ← elements[index]?
          some { typeId := element, writable := baseInfo.writable }
      | _ => none
  | .subslice base start stop fromEnd => do
      let baseInfo ← staticPlaceInfoFuel? unit ns context fuel base placeAt typeAt
      match typeAt baseInfo.typeId.index with
      | some (.vector _ none) => some baseInfo
      | some (.vector element (some (.integer length))) => do
          if length < 0 then none else
          let resultLength ← subsliceLength? length.toNat start stop fromEnd
          let resultTypeIndex ← ns.tables.types.findIdx? fun type =>
            type == .vector element (some (.integer (Int.ofNat resultLength)))
          some { typeId := ⟨resultTypeIndex⟩, writable := baseInfo.writable }
      | _ => none
  | .downcast base variant => do
      let baseInfo ← staticPlaceInfoFuel? unit ns context fuel base placeAt typeAt
      let (targetNs, declaration, _) ← nominalDeclarationForType? unit ns baseInfo.typeId typeAt
      let variantName ← sourceName? ns variant
      if declaration.variants.any (fun candidate =>
          sourceName? targetNs candidate.name == some variantName) then
        some { baseInfo with selectedVariant := some variantName }
      else none
  | .field base _ field => do
      let baseInfo ← staticPlaceInfoFuel? unit ns context fuel base placeAt typeAt
      let (targetNs, declaration, instantiations) ←
        nominalDeclarationForType? unit ns baseInfo.typeId typeAt
      let fieldName ← sourceName? ns field
      let selected ← selectedPlaceFieldTypes? targetNs declaration
        baseInfo.selectedVariant fieldName
      let typeId ← instantiatedCommonPlaceTypeId? targetNs instantiations selected
      some { typeId, writable := baseInfo.writable }

/-- `staticPlaceInfoFuel?` at the exact arena budget. -/
private def staticPlaceInfo? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (id : PlaceId) : Option StaticPlaceInfo :=
  staticPlaceInfoFuel? unit ns context (ns.places.size + 1) id

private def staticPlaceNodeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace)
    (context : ScanContext) (loc : LocId) : Place → Array Diagnostic
  | .localVar _ => #[]
  | .deref base => match staticPlaceInfo? unit ns context base with
      | some info => match ns.tables.types[info.typeId.index]? with
          | some (.reference _) => #[]
          | _ => typeMismatch loc s!"dereference place base is not a reference: base {repr (ns.places[base.index]?)} : {repr (ns.tables.types[info.typeId.index]?)}"
      | none => #[]
  | .index base index =>
      let indexErrors := match exprType? ns index with
        | some indexType => if isFixedIntegerType ns indexType ||
              (context.logical && isAnyIntegerType ns indexType) then #[]
            else typeMismatch loc "place index is not a nonzero fixed-width integer"
        | none => #[]
      match staticPlaceInfo? unit ns context base with
      | none => indexErrors
      | some info => match ns.tables.types[info.typeId.index]? with
          | some (.vector _ _) => indexErrors
          | some (.tuple elements) => match literalIndex? ns index with
              | some value => if value < elements.size then indexErrors else
                  indexErrors.push <| .at "LIR-SEMANTIC-PLACE-INDEX"
                    s!"tuple place index {value} is out of bounds for {elements.size} elements" loc
              | none => indexErrors.push <| .at "LIR-SEMANTIC-PLACE-INDEX"
                  "tuple place indexes require a nonnegative integer literal" loc
          | _ => indexErrors ++ typeMismatch loc "indexed place base is not a vector or tuple"
  | .subslice base start stop fromEnd =>
      match staticPlaceInfo? unit ns context base with
      | none => #[]
      | some info => match ns.tables.types[info.typeId.index]? with
          | some (.vector _ none) => if !fromEnd && stop < start then
              typeMismatch loc "subslice start exceeds its end" else #[]
          | some (.vector element (some (.integer length))) =>
              if length < 0 then typeMismatch loc "subslice base has a negative length" else
              match subsliceLength? length.toNat start stop fromEnd with
              | none => typeMismatch loc "subslice bounds exceed the fixed vector length"
              | some resultLength =>
                  if ns.tables.types.any fun type =>
                      type == .vector element (some (.integer (Int.ofNat resultLength))) then #[]
                  else typeMismatch loc "subslice result type is not interned"
          | _ => typeMismatch loc "subslice place base is not a vector"
  | .downcast base variant => match staticPlaceInfo? unit ns context base with
      | none => #[]
      | some info => match nominalDeclarationForType? unit ns info.typeId with
          | none => typeMismatch loc "downcast place base is not a resolved nominal type"
          | some (targetNs, declaration, _) =>
              if declaration.variants.isEmpty then
                typeMismatch loc "downcast place base is not an enum"
              else match sourceName? ns variant with
                | some variantName => if declaration.variants.any (fun candidate =>
                    sourceName? targetNs candidate.name == some variantName) then #[] else
                    #[.at "LIR-SEMANTIC-TARGET" "downcast place names an unknown variant" loc]
                | none => #[]
  | .field base _ field => match staticPlaceInfo? unit ns context base with
      | none => #[]
      | some info => match nominalDeclarationForType? unit ns info.typeId with
          | none => typeMismatch loc "field place base is not a resolved nominal type"
          | some (targetNs, declaration, instantiations) =>
              match sourceName? ns field with
              | none => #[]
              | some fieldName =>
                  match selectedPlaceFieldTypes? targetNs declaration
                      info.selectedVariant fieldName with
                  | none => #[.at "LIR-SEMANTIC-TARGET"
                      "field place does not name a field of the selected nominal shape" loc]
                  | some selected =>
                      match instantiatedCommonPlaceTypeId? targetNs instantiations selected with
                      | some _ => #[]
                      | none => if selected.any fun typeUse =>
                          fieldTypeNeedsSubstitution targetNs typeUse.typeId then
                          -- A frontend need not pre-intern every instantiated
                          -- field form; the missing interned node is a
                          -- preparation capability limit, not a typing error.
                          (if mode matches .typing then #[] else
                            #[.at "LIR-SEMANTIC-GENERIC-TYPE"
                              "instantiated nominal field type is not interned" loc])
                        else typeMismatch loc
                          "field place has different types across possible enum variants"

private def placeResultDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (place : PlaceId) : Array Diagnostic :=
  match staticPlaceInfo? unit ns context place with
  | some info =>
      if typesAgree ns info.typeId resultType then #[]
      else typeMismatch loc "place operation result type differs from the place type"
  | none => #[]

private def unitResultDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (operation : String) : Array Diagnostic :=
  if isUnitType ns resultType then #[]
  else typeMismatch loc s!"{operation} result is not Unit"

private def writePlaceDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (place : PlaceId)
    (arguments : Array ExprId) : Array Diagnostic :=
  let resultErrors := typingGate mode (unitResultDiagnostics ns loc resultType "write")
  let placeErrors := match staticPlaceInfo? unit ns context place with
    | some info =>
        let writableErrors := if !(mode matches .typing) && !info.writable then
          typeMismatch loc "write targets an immutable place" else #[]
        let valueErrors := typingGate mode (match arguments.toList with
          | [value] => if exprTypeAgrees ns value info.typeId then #[]
              else typeMismatch loc "written value type differs from the place type"
          | _ => #[])
        writableErrors ++ valueErrors
    | none => #[]
  resultErrors ++ placeErrors

private def placeBorrowDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (kind : BorrowKind)
    (place : PlaceId) : Array Diagnostic :=
  let resultErrors := typingGate mode (referenceResultDiagnostics ns loc resultType kind)
  let placeErrors := match staticPlaceInfo? unit ns context place,
      ns.tables.types[resultType.index]? with
    | some info, some (.reference reference) =>
        let referentErrors := typingGate mode
          (if typesAgree ns reference.referent info.typeId then #[]
           else typeMismatch loc "borrowed place type differs from the reference referent")
        let writableErrors := match kind with
          | .mutable => if !(mode matches .typing) && !info.writable then
              typeMismatch loc "mutable borrow targets an immutable place" else #[]
          | .immutable | .profile _ => #[]
        referentErrors ++ writableErrors
    | _, _ => #[]
  resultErrors ++ placeErrors

private def placeOperationTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (operation : Operation)
    (arguments : Array ExprId) : Array Diagnostic :=
  match operation with
  | .move place | .copy place | .read place =>
      typingGate mode (placeResultDiagnostics unit ns context loc resultType place)
  | .borrow kind place =>
      placeBorrowDiagnostics mode unit ns context loc resultType kind place
  | .write place => writePlaceDiagnostics mode unit ns context loc resultType place arguments
  | .drop _ => typingGate mode (unitResultDiagnostics ns loc resultType "drop")
  | .assert => typingGate mode (
      let resultErrors := unitResultDiagnostics ns loc resultType "assert"
      let argumentErrors := match arguments.toList with
        | [argument] => if (exprType? ns argument).any (isBoolType ns) then #[]
            else typeMismatch loc "assert operand is not Bool"
        | _ => #[]
      resultErrors ++ argumentErrors)
  | _ => #[]

private def argumentsHaveType (ns : ValidatedNamespace) (arguments : Array ExprId)
    (expected : TypeId) : Bool :=
  arguments.all fun argument => exprTypeAgrees ns argument expected

private def fixedIntegerPrimitiveDiagnostics (ns : ValidatedNamespace)
    (logical : Bool) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) (arity : Nat) : Array Diagnostic :=
  if !isFixedIntegerType ns resultType && !(logical && isLogicalNumType ns resultType) then
    typeMismatch loc "integer primitive result is not a nonzero fixed-width integer"
  else if arguments.size != arity then #[]
  else if argumentsHaveType ns arguments resultType ||
      (logical && arguments.all fun argument =>
        (exprType? ns argument).any (specReadsInteger ns)) then #[]
  else typeMismatch loc "integer primitive operand types differ from its result type"

private def booleanPrimitiveDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) (arity : Nat) : Array Diagnostic :=
  if !isBoolType ns resultType then
    typeMismatch loc "logical primitive result is not Bool"
  else if arguments.size != arity then #[]
  else if arguments.all fun argument => (exprType? ns argument).any (isBoolType ns) then #[]
  else typeMismatch loc "logical primitive operand is not Bool"

private def bitwisePrimitiveDiagnostics (ns : ValidatedNamespace)
    (logical : Bool) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  if isBoolType ns resultType then
    booleanPrimitiveDiagnostics ns loc resultType arguments 2
  else
    fixedIntegerPrimitiveDiagnostics ns logical loc resultType arguments 2

/-- Whether two operands have one type; in a specification, a projected
spec representation agrees with its source type too. -/
private def operandsAgree (ns : ValidatedNamespace) (logical : Bool) (left right : ExprId) :
    Bool :=
  match exprType? ns left, exprType? ns right with
  | some leftType, some rightType => typesAgree ns leftType rightType ||
      (logical && (specProjectedAgree ns leftType rightType ||
        specProjectedAgree ns rightType leftType))
  | _, _ => false

private def equalityPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  if !isBoolType ns resultType then
    typeMismatch loc "equality primitive result is not Bool"
  else match arguments.toList with
    | [left, right] =>
        if operandsAgree ns logical left right then #[]
        else typeMismatch loc "equality primitive operand types differ"
    | _ => #[]

/-- The structural order compares two values of one type and returns an
integer. -/
private def structuralOrderDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  let resultErrors := if isFixedIntegerType ns resultType ||
      (logical && isLogicalNumType ns resultType) then #[]
    else typeMismatch loc "a structural comparison must return an integer"
  match arguments.toList with
  | [left, right] =>
      if operandsAgree ns logical left right then resultErrors
      else resultErrors ++ typeMismatch loc "compared values differ in type"
  | _ => resultErrors

private def comparisonPrimitiveDiagnostics (ns : ValidatedNamespace)
    (logical : Bool) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  if !isBoolType ns resultType then
    typeMismatch loc "comparison result is not Bool"
  else match arguments.toList with
    | [left, right] => match exprType? ns left, exprType? ns right with
        | some leftType, some rightType =>
            if (typesAgree ns leftType rightType &&
                  (isFixedIntegerType ns leftType || isBoolType ns leftType ||
                    (logical && isLogicalNumType ns leftType) ||
                    ns.tables.types[leftType.index]? == some .character)) ||
                (logical && specReadsInteger ns leftType &&
                  specReadsInteger ns rightType) then #[]
            else typeMismatch loc
              "comparison operands are not the same Boolean, character, or nonzero fixed-width integer type"
        | _, _ => #[]
    | _ => #[]

private def tuplePrimitiveDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (logical : Bool) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]? with
  | some (.tuple elements) =>
      if arguments.size != elements.size then
        arityMismatch loc "tuple" elements.size arguments.size
      else if (arguments.zip elements).all fun pair =>
          exprTypeAgrees ns pair.1 pair.2 ||
            (logical && (exprType? ns pair.1).any fun operand =>
              specProjectedAgree ns operand pair.2) then #[]
      else typeMismatch loc
        s!"tuple operand types differ from its result element types: operands \
          {repr (arguments.map fun a => (exprType? ns a).bind fun t => ns.tables.types[t.index]?)}, \
          elements {repr (elements.map fun e => ns.tables.types[e.index]?)}"
  | _ => typeMismatch loc "tuple primitive result is not a tuple type"

private def vectorPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]? with
  | some (.vector element length) =>
      let elementErrors := if argumentsHaveType ns arguments element ||
          (logical && arguments.all fun argument =>
            (exprType? ns argument).any (specProjectedAgree ns · element)) then #[]
        else typeMismatch loc "vector operand type differs from its element type"
      let lengthErrors := match length with
        | none => #[]
        | some (.integer expected) =>
            if expected == Int.ofNat arguments.size then #[]
            else typeMismatch loc
              s!"fixed vector has {arguments.size} elements, expected {expected}"
        | some _ => typeMismatch loc "fixed vector length is not an integer constant"
      elementErrors ++ lengthErrors
  | _ => typeMismatch loc "vector primitive result is not a vector type"

/-- `pushVector` extends a vector by one element as a value. Its result and
its vector operand are the same vector type, and the pushed element carries
that vector's element type. A fixed-length vector cannot be the result: the
length would have to grow with it. -/
private def pushVectorPrimitiveDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]?, arguments.toList with
  | some (.vector element none), [vector, value] =>
      let vectorErrors := if exprTypeAgrees ns vector resultType then #[]
        else typeMismatch loc "pushed-vector operand type differs from the result vector type"
      let valueErrors := if exprTypeAgrees ns value element then #[]
        else typeMismatch loc "pushed element type differs from the vector element type"
      vectorErrors ++ valueErrors
  | some (.vector _ (some _)), _ =>
      typeMismatch loc "pushed-vector result has a fixed length"
  | _, _ => typeMismatch loc "pushed-vector result is not a vector type"

/-- `swapVector` returns the vector it was given, with two indexed elements
exchanged, so its result and vector operand share the vector type and the two
index operands are runtime indexes. -/
private def swapVectorPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]?, arguments.toList with
  | some (.vector _ none), [vector, left, right] =>
      let vectorErrors := if exprTypeAgrees ns vector resultType then #[]
        else typeMismatch loc "swapped-vector operand type differs from the result vector type"
      let indexErrors := [left, right].foldl (init := #[]) fun ds index =>
        match exprType? ns index with
        | some indexType =>
            if isFixedIntegerType ns indexType || (logical && isLogicalNumType ns indexType) then ds
            else ds ++ typeMismatch loc "vector swap index is not a nonzero fixed-width integer"
        | none => ds
      vectorErrors ++ indexErrors
  | some (.vector _ (some _)), _ =>
      typeMismatch loc "swapped-vector result has a fixed length"
  | _, _ => typeMismatch loc "swapped-vector result is not a vector type"

private def vectorEditIndexDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (index : ExprId) : Array Diagnostic :=
  match exprType? ns index with
  | some type =>
      if isFixedIntegerType ns type || (logical && isLogicalNumType ns type) then #[]
      else typeMismatch loc "vector edit index is not a nonzero fixed-width integer"
  | none => #[]

private def insertVectorPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [vector, index, value] =>
      pushVectorPrimitiveDiagnostics ns loc resultType #[vector, value] ++
        vectorEditIndexDiagnostics ns logical loc index
  | _ => #[] -- Arity checking reports malformed operand rows.

private def removeVectorPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId) (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [vector, index] =>
      let resultErrors := match ns.tables.types[resultType.index]?, exprType? ns vector with
        | some (.tuple #[elementResult, vectorResult]), some vectorType =>
          match ns.tables.types[vectorType.index]? with
          | some (.vector element none) =>
              if (typesAgree ns elementResult element ||
                  (logical && specProjectedAgree ns elementResult element)) &&
                  typesAgree ns vectorResult vectorType then #[]
              else typeMismatch loc "removed-vector result must contain the element and original vector types"
          | _ => typeMismatch loc "removed-vector operand must be a variable-length vector"
        | _, _ => typeMismatch loc "removed-vector result must be an element/vector pair"
      resultErrors ++ vectorEditIndexDiagnostics ns logical loc index
  | _ => #[]

private def repeatVectorPrimitiveDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]?, arguments.toList with
  | some (.vector element (some (.integer length))), [argument] =>
      let typeErrors := if exprTypeAgrees ns argument element then #[]
        else typeMismatch loc "repeated-vector operand type differs from its element type"
      if length < 0 then
        typeErrors ++ typeMismatch loc "repeated-vector length is negative"
      else typeErrors
  | some (.vector _ none), _ =>
      typeMismatch loc "repeated-vector result does not have a fixed length"
  | some (.vector _ (some _)), _ =>
      typeMismatch loc "repeated-vector length is not an integer constant"
  | _, _ => typeMismatch loc "repeated-vector result is not a vector type"

private def lengthPrimitiveDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  if !isFixedIntegerType ns resultType && !isLogicalNumType ns resultType then
    typeMismatch loc "length result is not an integer"
  else match arguments.toList with
    | [argument] => match exprType? ns argument with
        | some argumentType => if hasLengthType ns argumentType then #[]
            else typeMismatch loc "length operand is not a vector, string, or byte sequence"
        | none => #[]
    | _ => #[]

private def indexPrimitiveDiagnostics (ns : ValidatedNamespace)
    (logical : Bool) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [collection, index] => match exprType? ns collection, exprType? ns index with
      | some collectionType, some indexType =>
          match ns.tables.types[collectionType.index]? with
          | some (.vector element _) =>
              let resultErrors := if typesAgree ns resultType element ||
                  (logical && specProjectedAgree ns resultType element) then #[]
                else typeMismatch loc "index result differs from the vector element type"
              let indexErrors := if isFixedIntegerType ns indexType ||
                  (logical && isLogicalNumType ns indexType) then #[]
                else typeMismatch loc "vector index is not a nonzero fixed-width integer"
              resultErrors ++ indexErrors
          | _ => typeMismatch loc "index operand is not a vector"
      | _, _ => #[]
  | _ => #[]

private def slicePrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [collection, start, stop] =>
      let collectionErrors := match exprType? ns collection with
        | some collectionType =>
            if typesAgree ns collectionType resultType && isVectorType ns collectionType then #[]
            else typeMismatch loc "slice operand and result are not the same vector type"
        | none => #[]
      let isBound (typeId : TypeId) : Bool :=
        isFixedIntegerType ns typeId || (logical && isLogicalNumType ns typeId)
      let boundErrors := match exprType? ns start, exprType? ns stop with
        | some startType, some stopType =>
            if isBound startType && isBound stopType then #[]
            else typeMismatch loc "slice bound is not a nonzero fixed-width integer"
        | _, _ => #[]
      collectionErrors ++ boundErrors
  | _ => #[]

private def shiftPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  if !isFixedIntegerType ns resultType && !(logical && isAnyIntegerType ns resultType) then
    typeMismatch loc "shift result is not a nonzero fixed-width integer"
  else match arguments.toList with
    | [value, distance] =>
        let valueErrors := if exprTypeAgrees ns value resultType ||
            (logical && (exprType? ns value).any (specReadsInteger ns)) then #[]
          else typeMismatch loc "shifted operand type differs from its result type"
        let distanceErrors := match exprType? ns distance with
          | some distanceType => if isFixedIntegerType ns distanceType ||
                (logical && specReadsInteger ns distanceType) then #[]
              else typeMismatch loc "shift distance is not a nonzero fixed-width integer"
          | none => #[]
        valueErrors ++ distanceErrors
    | _ => #[]

private def unaryIdentityPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [argument] =>
      if exprTypeAgrees ns argument resultType ||
          (logical && (exprType? ns argument).any fun argumentType =>
            specProjectedAgree ns resultType argumentType ||
              specProjectedAgree ns argumentType resultType) then #[]
      else typeMismatch loc "value operation changes its operand type"
  | _ => #[]

private def overflowingPrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match ns.tables.types[resultType.index]? with
  | some (.tuple resultElements) => match resultElements.toList with
      | [valueType, overflowType] =>
          let resultErrors :=
            (if isFixedIntegerType ns valueType ||
                (logical && isAnyIntegerType ns valueType) then #[]
             else typeMismatch loc "overflowing arithmetic value is not a nonzero fixed-width integer") ++
            (if isBoolType ns overflowType then #[]
             else typeMismatch loc "overflowing arithmetic flag is not Boolean")
          let operandErrors := arguments.foldl (init := #[]) fun errors argument =>
            if exprType? ns argument == some valueType ||
                (logical && isAnyIntegerType ns valueType &&
                  (exprType? ns argument).any (specReadsInteger ns)) then errors
            else errors ++ typeMismatch loc
              "overflowing arithmetic operand differs from its value result type"
          resultErrors ++ operandErrors
      | _ => typeMismatch loc "overflowing arithmetic result is not a two-element tuple"
  | _ => typeMismatch loc "overflowing arithmetic result is not a tuple"

private def rangePrimitiveDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  let resultErrors := match ns.tables.types[resultType.index]? with
    | some .range => #[]
    | _ => typeMismatch loc "range primitive result is not a range type"
  let operandErrors := match arguments.toList with
    | [lower, upper] =>
        if (match exprType? ns lower, exprType? ns upper with
            | some lowerType, some upperType => typesAgree ns lowerType upperType ||
                (logical && specReadsInteger ns lowerType && specReadsInteger ns upperType)
            | _, _ => false) then #[]
        else typeMismatch loc "range bound types differ"
    | _ => #[]
  resultErrors ++ operandErrors

/-- Type obligations for the closed primitive vocabulary. Arity errors are
mostly emitted separately; tuple arity is determined by its result type. -/
private def primitiveTypeDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (operation : PrimitiveOperation)
    (arguments : Array ExprId) : Array Diagnostic :=
  match operation with
  | .tuple => tuplePrimitiveDiagnostics ns loc logical resultType arguments
  | .vector => vectorPrimitiveDiagnostics ns logical loc resultType arguments
  | .repeatVector => repeatVectorPrimitiveDiagnostics ns loc resultType arguments
  | .pushVector => pushVectorPrimitiveDiagnostics ns loc resultType arguments
  | .concatVector =>
      match ns.tables.types[resultType.index]? with
      | some (.vector _ none) =>
          arguments.foldl (init := #[]) fun errors argument =>
            if exprTypeAgrees ns argument resultType then errors
            else errors ++ typeMismatch loc "concatenated vector type differs from result"
      | _ => typeMismatch loc "concatenation requires a variable-length vector result"
  | .insertVector => insertVectorPrimitiveDiagnostics ns logical loc resultType arguments
  | .removeVector => removeVectorPrimitiveDiagnostics ns logical loc resultType arguments
  | .swapVector | .reverseSliceVector =>
      swapVectorPrimitiveDiagnostics ns logical loc resultType arguments
  | .checkVectorIndex _ =>
      let resultErrors := if isUnitType ns resultType then #[]
        else typeMismatch loc "vector index check must return unit"
      match arguments.toList with
      | [vector, index] =>
          let vectorErrors := match exprType? ns vector >>= (ns.tables.types[·.index]?) with
            | some (.vector ..) => #[]
            | _ => typeMismatch loc "index check requires a vector"
          resultErrors ++ vectorErrors ++ vectorEditIndexDiagnostics ns logical loc index
      | _ => resultErrors
  | .signerAddress =>
      let resultErrors := if ns.tables.types[resultType.index]? == some .address then #[]
        else typeMismatch loc "a signer's address must have type Address"
      match arguments.toList with
      | [signer] => match exprType? ns signer >>= (ns.tables.types[·.index]?) with
          | some (.reference reference) =>
              if ns.tables.types[reference.referent.index]? == some .signer then resultErrors
              else resultErrors ++ typeMismatch loc "a signer's address requires a &Signer"
          | some .signer => if logical then resultErrors
              else resultErrors ++ typeMismatch loc "a signer's address requires a &Signer"
          | _ => resultErrors ++ typeMismatch loc "a signer's address requires a &Signer"
      | _ => resultErrors
  | .destroyEmptyVector =>
      let resultErrors := if isUnitType ns resultType then #[]
        else typeMismatch loc "empty-vector destruction must return unit"
      match arguments.toList with
      | [vector] => match exprType? ns vector >>= (ns.tables.types[·.index]?) with
          | some (.vector ..) => resultErrors
          | _ => resultErrors ++ typeMismatch loc "empty-vector destruction requires a vector"
      | _ => resultErrors
  | .containsVector | .indexOfVector =>
      let resultErrors := if operation == .containsVector then
          if isBoolType ns resultType then #[] else typeMismatch loc "vector membership must return Bool"
        else match ns.tables.types[resultType.index]? with
          | some (.tuple #[found, index]) =>
              if isBoolType ns found &&
                  (isFixedIntegerType ns index || (logical && isLogicalNumType ns index)) then #[]
              else typeMismatch loc "vector search must return (Bool, integer index)"
          | _ => typeMismatch loc "vector search must return (Bool, integer index)"
      match arguments.toList with
      | [vector, needle] => match exprType? ns vector >>= (ns.tables.types[·.index]?) with
          | some (.vector element _) =>
              if exprTypeAgrees ns needle element ||
                  (logical && (exprType? ns needle).any (specProjectedAgree ns element)) then resultErrors
              else resultErrors ++ typeMismatch loc "vector search element type differs"
          | _ => resultErrors ++ typeMismatch loc "vector search requires a vector"
      | _ => resultErrors
  | .length => lengthPrimitiveDiagnostics ns loc resultType arguments
  | .index => indexPrimitiveDiagnostics ns logical loc resultType arguments
  | .slice => slicePrimitiveDiagnostics ns logical loc resultType arguments
  | .add | .checkedAdd _ | .subtract | .checkedSubtract _ | .multiply |
      .checkedMultiply _ | .modulo | .checkedModulo _ | .divide | .checkedDivide _ =>
      fixedIntegerPrimitiveDiagnostics ns logical loc resultType arguments 2
  | .overflowingAdd | .overflowingSubtract | .overflowingMultiply =>
      overflowingPrimitiveDiagnostics ns logical loc resultType arguments
  | .bitwiseOr | .bitwiseAnd | .bitwiseXor =>
      bitwisePrimitiveDiagnostics ns logical loc resultType arguments
  | .bitwiseNot => fixedIntegerPrimitiveDiagnostics ns logical loc resultType arguments 1
  | .shiftLeft | .checkedShiftLeft _ | .shiftRight | .checkedShiftRight _ =>
      shiftPrimitiveDiagnostics ns logical loc resultType arguments
  | .logicalAnd | .logicalOr | .implies | .equivalent =>
      booleanPrimitiveDiagnostics ns loc resultType arguments 2
  | .equal | .notEqual | .identical =>
      equalityPrimitiveDiagnostics ns logical loc resultType arguments
  | .compare => structuralOrderDiagnostics ns logical loc resultType arguments
  | .less | .greater | .lessEqual | .greaterEqual =>
      comparisonPrimitiveDiagnostics ns logical loc resultType arguments
  | .logicalNot => booleanPrimitiveDiagnostics ns loc resultType arguments 1
  | .negate | .checkedNegate _ =>
      fixedIntegerPrimitiveDiagnostics ns logical loc resultType arguments 1
  | .copyValue | .moveValue =>
      unaryIdentityPrimitiveDiagnostics ns logical loc resultType arguments
  | .cast | .checkedCast _ => match arguments.toList with
      | [argument] => match exprType? ns argument with
          | some argumentType =>
              let resultNode := ns.tables.types[resultType.index]?
              let argumentNode := ns.tables.types[argumentType.index]?
              let isCastInteger (typeId : TypeId) : Bool :=
                isFixedIntegerType ns typeId || (logical && isLogicalNumType ns typeId)
              let integerCast := isCastInteger resultType && isCastInteger argumentType
              -- Logical bodies see spec-projected types, where fixed integer
              -- widths (including the `u8` of a character cast) erase to the
              -- unbounded integer domain.
              let characterToInteger := isCastInteger resultType &&
                argumentNode == some .character
              let asciiToCharacter := resultNode == some .character &&
                (argumentNode == some (.integer (.bits 8) false) ||
                  (logical && isLogicalNumType ns argumentType))
              if integerCast || characterToInteger || asciiToCharacter then #[]
              else typeMismatch loc
                "cast types are not fixed integers, character-to-integer, or u8-to-character"
          | none => #[]
      | _ => #[]
  | .range => rangePrimitiveDiagnostics ns logical loc resultType arguments

private def referencedNameForValidation? (ns : ValidatedNamespace)
    (reference : QualifiedRef) : Option (NamespaceRef × String) := do
  let name ← ns.tables.names[reference.name.index]?
  if name.namespaceId != reference.namespaceId then none else
    resolvedName? ns reference.name

private def resolveFunctionDeclaration? (unit : ValidatedUnit) (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × FunctionDecl FunctionBody) := do
  let _ ← referencedNameForValidation? source reference
  let declarationId ← unit.resolution.function? reference.name
  let targetNs ← ownedNamespaceForName? unit reference.name
  let declaration ← targetNs.functions[declarationId.index]?
  some (targetNs, declaration)

private def resolveConstantDeclaration? (unit : ValidatedUnit) (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × ConstantDecl) := do
  let _ ← referencedNameForValidation? source reference
  let declarationId ← unit.resolution.constant? reference.name
  let targetNs ← ownedNamespaceForName? unit reference.name
  let declaration ← targetNs.constants[declarationId.index]?
  some (targetNs, declaration)

private def resolveSpecFunctionDeclaration? (unit : ValidatedUnit)
    (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × SpecFunctionDecl) := do
  let _ ← referencedNameForValidation? source reference
  let declarationId ← unit.resolution.specFunction? reference.name
  let targetNs ← ownedNamespaceForName? unit reference.name
  let declaration ← targetNs.specFunctions[declarationId.index]?
  some (targetNs, declaration)

private def resolveLemmaDeclaration? (unit : ValidatedUnit) (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × LemmaDecl) := do
  let _ ← referencedNameForValidation? source reference
  let index ← unit.resolution.lemma? reference.name
  let targetNs ← ownedNamespaceForName? unit reference.name
  let declaration ← targetNs.lemmas[index]?
  some (targetNs, declaration)

private def resolveTraitDeclaration? (unit : ValidatedUnit) (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × TraitDecl) := do
  let _ ← referencedNameForValidation? source reference
  let declarationId ← unit.resolution.trait? reference.name
  let targetNs ← ownedNamespaceForName? unit reference.name
  let declaration ← targetNs.traits[declarationId.index]?
  some (targetNs, declaration)

private def associatedPredicateDiagnostics (unit : ValidatedUnit)
    (source : ValidatedNamespace) (loc : LocId) (trait : TraitRef)
    (itemId : AssociatedItemId) (expectsType : Bool) : Array Diagnostic :=
  match resolveTraitDeclaration? unit source trait.trait with
  | none => #[]
  | some (targetNs, declaration) =>
      match targetNs.associatedItems[itemId.index]? with
      | none => #[.at "LIR-SEMANTIC-ASSOCIATED-ITEM"
          "associated equality names an item missing from the target namespace" loc]
      | some item =>
          if item.owner != declaration.id || !declaration.associatedItems.contains itemId then
            #[.at "LIR-SEMANTIC-ASSOCIATED-ITEM"
              "associated equality item does not belong to the referenced trait" loc]
          else
            let kindMatches := match expectsType, item.kind with
              | true, .type .. | false, .constant .. => true
              | true, .constant .. | true, .method .. |
                  false, .type .. | false, .method .. => false
            if kindMatches then #[] else #[.at "LIR-SEMANTIC-ASSOCIATED-ITEM"
              (if expectsType then
                "associated type equality does not name an associated type"
              else "associated constant equality does not name an associated constant") loc]


/-- Compare a source expression type with a declaration type after replacing
the declaration's type/lifetime parameters by one checked call-site
instantiation. All slots are pre-bound, so the unification engine acts as a
pure instantiation checker here. -/
private def typeMatchesSolution? (sourceNs : ValidatedNamespace) (source : TypeId)
    (targetNs : ValidatedNamespace) (target : TypeId) (solution : Unify.Solution) : Bool :=
  (Unify.matchType sourceNs source targetNs target ⟨0⟩ solution).isSome

private def typeMatchesInstantiation? (sourceNs : ValidatedNamespace) (source : TypeId)
    (targetNs : ValidatedNamespace) (target : TypeId)
    (instantiations : Array GenericArgument) : Bool :=
  typeMatchesSolution? sourceNs source targetNs target (Unify.Solution.bound instantiations)

/-- Whether an operation only observes its first operand: it inspects a tag or
length without exporting or copying an element of a potentially linear
aggregate. -/
def observesOperand (operation : Operation) : Bool :=
  operation matches .data (.discriminant _) | .data (.testVariants ..) | .primitive .length |
    .primitive (.checkVectorIndex _)

private def primitiveHasAbility (profile : Profile) (ty : Ty) (ability : Ability) : Bool :=
  let ordinaryScalar := match ty with
    | .unit | .never | .bool | .character | .string | .bytes | .address |
        .integer _ _ => true
    | _ => false
  match profile, ability with
  | .move, .copy | .move, .store => ordinaryScalar
  | .move, .drop => ordinaryScalar || (ty matches .signer)
  | .rust, .copy | .rust, .drop => ordinaryScalar || (ty matches .signer)
  | .extension _, .copy | .extension _, .drop => ordinaryScalar
  | _, _ => false

/-- Decide the first-order ability rules shared by semantic preparation. Move
uses declared abilities and structural propagation; Rust treats `Drop` as the
default and `Copy` as explicit for nominal/function types. Trait predicates
outside these four core abilities remain an evidence-checking milestone. -/
private partial def typeHasAbilityFuel : Nat → ValidatedUnit → ValidatedNamespace →
    Profile → Array GenericBinder → TypeId → Ability → Option Bool
  | 0, _, _, _, _, _, _ => none
  | fuel + 1, unit, ns, profile, generics, typeId, ability => do
      let ty ← ns.tables.types[typeId.index]?
      match ty with
      | .tuple elements =>
          let checks ← elements.mapM fun element =>
            typeHasAbilityFuel fuel unit ns profile generics element ability
          some (checks.all id)
      | .vector element _ =>
          typeHasAbilityFuel fuel unit ns profile generics element ability
      | .nominal _ arguments => do
          let (targetNs, declaration, _) ← nominalDeclarationForType? unit ns typeId
          let ownerProfile := targetNs.profile.getD profile
          let declared := if ownerProfile == .rust && ability == .drop then true
            else declaration.abilities.contains ability
          if !declared || declaration.generics.size != arguments.size then some false else
          let binderChecks ← (declaration.generics.zip arguments).mapM fun pair =>
            if pair.1.abilities.isEmpty then some true else match pair.2 with
            | .typeArg argument => do
                let abilities ← pair.1.abilities.mapM fun required =>
                  typeHasAbilityFuel fuel unit ns ownerProfile generics argument.typeId required
                some (abilities.all id)
            | _ => some false
          some (binderChecks.all id)
      | .function _ _ abilities =>
          some <| if profile == .rust && ability == .drop then true
            else abilities.contains ability
      | .reference reference => match profile, ability with
          | .move, .copy | .move, .drop => some true
          | .rust, .drop => some true
          | .rust, .copy => some (reference.kind == .shared)
          | _, _ => some false
      | .typeParameter index =>
          some <| (profile == .rust && ability == .drop) ||
            (generics[index]?).any (·.abilities.contains ability)
      | .profile _ => none
      | _ => some (primitiveHasAbility profile ty ability)

/-- Whether a type has an ability, under the abilities of the enclosing
declaration's type parameters; `none` when it cannot be decided. -/
def typeHasAbility? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (profile : Profile) (generics : Array GenericBinder) (typeId : TypeId)
    (ability : Ability) : Option Bool :=
  typeHasAbilityFuel (ns.tables.types.size + unit.namespaces.size + 1)
    unit ns profile generics typeId ability

/-- The type parameters as specification content sees them: it quantifies
over unconstrained type parameters, so a parameter discharges every ability
obligation there. -/
private def specificationGenerics (generics : Array GenericBinder) : Array GenericBinder :=
  generics.map fun binder => { binder with abilities := #[.copy, .drop, .store, .key] }

/-- Whether a resource type has `key` where a global operation names it, with
the type parameters of specification content unconstrained. -/
private def globalResourceHasKey (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (generics : Array GenericBinder) (logical : Bool) (resource : TypeId) : Bool :=
  let generics := if logical then specificationGenerics generics else generics
  typeHasAbility? unit ns .move generics resource .key == some true

private def contextTypeHasAbility? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (typeId : TypeId) (ability : Ability) : Bool :=
  typeHasAbility? unit ns context.profile context.generics typeId ability == some true

private def genericAbilityDiagnostics (unit : ValidatedUnit) (sourceNs : ValidatedNamespace)
    (profile : Profile) (sourceGenerics : Array GenericBinder) (description : String)
    (binders : Array GenericBinder) (arguments : Array GenericArgument)
    (logical : Bool := false) : Array Diagnostic :=
  -- Ability constraints are executable obligations. Specification content
  -- quantifies over unconstrained type parameters — a Move spec function
  -- never repeats its subject's ability bounds — so logical occurrences
  -- carry no ability obligations.
  if logical then #[] else
  (binders.zip arguments).zipIdx.foldl (init := #[]) fun ds (pair, index) =>
    match pair.1.kind, pair.2 with
    | .typeArg, .typeArg argument => pair.1.abilities.foldl (fun ds ability =>
        -- `none` marks an unresolvable or profile-unknown obligation; it is a
        -- deferred bound, not a definite violation.
        if typeHasAbility? unit sourceNs profile sourceGenerics argument.typeId ability != some false
          then ds
        else ds.push <| .at "LIR-SEMANTIC-ABILITY"
          s!"{description} type argument {index} does not satisfy {repr ability}" argument.loc) ds
    | _, _ => ds

private def fieldRequirementForDeclaredAbility (profile : Profile)
    (ability : Ability) : Ability :=
  match profile, ability with
  | .move, .key => .store
  | _, ability => ability

private def structAbilityDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : StructDecl) : Array Diagnostic :=
  let profile := ns.profile.getD .move
  let fields := declaration.fields ++ declaration.variants.flatMap (·.fields)
  declaration.abilities.foldl (fun ds declaredAbility =>
    let requiredAbility := fieldRequirementForDeclaredAbility profile declaredAbility
    fields.foldl (fun ds field =>
      -- A declared Move ability of a generic declaration is conditional:
      -- fields that depend on a type parameter transfer the obligation to
      -- each instantiation site instead of constraining the declaration.
      -- Rust declarations require the explicit binder bound instead.
      if ((profile matches .move) &&
            typeNeedsSubstitution ns.tables (ns.tables.types.size + 1) field.type.typeId) ||
          typeHasAbility? unit ns profile declaration.generics field.type.typeId requiredAbility !=
            some false then ds
      else ds.push <| .at "LIR-SEMANTIC-ABILITY"
        s!"field type does not satisfy {repr requiredAbility} required by the declaration's {repr declaredAbility} ability"
        field.loc) ds) #[]

/-- Whether a source type matches a declared target under one instantiation,
optionally through the specification projection: references erase on either
side and logical integers occupy the unbounded integer domain. -/
private def specMatchesSolution (sourceNs : ValidatedNamespace) (logical : Bool)
    (sourceType : TypeId) (targetNs : ValidatedNamespace) (targetType : TypeId)
    (solution : Unify.Solution) : Bool :=
  (Unify.specMatch sourceNs logical sourceType targetNs targetType ⟨0⟩ solution).isSome

private def specMatchesInstantiation (sourceNs : ValidatedNamespace) (logical : Bool)
    (sourceType : TypeId) (targetNs : ValidatedNamespace) (targetType : TypeId)
    (instantiations : Array GenericArgument) : Bool :=
  specMatchesSolution sourceNs logical sourceType targetNs targetType
    (Unify.Solution.bound instantiations)

private def argumentTypesMatchParameters (sourceNs targetNs : ValidatedNamespace)
    (logical : Bool)
    (instantiations : Array GenericArgument) (arguments : Array ExprId)
    (parameters : Array Parameter) : Bool :=
  arguments.size == parameters.size && (arguments.zip parameters).all fun pair =>
    match exprType? sourceNs pair.1 with
    | some argumentType =>
        specMatchesInstantiation sourceNs logical argumentType targetNs
          pair.2.typeUse.typeId instantiations
    | none => false

private def packedResultsMatch (sourceNs : ValidatedNamespace) (logical : Bool)
    (resultType : TypeId)
    (targetNs : ValidatedNamespace) (instantiations : Array GenericArgument)
    (results : Array TypeUse) : Bool :=
  match results.toList with
  | [] => isUnitType sourceNs resultType
  | [result] =>
      specMatchesInstantiation sourceNs logical resultType targetNs result.typeId instantiations
  | _ => match sourceNs.tables.types[resultType.index]? with
      | some (.tuple elements) =>
          elements.size == results.size && (elements.zip results).all fun pair =>
            specMatchesInstantiation sourceNs logical pair.1 targetNs pair.2.typeId
              instantiations
      | _ => false

/-- Compare function-boundary types while allowing distinct lifetime identities
only through structural constructors whose variance constraints are retained by
the borrow certificate. Nominal types remain exact until their authoritative
frontend variance is represented. -/
private partial def boundaryTypesMatchFuel (fuel : Nat) (ns : ValidatedNamespace)
    (valueType resultType : TypeId) : Bool :=
  if valueType == resultType then true else
    match fuel, ns.tables.types[valueType.index]?, ns.tables.types[resultType.index]? with
    | 0, _, _ | _, none, _ | _, _, none => false
    | fuel + 1, some (.tuple values), some (.tuple results) =>
        values.size == results.size && (values.zip results).all fun pair =>
          boundaryTypesMatchFuel fuel ns pair.1 pair.2
    | fuel + 1, some (.vector valueElement valueLength),
        some (.vector resultElement resultLength) =>
        valueLength == resultLength &&
          boundaryTypesMatchFuel fuel ns valueElement resultElement
    | fuel + 1, some (.reference value), some (.reference result) =>
        value.profile == result.profile && value.kind == result.kind &&
          lifetimeKindsEquivalent? ns value.lifetime ns result.lifetime &&
          boundaryTypesMatchFuel fuel ns value.referent result.referent
    | fuel + 1, some (.function valueArguments valueResult valueAbilities),
        some (.function resultArguments resultResult resultAbilities) =>
        valueAbilities == resultAbilities &&
          valueArguments.size == resultArguments.size &&
          (valueArguments.zip resultArguments).all (fun pair =>
            boundaryTypesMatchFuel fuel ns pair.1 pair.2) &&
          boundaryTypesMatchFuel fuel ns valueResult resultResult
    | _, _, _ => false

private def boundaryTypesMatch (ns : ValidatedNamespace)
    (valueType resultType : TypeId) : Bool :=
  -- Structural equivalence already treats lifetimes by kind; generic nominal
  -- types intern per located argument use, so identity alone is not enough.
  typesAgree ns valueType resultType ||
    boundaryTypesMatchFuel (ns.tables.types.size * 2 + 1) ns valueType resultType

private def packedDeclaredResultsMatch (ns : ValidatedNamespace) (resultType : TypeId)
    (results : Array TypeUse) : Bool :=
  match results.toList with
  | [] => isUnitType ns resultType
  | [result] => boundaryTypesMatch ns resultType result.typeId
  | _ => match ns.tables.types[resultType.index]? with
      | some (.tuple elements) =>
          elements.size == results.size && (elements.zip results).all fun pair =>
            boundaryTypesMatch ns pair.1 pair.2.typeId
      | _ => false

/-- Ground occurrence pairs `(call-site type, declaration type)` of one call:
value arguments against declared parameters, then the packed call result
against declared results. `none` when the call shape yields no pairs, which
inference reports as a signature mismatch. -/
private def callOccurrencePairs? (ns : ValidatedNamespace)
    (arguments : Array ExprId) (parameters : Array Parameter)
    (resultType : TypeId) (results : Array TypeUse) :
    Option (Array (TypeId × TypeId)) := do
  let argumentPairs ← (arguments.zip parameters).mapM fun pair => do
    let argumentType ← exprType? ns pair.1
    some (argumentType, pair.2.typeUse.typeId)
  let resultPairs ← match results.toList with
    | [] => if isUnitType ns resultType then some #[] else none
    | [result] => some #[(resultType, result.typeId)]
    | _ => match ns.tables.types[resultType.index]? with
        | some (.tuple elements) =>
            if elements.size == results.size then
              some (elements.zip (results.map (·.typeId)))
            else none
        | _ => none
  some (argumentPairs ++ resultPairs)

/-- Solve an elided generic instantiation from the call's occurrence pairs
and discharge the solved binder abilities. Explicit instantiations never take
this path, so the interpreter-facing unit is unchanged; global operations
still require their explicit resource argument. -/
private def inferredInstantiationDiagnostics (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext) (loc : LocId)
    (description : String) (targetNs : ValidatedNamespace) (profile : Profile)
    (binders : Array GenericBinder)
    (pairs? : Option (Array (TypeId × TypeId))) : Array Diagnostic :=
  match pairs? with
  | none => typeMismatch loc
      s!"no {description} matches the call's argument and result types"
  | some pairs => match Unify.solveCall ns context.logical targetNs binders loc pairs with
      | .solved inferred =>
          genericAbilityDiagnostics unit ns profile context.generics description binders
            inferred (logical := context.logical)
      | .mismatch => typeMismatch loc
          s!"no {description} matches the call's argument and result types"
      | .undetermined => #[.at "LIR-TYPE-UNDETERMINED"
          s!"the call site does not determine every generic parameter of the elided {description}"
          loc]

private def directCallTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match resolveFunctionDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "direct call target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let profileErrors := if context.profile == declaration.profile then #[] else
        #[.at "LIR-PROFILE-MISMATCH"
          "direct call crosses profiles without a validated boundary adapter" loc]
      if instantiations.isEmpty && !declaration.signature.generics.isEmpty then
        if arguments.size != declaration.signature.parameters.size then
          profileErrors ++ arityMismatch loc "function call"
            declaration.signature.parameters.size arguments.size
        else
          profileErrors ++ inferredInstantiationDiagnostics unit ns context loc
            "function instantiation" targetNs declaration.profile
            declaration.signature.generics
            (callOccurrencePairs? ns arguments declaration.signature.parameters resultType
              declaration.signature.results)
      else
      let genericErrors := genericInstantiationDiagnostics loc "function instantiation"
        declaration.signature.generics instantiations ++
        genericAbilityDiagnostics unit ns declaration.profile context.generics "function instantiation"
          declaration.signature.generics instantiations (logical := context.logical)
      let argumentErrors := if arguments.size != declaration.signature.parameters.size then
          arityMismatch loc "function call" declaration.signature.parameters.size arguments.size
        else if !argumentTypesMatchParameters ns targetNs context.logical instantiations arguments
            declaration.signature.parameters then
          typeMismatch loc "function argument types differ from the callee signature"
        else #[]
      let resultErrors := if !packedResultsMatch ns context.logical resultType targetNs instantiations
          declaration.signature.results then
        typeMismatch loc "function call result differs from the callee signature"
      else #[]
      profileErrors ++ genericErrors ++ argumentErrors ++ resultErrors

private def specFunctionCallTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext) (loc : LocId)
    (resultType : TypeId) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match resolveSpecFunctionDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET"
        "specification function call target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let profileErrors := if context.profile == declaration.profile then #[] else
        #[.at "LIR-PROFILE-MISMATCH"
          "specification function call crosses profiles without a validated boundary adapter" loc]
      if instantiations.isEmpty && !declaration.signature.generics.isEmpty then
        if arguments.size != declaration.signature.parameters.size then
          profileErrors ++ arityMismatch loc "specification function call"
            declaration.signature.parameters.size arguments.size
        else
          profileErrors ++ inferredInstantiationDiagnostics unit ns context loc
            "specification function instantiation" targetNs declaration.profile
            declaration.signature.generics
            (callOccurrencePairs? ns arguments declaration.signature.parameters resultType
              declaration.signature.results)
      else
      let genericErrors := genericInstantiationDiagnostics loc
          "specification function instantiation" declaration.signature.generics instantiations ++
        genericAbilityDiagnostics unit ns declaration.profile context.generics
          "specification function instantiation" declaration.signature.generics instantiations
          (logical := context.logical)
      let argumentErrors := if arguments.size != declaration.signature.parameters.size then
          arityMismatch loc "specification function call"
            declaration.signature.parameters.size arguments.size
        else if !argumentTypesMatchParameters ns targetNs context.logical instantiations arguments
            declaration.signature.parameters then
          typeMismatch loc
            "specification function argument types differ from the callee signature"
        else #[]
      let resultErrors := if packedResultsMatch ns context.logical resultType targetNs instantiations
          declaration.signature.results then #[] else
        typeMismatch loc "specification function result differs from the callee signature"
      profileErrors ++ genericErrors ++ argumentErrors ++ resultErrors

/-- A lemma instance: a proposition over the lemma's parameters, at the
lemma's instantiation of its type parameters. -/
private def lemmaInstanceTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext) (loc : LocId)
    (resultType : TypeId) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match resolveLemmaDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "lemma application target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let profileErrors := if context.profile == declaration.profile then #[] else
        #[.at "LIR-PROFILE-MISMATCH"
          "lemma application crosses profiles without a validated boundary adapter" loc]
      let genericErrors := genericInstantiationDiagnostics loc
          "lemma instantiation" declaration.signature.generics instantiations ++
        genericAbilityDiagnostics unit ns declaration.profile context.generics
          "lemma instantiation" declaration.signature.generics instantiations
          (logical := context.logical)
      let argumentErrors := if arguments.size != declaration.signature.parameters.size then
          arityMismatch loc "lemma application" declaration.signature.parameters.size
            arguments.size
        else if !argumentTypesMatchParameters ns targetNs context.logical instantiations arguments
            declaration.signature.parameters then
          typeMismatch loc "lemma argument types differ from the lemma's parameters"
        else #[]
      let resultErrors := if isBoolType ns resultType then #[] else
        typeMismatch loc "a lemma instance is not Bool"
      profileErrors ++ genericErrors ++ argumentErrors ++ resultErrors

/-- Whether an expression holds a lemma instance. -/
private def containsLemmaInstance (ns : ValidatedNamespace) : Nat → ExprId → Bool
  | 0, _ => false
  | fuel + 1, id => match ns.expressions[id.index]? with
    | none => false
    | some expression => match expression.kind with
      | .operation (.specification (.lemma _ _)) _ _ _ => true
      | kind => (expressionChildren kind).any (containsLemmaInstance ns fuel)

/-- The shape of an `apply` step: a lemma instance under implications and
universal quantifiers, with no lemma instance in a premise, a quantifier's
domain or condition, or an instance's arguments. -/
private def applyFormulaDiagnostics (ns : ValidatedNamespace) (loc : LocId) :
    Nat → ExprId → Array Diagnostic
  | 0, _ => #[]
  | fuel + 1, id =>
    let free (ids : Array ExprId) : Array Diagnostic :=
      if ids.any (containsLemmaInstance ns ns.expressions.size) then
        #[.at "LIR-SEMANTIC-CONDITION"
          "a lemma instance occurs in a premise, a domain, or an argument of an `apply` step" loc]
      else #[]
    match ns.expressions[id.index]? with
    | none => #[]
    | some expression => match expression.kind with
      | .operation (.specification (.lemma _ _)) _ arguments _ => free arguments
      | .operation (.primitive .implies) _ #[premise, conclusion] _ =>
          free #[premise] ++ applyFormulaDiagnostics ns loc fuel conclusion
      | .quantifier .forall binders _ condition body =>
          free (binders.map (·.domain) ++ condition.toArray) ++
            applyFormulaDiagnostics ns loc fuel body
      | _ => #[.at "LIR-SEMANTIC-CONDITION"
          "an `apply` step is a lemma instance under implications and universal quantifiers" loc]

/-- Whether a value of a type holds a reference. A function value holds
none, since its captures cannot; a Move nominal type or type argument is
never one. -/
private def valueHoldsReference (ns : ValidatedNamespace) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, typeId =>
      match ns.tables.types[typeId.index]? with
      | none => true
      | some ty => match ty with
        | .reference _ | .profile _ => true
        | .tuple elements => elements.any (valueHoldsReference ns fuel)
        | .vector element _ => valueHoldsReference ns fuel element
        | .nominal .. | .typeParameter _ => ns.profile != some .move
        | .function .. => false
        | .unit | .never | .bool | .character | .string | .bytes | .address | .signer
        | .integer .. | .range | .eventStore | .typeDomain _ | .resourceDomain .. | .stateDomain =>
            false

/-- The abilities a closure type may claim: those every capture has. A
closure without captures may claim any. -/
private def closureAbilityDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (loc : LocId) (claimed : Array Ability) (captures : Array ExprId) :
    Array Diagnostic :=
  claimed.foldl (init := #[]) fun diagnostics ability =>
    let lacking := captures.any fun capture =>
      !(exprType? ns capture).any (contextTypeHasAbility? unit ns context · ability)
    if lacking then diagnostics.push (.at "LIR-CLOSURE-ABILITY"
      s!"closure type claims {repr ability}, which a capture lacks" loc)
    else diagnostics

private def closureTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (reference : QualifiedRef) (mask : Nat)
    (instantiations : Array GenericArgument) (captures : Array ExprId) : Array Diagnostic :=
  match resolveFunctionDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "closure target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let profileErrors := if context.profile == declaration.profile then #[] else
        #[.at "LIR-PROFILE-MISMATCH"
          "closure target crosses profiles without a validated boundary adapter" loc]
      let parameters := declaration.signature.parameters.toList
      let capturedParameters := (ClosureMask.extract mask true parameters).toArray
      let remaining := (ClosureMask.extract mask false parameters).toArray
      let maskErrors := if mask < 2 ^ parameters.length then #[] else
        #[.at "LIR-CLOSURE-MASK" "closure mask captures parameters the target lacks" loc]
      let captureErrors := if captures.size == capturedParameters.size then
          if captures.any fun capture =>
              (exprType? ns capture).all (valueHoldsReference ns (ns.tables.types.size + 1))
          then #[.at "LIR-CLOSURE-CAPTURE" "a closure cannot capture a reference" loc] else #[]
        else arityMismatch loc "closure captures" capturedParameters.size captures.size
      let shapeErrors := profileErrors ++ maskErrors ++ captureErrors
      if !shapeErrors.isEmpty then shapeErrors else
      match ns.tables.types[resultType.index]? with
      | some (.function functionArguments functionResult abilities) =>
          let abilityErrors := closureAbilityDiagnostics unit ns context loc abilities captures
          if instantiations.isEmpty && !declaration.signature.generics.isEmpty then
            if functionArguments.size != remaining.size then
              abilityErrors ++ typeMismatch loc
                "closure type differs from the uncaptured target signature"
            else
              let pairs? := (callOccurrencePairs? ns captures capturedParameters
                  functionResult declaration.signature.results).map fun pairs =>
                pairs ++ functionArguments.zip (remaining.map (·.typeUse.typeId))
              abilityErrors ++ inferredInstantiationDiagnostics unit ns context loc
                "closure instantiation" targetNs declaration.profile
                declaration.signature.generics pairs?
          else
            let genericErrors := genericInstantiationDiagnostics loc "closure instantiation"
              declaration.signature.generics instantiations ++
              genericAbilityDiagnostics unit ns declaration.profile context.generics
                "closure instantiation" declaration.signature.generics instantiations
                (logical := context.logical)
            let captureTypeErrors := if !argumentTypesMatchParameters ns targetNs context.logical
                instantiations captures capturedParameters then
              typeMismatch loc "closure capture types differ from the target parameters"
            else #[]
            let argumentsMatch := functionArguments.size == remaining.size &&
              (functionArguments.zip remaining).all fun pair =>
                specMatchesInstantiation ns context.logical pair.1 targetNs
                  pair.2.typeUse.typeId instantiations
            let callableErrors := if !argumentsMatch ||
                !packedResultsMatch ns context.logical functionResult targetNs instantiations
                  declaration.signature.results then
              typeMismatch loc "closure type differs from the uncaptured target signature"
            else #[]
            abilityErrors ++ genericErrors ++ captureTypeErrors ++ callableErrors
      | _ => typeMismatch loc "closure construction result is not a function type"

/-- A function value keeps the signature its construction gave it, so an
invocation in the logical domain reads its arguments and result through the
specification projection, exactly as a call to a declared function does. -/
private def invokeTypeDiagnostics (ns : ValidatedNamespace) (logical : Bool) (loc : LocId)
    (resultType : TypeId) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [] => #[]
  | callable :: values => match exprType? ns callable with
      | some callableType => match ns.tables.types[callableType.index]? with
          | some (.function expected result _) =>
              let valueArray := values.toArray
              let agrees := fun (source target : TypeId) =>
                if logical then specProjectedAgree ns source target
                else typesAgree ns source target
              let arityErrors := exactArity loc "closure invocation" expected.size valueArray.size
              let argumentErrors := if valueArray.size == expected.size &&
                  !(valueArray.zip expected).all (fun pair =>
                    (exprType? ns pair.1).any (agrees · pair.2)) then
                typeMismatch loc "closure invocation argument types differ from its function type"
              else #[]
              let resultErrors := if agrees resultType result then #[]
                else typeMismatch loc "closure invocation result differs from its function type"
              arityErrors ++ argumentErrors ++ resultErrors
          | _ => typeMismatch loc "invoked operand is not a function type"
      | none => #[]

private def callTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (kind : CallKind)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match kind with
  | .function reference =>
      directCallTypeDiagnostics mode unit ns context loc resultType reference instantiations
        arguments
  | .closure reference mask =>
      closureTypeDiagnostics mode unit ns context loc resultType reference mask instantiations
        arguments
  | .invoke => if mode matches .execution then #[] else
      invokeTypeDiagnostics ns context.logical loc resultType arguments
  | .constructor _ _ | .destructor _ _ | .extension _ _ => #[]

private def resolveStructDeclaration? (unit : ValidatedUnit) (source : ValidatedNamespace)
    (reference : QualifiedRef) : Option (ValidatedNamespace × StructDecl) := do
  let _ ← referencedNameForValidation? source reference
  let owned : Option (ValidatedNamespace × StructDecl) := do
    let declarationId ← unit.resolution.nominal? reference.name
    let targetNs ← ownedNamespaceForName? unit reference.name
    let declaration ← targetNs.structs[declarationId.index]?
    some (targetNs, declaration)
  owned <|> (interfaceStructForName? unit reference.name).map fun declaration =>
    (source, declaration)

private def selectedConstructorFields? (targetNs : ValidatedNamespace)
    (declaration : StructDecl) (variant : Option String) : Option (Array FieldDecl) :=
  match variant with
  | none => if declaration.variants.isEmpty then some declaration.fields else none
  | some variantName => do
      let selected ← declaration.variants.find? fun candidate =>
        (targetNs.tables.names[candidate.name.index]?).any (·.name == variantName)
      some selected.fields

private def nominalTypeMatchesInstantiation (ns : ValidatedNamespace) (typeId : TypeId)
    (reference : QualifiedRef) (instantiations : Array GenericArgument) : Bool :=
  match ns.tables.types[typeId.index]? with
  | some (.nominal name arguments) =>
      resolvedName? ns name == referencedNameForValidation? ns reference &&
        genericArgumentsEquivalent? ns arguments ns instantiations
  | _ => false

private def fieldsMatchArguments (sourceNs targetNs : ValidatedNamespace)
    (logical : Bool)
    (instantiations : Array GenericArgument) (arguments : Array ExprId)
    (fields : Array FieldDecl) : Bool :=
  arguments.size == fields.size && (arguments.zip fields).all fun pair =>
    match exprType? sourceNs pair.1 with
    | some argumentType =>
        specMatchesInstantiation sourceNs logical argumentType targetNs
          pair.2.type.typeId instantiations
    | none => false

private def constructorTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (reference : QualifiedRef)
    (variant : Option String) (instantiations : Array GenericArgument)
    (arguments : Array ExprId) : Array Diagnostic :=
  match resolveStructDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "constructor target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let genericErrors := genericInstantiationDiagnostics loc "constructor instantiation"
        declaration.generics instantiations ++
        genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) context.generics
          "constructor instantiation" declaration.generics instantiations
          (logical := context.logical)
      let resultErrors := if nominalTypeMatchesInstantiation ns resultType reference instantiations
          then #[]
        else typeMismatch loc "constructor result is not its nominal target type"
      match selectedConstructorFields? targetNs declaration variant with
      | none => genericErrors ++ resultErrors ++ #[.at "LIR-SEMANTIC-TARGET"
          "constructor variant does not resolve" loc]
      | some fields =>
          let arityErrors := exactArity loc "constructor" fields.size arguments.size
          let fieldErrors := if arguments.size == fields.size &&
              !fieldsMatchArguments ns targetNs context.logical instantiations arguments fields then
            typeMismatch loc "constructor argument types differ from its fields"
          else #[]
          genericErrors ++ resultErrors ++ arityErrors ++ fieldErrors

private def resourceDomainTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (generics : Array GenericBinder) (loc : LocId)
    (name : NameId) (arguments : Option (Array TypeId)) : Array Diagnostic :=
  match ns.tables.names[name.index]? with
  | none => #[.at "LIR-SEMANTICS-INTERNAL"
      "validated resource-domain name is missing" loc]
  | some qualified =>
      let reference : QualifiedRef := {
        namespaceId := qualified.namespaceId
        name }
      match resolveStructDeclaration? unit ns reference with
      | none => if mode matches .typing then #[] else
          #[.at "LIR-SEMANTIC-TARGET"
            "resource-domain target does not resolve to a nominal declaration" loc]
      | some (targetNs, declaration) =>
          if mode matches .execution then #[] else
          let instantiations := (arguments.getD #[]).map fun typeId =>
            GenericArgument.typeArg { typeId, loc }
          genericInstantiationDiagnostics loc "resource domain"
              declaration.generics instantiations ++
            genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) generics
              "resource domain" declaration.generics instantiations (logical := true)

private def constructorPatternTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext) (pattern : Pattern)
    (name : NameId) (instantiations : Array GenericArgument)
    (variant : Option String) (children : Array PatternId) : Array Diagnostic :=
  match ns.tables.names[name.index]? with
  | none => #[.at "LIR-SEMANTICS-INTERNAL"
      s!"validated constructor-pattern name {name.index} is missing" pattern.loc]
  | some qualifiedName =>
      let reference : QualifiedRef := {
        namespaceId := qualifiedName.namespaceId
        name }
      match resolveStructDeclaration? unit ns reference with
      | none => if mode matches .typing then #[] else
          #[.at "LIR-SEMANTIC-TARGET"
            "constructor pattern target does not resolve" pattern.loc]
      | some (targetNs, declaration) =>
          if mode matches .execution then #[] else
          -- Like `select`, a constructor pattern may match through a reference
          -- to its nominal; the children then bind field projections of the
          -- same reference.
          let sourceReference : Option ReferenceType :=
            match ns.tables.types[pattern.typeId.index]? with
            | some (.reference reference) => some reference
            | _ => none
          let genericErrors :=
            genericInstantiationDiagnostics pattern.loc "constructor pattern instantiation"
              declaration.generics instantiations ++
            genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) context.generics
              "constructor pattern instantiation" declaration.generics instantiations
              (logical := context.logical)
          let annotated := match sourceReference with
            | none => pattern.typeId
            | some reference => reference.referent
          let resultErrors :=
            if nominalTypeMatchesInstantiation ns annotated reference instantiations then #[]
            else typeMismatch pattern.loc
              "constructor pattern is not annotated with its nominal target type"
          match selectedConstructorFields? targetNs declaration variant with
          | none => genericErrors ++ resultErrors ++ #[.at "LIR-SEMANTIC-TARGET"
              "constructor pattern variant does not resolve" pattern.loc]
          | some fields =>
              let arityErrors := exactArity pattern.loc "constructor pattern"
                fields.size children.size
              let fieldErrors := if children.size != fields.size then #[] else
                (children.zip fields).foldl (fun ds pair =>
                  match patternType? ns pair.1 with
                  | some childType =>
                      let childMatches := match sourceReference with
                        | none => specMatchesInstantiation ns context.logical childType targetNs
                            pair.2.type.typeId instantiations
                        | some reference => match ns.tables.types[childType.index]? with
                            | some (.reference childReference) =>
                                childReference.profile == reference.profile &&
                                  childReference.kind == reference.kind &&
                                  typeMatchesInstantiation? ns childReference.referent targetNs
                                    pair.2.type.typeId instantiations
                            | _ => false
                      if childMatches then ds
                      else ds ++ typeMismatch pattern.loc
                        "constructor pattern child types differ from its fields"
                  | none => ds) #[]
              genericErrors ++ resultErrors ++ arityErrors ++ fieldErrors

private def destructorTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (reference : QualifiedRef)
    (variant : Option String) (instantiations : Array GenericArgument)
    (arguments : Array ExprId) : Array Diagnostic :=
  match resolveStructDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "destructor target does not resolve" loc]
  | some (targetNs, declaration) =>
      if mode matches .execution then #[] else
      let genericErrors := genericInstantiationDiagnostics loc "destructor instantiation"
        declaration.generics instantiations ++
        genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) context.generics
          "destructor instantiation" declaration.generics instantiations
          (logical := context.logical)
      let operandErrors := match arguments.toList with
        | [argument] => match exprType? ns argument with
            | some argumentType => if nominalTypeMatchesInstantiation ns argumentType reference
                instantiations then #[]
                else typeMismatch loc "destructor operand is not its nominal target type"
            | none => #[]
        | _ => #[]
      match selectedConstructorFields? targetNs declaration variant with
      | none => genericErrors ++ operandErrors ++ #[.at "LIR-SEMANTIC-TARGET"
          "destructor variant does not resolve" loc]
      | some fields =>
          let resultErrors := if !packedResultsMatch ns context.logical resultType targetNs instantiations
              (fields.map (·.type)) then
            typeMismatch loc "destructor result differs from its packed field types"
          else #[]
          genericErrors ++ operandErrors ++ resultErrors

private def nominalCallTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (kind : CallKind)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match kind with
  | .constructor reference variant =>
      constructorTypeDiagnostics mode unit ns context loc resultType reference variant
        instantiations arguments
  | .destructor reference variant =>
      destructorTypeDiagnostics mode unit ns context loc resultType reference variant
        instantiations arguments
  | _ => #[]

private def fieldNamed? (targetNs : ValidatedNamespace) (fields : Array FieldDecl)
    (name : String) : Option FieldDecl :=
  fields.find? fun field =>
    (targetNs.tables.names[field.name.index]?).any (·.name == name)

private def commonFieldTypes? (targetNs : ValidatedNamespace) (logical : Bool)
    (declaration : StructDecl)
    (field : String) : Option (Array TypeUse) :=
  if declaration.variants.isEmpty then do
    let selected ← fieldNamed? targetNs declaration.fields field
    some #[selected.type]
  else match declaration.variants.mapM fun variant => do
      let selected ← fieldNamed? targetNs variant.fields field
      some selected.type with
    | some types => some types
    | none =>
        -- A specification field select reaches the variants that declare the
        -- field; the wrong-variant case is a logical abort, not a typing
        -- error.
        if logical then
          let declared := declaration.variants.filterMap fun variant =>
            fieldNamed? targetNs variant.fields field
          if declared.isEmpty then none else some (declared.map (fun f => f.type))
        else none

private def variantFieldTypes? (targetNs : ValidatedNamespace) (declaration : StructDecl)
    (fields : Array (String × String)) : Option (Array TypeUse) := do
  if fields.isEmpty then none else
  fields.mapM fun (variant, field) => do
    let declared ← StaticTyping.fieldsOf targetNs declaration variant
    let selected ← fieldNamed? targetNs declared field
    some selected.type

private structure DataOperandInfo where
  instantiations : Array GenericArgument
  reference : Option ReferenceType := none

private def dataOperandInfo? (ns : ValidatedNamespace)
    (arguments : Array ExprId) (reference : QualifiedRef)
    (allowReference : Bool := false) : Option DataOperandInfo := do
  let argument ← arguments[0]?
  let argumentType ← exprType? ns argument
  let (name, instantiations, sourceReference) ← match ns.tables.types[argumentType.index]? with
    | some (.nominal name instantiations) => some (name, instantiations, none)
    | some (.reference sourceReference) => do
        if !allowReference then none else
        let .nominal name instantiations ← ns.tables.types[sourceReference.referent.index]? | none
        some (name, instantiations, some sourceReference)
    | _ => none
  if resolvedName? ns name == referencedNameForValidation? ns reference then
    some { instantiations, reference := sourceReference }
  else none

/-- The subject's instantiation of the selected declaration's parameters:
the operand's, or, for the implicit subject of a data invariant, whatever
the selection's own types determine (the declaration's own parameters). -/
private def selectedTypesMatchResult (sourceNs targetNs : ValidatedNamespace)
    (logical : Bool)
    (resultType : TypeId) (solution : Unify.Solution)
    (sourceReference : Option ReferenceType) (selected : Array TypeUse) : Bool :=
  !selected.isEmpty && match sourceReference with
  | none => selected.all fun typeUse =>
      specMatchesSolution sourceNs logical resultType targetNs typeUse.typeId solution
  | some sourceReference => match sourceNs.tables.types[resultType.index]? with
      | some (.reference resultReference) =>
          sourceReference.profile == resultReference.profile &&
            sourceReference.kind == resultReference.kind &&
            lifetimeKindsEquivalent? sourceNs sourceReference.lifetime
              sourceNs resultReference.lifetime &&
            selected.all fun typeUse =>
              typeMatchesSolution? sourceNs resultReference.referent targetNs
                typeUse.typeId solution
      | _ =>
          -- The specification projection reads a field value through the
          -- reference-typed subject.
          logical && selected.all fun typeUse =>
            specMatchesSolution sourceNs logical resultType targetNs typeUse.typeId solution

private def dataOperationTypeDiagnostics (mode : PreparationMode) (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (context : ScanContext)
    (loc : LocId) (resultType : TypeId) (operation : DataOperation)
    (arguments : Array ExprId) : Array Diagnostic :=
  let reference := match operation with
    | .select reference _ | .selectVariants reference _ | .testVariants reference _ |
        .discriminant reference | .updateField reference _ => reference
  match resolveStructDeclaration? unit ns reference with
  | none => if mode matches .typing then #[] else
      #[.at "LIR-SEMANTIC-TARGET" "data operation target does not resolve" loc]
  | some (targetNs, declaration) =>
      /- Selecting a field of a *mutable* reference has no executable
      meaning: the evaluator reads a nominal value, while a mutable
      reference is a live loan whose referent may hold the hole of a
      focused reborrow.  A frontend that wants the field must reborrow
      through a place.  Typing accepts the shape because a specification
      reads it, so the rejection belongs to execution. -/
      let referenceOperandErrors :=
        if !(mode matches .execution) then #[] else
        match operation with
        | .select .. | .selectVariants .. =>
            match (do
              let argument ← arguments[0]?
              let argumentType ← exprType? ns argument
              ns.tables.types[argumentType.index]?) with
            | some (.reference source) =>
                if source.kind matches .mutable then
                  #[.at "LIR-SEMANTIC-REFERENCE"
                    "a field selection whose operand is a mutable reference \
                     has no executable meaning; reborrow the field as a place"
                    loc]
                else #[]
            | _ => #[]
        | _ => #[]
      if mode matches .execution then referenceOperandErrors else
      let implicitSubject := context.logical && arguments.isEmpty
      let operandInfo := dataOperandInfo? ns arguments reference
        (operation matches .select .. | .selectVariants ..)
      let operandErrors := if operandInfo.isSome || implicitSubject then #[] else
        typeMismatch loc "data operation operand is not its nominal target type or supported reference"
      let instantiations := operandInfo.map (·.instantiations) |>.getD #[]
      let solution := if implicitSubject then Unify.Solution.unbound declaration.generics.size
        else Unify.Solution.bound instantiations
      let sourceReference := operandInfo.bind (·.reference)
      match operation with
      | .select _ field => match commonFieldTypes? targetNs context.logical declaration field with
          | some selected =>
              let resultErrors := if selectedTypesMatchResult ns targetNs context.logical resultType
                  solution sourceReference selected &&
                  (sourceReference.isNone || declaration.variants.isEmpty) then #[]
                else typeMismatch loc "selected field types differ from the operation result"
              operandErrors ++ resultErrors
          | none => operandErrors ++ #[.at "LIR-SEMANTIC-TARGET"
              "selected field does not exist on every possible variant" loc]
      | .selectVariants _ fields => match variantFieldTypes? targetNs declaration fields with
          | some selected =>
              let resultErrors := if selectedTypesMatchResult ns targetNs context.logical resultType
                  solution sourceReference selected then #[]
                else typeMismatch loc "variant field types differ from the operation result"
              operandErrors ++ resultErrors
          | none => operandErrors ++ #[.at "LIR-SEMANTIC-TARGET"
              "variant field selection names no valid variant fields" loc]
      | .testVariants _ variants =>
          let resultErrors := if isBoolType ns resultType then #[]
            else typeMismatch loc "variant test result is not Bool"
          let variantErrors := if variants.all fun name =>
              declaration.variants.any fun variant =>
                (targetNs.tables.names[variant.name.index]?).any (·.name == name) then #[]
            else #[.at "LIR-SEMANTIC-TARGET" "variant test names an unknown variant" loc]
          operandErrors ++ resultErrors ++ variantErrors
      | .discriminant _ =>
          let resultErrors := match ns.tables.types[resultType.index]? with
            | some (.integer _ _) => #[]
            | _ => typeMismatch loc "discriminant result is not an integer"
          let declarationErrors := if !declaration.variants.isEmpty &&
              declaration.variants.all (·.discriminant.isSome) then #[] else
            #[.at "LIR-SEMANTIC-TARGET"
              "discriminant requires an enum whose variants have integer discriminants" loc]
          operandErrors ++ resultErrors ++ declarationErrors
      | .updateField _ field => match commonFieldTypes? targetNs context.logical declaration field with
          | some selected =>
              let resultErrors := if nominalTypeMatchesInstantiation ns resultType reference
                  instantiations then #[]
                else typeMismatch loc "field update result is not its nominal target type"
              -- Match after instantiating the field, including its logical
              -- projection: a generic field instantiated with u64 reads as num.
              let replaces (replacementType field : TypeId) : Bool :=
                specMatchesInstantiation ns context.logical replacementType targetNs field
                  instantiations
              let replacementErrors := match arguments[1]?.bind (exprType? ns) with
                | some replacementType =>
                    if selected.all (fun typeUse => replaces replacementType typeUse.typeId) then #[]
                    else typeMismatch loc "field replacement type differs from the selected field"
                | none => #[]
              operandErrors ++ resultErrors ++ replacementErrors
          | none => operandErrors ++ #[.at "LIR-SEMANTIC-TARGET"
              "updated field does not exist on every possible variant" loc]

private def globalOperationTypeDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (loc : LocId)
    (resultType : TypeId) (kind : GlobalKind) (instantiations : Array GenericArgument)
    (arguments : Array ExprId) : Array Diagnostic :=
  let moveAddressErrors := if context.profile != .move then #[] else
    match arguments[0]?.bind (exprType? ns) with
    | some keyType =>
        -- `publish` corresponds to Move's `move_to`, whose source operand is
        -- the signer; the transitional XAST frontend does not yet normalize
        -- that operand to an address key.
        let signerPublishKey := kind matches .publish &&
          (ns.tables.types[keyType.index]? == some .signer ||
            match ns.tables.types[keyType.index]? with
            | some (.reference reference) =>
                ns.tables.types[reference.referent.index]? == some .signer
            | _ => false)
        if ns.tables.types[keyType.index]? == some .address || signerPublishKey then #[]
        else typeMismatch loc "Move global operation key is not an address"
    | none => #[]
  match instantiations.toList with
  | [.typeArg resource] =>
      let moveAbilityErrors := if context.profile != .move ||
          globalResourceHasKey unit ns context.generics context.logical resource.typeId then #[]
        else #[.at "LIR-SEMANTIC-ABILITY"
          "Move global operation resource does not have Key" loc]
      moveAddressErrors ++ moveAbilityErrors ++ match kind with
      | .contains => if isBoolType ns resultType then #[]
          else typeMismatch loc "global contains result is not Bool"
      | .borrow borrowKind =>
          let kindErrors := referenceResultDiagnostics ns loc resultType borrowKind
          let referentErrors := match ns.tables.types[resultType.index]? with
            | some (.reference reference) =>
                if typesAgree ns reference.referent resource.typeId then #[]
                else typeMismatch loc "global borrow reference has the wrong resource referent"
            | _ => #[]
          kindErrors ++ referentErrors
      | .take => if typesAgree ns resultType resource.typeId then #[]
          else typeMismatch loc "global take result differs from its resource type"
      | .publish =>
          let resultErrors := unitResultDiagnostics ns loc resultType "global publish"
          let valueErrors := match arguments.toList with
            | [_, value] => if exprTypeAgrees ns value resource.typeId then #[]
                else typeMismatch loc "published value differs from its resource type"
            | _ => #[]
          resultErrors ++ valueErrors
  | _ => moveAddressErrors ++ typeMismatch loc "global operation requires one type argument"

private def expressionTypeNode? (ns : ValidatedNamespace) (expression : ExprId) : Option Ty := do
  let typeId ← exprType? ns expression
  ns.tables.types[typeId.index]?

private def vectorElementType? (ns : ValidatedNamespace) (typeId : TypeId) : Option TypeId :=
  match ns.tables.types[typeId.index]? with
  | some (.vector element _) => some element
  | _ => none

private def specificationGenericTypeDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (operation : SpecOperation)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  let expected? : Option TypeId := match operation with
    | .old | .trace _ => some resultType
    | .emptyVector | .singletonVector | .updateVector | .concatVector | .sliceVector =>
        vectorElementType? ns resultType
    | .indexOfVector | .containsVector | .lengthVector | .indexVector |
        .inVectorRange | .vectorRange => do
        let collection ← arguments[0]?
        let collectionType ← exprType? ns collection
        vectorElementType? ns collectionType
    | _ => none
  if operation matches .old | .trace _ |
      .emptyVector | .singletonVector | .updateVector | .concatVector | .sliceVector |
      .indexOfVector | .containsVector | .lengthVector | .indexVector |
      .inVectorRange | .vectorRange then
    match instantiations.toList with
    | [.typeArg argument] => match expected? with
        | some expected => if typesAgree ns argument.typeId expected ||
              specProjectedAgree ns argument.typeId expected then #[] else
            typeMismatch loc "specification builtin type argument differs from its value type"
        | none => #[]
    -- The type argument may be elided; the operand determines it.
    | [] => #[]
    | [_] => #[.at "LIR-SEMANTIC-GENERIC-KIND"
        "specification builtin instantiation requires a type argument" loc]
    | _ => #[.at "LIR-SEMANTIC-ARITY"
        s!"specification builtin instantiation expects 1 argument, but has {instantiations.size}"
        loc]
  else #[]

private def referenceOperationTypeDiagnostics (ns : ValidatedNamespace) (logical : Bool)
    (loc : LocId)
    (resultType : TypeId) (operation : ReferenceOperation)
    (arguments : Array ExprId) : Array Diagnostic :=
  -- The specification projection treats references transparently, so a
  -- logical reference operation is an identity up to that projection.
  if logical then
    match arguments.toList with
    | [argument] => match exprType? ns argument with
        | some argumentType =>
            if specProjectedAgree ns resultType argumentType ||
                specProjectedAgree ns argumentType resultType then #[]
            else typeMismatch loc
              "reference operation operand and result differ under the specification projection"
        | none => #[]
    | _ => #[]
  else
  match operation with
  | .borrow kind => referenceResultDiagnostics ns loc resultType kind
  | .dereference => match arguments.toList with
      | [argument] => match expressionTypeNode? ns argument with
          | some (.reference reference) =>
              if typesAgree ns resultType reference.referent then #[]
              else typeMismatch loc "dereference result differs from its reference referent"
          | _ => typeMismatch loc "dereference operand is not a reference"
      | _ => #[]
  | .freeze _ => match arguments.toList with
      | [argument] => match expressionTypeNode? ns argument,
          ns.tables.types[resultType.index]? with
          | some (.reference source), some (.reference result) =>
              let kindErrors := if result.kind == .shared then #[]
                else typeMismatch loc "freeze result is not an immutable reference"
              let shapeErrors := if source.profile == result.profile &&
                  typesAgree ns source.referent result.referent then #[]
                else typeMismatch loc "freeze changes the reference profile or referent"
              kindErrors ++ shapeErrors
          | _, _ => typeMismatch loc "freeze operand or result is not a reference"
      | _ => #[]
  | .mutate =>
      let resultErrors := unitResultDiagnostics ns loc resultType "reference mutation"
      let operandErrors := match arguments.toList with
        | [referenceExpression, value] =>
            match expressionTypeNode? ns referenceExpression with
            | some (.reference reference) =>
                let kindErrors := if reference.kind == .mutable then #[]
                  else typeMismatch loc "reference mutation uses an immutable reference"
                let valueErrors := if exprTypeAgrees ns value reference.referent then #[]
                  else typeMismatch loc "mutated value differs from the reference referent"
                kindErrors ++ valueErrors
            | actual => typeMismatch loc
                s!"reference mutation operand is not a reference: operand \
                  {repr ((ns.expressions[referenceExpression.index]?).map (·.kind))} : \
                  {repr actual}"
        | _ => #[]
      resultErrors ++ operandErrors

private partial def behaviorProjectedType (ns : ValidatedNamespace) (typeId : TypeId)
    (fuel : Nat := ns.tables.types.size + 1) : TypeId :=
  if fuel == 0 then typeId else
  match ns.tables.types[typeId.index]? with
  | some (.integer (.bits _) _) | some (.integer .pointer _) =>
      (ns.tables.types.findIdx? (fun type =>
        type == .integer .unbounded true)).map (⟨·⟩) |>.getD typeId
  | some (.reference reference) =>
      behaviorProjectedType ns reference.referent (fuel - 1)
  | some (.tuple elements) =>
      let projected := elements.map fun element =>
        behaviorProjectedType ns element (fuel - 1)
      (ns.tables.types.findIdx? (fun type => type == .tuple projected)).map (⟨·⟩)
        |>.getD typeId
  | some (.function arguments result abilities) =>
      let arguments := arguments.map fun argument =>
        behaviorProjectedType ns argument (fuel - 1)
      let result := behaviorProjectedType ns result (fuel - 1)
      (ns.tables.types.findIdx? (fun type =>
        type == .function arguments result abilities)).map (⟨·⟩) |>.getD typeId
  | _ => typeId

private def behaviorResultSlots (ns : ValidatedNamespace) (typeId : TypeId) : Array TypeId :=
  let typeId := behaviorProjectedType ns typeId
  match ns.tables.types[typeId.index]? with
  | some .unit => #[]
  | some (.tuple elements) => elements
  | _ => #[typeId]

private def behaviorMutablePostSlots (ns : ValidatedNamespace)
    (arguments : Array TypeId) : Array TypeId :=
  arguments.filterMap (fun typeId =>
    match ns.tables.types[typeId.index]? with
    | some (Ty.reference reference) =>
        if reference.kind == ReferenceKind.mutable then
          some (behaviorProjectedType ns reference.referent)
        else none
    | _ => none)

private def behaviorOperationTypeDiagnostics (ns : ValidatedNamespace) (loc : LocId)
    (resultType : TypeId) (kind : BehaviorKind) (arguments : Array ExprId) : Array Diagnostic :=
  match arguments.toList with
  | [] => #[]
  | callable :: values => match expressionTypeNode? ns callable with
      | some (.function parameters functionResult _) =>
          let inputs := parameters.map (behaviorProjectedType ns)
          let resultSlots := behaviorResultSlots ns functionResult
          let postSlots := behaviorMutablePostSlots ns parameters
          let expectedForms : Array (Array TypeId) := match kind with
            | .ensuresOf =>
                let minimum := inputs ++ resultSlots
                let canonical := minimum ++ postSlots
                if canonical == minimum then #[minimum] else #[minimum, canonical]
            | .resultOf =>
                let canonical := inputs ++ postSlots
                if canonical == inputs then #[inputs] else #[inputs, canonical]
            | .foldsOf => #[]
            | .requiresOf | .abortsOf | .unchangedOf | .writeOf _ => #[inputs]
          let values := values.toArray
          let argumentErrors := if kind == .foldsOf then #[] else
            -- A value operand is read at the projection of its type, as the
            -- callable's parameters are.
            let matching := expectedForms.any fun expected =>
              values.size == expected.size && (values.zip expected).all fun pair =>
                (exprType? ns pair.1).map (behaviorProjectedType ns) == some pair.2
            if matching then #[] else
              let expectedArities := " or ".intercalate <|
                expectedForms.map (toString ·.size) |>.toList
              #[.at "LIR-SEMANTIC-TYPE"
                s!"behavior predicate arguments differ from the callable type; expected {expectedArities} value operand(s), got {values.size}"
                loc]
          let resultErrors := match kind with
            | .resultOf =>
                if resultSlots.isEmpty then
                  #[.at "LIR-SEMANTIC-TYPE"
                    "result_of cannot summarize a function with no return value" loc]
                else if behaviorProjectedType ns resultType ==
                    behaviorProjectedType ns functionResult then #[] else
                  typeMismatch loc "result_of result differs from the callable result type"
            | .writeOf index => match postSlots[index]? with
                | some expected => if resultType == expected then #[] else
                    typeMismatch loc "write_of result differs from its mutable-reference referent"
                | none => #[.at "LIR-SEMANTIC-RESULT"
                    s!"write_of mutable-reference index {index} is out of range; callable has {postSlots.size} mutable-reference parameter(s)"
                    loc]
            | .requiresOf | .abortsOf | .ensuresOf | .unchangedOf | .foldsOf =>
                if isBoolType ns resultType then #[] else
                  typeMismatch loc "behavior predicate result is not Bool"
          argumentErrors ++ resultErrors
      | _ => typeMismatch loc "behavior predicate target is not a function type"

private def specOperationTypeDiagnostics (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (context : ScanContext) (loc : LocId) (resultType : TypeId) (operation : SpecOperation)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Array Diagnostic :=
  match operation with
  | .behavior kind _ =>
      let genericErrors := if instantiations.isEmpty then #[] else
        #[.at "LIR-SEMANTIC-ARITY"
          s!"behavior predicates accept no generic arguments, but have {instantiations.size}" loc]
      genericErrors ++ behaviorOperationTypeDiagnostics ns loc resultType kind arguments
  | .result index =>
      -- The Move profile has no first-class tuple values: a tuple-typed
      -- single result is the packed multi-result row, and `result i` names
      -- its component.
      let resultTypes : Array TypeId :=
        match context.results.map (·.typeId), context.profile with
        | #[only], .move =>
            match ns.tables.types[only.index]? with
            | some (Ty.tuple components) => components
            | _ => #[only]
        | results, _ => results
      match resultTypes[index]? with
      | some expected => if specProjectedAgree ns resultType expected then #[] else
          typeMismatch loc "specification result type differs from the function result"
      | none => #[.at "LIR-SEMANTIC-RESULT"
          s!"specification result {index} is out of range; function has {resultTypes.size} results"
          loc]
  | .typeDomain => match instantiations.toList with
      | [.typeArg type] => match ns.tables.types[resultType.index]? with
          | some (.typeDomain element) => if element == type.typeId then #[] else
              typeMismatch loc "type-domain result differs from its type argument"
          | _ => typeMismatch loc "type-domain result is not a type domain"
      | [_] => #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "type-domain instantiation requires a type argument" loc]
      | _ => #[.at "LIR-SEMANTIC-ARITY"
          s!"type-domain instantiation expects 1 argument, but has {instantiations.size}" loc]
  | .resourceDomain => match instantiations.toList with
      | [.typeArg resource] =>
          match ns.tables.types[resource.typeId.index]?, ns.tables.types[resultType.index]? with
          | some (.nominal resourceName resourceArguments),
              some (.resourceDomain domainName domainArguments) =>
              let typeArguments := resourceArguments.mapM fun
                | .typeArg argument => some argument.typeId
                | _ => none
              if resourceName == domainName && typeArguments == some (domainArguments.getD #[]) then
                #[]
              else typeMismatch loc
                "resource-domain result differs from its nominal type argument"
          | some (.nominal ..), _ =>
              typeMismatch loc "resource-domain result is not a resource domain"
          | _, _ => typeMismatch loc "resource-domain type argument is not nominal"
      | [_] => #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "resource-domain instantiation requires a type argument" loc]
      | _ => #[.at "LIR-SEMANTIC-ARITY"
          s!"resource-domain instantiation expects 1 argument, but has {instantiations.size}" loc]
  | .old | .withStateAnchor _ | .trace _ | .noOp =>
      unaryIdentityPrimitiveDiagnostics ns true loc resultType arguments
  | .final => match arguments.toList with
      | [reference] => match ns.expressions[reference.index]?.map (·.kind) with
          | some (ExprKind.operation (.specification (.result _)) _ _ _) =>
              unaryIdentityPrimitiveDiagnostics ns true loc resultType arguments
          | _ => typeMismatch loc "`final` reads a returned mutable reference"
      | _ => #[.at "LIR-SEMANTIC-ARITY" "`final` expects one operand" loc]
  | .bitVectorToInt => match arguments.toList with
      | [value] =>
          let resultErrors :=
            if ns.tables.types[resultType.index]? == some (.integer .unbounded true) then #[]
            else typeMismatch loc "bit-vector-to-int result is not logical num"
          let operandErrors := match exprType? ns value with
            | some valueType => if specReadsInteger ns valueType then #[] else
                typeMismatch loc "bit-vector-to-int operand is not a fixed-width integer"
            | none => #[]
          resultErrors ++ operandErrors
      | _ => #[]
  | .intToBitVector => match arguments.toList with
      | [value] =>
          let resultErrors := if isAnyIntegerType ns resultType then #[] else
            typeMismatch loc "int-to-bit-vector result is not a fixed-width integer"
          let operandErrors := match exprType? ns value with
            | some valueType =>
                if specReadsInteger ns valueType then #[]
                else typeMismatch loc "int-to-bit-vector operand is not logical num"
            | none => #[]
          resultErrors ++ operandErrors
      | _ => #[]
  | .saveStateAnchor _ | .foldsCaptureAnchor _ =>
      if isBoolType ns resultType then #[] else
        typeMismatch loc "state-anchor marker result is not Bool"
  | .inlineCallSummary =>
      let resultErrors := if isBoolType ns resultType then #[] else
        typeMismatch loc "inline-call summary result is not Bool"
      let abortErrors := match arguments[1]?.bind (exprType? ns) with
        | some abortType => if isBoolType ns abortType then #[] else
            typeMismatch loc "inline-call summary abort operand is not Bool"
        | none => #[]
      resultErrors ++ abortErrors
  | .wellFormed =>
      if isBoolType ns resultType then #[] else
        typeMismatch loc "well-formedness result is not Bool"
  | .abortFlag =>
      if isBoolType ns resultType then #[] else
        typeMismatch loc "abort flag result is not Bool"
  | .abortCode =>
      if ns.tables.types[resultType.index]? == some (.integer .unbounded true) then #[] else
        typeMismatch loc "abort code result is not logical num"
  | .emptyVector | .singletonVector =>
      vectorPrimitiveDiagnostics ns true loc resultType arguments
  | .updateVector => match arguments.toList with
      | [collection, index, value] =>
          let collectionErrors := match exprType? ns collection,
              ns.tables.types[resultType.index]? with
            | some collectionType, some (.vector element _) =>
                let resultErrors :=
                  if (specVectorElement? ns collectionType).any (typesAgree ns · element) then #[]
                  else typeMismatch loc
                    "vector update operand and result are not the same vector type"
                let valueErrors :=
                  if (exprType? ns value).any (specProjectedAgree ns · element) then #[] else
                  typeMismatch loc "vector update value differs from its element type"
                resultErrors ++ valueErrors
            | _, _ => typeMismatch loc "vector update result is not a vector type"
          let indexErrors := match exprType? ns index with
            | some indexType =>
                if specReadsInteger ns indexType then #[]
                else typeMismatch loc "specification vector update index is not an integer"
            | none => #[]
          collectionErrors ++ indexErrors
      | _ => #[]
  | .concatVector => match arguments.toList with
      | [left, right] =>
          let agrees (operand : ExprId) : Bool := match specVectorElement? ns resultType with
            | some element => (exprType? ns operand).any fun operandType =>
                (specVectorElement? ns operandType).any (typesAgree ns · element)
            | none => false
          if isVectorType ns resultType && agrees left && agrees right then #[] else
            typeMismatch loc
              "vector concatenation operands and result are not the same vector type"
      | _ => #[]
  | .lengthVector => match arguments.toList with
      | [collection] =>
          let resultErrors :=
            if isAnyIntegerType ns resultType then #[]
            else typeMismatch loc "specification vector length result is not logical num"
          let collectionErrors := match exprType? ns collection with
            | some collectionType => if (specVectorElement? ns collectionType).isSome then #[]
                else typeMismatch loc "specification vector length operand is not a vector"
            | none => #[]
          resultErrors ++ collectionErrors
      | _ => #[]
  | .indexVector => match arguments.toList with
      | [collection, index] =>
          let collectionErrors := match exprType? ns collection with
            | some collectionType => match specVectorElement? ns collectionType with
                | some element =>
                    if specProjectedAgree ns resultType element then #[] else
                    typeMismatch loc "specification vector index result differs from its element type"
                | none => typeMismatch loc "specification vector index operand is not a vector"
            | none => #[]
          let indexErrors := match exprType? ns index with
            | some indexType =>
                if specReadsInteger ns indexType then #[]
                else typeMismatch loc "specification vector index is not an integer"
            | none => #[]
          collectionErrors ++ indexErrors
      | _ => #[]
  | .sliceVector => match arguments.toList with
      | [collection, range] =>
          let collectionErrors :=
            if isVectorType ns resultType &&
                (match specVectorElement? ns resultType, exprType? ns collection with
                 | some element, some collectionType =>
                     (specVectorElement? ns collectionType).any (typesAgree ns · element)
                 | _, _ => false) then #[]
            else typeMismatch loc
              "vector slice operand and result are not the same vector type"
          let rangeErrors := match exprType? ns range with
            | some rangeType => if ns.tables.types[rangeType.index]? == some .range then #[] else
                typeMismatch loc "vector slice bound is not a range"
            | none => #[]
          collectionErrors ++ rangeErrors
      | _ => #[]
  | .containsVector => match arguments.toList with
      | [collection, value] =>
          let resultErrors := if isBoolType ns resultType then #[] else
            typeMismatch loc "vector containment result is not Bool"
          let operandErrors := match exprType? ns collection with
            | some collectionType => match specVectorElement? ns collectionType with
                | some element =>
                    if (exprType? ns value).any (specProjectedAgree ns · element) then #[] else
                    typeMismatch loc "vector containment value differs from its element type"
                | none => typeMismatch loc "vector containment operand is not a vector"
            | none => #[]
          resultErrors ++ operandErrors
      | _ => #[]
  | .indexOfVector => match arguments.toList with
      | [collection, value] =>
          let resultErrors :=
            if ns.tables.types[resultType.index]? == some (.integer .unbounded true) then #[] else
              typeMismatch loc "vector index-of result is not an unbounded integer"
          let operandErrors := match exprType? ns collection with
            | some collectionType => match specVectorElement? ns collectionType with
                | some element =>
                    if (exprType? ns value).any (specProjectedAgree ns · element) then #[] else
                    typeMismatch loc "vector index-of value differs from its element type"
                | none => typeMismatch loc "vector index-of operand is not a vector"
            | none => #[]
          resultErrors ++ operandErrors
      | _ => #[]
  | .inVectorRange => match arguments.toList with
      | [collection, index] =>
          let resultErrors := if isBoolType ns resultType then #[] else
            typeMismatch loc "vector in-range result is not Bool"
          let collectionErrors := match exprType? ns collection with
            | some collectionType => if (specVectorElement? ns collectionType).isSome then #[] else
                typeMismatch loc "vector in-range operand is not a vector"
            | none => #[]
          let indexErrors := match exprType? ns index with
            | some indexType =>
                if specReadsInteger ns indexType then #[]
                else typeMismatch loc "vector in-range index is not an integer"
            | none => #[]
          resultErrors ++ collectionErrors ++ indexErrors
      | _ => #[]
  | .inRange => match arguments.toList with
      | [range, index] =>
          let resultErrors := if isBoolType ns resultType then #[] else
            typeMismatch loc "range-membership result is not Bool"
          let rangeErrors := match exprType? ns range with
            | some rangeType => if ns.tables.types[rangeType.index]? == some .range then #[] else
                typeMismatch loc "range-membership operand is not a range"
            | none => #[]
          let indexErrors := match exprType? ns index with
            | some indexType =>
                if specReadsInteger ns indexType then #[]
                else typeMismatch loc "range-membership index is not an integer"
            | none => #[]
          resultErrors ++ rangeErrors ++ indexErrors
      | _ => #[]
  | .vectorRange => match arguments.toList with
      | [collection] =>
          let resultErrors := if ns.tables.types[resultType.index]? == some .range then #[] else
            typeMismatch loc "vector range result is not a range"
          let operandErrors := match exprType? ns collection with
            | some collectionType => if (specVectorElement? ns collectionType).isSome then #[] else
                typeMismatch loc "vector range operand is not a vector"
            | none => #[]
          resultErrors ++ operandErrors
      | _ => #[]
  | .maxValue width =>
      if ns.tables.types[resultType.index]? == some (.integer (.bits width) false) ||
          isLogicalNumType ns resultType then #[] else
        typeMismatch loc
          s!"maximum-value result is neither u{width} nor logical num"
  | .emptyEventStore =>
      if ns.tables.types[resultType.index]? == some .eventStore then #[] else
        typeMismatch loc "empty event-store result is not EventStore"
  | .extendEventStore =>
      let resultErrors := if ns.tables.types[resultType.index]? == some .eventStore then #[] else
        typeMismatch loc "extended event-store result is not EventStore"
      let storeErrors := match arguments[0]?.bind (exprType? ns) with
        | some storeType => if ns.tables.types[storeType.index]? == some .eventStore then #[] else
            typeMismatch loc "event-store extension operand is not EventStore"
        | none => #[]
      let conditionErrors := match arguments[3]?.bind (exprType? ns) with
        | some conditionType => if isBoolType ns conditionType then #[] else
            typeMismatch loc "event-store extension condition is not Bool"
        | none => #[]
      resultErrors ++ storeErrors ++ conditionErrors
  | .stateDomain =>
      if ns.tables.types[resultType.index]? == some .stateDomain then #[] else
        typeMismatch loc "state-domain result is not a state domain"
  | .eventStoreIncludes | .eventStoreIncludedIn =>
      let resultErrors := if isBoolType ns resultType then #[] else
        typeMismatch loc "event-store inclusion result is not Bool"
      let operandErrors := arguments.foldl (fun ds argument =>
        match exprType? ns argument with
        | some type => if ns.tables.types[type.index]? == some .eventStore then ds else
            ds ++ typeMismatch loc "event-store inclusion operand is not EventStore"
        | none => ds) #[]
      resultErrors ++ operandErrors
  | .publish _ | .remove _ | .update _ =>
      let resultErrors := if isBoolType ns resultType then #[] else
        typeMismatch loc "specification resource mutation result is not Bool"
      match instantiations.toList with
      | [.typeArg resource] =>
          let abilityErrors := if context.profile != .move ||
              globalResourceHasKey unit ns context.generics true resource.typeId then #[]
            else #[.at "LIR-SEMANTIC-ABILITY"
              "Move specification resource type does not have Key" loc]
          let addressErrors := match arguments[0]?.bind (exprType? ns) with
            | some addressType => if ns.tables.types[addressType.index]? == some .address then #[]
                else typeMismatch loc "specification resource address is not an address"
            | none => #[]
          let valueErrors := match operation, arguments.toList with
            | .publish _, [_, value] | .update _, [_, value] =>
                if exprType? ns value == some resource.typeId then #[] else
                  typeMismatch loc "specification resource value differs from its type argument"
            | _, _ => #[]
          resultErrors ++ abilityErrors ++ addressErrors ++ valueErrors
      | [_] => resultErrors ++ #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "specification resource mutation requires a type argument" loc]
      | _ => resultErrors ++ #[.at "LIR-SEMANTIC-ARITY"
          s!"specification resource mutation expects 1 type argument, but has {instantiations.size}"
          loc]
  | .global _ => match instantiations.toList with
      | [.typeArg resource] =>
          let abilityErrors := if context.profile != .move ||
              globalResourceHasKey unit ns context.generics true resource.typeId then #[]
            else #[.at "LIR-SEMANTIC-ABILITY"
              "Move specification global resource does not have Key" loc]
          let addressErrors := if context.profile != .move then #[] else
            match arguments[0]?.bind (exprType? ns) with
            | some addressType => if ns.tables.types[addressType.index]? == some .address then #[]
                else typeMismatch loc "Move specification global key is not an address"
            | none => #[]
          let resultErrors := if resultType == resource.typeId then #[] else
            typeMismatch loc "specification global result differs from its resource type"
          abilityErrors ++ addressErrors ++ resultErrors
      | [_] => #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "specification global requires a type argument" loc]
      | _ => #[.at "LIR-SEMANTIC-ARITY"
          s!"specification global expects 1 type argument, but has {instantiations.size}" loc]
  | .exists _ => match instantiations.toList with
      | [.typeArg resource] =>
          let abilityErrors := if context.profile != .move ||
              globalResourceHasKey unit ns context.generics true resource.typeId then #[]
            else #[.at "LIR-SEMANTIC-ABILITY"
              "Move specification exists resource does not have Key" loc]
          let addressErrors := if context.profile != .move then #[] else
            match arguments[0]?.bind (exprType? ns) with
            | some addressType => if ns.tables.types[addressType.index]? == some .address then #[]
                else typeMismatch loc "Move specification exists key is not an address"
            | none => #[]
          let resultErrors := if isBoolType ns resultType then #[] else
            typeMismatch loc "specification exists result is not Bool"
          abilityErrors ++ addressErrors ++ resultErrors
      | [_] => #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "specification exists requires a type argument" loc]
      | _ => #[.at "LIR-SEMANTIC-ARITY"
          s!"specification exists expects 1 type argument, but has {instantiations.size}" loc]
  | .canModify =>
      let resultErrors := if isBoolType ns resultType then #[] else
        typeMismatch loc "can-modify result is not Bool"
      match instantiations.toList with
      | [.typeArg resource] =>
          let abilityErrors := if context.profile != .move ||
              globalResourceHasKey unit ns context.generics true resource.typeId then #[]
            else #[.at "LIR-SEMANTIC-ABILITY"
              "Move can-modify resource does not have Key" loc]
          let addressErrors := match arguments[0]?.bind (exprType? ns) with
            | some addressType => if ns.tables.types[addressType.index]? == some .address then #[]
                else typeMismatch loc "can-modify key is not an address"
            | none => #[]
          resultErrors ++ abilityErrors ++ addressErrors
      | [_] => resultErrors ++ #[.at "LIR-SEMANTIC-GENERIC-KIND"
          "can-modify requires a type argument" loc]
      | _ => resultErrors ++ #[.at "LIR-SEMANTIC-ARITY"
          s!"can-modify expects 1 type argument, but has {instantiations.size}" loc]
  | _ => #[]

/-- Check a literal against a trait-owned associated constant type after
substituting the referenced trait's generic arguments. -/
private partial def constMatchesInstantiationFuel : Nat → ValidatedNamespace → ConstValue →
    ValidatedNamespace → TypeId → Array GenericArgument → Option Bool
  | 0, _, _, _, _, _ => none
  | fuel + 1, sourceNs, value, targetNs, target, instantiations => do
      let targetType ← targetNs.tables.types[target.index]?
      match targetType with
      | .typeParameter index => match instantiations[index]? with
          | some (.typeArg instantiated) =>
              firstSliceConstMatchesType sourceNs.tables value instantiated.typeId
          | _ => none
      | .tuple types => match value with
          | .tuple values =>
              if values.size != types.size then some false else
                let checks := values.zip types |>.map fun pair =>
                  constMatchesInstantiationFuel fuel sourceNs pair.1 targetNs pair.2 instantiations
                if checks.any (· == some false) then some false
                else if checks.all (· == some true) then some true else none
          | .unit => some types.isEmpty
          | _ => firstSliceConstMatchesType targetNs.tables value target
      | .unit => match value with
          | .tuple values => some values.isEmpty
          | _ => firstSliceConstMatchesType targetNs.tables value target
      | .vector element length => match value with
          | .vector values =>
              let lengthMatches := match length with
                | none => true
                | some (.integer expected) => expected == Int.ofNat values.size
                | _ => false
              if !lengthMatches then some false else
                let checks := values.map fun elementValue =>
                  constMatchesInstantiationFuel fuel sourceNs elementValue targetNs element
                    instantiations
                if checks.any (· == some false) then some false
                else if checks.all (· == some true) then some true else none
          | _ => firstSliceConstMatchesType targetNs.tables value target
      | _ => firstSliceConstMatchesType targetNs.tables value target

private def associatedConstantPredicateMatches? (unit : ValidatedUnit)
    (source : ValidatedNamespace) (trait : TraitRef) (itemId : AssociatedItemId)
    (value : ConstValue) : Option Bool := do
  let (targetNs, declaration) ← resolveTraitDeclaration? unit source trait.trait
  let item ← targetNs.associatedItems[itemId.index]?
  if item.owner != declaration.id || !declaration.associatedItems.contains itemId then none else
    let .constant declaredType _ := item.kind | none
    constMatchesInstantiationFuel
      (source.tables.types.size + targetNs.tables.types.size + 1)
      source value targetNs declaredType.typeId trait.arguments

private def tuplePatternTypeDiagnostics (ns : ValidatedNamespace) (pattern : Pattern)
    (elements : Array PatternId) : Array Diagnostic :=
  match ns.tables.types[pattern.typeId.index]? with
  | some (.tuple types) =>
      if elements.size != types.size then
        typeMismatch pattern.loc
          s!"tuple pattern has {elements.size} elements, expected {types.size}"
      else
        (elements.zip types).foldl (fun ds pair =>
          if (patternType? ns pair.1).any (typesAgree ns · pair.2) then ds
          else ds ++ typeMismatch pattern.loc
            "tuple pattern child type differs from its tuple element type") #[]
  | _ => typeMismatch pattern.loc "tuple pattern is not annotated with a tuple type"

private def rangePatternTypeDiagnostics (ns : ValidatedNamespace) (pattern : Pattern)
    (lower upper : Option ConstValue) (inclusive : Bool) : Array Diagnostic :=
  let scalarTypeErrors := match ns.tables.types[pattern.typeId.index]? with
    | some (.integer _ _) | some .character => #[]
    | _ => typeMismatch pattern.loc
        "range pattern is not annotated with an integer or character type"
  let boundErrors := (lower.toArray ++ upper.toArray).foldl (fun ds bound =>
    match firstSliceConstMatchesType ns.tables bound pattern.typeId with
    | some false => ds ++ typeMismatch pattern.loc
        "range-pattern bound does not match the annotated integer type"
    | _ => ds) #[]
  let orderErrors := match lower, upper with
    | some (.integer lower), some (.integer upper) =>
        if (if inclusive then lower <= upper else lower < upper) then #[]
        else typeMismatch pattern.loc "range-pattern bounds are reversed or empty"
    | some (.character lower), some (.character upper) =>
        if (if inclusive then lower <= upper else lower < upper) then #[]
        else typeMismatch pattern.loc "range-pattern bounds are reversed or empty"
    | _, _ => #[]
  scalarTypeErrors ++ boundErrors ++ orderErrors

private def genericParameterScopeDiagnostics (loc : LocId)
    (generics : Array GenericBinder) (expected : BinderKind) (index : Nat) : Array Diagnostic :=
  match generics[index]? with
  | none => #[.at "LIR-SEMANTIC-GENERIC-SCOPE"
      s!"generic parameter {index} is out of scope; context has {generics.size} binders" loc]
  | some binder => if binder.kind == expected then #[] else #[.at
      "LIR-SEMANTIC-GENERIC-SCOPE"
      s!"generic parameter {index} has kind {repr binder.kind}, expected {repr expected}" loc]

private def scanLifetime (mode : PreparationMode) (ns : ValidatedNamespace)
    (generics : Array GenericBinder) (loc : LocId) (id : LifetimeId) : Array Diagnostic :=
  match ns.tables.lifetimes[id.index]? with
  | none => #[.at "LIR-SEMANTICS-INTERNAL" s!"validated lifetime {id.index} is missing" loc]
  | some lifetime =>
      let ds := classificationDiagnostics mode lifetime.loc
        (coreLifetimeKindFeature lifetime.kind)
      match lifetime.kind with
      | .parameter index => ds ++ typingGate mode
          (genericParameterScopeDiagnostics loc generics .lifetime index)
      | .static | .inference | .local => ds

mutual
  private partial def scanType (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (loc : LocId)
      (id : TypeId) (generics : Array GenericBinder := #[])
      (logical : Bool := false) : Array Diagnostic :=
    match ns.tables.types[id.index]? with
    | none => #[.at "LIR-SEMANTICS-INTERNAL" s!"validated type {id.index} is missing" loc]
    | some ty =>
        let nodeErrors := match ty with
          | .profile value => profileDiagnostics registry unit mode .type loc value
          | .integer .pointer _ =>
              if (targetPointerWidth? unit).isSome then
                classificationDiagnostics mode loc (coreTypeFeature ty)
              else if mode matches .typing then #[] else #[.at (unsupportedCode mode)
                s!"type.integer.pointer is not supported for {modeName mode}: the unit has no valid Rust `target_pointer_width` profile option" loc]
          | _ => classificationDiagnostics mode loc (coreTypeFeature ty)
        let childErrors := match ty with
          | .tuple elements => elements.foldl
              (fun ds element => ds ++ scanType registry unit mode ns loc element generics (logical := logical)) #[]
          | .vector element length =>
              let ds := scanType registry unit mode ns loc element generics
                (logical := logical)
              match length with
              | some value => ds ++ scanConst registry unit mode ns loc value
              | none => ds
          | .typeDomain type =>
              scanType registry unit mode ns loc type generics (logical := logical)
          | .resourceDomain name arguments =>
              arguments.toArray.flatten.foldl
                  (fun ds type => ds ++ scanType registry unit mode ns loc type generics (logical := logical)) #[] ++
                resourceDomainTypeDiagnostics mode unit ns generics loc name arguments
          | .nominal name arguments =>
              let argumentErrors := arguments.foldl
                (fun ds argument => ds ++ scanGenericArgument registry unit mode ns loc argument
                  generics (logical := logical)) #[]
              let targetErrors := match ns.tables.names[name.index]? with
                | none => #[.at "LIR-SEMANTICS-INTERNAL"
                    "validated nominal type name is missing" loc]
                | some qualified =>
                    let reference : QualifiedRef := {
                      namespaceId := qualified.namespaceId, name }
                    match resolveStructDeclaration? unit ns reference with
                    | none => if mode matches .typing then #[] else
                        #[.at "LIR-SEMANTIC-TARGET"
                          "nominal type target does not resolve" loc]
                    | some (targetNs, declaration) =>
                        if mode matches .execution then #[] else
                        genericInstantiationDiagnostics loc "nominal type"
                          declaration.generics arguments ++
                        genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) generics
                          "nominal type" declaration.generics arguments (logical := logical)
              argumentErrors ++ targetErrors
          | .function arguments result abilities =>
              let ds := arguments.foldl
                (fun ds argument => ds ++ scanType registry unit mode ns loc argument generics (logical := logical)) #[]
              let ds := ds ++ scanType registry unit mode ns loc result generics
                (logical := logical)
              abilities.foldl (fun ds ability =>
                ds ++ scanAbility mode loc ability) ds
          | .reference reference =>
              let ds := classificationDiagnostics mode loc
                (coreReferenceKindFeature reference.kind) ++
                scanType registry unit mode ns loc reference.referent generics
                  (logical := logical)
              ds ++ scanLifetime mode ns generics loc reference.lifetime
          | .typeParameter index =>
              typingGate mode (genericParameterScopeDiagnostics loc generics .typeArg index)
          | _ => #[]
        nodeErrors ++ childErrors

  private partial def scanGenericArgument (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (loc : LocId)
      (argument : GenericArgument) (generics : Array GenericBinder := #[])
      (logical : Bool := false) : Array Diagnostic :=
    let nodeErrors := classificationDiagnostics mode loc (coreGenericArgumentFeature argument)
    let childErrors := match argument with
      | .typeArg value => scanType registry unit mode ns value.loc value.typeId generics
          (logical := logical)
      | .const value => scanConst registry unit mode ns loc value
      | .lifetime lifetime => scanLifetime mode ns generics loc lifetime
      | .evidence _ => #[]
    nodeErrors ++ childErrors

  private partial def scanTraitRef (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (loc : LocId)
      (trait : TraitRef) (generics : Array GenericBinder := #[]) : Array Diagnostic :=
    let argumentErrors := trait.arguments.foldl
      (fun ds argument => ds ++ scanGenericArgument registry unit mode ns loc argument generics) #[]
    let targetErrors := match resolveTraitDeclaration? unit ns trait.trait with
      | none => if mode matches .typing then #[] else
          #[.at "LIR-SEMANTIC-TARGET" "trait target does not resolve" loc]
      | some (targetNs, declaration) =>
          if mode matches .execution then #[] else
          genericInstantiationDiagnostics loc "trait instantiation"
              declaration.generics trait.arguments ++
            genericAbilityDiagnostics unit ns (targetNs.profile.getD .move) generics
              "trait instantiation" declaration.generics trait.arguments
    argumentErrors ++ targetErrors

  private partial def scanGenericPredicate (registry : SemanticsRegistry)
      (unit : ValidatedUnit) (mode : PreparationMode) (ns : ValidatedNamespace)
      (loc : LocId) (predicate : GenericPredicate)
      (generics : Array GenericBinder := #[]) : Array Diagnostic :=
    let nodeErrors := match predicate with
      | .profile value => profileDiagnostics registry unit mode .property loc value
      | _ => classificationDiagnostics mode loc (coreGenericPredicateFeature predicate)
    let childErrors := match predicate with
      | .ability type _ => scanType registry unit mode ns loc type generics
      | .implements type trait =>
          scanType registry unit mode ns loc type generics ++
            scanTraitRef registry unit mode ns loc trait generics
      | .associatedTypeEq trait item value =>
          scanTraitRef registry unit mode ns loc trait generics ++
            typingGate mode (associatedPredicateDiagnostics unit ns loc trait item true) ++
            scanType registry unit mode ns loc value generics
      | .associatedConstEq trait item value =>
          scanTraitRef registry unit mode ns loc trait generics ++
            typingGate mode (associatedPredicateDiagnostics unit ns loc trait item false) ++
            scanConst registry unit mode ns loc value ++
            typingGate mode
              (match associatedConstantPredicateMatches? unit ns trait item value with
              | some false => typeMismatch loc
                  "associated constant equality value differs from its declaration type"
              | _ => #[])
      | .constEq left right =>
          scanConst registry unit mode ns loc left ++ scanConst registry unit mode ns loc right
      | .lifetimeOutlives longer shorter =>
          scanLifetime mode ns generics loc longer ++
            scanLifetime mode ns generics loc shorter
      | .profile _ => #[]
    nodeErrors ++ childErrors

  private partial def scanAbility (mode : PreparationMode) (loc : LocId)
      (ability : Ability) : Array Diagnostic :=
    classificationDiagnostics mode loc (coreAbilityFeature ability)

  private partial def scanConst (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (loc : LocId)
      (value : ConstValue) : Array Diagnostic :=
    let nodeErrors := match value with
      | .profile profile => profileDiagnostics registry unit mode .constant loc profile
      | _ => classificationDiagnostics mode loc (coreConstFeature value)
    let childErrors := match value with
      | .vector values | .tuple values => values.foldl
          (fun ds value => ds ++ scanConst registry unit mode ns loc value) #[]
      | _ => #[]
    nodeErrors ++ childErrors

  private partial def scanPattern (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (id : PatternId) : Array Diagnostic :=
    match ns.patterns[id.index]? with
    | none => #[.error "LIR-SEMANTICS-INTERNAL" s!"validated pattern {id.index} is missing"]
    | some pattern =>
        let nodeErrors := classificationDiagnostics mode pattern.loc (corePatternFeature pattern.kind) ++
          scanType registry unit mode ns pattern.loc pattern.typeId context.generics
            (logical := context.logical)
        let childErrors := match pattern.kind with
          | .wildcard => #[]
          | .variable localId =>
              typingGate mode (match context.locals[localId.index]? with
                | some declaration =>
                    if typesAgree ns declaration.type.typeId pattern.typeId ||
                        (context.logical &&
                          specProjectedAgree ns pattern.typeId declaration.type.typeId) then #[]
                    else typeMismatch pattern.loc
                      s!"variable pattern type differs from its local declaration: \
                        local {localId.index} : \
                        {repr (ns.tables.types[declaration.type.typeId.index]?)}, \
                        pattern : {repr (ns.tables.types[pattern.typeId.index]?)}"
                | none => #[.at "LIR-SEMANTIC-LOCAL" s!"unknown local {localId.index}" pattern.loc])
          | .tuple elements =>
              typingGate mode (tuplePatternTypeDiagnostics ns pattern elements) ++ elements.foldl
                (fun ds child => ds ++ scanPattern registry unit mode ns context child) #[]
          | .constructor name instantiations variant fields =>
              let ds := constructorPatternTypeDiagnostics mode unit ns context pattern name
                instantiations variant fields
              let ds := instantiations.foldl (fun ds value =>
                ds ++ scanGenericArgument registry unit mode ns pattern.loc value
                  context.generics (logical := context.logical)) ds
              fields.foldl (fun ds child =>
                ds ++ scanPattern registry unit mode ns context child) ds
          | .literal value => scanConst registry unit mode ns pattern.loc value ++
              typingGate mode (match firstSliceConstMatchesType ns.tables value pattern.typeId with
              | some false => typeMismatch pattern.loc "literal pattern does not match its annotated type"
              | _ => #[])
          | .range lower upper inclusive =>
              typingGate mode (rangePatternTypeDiagnostics ns pattern lower upper inclusive) ++
              lower.toArray.foldl (fun ds value =>
                ds ++ scanConst registry unit mode ns pattern.loc value) #[] ++
              upper.toArray.foldl (fun ds value =>
                ds ++ scanConst registry unit mode ns pattern.loc value) #[]
        nodeErrors ++ childErrors

  private partial def scanPlace (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (loc : LocId) (id : PlaceId) : Array Diagnostic :=
    match ns.places[id.index]? with
    | none => #[.at "LIR-SEMANTICS-INTERNAL" s!"validated place {id.index} is missing" loc]
    | some place =>
        let nodeErrors := classificationDiagnostics mode loc (corePlaceFeature place)
        let typeErrors := typingGate mode
          (staticPlaceNodeDiagnostics mode unit ns context loc place)
        let childErrors := match place with
          | .localVar localId =>
              typingGate mode (if localId.index < context.locals.size then #[]
                else #[.at "LIR-SEMANTIC-LOCAL" s!"unknown local {localId.index}" loc])
          | .deref base | .field base .. | .subslice base .. | .downcast base _ =>
              scanPlace registry unit mode ns context loc base
          | .index base index =>
              let ds := scanPlace registry unit mode ns context loc base ++
                scanExpr registry unit mode ns context index
              if (mode matches .typing) || isSimplePlaceIndex ns index then ds
              else ds.push <| .at "LIR-SEMANTIC-PLACE-INDEX"
                "place indexes currently require an integer literal, local expression, copy of a local place, or side-effect-free length-minus-offset expression" loc
        nodeErrors ++ typeErrors ++ childErrors

  private partial def scanOperation (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (loc : LocId) (resultType : TypeId) (operation : Operation)
      (instantiations : Array GenericArgument) (arguments : Array ExprId)
      (allowNonCopyPlaceRead : Bool := false) : Array Diagnostic :=
    let nodeErrors := match operation with
      | .profile value _ => profileDiagnostics registry unit mode .operation loc value
      | .borrow (.profile value) _ =>
          profileDiagnostics registry unit mode .borrow loc value ++
            classificationDiagnostics mode loc (coreOperationFeature operation)
      | .global (.borrow (.profile value)) =>
          profileDiagnostics registry unit mode .borrow loc value ++
            classificationDiagnostics mode loc (coreOperationFeature operation)
      | .primitive (.checkVectorIndex (.profile value)) |
          .primitive (.checkedAdd (.profile value)) |
          .primitive (.checkedSubtract (.profile value)) |
          .primitive (.checkedMultiply (.profile value)) |
          .primitive (.checkedModulo (.profile value)) |
          .primitive (.checkedDivide (.profile value)) |
          .primitive (.checkedShiftLeft (.profile value)) |
          .primitive (.checkedShiftRight (.profile value)) |
          .primitive (.checkedCast (.profile value)) |
          .primitive (.checkedNegate (.profile value)) =>
          profileDiagnostics registry unit mode .throw_ loc value ++
            classificationDiagnostics mode loc (coreOperationFeature operation)
      | .reference (.borrow (.profile value)) =>
          profileDiagnostics registry unit mode .borrow loc value ++
            classificationDiagnostics mode loc (coreOperationFeature operation)
      | .call (.extension value _) => profileDiagnostics registry unit mode .call loc value
      | _ => classificationDiagnostics mode loc (coreOperationFeature operation)
    let placeErrors := match operation with
      | .move place | .copy place | .read place | .write place | .drop place |
          .borrow _ place => scanPlace registry unit mode ns context loc place
      | _ => #[]
    let arityErrors := if mode matches .execution then #[] else
      match operation with
      | .move _ => exactArity loc "move" 0 arguments.size
      | .copy _ => exactArity loc "copy" 0 arguments.size
      | .borrow _ _ => exactArity loc "borrow" 0 arguments.size
      | .read _ => exactArity loc "read" 0 arguments.size
      | .write _ => exactArity loc "write" 1 arguments.size
      | .assert => exactArity loc "assert" 1 arguments.size
      | .drop _ => exactArity loc "drop" 0 arguments.size
      | .call (.destructor ..) => exactArity loc "destructor" 1 arguments.size
      | .call .invoke => if arguments.isEmpty then
          #[.at "LIR-SEMANTIC-ARITY" "invoke expects a callable operand" loc]
        else #[]
      | .global .contains => exactArity loc "global contains" 1 arguments.size
      | .global (.borrow _) => exactArity loc "global borrow" 1 arguments.size
      | .global .take => exactArity loc "global take" 1 arguments.size
      | .global .publish => exactArity loc "global publish" 2 arguments.size
      | .primitive .tuple | .primitive .vector => #[]
      | .primitive .repeatVector => exactArity loc "repeated vector" 1 arguments.size
      | .primitive .pushVector => exactArity loc "vector push" 2 arguments.size
      | .primitive .insertVector => exactArity loc "vector insert" 3 arguments.size
      | .primitive .removeVector => exactArity loc "vector remove" 2 arguments.size
      | .primitive .swapVector => exactArity loc "vector swap" 3 arguments.size
      | .primitive .reverseSliceVector => exactArity loc "vector range reversal" 3 arguments.size
      | .primitive .destroyEmptyVector => exactArity loc "empty-vector destruction" 1 arguments.size
      | .primitive .length | .primitive .signerAddress | .primitive .logicalNot |
          .primitive .bitwiseNot |
          .primitive .negate |
          .primitive (.checkedNegate _) | .primitive .copyValue | .primitive .moveValue |
          .primitive .cast | .primitive (.checkedCast _) =>
          exactArity loc "unary primitive" 1 arguments.size
      | .primitive .slice => exactArity loc "slice" 3 arguments.size
      | .primitive _ => exactArity loc "binary primitive" 2 arguments.size
      | .reference .mutate => exactArity loc "reference mutation" 2 arguments.size
      | .reference _ => exactArity loc "reference operation" 1 arguments.size
      | .data (.updateField _ _) => exactArity loc "field update" 2 arguments.size
      | .data _ =>
          -- A structure contract references fields of its implicit subject
          -- without an operand.
          if context.logical && arguments.isEmpty then #[]
          else exactArity loc "data operation" 1 arguments.size
      | .specification (.result _) | .specification .typeDomain |
          .specification .resourceDomain |
          .specification .abortFlag | .specification .abortCode |
          .specification (.saveStateAnchor _) |
          .specification (.foldsCaptureAnchor _) =>
          exactArity loc "nullary specification operation" 0 arguments.size
      | .specification (.behavior .foldsOf _) =>
          exactArity loc "folds_of behavior predicate" 3 arguments.size
      | .specification (.behavior _ _) => if arguments.isEmpty then
          #[.at "LIR-SEMANTIC-ARITY" "behavior predicate expects a callable operand" loc]
        else #[]
      | .specification .old | .specification .bitVectorToInt |
          .specification .intToBitVector | .specification (.trace _) |
          .specification .noOp | .specification .wellFormed |
          .specification (.withStateAnchor _) =>
          exactArity loc "unary specification operation" 1 arguments.size
      | .specification .inlineCallSummary =>
          exactArity loc "inline-call summary" 2 arguments.size
      | .specification .emptyVector =>
          exactArity loc "empty specification vector" 0 arguments.size
      | .specification .singletonVector =>
          exactArity loc "singleton specification vector" 1 arguments.size
      | .specification .concatVector | .specification .indexOfVector |
          .specification .indexVector |
          .specification .sliceVector |
          .specification .containsVector | .specification .inVectorRange =>
          exactArity loc "binary specification vector operation" 2 arguments.size
      | .specification .updateVector =>
          exactArity loc "specification vector update" 3 arguments.size
      | .specification (.maxValue _) | .specification .emptyEventStore |
          .specification .stateDomain =>
          exactArity loc "nullary typed specification operation" 0 arguments.size
      | .specification .extendEventStore =>
          if arguments.size == 3 || arguments.size == 4 then #[] else
            #[.at "LIR-SEMANTIC-ARITY"
              s!"event-store extension expects 3 or 4 arguments, but has {arguments.size}" loc]
      | .specification .inRange =>
          exactArity loc "range membership" 2 arguments.size
      | .specification .lengthVector | .specification .vectorRange =>
          exactArity loc "specification vector range" 1 arguments.size
      | .specification .eventStoreIncludes | .specification .eventStoreIncludedIn =>
          exactArity loc "event-store inclusion" 1 arguments.size
      | .specification (.publish _) | .specification (.update _) =>
          exactArity loc "specification resource mutation" 2 arguments.size
      | .specification (.remove _) =>
          exactArity loc "specification resource removal" 1 arguments.size
      | .specification (.global _) =>
          exactArity loc "specification global" 1 arguments.size
      | .specification .canModify =>
          exactArity loc "can-modify" 1 arguments.size
      | .specification _ => #[]
      | .call _ | .profile _ _ => #[]
    let shapeErrors := match operation with
      | .primitive primitive => typingGate mode
          (primitiveTypeDiagnostics ns context.logical loc resultType primitive arguments)
      | .call kind =>
          callTypeDiagnostics mode unit ns context loc resultType kind instantiations arguments
          ++ nominalCallTypeDiagnostics mode unit ns context loc resultType kind instantiations
            arguments
      | .data kind =>
          dataOperationTypeDiagnostics mode unit ns context loc resultType kind arguments
      | .global kind => if mode matches .execution then #[] else
          globalOperationTypeDiagnostics unit ns context loc resultType kind instantiations arguments
      | .reference kind => typingGate mode
          (referenceOperationTypeDiagnostics ns context.logical loc resultType kind arguments)
      | .specification (.functionCall reference _) =>
          specFunctionCallTypeDiagnostics mode unit ns context loc resultType reference
            instantiations arguments
      | .specification (.lemma reference _) =>
          if context.lemmaInstances then
            lemmaInstanceTypeDiagnostics mode unit ns context loc resultType reference
              instantiations arguments
          else #[.at "LIR-SEMANTIC-CONDITION"
            "a lemma instance occurs only in an `apply` step" loc]
      | .specification specification => if mode matches .execution then #[] else
          specOperationTypeDiagnostics unit ns context loc resultType specification instantiations
            arguments ++
          specificationGenericTypeDiagnostics ns loc resultType specification instantiations
            arguments
      | .move place | .drop place =>
          let typeErrors :=
            placeOperationTypeDiagnostics mode unit ns context loc resultType operation arguments
          if (mode matches .typing) || isConsumableLocalPlace ns place then typeErrors
          else typeErrors ++ #[.at "LIR-SEMANTIC-PLACE-CONSUME"
            "move and drop currently require a local-rooted owned projection without dereference" loc]
      | .copy _ | .borrow _ _ | .read _ | .write _ | .assert =>
          placeOperationTypeDiagnostics mode unit ns context loc resultType operation arguments
      | _ => #[]
    let instantiationErrors := if mode matches .execution then #[] else
      match operation with
      | .global _ => exactArity loc "global resource type" 1 instantiations.size
      | _ => #[]
    let abilityErrors := if mode matches .typing then #[] else
      match operation with
      | .copy place => match staticPlaceInfo? unit ns context place with
          | some info => if contextTypeHasAbility? unit ns context info.typeId .copy then #[] else
              #[.at "LIR-SEMANTIC-ABILITY" "copy requires the place type to have Copy" loc]
          | none => #[]
      | .read place => match staticPlaceInfo? unit ns context place with
          | some info => if allowNonCopyPlaceRead ||
              contextTypeHasAbility? unit ns context info.typeId .copy then #[] else
              #[.at "LIR-SEMANTIC-ABILITY"
                "non-consuming place reads require the place type to have Copy" loc]
          | none => #[]
      | .drop place => match staticPlaceInfo? unit ns context place with
          | some info => if contextTypeHasAbility? unit ns context info.typeId .drop then #[] else
              #[.at "LIR-SEMANTIC-ABILITY" "drop requires the place type to have Drop" loc]
          | none => #[]
      | .primitive .copyValue =>
          if allowNonCopyPlaceRead || contextTypeHasAbility? unit ns context resultType .copy then #[] else
            #[.at "LIR-SEMANTIC-ABILITY" "value copy requires its type to have Copy" loc]
      | .primitive .repeatVector => match ns.tables.types[resultType.index]? with
          | some (.vector element _) =>
              if contextTypeHasAbility? unit ns context element .copy then #[] else
                #[.at "LIR-SEMANTIC-ABILITY"
                  "repeated-vector construction requires its element type to have Copy" loc]
          | _ => #[]
      | _ => #[]
    nodeErrors ++ placeErrors ++ arityErrors ++ shapeErrors ++ instantiationErrors ++ abilityErrors

  private partial def scanCondition (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (condition : Condition) : Array Diagnostic :=
    let context := { context with logical := true, lemmaInstances := condition.kind matches .apply }
    let ds := classificationDiagnostics mode condition.loc
      (coreConditionFeature condition.kind) ++
      typingGate mode (conditionAuxiliaryDiagnostics condition) ++
      (if condition.kind matches .apply then
        applyFormulaDiagnostics ns condition.loc (ns.expressions.size + 1) condition.expression
      else #[]) ++
      scanExpr registry unit mode ns context condition.expression
    let ds := if !conditionExpressionIsProposition condition.kind ||
        (exprType? ns condition.expression).any (isBoolType ns) then ds
      else ds ++ typingGate mode
        (typeMismatch condition.loc "predicate condition expression is not Bool")
    let ds := match condition.kind with
      | .abortsWith =>
          if (exprType? ns condition.expression).any (isLogicalNumType ns) then ds
          else ds ++ typingGate mode
            (typeMismatch condition.loc "aborts-with code is not logical num")
      | _ => ds
    condition.auxiliary.foldl (fun ds auxiliary =>
      let ds := ds ++ scanExpr registry unit mode ns context auxiliary.2
      typingGate mode (match condition.kind, auxiliary.1 with
      | .emits, "emitsCondition" =>
          if (exprType? ns auxiliary.2).any (isBoolType ns) then #[]
          else typeMismatch condition.loc "emits condition auxiliary is not Bool"
      | .abortsIf, "abortCode" | .abortsWith, "additionalCode" =>
          if (exprType? ns auxiliary.2).any (isLogicalNumType ns) then #[]
          else typeMismatch condition.loc "abort-code auxiliary is not logical num"
      | .update, "updateTarget" =>
          if (match exprType? ns auxiliary.2, exprType? ns condition.expression with
              | some updateTy, some valueTy => typesAgree ns updateTy valueTy
              | _, _ => false) then #[]
          else typeMismatch condition.loc
            "update target and value have different types"
      | _, _ => #[]) ++ ds) ds

  private partial def scanSpecBlock (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (block : SpecBlock) : Array Diagnostic :=
    let context := { context with logical := true }
    let conditionErrors := block.conditions.foldl (fun ds condition =>
      ds ++ scanCondition registry unit mode ns context condition) #[]
    match block.frame with
    | none => conditionErrors
    | some frame =>
        let ds := frame.modifies.foldl (fun ds expression =>
          ds ++ scanExpr registry unit mode ns context expression) conditionErrors
        frame.reads.foldl (fun ds typeUse =>
          ds ++ scanType registry unit mode ns typeUse.loc typeUse.typeId
            context.generics) ds

  private partial def scanExpr (registry : SemanticsRegistry) (unit : ValidatedUnit)
      (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
      (id : ExprId) (allowNonCopyPlaceRead : Bool := false) : Array Diagnostic :=
    match ns.expressions[id.index]? with
    | none => #[.error "LIR-SEMANTICS-INTERNAL" s!"validated expression {id.index} is missing"]
    | some expression =>
        let erasedSpec := mode == .execution && (expression.kind matches .spec _)
        let nodeErrors :=
          (if erasedSpec then #[] else classificationDiagnostics mode expression.loc
            (coreExprFeature expression.kind)) ++
          scanType registry unit mode ns expression.loc expression.typeId context.generics
            (logical := context.logical)
        let childErrors := match expression.kind with
          | .value value _ =>
              scanConst registry unit mode ns expression.loc value ++
                typingGate mode
                  (match firstSliceConstMatchesType ns.tables value expression.typeId with
                  | some false => typeMismatch expression.loc
                      "constant value does not match its annotated expression type"
                  | _ => #[])
          | .constant reference =>
              match resolveConstantDeclaration? unit ns reference with
              | none => if mode matches .typing then #[] else
                  #[.at "LIR-SEMANTIC-TARGET"
                    "constant target does not resolve" expression.loc]
              | some (targetNs, declaration) =>
                  if (mode matches .execution) ||
                      typeIdsEquivalent? ns expression.typeId targetNs declaration.type.typeId
                    then #[]
                  else typeMismatch expression.loc
                    "constant expression type differs from its declaration"
          | .localVar localId =>
              match context.locals[localId.index]? with
              | some declaration =>
                  let typeErrors := typingGate mode
                    (if typesAgree ns declaration.type.typeId expression.typeId ||
                        (context.logical &&
                          specProjectedAgree ns expression.typeId declaration.type.typeId) then #[]
                     else typeMismatch expression.loc
                       "local expression type differs from its declaration")
                  let abilityErrors := if mode matches .typing then #[] else
                    if contextTypeHasAbility? unit ns context expression.typeId .copy then #[]
                    else #[.at "LIR-SEMANTIC-ABILITY"
                      "non-consuming local reads require Copy; use a place move to consume the local"
                      expression.loc]
                  typeErrors ++ abilityErrors
              | none => typingGate mode
                  #[.at "LIR-SEMANTIC-LOCAL" s!"unknown local {localId.index}" expression.loc]
          | .operation operation instantiations arguments surface =>
              let ds := scanOperation registry unit mode ns context expression.loc
                expression.typeId operation instantiations arguments allowNonCopyPlaceRead
              let ds := instantiations.foldl (fun ds value =>
                ds ++ scanGenericArgument registry unit mode ns expression.loc value
                  context.generics (logical := context.logical)) ds
              let observesProjection := allowNonCopyPlaceRead && (operation matches
                .data (.select ..) | .primitive .index | .primitive .copyValue |
                .reference .dereference)
              let ds := arguments.zipIdx.foldl (fun ds pair =>
                ds ++ scanExpr registry unit mode ns context pair.1
                  ((observesOperand operation || observesProjection) && pair.2 == 0)) ds
              match surface with
              | none => ds
              | some (.extension value) =>
                  ds ++ profileDiagnostics registry unit mode .surface expression.loc value
              | some value => ds ++ classificationDiagnostics mode expression.loc
                  (coreSurfaceFeature value)
          | .block statements result =>
              let ds := statements.foldl (fun ds statement =>
                ds ++ scanExpr registry unit mode ns context statement) #[]
              match result with
              | none =>
                  if (mode matches .execution) || isUnitType ns expression.typeId ||
                      !expressionCanFallThrough ns id then ds
                  else ds ++ typeMismatch expression.loc
                    "fallthrough block without a result is not Unit"
              | some result =>
                  let ds := ds ++ scanExpr registry unit mode ns context result
                  if (mode matches .execution) ||
                      (exprMatchesOrIsAbrupt ns result expression.typeId ||
                      (context.logical && (exprType? ns result).any fun resultTy =>
                        specProjectedAgree ns resultTy expression.typeId)) then ds
                  else ds ++ typeMismatch expression.loc "block result type differs from its annotated type"
          | .letDecl pattern value body =>
              let ds := scanPattern registry unit mode ns context pattern
              let ds := match value with
                | some value =>
                    let ds := ds ++ scanExpr registry unit mode ns context value
                    -- An abrupt initializer never reaches the binding. Like
                    -- block and branch results, it is bottom-polymorphic.
                    if (mode matches .execution) || !expressionCanFallThrough ns value ||
                        (match patternType? ns pattern, exprType? ns value with
                         | some patternTy, some valueTy =>
                             typesAgree ns patternTy valueTy ||
                               (context.logical &&
                                 specProjectedAgree ns patternTy valueTy)
                         | _, _ => false) then ds
                    else ds ++ typeMismatch expression.loc "let pattern and initializer types differ"
                | none => ds
              let ds := ds ++ scanExpr registry unit mode ns context body
              if (mode matches .execution) ||
                  (exprMatchesOrIsAbrupt ns body expression.typeId ||
                  (context.logical && (exprType? ns body).any fun bodyTy =>
                    specProjectedAgree ns bodyTy expression.typeId)) then ds
              else ds ++ typeMismatch expression.loc "let body type differs from its annotated type"
          | .ifElse condition thenBranch elseBranch =>
              let ds := scanExpr registry unit mode ns context condition ++
                scanExpr registry unit mode ns context thenBranch
              let ds := if (mode matches .execution) ||
                  (exprType? ns condition).any (isBoolType ns) then ds
                else ds ++ typeMismatch expression.loc "if condition is not Bool"
              let ds := if (mode matches .execution) ||
                  (exprMatchesOrIsAbrupt ns thenBranch expression.typeId ||
                  (context.logical && (exprType? ns thenBranch).any fun branchTy =>
                    specProjectedAgree ns branchTy expression.typeId)) then ds
                else ds ++ typeMismatch expression.loc "then branch type differs from the if expression"
              match elseBranch with
              | none => ds ++ typingGate mode (unitResultDiagnostics ns expression.loc
                  expression.typeId "if without an else branch")
              | some elseBranch =>
                  let ds := ds ++ scanExpr registry unit mode ns context elseBranch
                  if (mode matches .execution) ||
                      (exprMatchesOrIsAbrupt ns elseBranch expression.typeId ||
                      (context.logical && (exprType? ns elseBranch).any fun branchTy =>
                        specProjectedAgree ns branchTy expression.typeId)) then ds
                  else ds ++ typeMismatch expression.loc "else branch type differs from the if expression"
          | .match_ scrutinee arms =>
              let ds := scanExpr registry unit mode ns context scrutinee
              arms.foldl (fun ds arm =>
                let ds := ds ++ scanPattern registry unit mode ns context arm.pattern
                let ds := if (mode matches .execution) ||
                    (match patternType? ns arm.pattern, exprType? ns scrutinee with
                     | some patternTy, some scrutineeTy => typesAgree ns patternTy scrutineeTy ||
                         (context.logical && specProjectedAgree ns patternTy scrutineeTy)
                     | _, _ => false) then ds
                  else ds ++ typeMismatch expression.loc "match pattern and scrutinee types differ"
                let ds := match arm.guard with
                  | none => ds
                  | some guard =>
                      let ds := ds ++ scanExpr registry unit mode ns context guard
                      if (mode matches .execution) ||
                          (exprType? ns guard).any (isBoolType ns) then ds
                      else ds ++ typeMismatch expression.loc "match guard is not Bool"
                let ds := ds ++ scanExpr registry unit mode ns context arm.body
                if (mode matches .execution) ||
                    (exprMatchesOrIsAbrupt ns arm.body expression.typeId ||
                    (context.logical && (exprType? ns arm.body).any fun armTy =>
                      specProjectedAgree ns armTy expression.typeId)) then ds
                else ds ++ typeMismatch expression.loc "match arm type differs from the match expression") ds
          | .loop _ body =>
              let ds := scanExpr registry unit mode ns
                { context with loopResults := context.loopResults.push expression.typeId } body
              if !(mode matches .execution) && expressionCanFallThrough ns body &&
                  !(exprType? ns body).any (isUnitType ns) then
                ds ++ typeMismatch expression.loc "fallthrough loop body is not Unit"
              else ds
          | .break_ nest value =>
              let ds := value.toArray.foldl (fun ds value =>
                ds ++ scanExpr registry unit mode ns context value) #[]
              match loopResultType? context nest with
              | none => ds ++ typingGate mode #[.at "LIR-SEMANTIC-CONTROL"
                  s!"break targets unavailable loop depth {nest}" expression.loc]
              | some targetType => match value with
                  | none => if (mode matches .execution) || isUnitType ns targetType then ds else
                      ds ++ typeMismatch expression.loc
                        "break without a value targets a non-Unit loop"
                  | some value => if (mode matches .execution) ||
                        exprTypeAgrees ns value targetType then ds else
                      ds ++ typeMismatch expression.loc
                        "break value type differs from its target loop"
          | .continue_ nest =>
              typingGate mode (if (loopResultType? context nest).isSome then #[] else
                #[.at "LIR-SEMANTIC-CONTROL"
                  s!"continue targets unavailable loop depth {nest}" expression.loc])
          | .return_ values =>
              let ds := values.foldl (fun ds value =>
                ds ++ scanExpr registry unit mode ns context value) #[]
              if mode matches .execution then ds
              -- A unit-valued return matches a resultless signature, mirroring
              -- the packed-result unit rule for fallthrough bodies.
              else if context.results.isEmpty && values.size == 1 &&
                  (values[0]?.any fun value => (exprType? ns value).any (isUnitType ns)) then ds
              else if values.size != context.results.size then
                ds ++ typeMismatch expression.loc
                  s!"return has {values.size} values, expected {context.results.size}"
              else
                values.zip context.results |>.foldl (fun ds pair =>
                  match exprType? ns pair.1 with
                  | some valueType =>
                      if boundaryTypesMatch ns valueType pair.2.typeId then ds
                      else ds ++ typeMismatch expression.loc
                        "return value type differs from the signature"
                  | none => ds) ds
          | .throw_ kind arguments =>
              let ds := match kind with
                | .profile value => profileDiagnostics registry unit mode .throw_ expression.loc value
                | _ => classificationDiagnostics mode expression.loc (coreThrowFeature kind)
              arguments.foldl (fun ds argument =>
                ds ++ scanExpr registry unit mode ns context argument) ds
          | .assign place value =>
              let ds := scanPlace registry unit mode ns context expression.loc place ++
                scanExpr registry unit mode ns context value
              let resultErrors := typingGate mode (unitResultDiagnostics ns expression.loc
                expression.typeId "assignment")
              let placeErrors := if mode matches .execution then #[] else
                match staticPlaceInfo? unit ns context place with
                | some info =>
                    let writableErrors := if (mode matches .typing) || info.writable then #[]
                      else typeMismatch expression.loc "assignment targets an immutable place"
                    let valueErrors := if exprTypeAgrees ns value info.typeId ||
                        (context.logical && (exprType? ns value).any fun valueTy =>
                          specProjectedAgree ns valueTy info.typeId) then #[]
                      else typeMismatch expression.loc
                        "assigned value type differs from the place type"
                    writableErrors ++ valueErrors
                | none => #[]
              ds ++ resultErrors ++ placeErrors
          | .assignPattern pattern value =>
              let ds := scanPattern registry unit mode ns context pattern ++
                scanExpr registry unit mode ns context value
              let ds := if (mode matches .execution) ||
                  (match patternType? ns pattern, exprType? ns value with
                   | some patternTy, some valueTy => typesAgree ns patternTy valueTy
                   | _, _ => false) then ds
                else ds ++ typeMismatch expression.loc "assignment pattern and value types differ"
              ds ++ typingGate mode (unitResultDiagnostics ns expression.loc expression.typeId
                "pattern assignment")
          | .quantifier kind binders triggers condition body =>
              let context := { context with logical := true }
              let ds := match kind with
                | .profile value => profileDiagnostics registry unit mode .quantifier expression.loc value
                | _ => classificationDiagnostics mode expression.loc (coreQuantifierFeature kind)
              let ds := binders.foldl (fun ds binder =>
                ds ++ scanPattern registry unit mode ns context binder.pattern ++
                  scanExpr registry unit mode ns context binder.domain) ds
              let ds := triggers.foldl (fun ds trigger => trigger.foldl (fun ds expression =>
                ds ++ scanExpr registry unit mode ns context expression) ds) ds
              let ds := match condition with
                | some condition =>
                    let ds := ds ++ scanExpr registry unit mode ns context condition
                    if (mode matches .execution) ||
                        (exprType? ns condition).any (isBoolType ns) then ds
                    else ds ++ typeMismatch expression.loc "quantifier condition is not Bool"
                | none => ds
              let ds := ds ++ scanExpr registry unit mode ns context body
              match kind with
              | .forall | .exists =>
                  let ds := if (mode matches .execution) ||
                      isBoolType ns expression.typeId then ds else
                    ds ++ typeMismatch expression.loc "logical quantifier result is not Bool"
                  if (mode matches .execution) ||
                      (exprType? ns body).any (isBoolType ns) then ds else
                    ds ++ typeMismatch expression.loc "logical quantifier body is not Bool"
              | .choose | .chooseMin | .profile _ => ds
          | .spec block =>
              match mode with
              | .verification | .typing => scanSpecBlock registry unit mode ns context block
              | .execution => #[]
        nodeErrors ++ childErrors
end

private def scanGenericBindersInScope (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (binders scope : Array GenericBinder) : Array Diagnostic :=
  binders.foldl (fun ds binder =>
    let ds := ds ++ classificationDiagnostics mode binder.loc (match binder.kind with
      | .typeArg => feature "binder.type" .executable
      | .const => feature "binder.const" .executable
      | .lifetime => feature "binder.lifetime" .executable
      | .evidence => feature "binder.evidence" .executable)
    let ds := binder.abilities.foldl (fun ds ability =>
      ds ++ scanAbility mode binder.loc ability) ds
    binder.predicates.foldl (fun ds predicate =>
      ds ++ scanGenericPredicate registry unit mode ns binder.loc predicate scope) ds) #[]

private def scanGenericBinders (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (binders : Array GenericBinder) : Array Diagnostic :=
  scanGenericBindersInScope registry unit mode ns binders binders

private def scanSignature (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace) (signature : Signature)
    (outerGenerics : Array GenericBinder := #[]) (logical : Bool := false) :
    Array Diagnostic :=
  let scopedGenerics := outerGenerics ++ signature.generics
  let genericErrors := scanGenericBindersInScope registry unit mode ns signature.generics
    scopedGenerics
  let parameterErrors := signature.parameters.foldl (fun ds parameter =>
    ds ++ scanType registry unit mode ns parameter.typeUse.loc parameter.typeUse.typeId
      scopedGenerics (logical := logical)) #[]
  let resultErrors := signature.results.foldl (fun ds result =>
    ds ++ scanType registry unit mode ns result.loc result.typeId scopedGenerics
      (logical := logical)) #[]
  let predicateErrors := signature.predicates.foldl (fun ds predicate =>
    ds ++ scanGenericPredicate registry unit mode ns
      (signature.parameters[0]?.map (·.typeUse.loc) |>.getD ns.loc) predicate
      scopedGenerics) #[]
  genericErrors ++ parameterErrors ++ resultErrors ++ predicateErrors

private def scanAssociatedItem (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (item : AssociatedItemDecl) (ownerGenerics : Array GenericBinder) : Array Diagnostic :=
  match item.kind with
  | .type bounds default =>
      let predicateErrors := bounds.foldl (fun ds predicate =>
        ds ++ scanGenericPredicate registry unit mode ns item.loc predicate ownerGenerics) #[]
      match default with
      | some value => predicateErrors ++
          scanType registry unit mode ns value.loc value.typeId ownerGenerics
      | none => predicateErrors
  | .constant type default =>
      let typeErrors := scanType registry unit mode ns type.loc type.typeId ownerGenerics
      match default with
      | some value =>
          let expressionErrors := scanExpr registry unit mode ns {
            profile := ns.profile.getD .move, generics := ownerGenerics } value
          let resultErrors := typingGate mode
            (if exprTypeAgrees ns value type.typeId then #[] else
              typeMismatch item.loc
                "associated constant default type differs from its declaration")
          typeErrors ++ expressionErrors ++ resultErrors
      | none => typeErrors
  | .method signature defaultImplementation =>
      let signatureErrors := scanSignature registry unit mode ns signature ownerGenerics
      match defaultImplementation with
      | some target =>
          if (mode matches .typing) ||
              (resolveFunctionDeclaration? unit ns target).isSome then signatureErrors
          else signatureErrors.push <| .at "LIR-SEMANTIC-TARGET"
            "associated method default target does not resolve to a function" item.loc
      | none => signatureErrors

private def scanTrait (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace) (trait : TraitDecl) : Array Diagnostic :=
  let genericErrors := scanGenericBinders registry unit mode ns trait.generics
  let superErrors := trait.superTraits.foldl (fun ds parent =>
    ds ++ scanTraitRef registry unit mode ns trait.loc parent trait.generics) #[]
  let predicateErrors := trait.predicates.foldl (fun ds predicate =>
    ds ++ scanGenericPredicate registry unit mode ns trait.loc predicate trait.generics) #[]
  genericErrors ++ superErrors ++ predicateErrors

private def scanImplementation (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (implementation : ImplDecl) : Array Diagnostic :=
  let genericErrors := scanGenericBinders registry unit mode ns implementation.generics
  let traitErrors := scanTraitRef registry unit mode ns implementation.loc implementation.trait
    implementation.generics
  let targetErrors := scanType registry unit mode ns implementation.target.loc
    implementation.target.typeId implementation.generics
  let predicateErrors := implementation.predicates.foldl (fun ds predicate =>
    ds ++ scanGenericPredicate registry unit mode ns implementation.loc predicate
      implementation.generics) #[]
  let implementedTrait := resolveTraitDeclaration? unit ns implementation.trait.trait
  let bindingErrors := implementation.bindings.foldl (fun ds binding =>
    let valueErrors := match binding.value with
      | .type value => scanType registry unit mode ns value.loc value.typeId
          implementation.generics
      | .constant value => scanExpr registry unit mode ns {
          profile := ns.profile.getD .move, generics := implementation.generics } value
      | .method target =>
          if (mode matches .typing) ||
              (resolveFunctionDeclaration? unit ns target).isSome then #[]
          else #[.at "LIR-SEMANTIC-TARGET"
            "associated method binding target does not resolve to a function" binding.loc]
    let typeErrors := if mode matches .execution then #[] else
      match implementedTrait, binding.value with
      | some (targetNs, trait), .constant value =>
          match targetNs.associatedItems[binding.item.index]? with
          | some item =>
              match item.kind with
              | .constant declaredType _ => match exprType? ns value with
                  | some valueType =>
                      if item.owner == trait.id && typeMatchesInstantiation? ns valueType targetNs
                          declaredType.typeId implementation.trait.arguments then #[]
                      else typeMismatch binding.loc
                        "associated constant binding type differs from its declaration"
                  | none => #[]
              | _ => #[]
          | none => #[]
      | _, _ => #[]
    ds ++ valueErrors ++ typeErrors) #[]
  genericErrors ++ traitErrors ++ targetErrors ++ predicateErrors ++ bindingErrors

/-- The state labels a contract's conditions read: a label is a name of the
unit, bound by a binder over the state domain or defined by a two-state
operation whose post-state it is (`publish`, `remove`, `update`, or an
invocation's `ensures_of` or `result_of`), in any clause of the contract; a
definition may be restated, and a quantified label may be defined in its
body. A binder over the state domain binds a label, and no other binder
does. -/
private def stateLabelDiagnostics (ns : ValidatedNamespace) (contract : FunctionContract) :
    Array Diagnostic := Id.run do
  let mut ds : Array Diagnostic := #[]
  let mut bound : Array Nat := #[]
  let mut defined : Array Nat := #[]
  let mut used : Array (Nat × LocId) := #[]
  let labelName (label : Nat) : String :=
    (ns.tables.names[label]?.map (·.name)).getD s!"{label}"
  for condition in contract.conditions do
    let mut pending := #[condition.expression]
    let mut fuel := ns.expressions.size + 1
    while fuel > 0 do
      fuel := fuel - 1
      let some id := pending.back? | break
      pending := pending.pop
      let some expression := ns.expressions[id.index]? | continue
      pending := pending ++ expressionChildren expression.kind
      match expression.kind with
      | .quantifier _ binders _ _ _ =>
          for binder in binders do
            let stateDomain := (ns.expressions[binder.domain.index]?).any fun domain =>
              domain.kind matches .operation (.specification .stateDomain) _ _ _
            match binder.label, stateDomain with
            | some label, true => bound := bound.push label
            | some _, false =>
                ds := ds.push (.at "LIR-SEMANTIC-STATE-LABEL"
                  "a binder not over the state domain binds a state label" expression.loc)
            | none, true =>
                ds := ds.push (.at "LIR-SEMANTIC-STATE-LABEL"
                  "a binder over the state domain binds no state label" expression.loc)
            | none, false => pure ()
      | .operation (.specification operation) _ _ _ =>
          let range? : Option MemoryRange := match operation with
            | .behavior _ range | .functionCall _ range | .lemma _ range => some range
            | .publish range | .remove range | .update range => some range
            | .global (some label) | .exists (some label) => some { pre := none, post := some label }
            | _ => none
          if let some range := range? then
            for label in [range.pre, range.post].filterMap (fun label => label) do
              used := used.push (label, expression.loc)
            -- A state-change predicate defines its post-state; so do an
            -- invocation's `ensures_of` and `result_of`, whose post-state is
            -- the state after it.
            if operation matches .publish _ | .remove _ | .update _ | .behavior .ensuresOf _ |
                .behavior .resultOf _ then
              if let some label := range.post then defined := defined.push label
      | _ => pure ()
  for (label, loc) in used do
    if label ≥ ns.tables.names.size then
      ds := ds.push (.at "LIR-SEMANTIC-STATE-LABEL" s!"state label {label} is not a name of the unit" loc)
    else if !bound.contains label && !defined.contains label then
      ds := ds.push (.at "LIR-SEMANTIC-STATE-LABEL"
        s!"state label `{labelName label}` is neither bound by a quantifier over the state domain nor defined by a state-change predicate"
        loc)
  return ds

private def scanContract (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace) (context : ScanContext)
    (contract : FunctionContract) : Array Diagnostic :=
  let conditionErrors := contract.conditions.foldl (fun ds condition =>
    ds ++ scanCondition registry unit mode ns context condition) (stateLabelDiagnostics ns contract)
  -- A frame target is a specification expression, as in a spec block.
  let modifiesErrors := contract.modifies.foldl (fun ds expression =>
    ds ++ scanExpr registry unit mode ns { context with logical := true } expression)
    conditionErrors
  -- A parameter frame's targets are specification expressions over its
  -- formals, which bind an invocation's arguments at their types.
  let env := StaticTyping.staticEnv context.generics
  let localType (id : LocalId) : Option SemTy :=
    context.locals[id.index]?.bind (StaticTyping.resolveIn ns env ·.type.typeId)
  let frameErrors := contract.parameterFrames.foldl (fun ds frame =>
    let ds := frame.modifies.foldl (fun ds expression =>
      ds ++ scanExpr registry unit mode ns { context with logical := true } expression) ds
    match localType frame.parameter with
    | some (.function arguments _) =>
        -- Shared arguments are passed by value to behavioral predicates;
        -- their specification formals name the referent, as in `ensures_of`.
        let logicalArguments := arguments.map fun
          | .reference .shared referent => referent
          | argument => argument
        if frame.formals.toList.map localType == arguments.map some ||
            frame.formals.toList.map localType == logicalArguments.map some ||
            frame.modifiesAll && frame.formals.isEmpty then ds
        else ds.push <| .at "LIR-PARAMETER-FRAME"
          "a frame's formals differ from its parameter's argument types" frame.loc
    | _ => ds.push <| .at "LIR-PARAMETER-FRAME"
        "a frame names a parameter that is not a function value" frame.loc) modifiesErrors
  contract.reads.foldl (fun ds typeUse =>
    ds ++ scanType registry unit mode ns typeUse.loc typeUse.typeId
      context.generics) frameErrors

private def scanSpecFunction (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (function : SpecFunctionDecl) : Array Diagnostic :=
  let context : ScanContext := {
    locals := function.locals
    results := function.signature.results
    profile := function.profile
    generics := function.signature.generics
    logical := true }
  let unsupported := if mode matches .typing then #[] else
    #[.at "LIR-VERIFY-UNSUPPORTED"
      "specification functions require M4 semantics" function.loc]
  let signatureErrors := scanSignature registry unit mode ns function.signature
    (logical := true)
  let localErrors := function.locals.foldl (fun ds localDecl =>
    ds ++ scanType registry unit mode ns localDecl.type.loc localDecl.type.typeId
      function.signature.generics (logical := true)) #[]
  let profileErrors := function.profileData.foldl (fun ds value =>
    ds ++ profileDiagnostics registry unit mode .property function.loc value) #[]
  let bodyErrors := match function.body with
    | none => #[]
    | some root =>
        let ds := scanExpr registry unit mode ns context root
        match ns.expressions[root.index]? with
        | none => ds
        | some expression =>
            if !expressionCanFallThrough ns root ||
                packedDeclaredResultsMatch ns expression.typeId function.signature.results ||
                (function.signature.results.toList.all fun result =>
                  specProjectedAgree ns expression.typeId result.typeId) then ds
            else ds ++ typingGate mode (typeMismatch expression.loc
              "specification function body type differs from its declared results")
  unsupported ++ signatureErrors ++ localErrors ++ profileErrors ++ bodyErrors ++
    scanContract registry unit mode ns context function.contract

/-- A lemma: its contract states `requires`, `ensures`, and `decreases`;
its proof asserts, assumes, applies lemmas, and splits cases. -/
private def scanLemma (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (declaration : LemmaDecl) : Array Diagnostic :=
  let context : ScanContext := {
    locals := declaration.locals
    profile := declaration.profile
    generics := declaration.signature.generics
    logical := true }
  let unsupported := if mode matches .typing then #[] else
    #[.at "LIR-VERIFY-UNSUPPORTED" "lemmas require M4 semantics" declaration.loc]
  let signatureErrors := scanSignature registry unit mode ns declaration.signature
    (logical := true)
  let resultErrors := if declaration.signature.results.isEmpty then #[] else
    #[.at "LIR-LEMMA" "a lemma has no results" declaration.loc]
  let localErrors := declaration.locals.foldl (fun ds localDecl =>
    ds ++ scanType registry unit mode ns localDecl.type.loc localDecl.type.typeId
      declaration.signature.generics (logical := true)) #[]
  let profileErrors := declaration.profileData.foldl (fun ds value =>
    ds ++ profileDiagnostics registry unit mode .property declaration.loc value) #[]
  let roleErrors := declaration.contract.conditions.foldl (fun ds condition =>
    match condition.kind with
    | .requires | .ensures | .decreases => ds
    | _ => ds.push <| .at "LIR-LEMMA"
        "a lemma states only `requires`, `ensures`, and `decreases`" condition.loc) #[]
  let proofErrors := declaration.proof.foldl (fun ds condition =>
    let ds := match condition.kind with
      | .assertion | .assumption | .apply | .split => ds
      | _ => ds.push <| .at "LIR-LEMMA"
          "a proof step asserts, assumes, applies lemmas, or splits cases" condition.loc
    ds ++ scanCondition registry unit mode ns context condition) #[]
  unsupported ++ signatureErrors ++ resultErrors ++ localErrors ++ profileErrors ++ roleErrors ++
    scanContract registry unit mode ns context declaration.contract ++ proofErrors

private def scanSpecVar (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (declaration : SpecVarDecl) : Array Diagnostic :=
  let context : ScanContext := {
    locals := declaration.locals
    profile := declaration.profile
    generics := declaration.generics
    logical := true }
  let unsupported := if mode matches .typing then #[] else
    #[.at "LIR-VERIFY-UNSUPPORTED"
      "specification variables require M4 semantics" declaration.loc]
  let genericErrors := scanGenericBinders registry unit mode ns declaration.generics
  let typeErrors := scanType registry unit mode ns declaration.type.loc
    declaration.type.typeId declaration.generics (logical := true)
  let localErrors := declaration.locals.foldl (fun ds localDecl =>
    ds ++ scanType registry unit mode ns localDecl.type.loc localDecl.type.typeId
      declaration.generics (logical := true)) #[]
  let profileErrors := declaration.profileData.foldl (fun ds value =>
    ds ++ profileDiagnostics registry unit mode .property declaration.loc value) #[]
  let initializerErrors := match declaration.init with
    | none => #[]
    | some initializer =>
        let ds := scanExpr registry unit mode ns context initializer
        if exprMatchesOrIsAbrupt ns initializer declaration.type.typeId then ds
        else ds ++ typingGate mode (typeMismatch declaration.loc
          "specification variable initializer type differs from its declaration")
  unsupported ++ genericErrors ++ typeErrors ++ localErrors ++ profileErrors ++ initializerErrors

private def scanNamespaceInvariant (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (declaration : NamespaceInvariant) : Array Diagnostic :=
  let context : ScanContext := {
    locals := declaration.locals
    profile := ns.profile.getD .move
    logical := true }
  let unsupported := if mode matches .typing then #[] else
    #[.at "LIR-VERIFY-UNSUPPORTED"
      "namespace invariants require M4 semantics" declaration.loc]
  let localErrors := if mode matches .typing then #[] else
    declaration.locals.foldl (fun ds localDecl =>
      ds ++ scanType registry unit mode ns localDecl.type.loc localDecl.type.typeId
        (logical := true)) #[]
  -- The schema does not yet carry an invariant's own generic binders, so
  -- generic axioms cannot be scoped here; invariant conditions stay a
  -- preparation-stage boundary until the schema learns invariant generics.
  let conditionErrors := if mode matches .typing then #[] else
    scanCondition registry unit mode ns context declaration.condition
  unsupported ++ localErrors ++ conditionErrors

private def scanFunction (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace)
    (function : FunctionDecl FunctionBody) : Array Diagnostic :=
  let context : ScanContext := {
    locals := function.locals
    results := function.signature.results
    profile := function.profile
    generics := function.signature.generics }
  let signatureErrors := scanSignature registry unit mode ns function.signature
  -- A function's locals are shared with its specification: a `let` inside a
  -- spec block binds a local in this same table, typed in the logical domain.
  -- Execution never reaches those, and requiring them to be executable
  -- rejects the whole unit over a specification the runtime does not run,
  -- which is why namespace-level specification declarations are already
  -- skipped in this mode.
  --
  -- Dropping the declaration scan for execution loses nothing: a local the
  -- body uses carries its type on every expression that reads or writes it,
  -- and those are scanned; a local the body never uses cannot affect
  -- execution whatever its type. Parameter types are covered by the
  -- signature scan above.
  let localErrors := if mode matches .execution then #[] else
    function.locals.foldl (fun ds localDecl =>
      ds ++ scanType registry unit mode ns localDecl.type.loc localDecl.type.typeId
        function.signature.generics) #[]
  let profileErrors := function.profileData.foldl (fun ds value =>
    ds ++ profileDiagnostics registry unit mode .property function.loc value) #[]
  let bodyErrors := match function.body with
    | .absent => if mode matches .typing then #[] else
        #[.at (unsupportedCode mode)
          "absent function bodies require an external semantic implementation" function.loc]
    | .structured root =>
        let ds := scanExpr registry unit mode ns context root
        match ns.expressions[root.index]? with
        | none => ds
        | some expression =>
            if (mode matches .execution) || !expressionCanFallThrough ns root ||
                packedDeclaredResultsMatch ns expression.typeId
                function.signature.results then ds
            else ds ++ typeMismatch expression.loc
              "function fallthrough type differs from its declared results"
  let contractErrors := if mode matches .verification | .typing then
      scanContract registry unit mode ns context function.contract else #[]
  signatureErrors ++ localErrors ++ profileErrors ++ bodyErrors ++ contractErrors

/-- Attribute a declaration's diagnostics to their owner, so a report over a
whole namespace names the function or declaration each message belongs to. -/
private def ownedBy (ns : ValidatedNamespace) (kind : String) (name : NameId)
    (ds : Array Diagnostic) : Array Diagnostic :=
  match ns.tables.names[name.index]? with
  | some qualified =>
      ds.map fun d => { d with message := s!"in {kind} {qualified.name}: {d.message}" }
  | none => ds

private def scanNamespace (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) (ns : ValidatedNamespace) : Array Diagnostic :=
  let functionErrors := ns.functions.foldl (fun ds function =>
    ds ++ ownedBy ns "fun" function.name
      (scanFunction registry unit mode ns function)) #[]
  let constantErrors := ns.constants.foldl (fun ds constant =>
    let ds := ds ++ scanType registry unit mode ns constant.type.loc constant.type.typeId ++
      scanExpr registry unit mode ns { profile := ns.profile.getD .move } constant.value
    let ds := if (mode matches .execution) ||
        exprTypeAgrees ns constant.value constant.type.typeId then ds else
      ds ++ typeMismatch constant.loc "constant initializer type differs from its declaration"
    constant.profileData.foldl (fun ds value =>
      ds ++ ownedBy ns "const" constant.name
        (profileDiagnostics registry unit mode .property constant.loc value)) ds) #[]
  let structErrors := ns.structs.foldl (fun ds declaration =>
    let own := scanGenericBinders registry unit mode ns declaration.generics
    let own := declaration.abilities.foldl (fun own ability =>
      own ++ scanAbility mode declaration.loc ability) own
    let own := declaration.properties.foldl (fun own value =>
      own ++ profileDiagnostics registry unit mode .property declaration.loc value) own
    let own := declaration.fields.foldl (fun own field =>
      own ++ scanType registry unit mode ns field.type.loc field.type.typeId
        declaration.generics) own
    let own := declaration.variants.foldl (fun own variant =>
      variant.fields.foldl (fun own field =>
        own ++ scanType registry unit mode ns field.type.loc field.type.typeId
          declaration.generics) own) own
    let own := own ++ typingGate mode (structAbilityDiagnostics unit ns declaration)
    let own := if mode matches .verification | .typing then
        own ++ scanContract registry unit mode ns {
          locals := declaration.locals
          profile := ns.profile.getD .move
          generics := declaration.generics } declaration.contract
      else own
    ds ++ ownedBy ns "struct" declaration.name own) #[]
  let associatedItemErrors := ns.associatedItems.foldl (fun ds item =>
    let ownerGenerics := (ns.traits[item.owner.index]?).map (·.generics) |>.getD #[]
    ds ++ scanAssociatedItem registry unit mode ns item ownerGenerics) #[]
  let traitErrors := ns.traits.foldl (fun ds trait =>
    ds ++ scanTrait registry unit mode ns trait) #[]
  let implementationErrors := ns.implementations.foldl (fun ds implementation =>
    ds ++ scanImplementation registry unit mode ns implementation) #[]
  let logicalDeclErrors := if mode matches .execution then #[] else
    let ds := ns.specFunctions.foldl (fun ds declaration =>
      ds ++ ownedBy ns "spec fun" declaration.name
        (scanSpecFunction registry unit mode ns declaration)) #[]
    let ds := ns.specVars.foldl (fun ds declaration =>
      ds ++ ownedBy ns "spec var" declaration.name
        (scanSpecVar registry unit mode ns declaration)) ds
    let ds := ns.lemmas.foldl (fun ds declaration =>
      ds ++ ownedBy ns "lemma" declaration.name
        (scanLemma registry unit mode ns declaration)) ds
    ns.invariants.foldl (fun ds declaration =>
      ds ++ scanNamespaceInvariant registry unit mode ns declaration) ds
  let intrinsicErrors := if ns.intrinsics.isEmpty || (mode matches .typing) then #[] else
    ns.intrinsics.foldl (fun ds declaration =>
      ds.push <| .at (unsupportedCode mode)
        "intrinsic semantic models are not implemented" declaration.loc) #[]
  let metadataErrors := ns.profileMetadata.foldl (fun ds value =>
    ds ++ profileDiagnostics registry unit mode .property ns.loc value) #[]
  functionErrors ++ constantErrors ++ structErrors ++ associatedItemErrors ++ traitErrors ++
    implementationErrors ++ logicalDeclErrors ++ intrinsicErrors ++ metadataErrors

private def prepareDiagnostics (registry : SemanticsRegistry) (unit : ValidatedUnit)
    (mode : PreparationMode) : Array Diagnostic :=
  dedupDiagnostics <| unit.namespaces.foldl (fun ds ns =>
      ds ++ scanNamespace registry unit mode ns)
    (registryDiagnostics registry unit) ++
    -- Recorded once by validate; replayed here until the borrow analysis is
    -- precise enough to gate validation itself.
    unit.borrowDiagnostics

/-- Authoritative typing pass invoked by `validate` over the constructed unit.
It runs the executable and declaration typing rules exactly once; capability
classification, specification typing, and the initialization/borrow analyses
remain preparation-stage concerns until their own passes move. -/
def typingDiagnostics (unit : ValidatedUnit) : Array Diagnostic :=
  unit.namespaces.foldl (fun ds ns =>
    ds ++ scanNamespace #[] unit .typing ns) #[]

/-! ## Shared references

Certified exclusivity makes a shared reference the observed value itself
([`designs/prophetic-references.md`](../../../designs/prophetic-references.md)
§2.1): a dereference or freeze whose operand is a shared reference reads
the operand, and a place dereferencing a shared-typed base is that base.
Mutable loans keep the full prophetic treatment. The semantics decides the
first at the node, from the operand's recorded type (`sharedOperand`); the
second needs the owning function's local types, so validation records the
places in the function's borrow certificate (`sharedDereferenceSites`). -/

def isSharedReferenceType (ns : ValidatedNamespace) (typeId : TypeId)
    (typeAt : Nat → Option Ty := fun index => ns.tables.types[index]?) : Bool :=
  match typeAt typeId.index with
  | some (.reference reference) => reference.kind == .shared
  | _ => false

def sharedOperand (ns : ValidatedNamespace) (expressionAt : Nat → Option Expr)
    (typeAt : Nat → Option Ty) (arguments : Array ExprId) : Bool :=
  match arguments[0]? with
  | some id => match expressionAt id.index with
    | some expression => isSharedReferenceType ns expression.typeId typeAt
    | none => false
  | none => false

/-- The place identifiers a place-carrying operation mentions. -/
private def mentionedPlaces : ExprKind → Array PlaceId
  | .operation operation _ _ _ =>
      match operation with
      | .move place | .copy place | .read place | .write place | .drop place => #[place]
      | .borrow _ place => #[place]
      | _ => #[]
  | .assign place _ => #[place]
  | _ => #[]

private structure ReachabilityWork where
  exprSeen : Nat := 0
  placeSeen : Nat := 0
  stack : List (Expr ⊕ (Nat × Place)) := []

/-- Mark when enqueuing, so duplicate edges never consume traversal fuel. -/
private def queueReachable (expressionAt : Nat → Option Expr)
    (placeAt : Nat → Option Place) (work : ReachabilityWork)
    (node : ExprId ⊕ PlaceId) : ReachabilityWork :=
  match node with
  | .inl id =>
      if work.exprSeen.testBit id.index then work else
      match expressionAt id.index with
      | some expression =>
          { work with
            exprSeen := work.exprSeen + 2 ^ id.index,
            stack := .inl expression :: work.stack }
      | none => work
  | .inr id =>
      if work.placeSeen.testBit id.index then work else
      match placeAt id.index with
      | some place =>
          { work with
            placeSeen := work.placeSeen + 2 ^ id.index,
            stack := .inr (id.index, place) :: work.stack }
      | none => work

/-- Each valid expression/place enters the worklist at most once. The sparse
result records only dereference sites; consumers impose arena order. -/
private def markReachable (expressionAt : Nat → Option Expr)
    (placeAt : Nat → Option Place) (fuel : Nat) :
    ReachabilityWork → List (Nat × PlaceId) :=
  Nat.rec (motive := fun _ => ReachabilityWork → List (Nat × PlaceId))
    (fun _ => [])
    (fun _ visit work =>
      match work.stack with
      | [] => []
      | .inl node :: stack =>
          let work := { work with stack }
          let work := (expressionChildren node.kind).foldl
            (fun work child => queueReachable expressionAt placeAt work (.inl child)) work
          let work := (mentionedPlaces node.kind).foldl
            (fun work place => queueReachable expressionAt placeAt work (.inr place)) work
          visit work
      | .inr (index, place) :: stack =>
          let work := { work with stack }
          match place with
          | .deref base =>
              (index, base) :: visit (queueReachable expressionAt placeAt work (.inr base))
          | .field base ..
          | .subslice base _ _ _ | .downcast base _ =>
              visit (queueReachable expressionAt placeAt work (.inr base))
          | .index base index =>
              visit (queueReachable expressionAt placeAt
                (queueReachable expressionAt placeAt work (.inr base)) (.inl index))
          | .localVar _ => visit work) fuel

/-- Dereference sites reachable through expression and place edges. Marking
on enqueue makes the exact traversal bound depend only on arena sizes,
rather than repeatedly counting every namespace-wide edge for each body. -/
private def reachableDereferencesWith (ns : ValidatedNamespace)
    (expressionAt : Nat → Option Expr) (placeAt : Nat → Option Place)
    (root : ExprId) : List (Nat × PlaceId) :=
  markReachable expressionAt placeAt (ns.expressions.size + ns.places.size + 1)
    (queueReachable expressionAt placeAt {} (.inl root))

def reachableDereferences (ns : ValidatedNamespace) (root : ExprId) : List (Nat × PlaceId) :=
  reachableDereferencesWith ns (arenaGet? ns.expressions) (arenaGet? ns.places) root

/-- Whether a type may hold a shared reference; a body without such locals
has no shared dereference. -/
private def mayContainSharedReference (typeAt : Nat → Option Ty) : Nat → TypeId → Bool
  | 0, _ => true
  | fuel + 1, typeId =>
      match typeAt typeId.index with
      | some (.reference reference) =>
          reference.kind == .shared || mayContainSharedReference typeAt fuel reference.referent
      | some (.vector element _) => mayContainSharedReference typeAt fuel element
      | some (.tuple elements) =>
          elements.toList.any (mayContainSharedReference typeAt fuel)
      | some .unit | some .never | some .bool | some .character | some .string
      | some .bytes | some .address | some .signer | some (.integer ..) => false
      -- Nominal fields, generic parameters, and profile-owned types keep
      -- the full path analysis. Unknown or cyclic types also cannot justify
      -- skipping it. Mutable references may themselves contain shared ones.
      | _ => true

/-- Reuse an authored place node when possible and otherwise append it without
disturbing any producer-authored place identity. -/
def internNormalizedPlace (places : Array Place) (place : Place) : PlaceId × Array Place :=
  match places.findIdx? (· == place) with
  | some index => (⟨index⟩, places)
  | none => (⟨places.size⟩, places.push place)

/-- The variants of a nominal declaration of this namespace that declare a
field of this name. -/
private def declaringVariants? {β : Type} (tables : Tables) (ns : Namespace β)
    (reference : QualifiedRef) (field : String) : Option (Array String) := do
  if reference.namespaceId != ns.identity then none else
  let declaration ← ns.structs.find? (·.name == reference.name)
  return StaticTyping.declaringVariants tables declaration field

/-- Recover the storage path represented by the value-shaped trees emitted by
legacy frontends. In particular, Move's `borrow_field` arrives as
`borrow(select(referenceLocal))`; the reference-local base denotes a
dereferenced place before its field projection. -/
partial def normalizedExpressionPlace? {β : Type} (tables : Tables) (ns : Namespace β)
    (id : ExprId) (places : Array Place) (fuel : Nat := 0) : Option (PlaceId × Array Place) := do
  let fuel := if fuel == 0 then ns.expressions.size + 1 else fuel
  if fuel == 0 then none else
    let expression ← ns.expressions[id.index]?
    match expression.kind with
    | .localVar localId => some (internNormalizedPlace places (.localVar localId))
    | .operation (.reference .dereference) instantiations arguments _ => do
        if !instantiations.isEmpty then none else
        let [baseExpression] := arguments.toList | none
        let (base, places) ←
          normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        -- Dereferencing a reference-typed field select cancels: the select's
        -- normalized place already denotes the referent's storage path.
        let selectReference := (ns.expressions[baseExpression.index]?).any fun node =>
          (node.kind matches .operation (.data _) _ _ _) &&
            ((tables.types[node.typeId.index]?).any fun ty => ty matches .reference _)
        if selectReference then some (base, places)
        else some (internNormalizedPlace places (.deref base))
    | .operation (.data (.select reference field)) _ arguments _ => do
        let [baseExpression] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        let fieldIndex ← tables.names.findIdx? fun name =>
          name.namespaceId == reference.namespaceId && name.name == field
        some (internNormalizedPlace places (.field base reference ⟨fieldIndex⟩))
    | .operation (.data (.selectVariants reference fields)) _ arguments _ => do
        let (_, field) ← fields[0]?
        if !fields.all (·.2 == field) then none else
        let [baseExpression] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        let nameIndex (text : String) := tables.names.findIdx? fun name =>
          name.namespaceId == reference.namespaceId && name.name == text
        let fieldIndex ← nameIndex field
        -- A field place reads the field of whichever variant declares it,
        -- so it is the selection when every such variant is listed; the
        -- field of one variant is read through a downcast to it.
        let listed := fields.map (·.1)
        if (declaringVariants? tables ns reference field).any
            (StaticTyping.listsDeclaringVariants · listed) then
          some (internNormalizedPlace places (.field base reference ⟨fieldIndex⟩))
        else
          let #[variant] := listed | none
          let (base, places) := internNormalizedPlace places (.downcast base ⟨← nameIndex variant⟩)
          some (internNormalizedPlace places (.field base reference ⟨fieldIndex⟩))
    | .operation (.primitive .index) _ arguments _ => do
        let [baseExpression, index] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        some (internNormalizedPlace places (.index base index))
    | _ => none

/-! ## Local access

A direct local read is non-consuming, which a value type without `Copy` does
not allow. In executable code the access of such a read follows from the
checked types: a selection or index of it with `Copy` copies its place, the
operand of an observer (a discriminant, variant test, length, or index check)
reads its place without consuming it, and otherwise the value is moved.
References keep their reads, which the reference operations take at rest, and
specifications read locals logically. -/

/-- The type of the local a selection or index chain reads by value, when the
chain is rooted at one. -/
private def chainLocalType? {β : Type} (ns : Namespace β) : Nat → ExprId → Option TypeId
  | 0, _ => none
  | fuel + 1, id => do
      let expression ← ns.expressions[id.index]?
      match expression.kind with
      | .localVar _ => some expression.typeId
      | .operation (.data (.select ..)) _ arguments _
      | .operation (.data (.selectVariants ..)) _ arguments _
      | .operation (.primitive .index) _ arguments _ =>
          chainLocalType? ns fuel (← arguments[0]?)
      | _ => none

private partial def inferLocalAccessFrom (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (withoutCopy : TypeId → Bool) (id : ExprId) (observed : Bool)
    (state : Array Expr × Array Place) : Array Expr × Array Place :=
  let (expressions, places) := state
  let children (ids : Array ExprId) (observer : Bool := false) :=
    ids.zipIdx.foldl (init := state) fun state (child, index) =>
      inferLocalAccessFrom unit ns withoutCopy child (observer && index == 0) state
  let load (expression : Expr) (operation : Operation) (places : Array Place) :=
    (expressions.set! id.index { expression with kind := .operation operation #[] #[] }, places)
  match ns.expressions[id.index]? with
  | none => state
  | some expression => match expression.kind with
    | .localVar localId =>
        if !withoutCopy expression.typeId then state else
        let (place, places) := internNormalizedPlace places (.localVar localId)
        load expression (if observed then .read place else .move place) places
    | .operation operation _ arguments _ =>
        let chain := if operation matches .data (.select ..) | .data (.selectVariants ..) |
            .primitive .index then do
          let root ← chainLocalType? ns.toNamespace (ns.expressions.size + 1) id
          if !withoutCopy root || (!observed && withoutCopy expression.typeId) then none
          else normalizedExpressionPlace? ns.tables ns.toNamespace id places
        else none
        match chain with
        | some (place, places) =>
            load expression (if observed then .read place else .copy place) places
        | none => children arguments (observesOperand operation)
    | .spec _ | .quantifier .. => state
    | kind => children (expressionChildren kind)

/-- The expressions a function body runs: its expressions below the root,
not entering specification blocks or quantifiers, with the index
expressions of the places it accesses. -/
private def executableExpressions (ns : ValidatedNamespace) : Array Bool := Id.run do
  let mut reached := Array.replicate ns.expressions.size false
  let mut pending : Array ExprId := ns.functions.filterMap fun function =>
    match function.body with
    | .structured root => some root
    | .absent => none
  let mut places : Array PlaceId := #[]
  let mut visitedPlaces := Array.replicate ns.places.size false
  -- Each expression and place is visited once, so the arenas bound the work.
  for _ in [0:ns.expressions.size + ns.places.size + 1] do
    while let some place := places.back? do
      places := places.pop
      if visitedPlaces[place.index]?.getD true then continue
      visitedPlaces := visitedPlaces.set! place.index true
      match ns.places[place.index]? with
      | some (.deref base) | some (.field base _ _) | some (.subslice base _ _ _)
      | some (.downcast base _) => places := places.push base
      | some (.index base index) =>
          places := places.push base
          pending := pending.push index
      | some (.localVar _) | none => pure ()
    let some id := pending.back? | break
    pending := pending.pop
    if reached[id.index]?.getD true then continue
    reached := reached.set! id.index true
    let some expression := ns.expressions[id.index]? | continue
    match expression.kind with
    | .spec _ | .quantifier .. => continue
    | .operation operation _ _ _ =>
        match operation with
        | .move place | .copy place | .borrow _ place | .read place | .write place
        | .drop place => places := places.push place
        | _ => pure ()
    | .assign place _ => places := places.push place
    | _ => pure ()
    pending := pending ++ expressionChildren expression.kind
  return reached

/-- The instantiation validation solves for an elided generic call or
closure, the runtime's frame for its target. -/
private def solvedInstantiation? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (loc : LocId) (resultType : TypeId) : Operation → Array ExprId → Option (Array GenericArgument)
  | .call (.function reference), arguments => do
      let (targetNs, declaration) ← resolveFunctionDeclaration? unit ns reference
      if declaration.signature.generics.isEmpty ||
          arguments.size != declaration.signature.parameters.size then none
      let pairs ← callOccurrencePairs? ns arguments declaration.signature.parameters resultType
        declaration.signature.results
      match Unify.solveCall ns false targetNs declaration.signature.generics loc pairs with
      | .solved inferred => some inferred
      | _ => none
  | .call (.closure reference mask), captures => do
      let (targetNs, declaration) ← resolveFunctionDeclaration? unit ns reference
      if declaration.signature.generics.isEmpty then none
      let .function functionArguments functionResult _ ← ns.tables.types[resultType.index]?
        | none
      let parameters := declaration.signature.parameters.toList
      let capturedParameters := (ClosureMask.extract mask true parameters).toArray
      let remaining := (ClosureMask.extract mask false parameters).toArray
      if functionArguments.size != remaining.size then none
      let pairs ← callOccurrencePairs? ns captures capturedParameters functionResult
        declaration.signature.results
      let pairs := pairs ++ functionArguments.zip (remaining.map (·.typeUse.typeId))
      match Unify.solveCall ns false targetNs declaration.signature.generics loc pairs with
      | .solved inferred => some inferred
      | _ => none
  | _, _ => none

/-- A namespace whose executable generic calls and closures carry the
instantiations validation solved for them where the source elided them: a
target runs under its arguments, never in its symbolic frame. -/
def inferInstantiations (unit : ValidatedUnit) (ns : ValidatedNamespace) : ValidatedNamespace :=
  let executable := executableExpressions ns
  let expressions := ns.expressions.zipIdx.map fun (expression, index) =>
    if !(executable[index]?.getD false) then expression else
    match expression.kind with
    | .operation operation instantiations arguments surface =>
        if !instantiations.isEmpty then expression else
        match solvedInstantiation? unit ns expression.loc expression.typeId operation arguments with
        | some inferred => { expression with kind := .operation operation inferred arguments surface }
        | none => expression
    | _ => expression
  { ns with expressions }

/-- A namespace with the access of each direct local read in its executable
bodies inferred from the checked types. -/
def inferLocalAccess (unit : ValidatedUnit) (ns : ValidatedNamespace) : ValidatedNamespace :=
  let (expressions, places) := ns.functions.foldl (init := (ns.expressions, ns.places))
    fun state function => match function.body with
      | .absent => state
      | .structured root =>
          let withoutCopy := fun typeId =>
            !(ns.tables.types[typeId.index]? matches some (.reference _)) &&
              typeHasAbility? unit ns function.profile function.signature.generics typeId
                .copy == some false
          inferLocalAccessFrom unit ns withoutCopy root false state
  { ns with expressions, places }

/-- The places of a function's body that dereference a shared reference,
ascending: those whose base's static type, against the function's locals,
is a shared reference. -/
def sharedDereferenceSites (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) : Array PlaceId :=
  match declaration.body with
  | .absent => #[]
  | .structured root =>
      let typeAt := fun index => ns.tables.types[index]?
      if !declaration.locals.toList.any (fun entry =>
          mayContainSharedReference typeAt (ns.tables.types.size + 1) entry.type.typeId) then
        #[]
      else
        let context : ScanContext := { locals := declaration.locals }
        let sites := (reachableDereferences ns root).filter fun (_, base) =>
          (staticPlaceInfoFuel? unit ns context (ns.places.size + 1) base).any fun info =>
            isSharedReferenceType ns info.typeId
        ((sortByIndex sites).map fun (index, _) => (⟨index⟩ : PlaceId)).toArray

/-- The places of a namespace that dereference a shared reference, indexed. -/
def sharedDereferenceIndex (unit : ValidatedUnit) (namespaceId : NamespaceId) : KeyTree Unit :=
  let places := unit.borrowCertificates.toList.foldl (init := []) fun places certificate =>
    if certificate.namespaceId != namespaceId then places else
    certificate.sharedDereferences.toList.foldl (init := places) fun places place =>
      (place.index, ()) :: places
  let distinct := (sortByIndex places).foldl (init := []) fun distinct entry =>
    match distinct with
    | last :: _ => if last.1 == entry.1 then distinct else entry :: distinct
    | [] => [entry]
  KeyTree.ofSorted distinct.reverse

/-- Whether the operand of the operation at a site is a shared reference. -/
def sharedOperandAt (ns : ValidatedNamespace) (site : ExprId) : Bool :=
  match ns.expressions[site.index]? with
  | some { kind := .operation _ _ arguments _, .. } =>
      sharedOperand ns (ns.expressions[·]?) (ns.tables.types[·]?) arguments
  | _ => false

/-- Check that every reachable runtime node has a classified, currently
supported meaning and return the private interpreter input wrapper. -/
def prepareExecution (registry : SemanticsRegistry) (unit : ValidatedUnit) :
    Except (Array Diagnostic) (ExecutableUnit unit) :=
  let diagnostics := prepareDiagnostics registry unit .execution ++ loanDeathDiagnostics unit
  if diagnostics.any (·.severity == .error) then .error diagnostics
  else if typed : StaticTyping.checkUnit unit (targetPointerWidth? unit) = true then
    .ok (.mk registry (targetPointerWidth? unit)
      unit.initializationCertificates unit.borrowCertificates typed rfl)
  else .error ((StaticTyping.unitFailures unit (targetPointerWidth? unit)).map
    (Diagnostic.error "LIR-STATIC-TYPE" ·))

/-- Check that every reachable body and specification node has a classified,
currently supported logical meaning and return the private verifier input
wrapper. -/
def prepareVerification (registry : SemanticsRegistry) (unit : ValidatedUnit) :
    Except (Array Diagnostic) VerifiableUnit :=
  let diagnostics := prepareDiagnostics registry unit .verification ++ loanDeathDiagnostics unit
  if diagnostics.any (·.severity == .error) then .error diagnostics
  else .ok (.mk unit registry (targetPointerWidth? unit)
    unit.initializationCertificates unit.borrowCertificates)

/-! ## Projections of a prepared unit

A verification statement can quantify over the prepared unit and pin only
what its proof needs: the registry it was prepared under, and the target
pointer width that preparation selected. -/

theorem prepareExecution_semantics {registry : SemanticsRegistry}
    {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (prepared : prepareExecution registry unit = .ok executable) :
    executable.semantics = registry := by
  unfold prepareExecution at prepared
  simp only [] at prepared
  split at prepared
  · cases prepared
  · split at prepared
    · cases prepared; rfl
    · cases prepared

theorem prepareExecution_targetPointerWidth {registry : SemanticsRegistry}
    {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (prepared : prepareExecution registry unit = .ok executable) :
    executable.targetPointerWidth = targetPointerWidth? unit := by
  unfold prepareExecution at prepared
  simp only [] at prepared
  split at prepared
  · cases prepared
  · split at prepared
    · cases prepared; rfl
    · cases prepared

end LeanerIR.Validation

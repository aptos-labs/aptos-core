-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Registry
import LeanerLang.Operators
import LeanerLang.Syntax
import LeanerLang.Comments

namespace LeanerLang

open Lean
open Lean.Elab
open Lean.Elab.Command

-- The surface elaborator dispatches over one long syntax-kind chain, whose
-- elaboration nests one level per branch.
set_option maxRecDepth 4096

private def spanOf (stx : Syntax) : Span := {
  startByte := stx.getPos?.map (·.byteIdx) |>.getD 0
  endByte := stx.getTailPos?.map (·.byteIdx) |>.getD 0 }

private partial def identStringsOf (stx : Syntax) : Array String :=
  if stx.isIdent then #[stx.getId.getString!]
  else stx.getArgs.flatMap identStringsOf

private partial def childrenWhere (predicate : Syntax → Bool)
    (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if predicate child then found.push child
    else found ++ childrenWhere predicate child) #[]

private def isAttributeSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerAttributeSyntax || stx.isOfKind ``leanerAttributeAssignmentSyntax

private partial def attributeOf (stx : Syntax) : Except String SourceAttribute := do
  let some name := stx.getArgs.find? (·.isIdent)
    | throw "an attribute must have a name"
  if stx.isOfKind ``leanerAttributeAssignmentSyntax then
    let value := stx[2]
    let value ← if let some number := value.isNatLit? then pure (.number number)
      else if let some string := value.isStrLit? then pure (.string string)
      else if value.isIdent then pure (.name value.getId.getString!)
      else throw "an attribute value must be a number, string, or name"
    return .assign name.getId.getString! value (spanOf stx)
  return .call name.getId.getString!
    (← (childrenWhere isAttributeSyntax stx).mapM attributeOf) (spanOf stx)

private def isTypeSyntax (stx : Syntax) : Bool :=
  [``leanerUnitType, ``leanerNeverType, ``leanerBoolType, ``leanerCharType,
    ``leanerStringType, ``leanerBytesType, ``leanerAddressType, ``leanerSignerType,
    ``leanerUIntType, ``leanerSIntType, ``leanerUPtrType, ``leanerIPtrType,
    ``leanerU8Type, ``leanerU16Type, ``leanerU32Type, ``leanerU64Type,
    ``leanerU128Type, ``leanerU256Type,
    ``leanerI8Type, ``leanerI16Type, ``leanerI32Type, ``leanerI64Type,
    ``leanerI128Type, ``leanerI256Type, ``leanerUSizeType, ``leanerISizeType,
    ``leanerNatType, ``leanerIntType, ``leanerRangeType,
    ``leanerVectorType, ``leanerFixedVectorType, ``leanerFunctionType,
    ``leanerReferenceType, ``leanerTupleType, ``leanerAppliedType,
    ``leanerStandardAppliedType,
    ``leanerNamedType].contains stx.getKind

private def isTypeArgumentSyntax (stx : Syntax) : Bool :=
  [``leanerTypeArgumentSyntax, ``leanerIntegerConstTypeArgument,
    ``leanerTrueConstTypeArgument, ``leanerFalseConstTypeArgument,
    ``leanerLifetimeTypeArgument].contains stx.getKind

private def isLifetimeSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerInferenceLifetime || stx.isOfKind ``leanerNamedLifetime

private def operatorOfSyntax? (kind : SyntaxNodeKind) : Option CoreOperator :=
  if kind == ``leanerLogicalNotExpr then some .logicalNot
  else if kind == ``leanerBitwiseNotExpr then some .bitwiseNot
  else if kind == ``leanerNegateExpr then some .negate
  else if kind == ``leanerMultiplyExpr then some .multiply
  else if kind == ``leanerDivideExpr then some .divide
  else if kind == ``leanerModuloExpr then some .modulo
  else if kind == ``leanerAddExpr then some .add
  else if kind == ``leanerSubtractExpr then some .subtract
  else if kind == ``leanerShiftLeftExpr then some .shiftLeft
  else if kind == ``leanerShiftRightExpr then some .shiftRight
  else if kind == ``leanerBitwiseAndExpr then some .bitwiseAnd
  else if kind == ``leanerBitwiseXorExpr then some .bitwiseXor
  else if kind == ``leanerBitwiseOrExpr then some .bitwiseOr
  else if kind == ``leanerRangeExpr then some .range
  else if kind == ``leanerEqualExpr then some .equal
  else if kind == ``leanerNotEqualExpr then some .notEqual
  else if kind == ``leanerLessExpr then some .less
  else if kind == ``leanerGreaterExpr then some .greater
  else if kind == ``leanerLessEqualExpr then some .lessEqual
  else if kind == ``leanerGreaterEqualExpr then some .greaterEqual
  else if kind == ``leanerLogicalAndExpr then some .logicalAnd
  else if kind == ``leanerLogicalOrExpr then some .logicalOr
  else if kind == ``leanerImpliesExpr then some .implies
  else if kind == ``leanerEquivalentExpr then some .equivalent
  else none

private def isExprSyntax (stx : Syntax) : Bool :=
  (operatorOfSyntax? stx.getKind).isSome ||
  [``leanerUnitExpr, ``leanerTrueExpr, ``leanerFalseExpr, ``leanerCharExpr,
    ``leanerIntegerExpr,
    ``leanerNegativeIntegerExpr, ``leanerTypedIntegerExpr,
    ``leanerNegativeTypedIntegerExpr, ``leanerAddressExpr, ``leanerMoveAddressExpr,
    ``leanerStringExpr, ``leanerByteStringExpr,
    ``leanerBytesExpr, ``leanerLocalExpr,
    ``leanerParenExpr, ``leanerSingletonTupleExpr, ``leanerTupleExpr,
    ``leanerVectorExpr, ``leanerRepeatVectorExpr,
    ``leanerTypedVectorExpr,
    ``leanerConstructExpr, ``leanerAppliedConstructExpr,
    ``leanerStandardAppliedConstructExpr, ``leanerNamedConstructExpr,
    ``leanerSelectExpr, ``leanerFieldExpr, ``leanerStorageIndexExpr,
    ``leanerIndexExpr, ``leanerMembershipExpr,
    ``leanerVariantTestSurfaceExpr,
    ``leanerSelectVariantsExpr,
    ``leanerTestVariantsExpr, ``leanerDiscriminantExpr,
    ``leanerDiscriminantSurfaceExpr,
    ``leanerMatchExpr,
    ``leanerQuantifierExpr, ``leanerSpecificationAnchorMarkerExpr,
    ``leanerSpecificationAnchorMarkerBuiltinExpr,
    ``leanerSpecificationWithStateExpr, ``leanerSpecificationWithStateBuiltinExpr,
    ``leanerSpecificationOldExpr, ``leanerSpecificationInlineCallSummaryExpr,
    ``leanerSpecificationResultExpr,
    ``leanerBehaviorExpr, ``leanerBehaviorAtExpr,
    ``leanerBehaviorPreRangeExpr, ``leanerBehaviorPostRangeExpr,
    ``leanerBehaviorFullRangeExpr,
    ``leanerSpecificationVectorExpr, ``leanerSpecificationEmptyVectorBuiltinExpr,
    ``leanerSpecificationGlobalExpr, ``leanerSpecificationGlobalSurfaceExpr,
    ``leanerSpecificationInRangeExpr,
    ``leanerSpecificationBitVectorToIntExpr,
    ``leanerSpecificationIntToBitVectorExpr,
    ``leanerSpecificationIntToBitVectorInferExpr, ``leanerRuntimeAssertExpr,
    ``leanerRuntimeAssertMacroExpr, ``leanerThrowSurfaceExpr,
    ``leanerSpecBlockExpr, ``leanerSingleSpecExpr,
    ``leanerBlockExpr, ``leanerGenericCallExpr, ``leanerDirectGenericCallExpr,
    ``leanerTypedCallExpr,
    ``leanerTypedGenericCallExpr, ``leanerTypedCallSurfaceExpr,
    ``leanerTypedGenericCallSurfaceExpr,
    ``leanerMethodCallExpr, ``leanerTypedMethodCallExpr,
    ``leanerGlobalContainsExpr, ``leanerGlobalBorrowExpr,
    ``leanerGlobalTakeExpr, ``leanerGlobalPublishExpr,
    ``leanerGlobalExistsSurfaceExpr, ``leanerBorrowGlobalSurfaceExpr,
    ``leanerBorrowGlobalMutSurfaceExpr, ``leanerMoveFromSurfaceExpr,
    ``leanerMoveToSurfaceExpr,
    ``leanerInvokeExpr, ``leanerClosureExpr, ``leanerInvokeSurfaceExpr,
    ``leanerFunctionValueExpr,
    ``leanerBorrowPlaceSurfaceExpr, ``leanerBorrowValueSurfaceExpr,
    ``leanerDereferenceSurfaceExpr,
    ``leanerPlaceBorrowExpr, ``leanerRawPlaceBorrowExpr, ``leanerRawPlaceAssignExpr,
    ``leanerDropPlaceExpr, ``leanerDropPlaceSurfaceExpr,
    ``leanerMovePlaceSurfaceExpr, ``leanerCopyPlaceSurfaceExpr,
    ``leanerReadPlaceSurfaceExpr,
    ``leanerValueBorrowExpr, ``leanerFreezeReferenceExpr,
    ``leanerFreezeExplicitReferenceExpr, ``leanerDereferenceExpr,
    ``leanerMutateReferenceExpr, ``leanerAssignmentExpr,
    ``leanerPatternAssignmentExpr, ``leanerPatternAssignmentSurfaceExpr,
    ``leanerPrimitiveExpr, ``leanerCheckedPrimitiveExpr,
    ``leanerTypedCastExpr, ``leanerTypedCheckedCastExpr,
    ``leanerCastSurfaceExpr,
    ``leanerIfExpr, ``leanerLoopExpr, ``leanerWhileExpr, ``leanerForRangeExpr,
    ``leanerBreakExpr, ``leanerContinueExpr,
    ``leanerReturnExpr].contains stx.getKind

private def isBlockEntrySyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerStatementSyntax ||
    stx.isOfKind ``leanerLetStatementSyntax ||
    stx.isOfKind ``leanerInferredLetStatementSyntax ||
    stx.isOfKind ``leanerBareExpressionEntry ||
    stx.isOfKind ``leanerReturnBlockEntry

private def isBindingPatternSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerVariableBindingPattern ||
    stx.isOfKind ``leanerWildcardBindingPattern ||
    stx.isOfKind ``leanerTrueBindingPattern ||
    stx.isOfKind ``leanerFalseBindingPattern ||
    stx.isOfKind ``leanerCharBindingPattern ||
    stx.isOfKind ``leanerIntegerBindingPattern ||
    stx.isOfKind ``leanerNegativeIntegerBindingPattern ||
    stx.isOfKind ``leanerTupleBindingPattern ||
    stx.isOfKind ``leanerSingletonTupleBindingPattern ||
    stx.isOfKind ``leanerConstructorBindingPattern

private def isMatchArmSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerMatchArmSyntax ||
    stx.isOfKind ``leanerGuardedMatchArmSyntax

private def isIfBranchSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerInlineIfBranch ||
    stx.isOfKind ``leanerIndentedIfBranch

/-- Find only entries owned by this block. Nested block expressions have their
own statement/result scopes and must not be mistaken for entries of the outer
block. -/
private partial def blockEntries (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if isBlockEntrySyntax child then found.push child
    else if isExprSyntax child then found
    else found ++ blockEntries child) #[]

private def isPlaceSyntax (stx : Syntax) : Bool :=
  [``leanerLocalPlace, ``leanerParenPlace, ``leanerDerefPlace,
    ``leanerFieldPlace].contains stx.getKind

private def isModifierSyntax (stx : Syntax) : Bool :=
  [``leanerPrivateModifier, ``leanerPublicModifier, ``leanerPackageModifier,
    ``leanerFriendModifier, ``leanerEntryModifier, ``leanerNativeModifier,
    ``leanerOpaqueModifier, ``leanerDeprecatedModifier,
    ``leanerViewModifier].contains stx.getKind

private def isClauseSyntax (stx : Syntax) : Bool :=
  [``leanerLetPreClause, ``leanerLetPostClause,
    ``leanerRequiresClause, ``leanerEnsuresClause,
    ``leanerAbortsIfClause, ``leanerInvariantClause,
    ``leanerModifiesClause, ``leanerLooseModifiesClause, ``leanerModifiesAllClause,
    ``leanerReadsClause, ``leanerReadsAllClause].contains stx.getKind

private def isPragmaSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerPragmaClause

private def isAbilitySyntax (stx : Syntax) : Bool :=
  [``leanerCopyAbility, ``leanerDropAbility, ``leanerStoreAbility,
    ``leanerKeyAbility].contains stx.getKind

private def isGenericBinderSyntax (stx : Syntax) : Bool :=
  [``leanerTypeBinder, ``leanerInferredTypeBinder,
    ``leanerConstBinder, ``leanerTypedConstBinder, ``leanerLifetimeBinder,
    ``leanerEvidenceBinder].contains stx.getKind

private partial def declarationAbilities (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if isGenericBinderSyntax child then found
    else if isAbilitySyntax child then found.push child
    else found ++ declarationAbilities child) #[]

private def isItemSyntax (stx : Syntax) : Bool :=
  [``leanerUseItem, ``leanerFriendItem, ``leanerNamespacePragmaItem, ``leanerConstantItem, ``leanerStructItem, ``leanerEnumItem,
    ``leanerFunctionItem, ``leanerSpecFunctionItem, ``leanerContractItem,
    ``leanerContractWhereItem, ``leanerNamespaceInvariantItem].contains stx.getKind

/-- Collect category nodes without crossing into a nested expression: a
subexpression owns its own branches, arms, and binders. -/
private partial def childrenWhereOutsideExpressions (predicate : Syntax → Bool)
    (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if predicate child then found.push child
    else if isExprSyntax child then found
    else found ++ childrenWhereOutsideExpressions predicate child) #[]

/-- Nodes of `kind` that belong to this construct rather than to one of its
operand expressions. A nested call's type arguments, variant, or field names
are that call's own, the same way its identifiers are. -/
private def childrenOfKind (kind : SyntaxNodeKind) (stx : Syntax) : Array Syntax :=
  childrenWhereOutsideExpressions (·.isOfKind kind) stx

-- A construct's own type annotations are its own: a type written inside one of
-- its operand expressions — the `u64` of `f(x as u64) as u16` — belongs to that
-- expression, and collecting it here would make the cast adopt its own
-- operand's type.
private def typeChildren := childrenWhereOutsideExpressions isTypeSyntax
private def typeArgumentChildren := childrenWhereOutsideExpressions isTypeArgumentSyntax
private def lifetimeChildren := childrenWhereOutsideExpressions isLifetimeSyntax
private def exprChildren (stx : Syntax) : Array Syntax :=
  let found := childrenWhere isExprSyntax stx
  found.zipIdx.filterMap fun (child, index) =>
    let span := spanOf child
    if found.drop (index + 1) |>.any (spanOf · == span) then none else some child
private def placeChildren := childrenWhere isPlaceSyntax
private def isPathSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerPathSyntax
private def pathChildren := childrenWhere isPathSyntax
private partial def pathOutsideTypes? (stx : Syntax) : Option Syntax :=
  stx.getArgs.findSome? fun child =>
    if isPathSyntax child then some child
    else if isTypeSyntax child then none
    else pathOutsideTypes? child
private partial def signatureTypeChildren (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if isExprSyntax child then found
    else if isTypeSyntax child then found.push child
    else found ++ signatureTypeChildren child) #[]
private partial def throwChildren (stx : Syntax) : Array Syntax :=
  stx.getArgs.foldl (fun found child =>
    if child.isOfKind ``leanerAbortThrow || child.isOfKind ``leanerPanicThrow ||
        child.isOfKind ``leanerMoveVectorErrorThrow then
      found.push child
    else if isExprSyntax child then
      found
    else
      found ++ throwChildren child) #[]

private partial def identifiers (stx : Syntax) : Array String :=
  if stx.isIdent then
    let name := stx.getId
    #[if name.getPrefix.isAnonymous then name.getString! else name.toString]
  else stx.getArgs.flatMap identifiers

private partial def pathSegments (stx : Syntax) : Array String :=
  if stx.isIdent then
    let name := stx.getId
    #[if name.getPrefix.isAnonymous then name.getString! else name.toString]
  else if stx.isNatLit?.isSome then
    #[stx.reprint.getD stx.prettyPrint.pretty |>.trimAscii |>.toString]
  else stx.getArgs.flatMap pathSegments

private def unquoteIdentifier (value : String) : String :=
  if value.startsWith "«" && value.endsWith "»" then
    value.drop 1 |>.dropEnd 1 |>.toString
  else value

private def identifiersOutsideExpressions (stx : Syntax) : Array String :=
  stx.getArgs.flatMap fun child =>
    if isExprSyntax child then #[] else identifiers child

private partial def firstNat? (stx : Syntax) : Option Nat :=
  if let some value := stx.isNatLit? then some value
  else stx.getArgs.findSome? firstNat?

private def isFieldIdentifierSyntax (stx : Syntax) : Bool :=
  stx.isOfKind ``leanerNamedFieldIdentifier ||
    stx.isOfKind ``leanerNumericFieldIdentifier

private def fieldIdentifierOf (stx : Syntax) : Except String String := do
  if stx.isOfKind ``leanerNamedFieldIdentifier then
    let some name := (identifiers stx)[0]? | throw "a field must have a name"
    pure name
  else if stx.isOfKind ``leanerNumericFieldIdentifier then
    let some name := firstNat? stx | throw "a numeric field has no index"
    pure (toString name)
  else throw "expected a field identifier"

private partial def fieldIdentifierChildren (stx : Syntax) : Array Syntax :=
  if isFieldIdentifierSyntax stx then #[stx]
  else stx.getArgs.flatMap fieldIdentifierChildren

/-- Find numeric punctuation owned by a surrounding construct without
descending into its expression operands. This distinguishes `#[1u8; 4]`'s
repeat count from the value being repeated. -/
private partial def firstNatOutsideExpressions? (stx : Syntax) : Option Nat :=
  stx.getArgs.findSome? fun child =>
    if isExprSyntax child then none
    else if let some value := child.isNatLit? then some value
    else firstNatOutsideExpressions? child

private partial def allNatsOutsideExpressions (stx : Syntax) : Array Nat :=
  stx.getArgs.flatMap fun child =>
    if isExprSyntax child then #[]
    else if let some value := child.isNatLit? then #[value]
    else allNatsOutsideExpressions child

private partial def allNats (stx : Syntax) : Array Nat :=
  if let some value := stx.isNatLit? then #[value]
  else stx.getArgs.flatMap allNats

private partial def firstString? (stx : Syntax) : Option String :=
  if let some value := stx.isStrLit? then some value
  else stx.getArgs.findSome? firstString?

private partial def firstChar? (stx : Syntax) : Option Char :=
  if let some value := stx.isCharLit? then some value
  else stx.getArgs.findSome? firstChar?

private partial def containsAtom (value : String) (stx : Syntax) : Bool :=
  (stx.isAtom && stx.getAtomVal == value) || stx.getArgs.any (containsAtom value)

private def containsDirectAtom (value : String) (stx : Syntax) : Bool :=
  stx.getArgs.any fun child => child.isAtom && child.getAtomVal == value

/-- A declaration's own keyword, as opposed to one inside the binding pattern,
the declared type, or the initializer: `let bucket := &mut values` declares an
immutable binding, and so does a `let` whose initializer block has `mut` in it. -/
private partial def containsDeclarationAtom (value : String) (stx : Syntax) : Bool :=
  if isBindingPatternSyntax stx || isTypeSyntax stx || isExprSyntax stx then false
  else (stx.isAtom && stx.getAtomVal == value) ||
    stx.getArgs.any (containsDeclarationAtom value)

private partial def containsAtomOutsideTypes (value : String) (stx : Syntax) : Bool :=
  if isTypeSyntax stx then false
  else (stx.isAtom && stx.getAtomVal == value) ||
    stx.getArgs.any (containsAtomOutsideTypes value)

/-- An atom this construct writes itself, as opposed to one inside an operand
it takes: the `mut` of `&mut x` is the borrow's own, while the one in
`&(do let y := &mut z; return y)` belongs to the value being borrowed. -/
private partial def containsAtomOutsideExpressions (value : String) (stx : Syntax) : Bool :=
  (stx.isAtom && stx.getAtomVal == value) || stx.getArgs.any fun child =>
    if isExprSyntax child || isPlaceSyntax child || isTypeSyntax child then false
    else containsAtomOutsideExpressions value child

private def pathOf (stx : Syntax) : Except String (Array String) := do
  let path := pathSegments stx
  if path.isEmpty then throw "expected a nonempty Leaner path"
  if path[0]!.front?.any Char.isDigit && !path[0]!.startsWith "0x" then
    throw s!"numeric Leaner path `{String.intercalate "::" path.toList}` must start with a hexadecimal Move address"
  return path

private partial def placeOf (stx : Syntax) : Except String Place := do
  let span := spanOf stx
  if stx.isOfKind ``leanerLocalPlace then
    let some name := (identifiers stx)[0]? | throw "a local place must have a name"
    let segments := name.splitOn "." |>.map unquoteIdentifier
    let some base := segments[0]? | throw "a local place must have a name"
    pure <| (segments.drop 1).foldl (fun place field => .field place field span)
      (.local base span)
  else if stx.isOfKind ``leanerParenPlace then
    let some base := (placeChildren stx)[0]? | throw "a parenthesized place is empty"
    placeOf base
  else if stx.isOfKind ``leanerDerefPlace then
    let some base := (placeChildren stx)[0]? | throw "a dereference place has no base"
    pure (.deref (← placeOf base) span)
  else if stx.isOfKind ``leanerFieldPlace then
    let some base := (placeChildren stx)[0]? | throw "a field place has no base"
    let some fieldSyntax := (fieldIdentifierChildren stx).back?
      | throw "a field place has no field name"
    let name ← fieldIdentifierOf fieldSyntax
    pure <| name.splitOn "." |>.map unquoteIdentifier |>.foldl
      (fun place field => .field place field span)
      (← placeOf base)
  else
    throw s!"unknown Leaner place syntax `{stx.getKind}`"

private partial def placeOfExpression : Expr → Except String Place
  | .local name span => pure (.local name span)
  | .field value name span => do pure (.field (← placeOfExpression value) name span)
  | .dereference value span => do pure (.deref (← placeOfExpression value) span)
  | expression => throw s!"expression `{repr expression}` is not an assignable place"

private def profileOf (stx : Syntax) : Except String ProfileName :=
  if stx.isOfKind ``leanerMoveProfile then pure .move
  else if stx.isOfKind ``leanerRustProfile then pure .rust
  else throw s!"unknown Leaner profile stx `{stx.getKind}`"

/-- Lean's identifier token may absorb `receiver.field.method` before the
LeanerLang postfix parser sees the dots. Recover the intended receiver chain
from that token at the AST boundary. `::` remains the namespace separator, so
an identifier containing a dot is unambiguously a field/method surface. -/
private def dottedReceiver? (value : String) (span : Span) : Option (Expr × String) := do
  let segments := value.splitOn "." |>.toArray.map unquoteIdentifier
  guard (segments.size >= 2)
  let receiverSegments := segments.extract 0 (segments.size - 1)
  let base ← receiverSegments[0]?
  let receiver := (receiverSegments.drop 1).foldl
    (fun value field => Expr.field value field span) (.local base span)
  pure (receiver, segments.back!)

private def abilityOf (stx : Syntax) : Except String Ability :=
  if stx.isOfKind ``leanerCopyAbility then pure .copy
  else if stx.isOfKind ``leanerDropAbility then pure .drop
  else if stx.isOfKind ``leanerStoreAbility then pure .store
  else if stx.isOfKind ``leanerKeyAbility then pure .key
  else throw s!"unknown ability `{stx.getKind}`"

private def lifetimeOf (stx : Syntax) : Except String SourceLifetime := do
  if stx.isOfKind ``leanerInferenceLifetime then pure .inference
  else if stx.isOfKind ``leanerNamedLifetime then
    let some name := (identifiers stx)[0]?
      | throw "a named lifetime must have a name"
    let name := unquoteIdentifier name
    if name == "'static" || name == "static" then pure .static
    else pure (.parameter name)
  else throw s!"unsupported lifetime syntax `{stx.getKind}`"

mutual
private partial def typeOf (stx : Syntax) : Except String (Located Ty) := do
  let span := spanOf stx
  let value ←
    if stx.isOfKind ``leanerUnitType then pure Ty.unit
    else if stx.isOfKind ``leanerNeverType then pure .never
    else if stx.isOfKind ``leanerBoolType then pure .bool
    else if stx.isOfKind ``leanerCharType then pure .char
    else if stx.isOfKind ``leanerStringType then pure .string
    else if stx.isOfKind ``leanerBytesType then pure .bytes
    else if stx.isOfKind ``leanerAddressType then pure .address
    else if stx.isOfKind ``leanerSignerType then pure .signer
    else if stx.isOfKind ``leanerUIntType then
      let some width := firstNat? stx | throw "`UInt` requires a numeric width"
      pure (.uint width)
    else if stx.isOfKind ``leanerSIntType then
      let some width := firstNat? stx | throw "`SInt` requires a numeric width"
      pure (.sint width)
    else if stx.isOfKind ``leanerUPtrType then pure .uptr
    else if stx.isOfKind ``leanerIPtrType then pure .iptr
    else if stx.isOfKind ``leanerU8Type then pure (.uint 8)
    else if stx.isOfKind ``leanerU16Type then pure (.uint 16)
    else if stx.isOfKind ``leanerU32Type then pure (.uint 32)
    else if stx.isOfKind ``leanerU64Type then pure (.uint 64)
    else if stx.isOfKind ``leanerU128Type then pure (.uint 128)
    else if stx.isOfKind ``leanerU256Type then pure (.uint 256)
    else if stx.isOfKind ``leanerI8Type then pure (.sint 8)
    else if stx.isOfKind ``leanerI16Type then pure (.sint 16)
    else if stx.isOfKind ``leanerI32Type then pure (.sint 32)
    else if stx.isOfKind ``leanerI64Type then pure (.sint 64)
    else if stx.isOfKind ``leanerI128Type then pure (.sint 128)
    else if stx.isOfKind ``leanerI256Type then pure (.sint 256)
    else if stx.isOfKind ``leanerUSizeType then pure .uptr
    else if stx.isOfKind ``leanerISizeType then pure .iptr
    else if stx.isOfKind ``leanerNatType then pure .nat
    else if stx.isOfKind ``leanerIntType then pure .int
    else if stx.isOfKind ``leanerRangeType then pure .range
    else if stx.isOfKind ``leanerVectorType then
      let some element := (typeChildren stx)[0]?
        | throw "`Vector` requires an element type"
      pure (.vector (← typeOf element))
    else if stx.isOfKind ``leanerFixedVectorType then
      let some element := (typeChildren stx)[0]?
        | throw "fixed `Vector` requires an element type"
      let some length := firstNat? stx | throw "fixed `Vector` requires a length"
      pure (.vector (← typeOf element) (some (Int.ofNat length)))
    else if stx.isOfKind ``leanerFunctionType then
      let types := typeChildren stx
      let some result := types.back? | throw "a function type requires a result type"
      let arguments ← (types.pop).mapM typeOf
      pure (.function arguments (← typeOf result)
        (← (childrenWhere isAbilitySyntax stx).mapM abilityOf))
    else if stx.isOfKind ``leanerReferenceType then
      let some referent := (typeChildren stx)[0]?
        | throw "a reference requires a referent type"
      let lifetime ← match (lifetimeChildren stx)[0]? with
        | some lifetime => lifetimeOf lifetime
        | none => pure .inference
      pure (.reference (containsAtomOutsideExpressions "mut" stx) (← typeOf referent) lifetime)
    else if stx.isOfKind ``leanerTupleType then
      pure (.tuple (← (typeChildren stx).mapM typeOf))
    else if stx.isOfKind ``leanerAppliedType || stx.isOfKind ``leanerStandardAppliedType then
      let some path := (pathChildren stx)[0]?
        | throw "an applied nominal type requires a path"
      pure (.named (← pathOf path) (← (typeArgumentChildren stx).mapM typeArgumentOf))
    else if stx.isOfKind ``leanerNamedType then
      let some path := (pathChildren stx)[0]?
        | throw "a nominal type requires a path"
      pure (.named (← pathOf path) #[])
    else throw s!"unsupported Leaner type stx `{stx.getKind}`"
  return { value, span }

private partial def typeArgumentOf (stx : Syntax) : Except String TypeArgument := do
  if stx.isOfKind ``leanerTypeArgumentSyntax then
    let some type := (typeChildren stx)[0]?
      | throw "a type generic argument requires a type"
    pure (.type (← typeOf type).value)
  else if stx.isOfKind ``leanerIntegerConstTypeArgument then
    let some value := firstNat? stx | throw "a const generic argument requires an integer"
    pure (.constInteger (if containsAtom "-" stx then -(Int.ofNat value) else Int.ofNat value))
  else if stx.isOfKind ``leanerTrueConstTypeArgument then pure (.constBool true)
  else if stx.isOfKind ``leanerFalseConstTypeArgument then pure (.constBool false)
  else if stx.isOfKind ``leanerLifetimeTypeArgument then
    let some lifetime := (lifetimeChildren stx)[0]?
      | throw "a lifetime generic argument requires a lifetime"
    pure (.lifetime (← lifetimeOf lifetime))
  else throw s!"unsupported generic argument syntax `{stx.getKind}`"
end

private def storageHeadOf (stx : Syntax) : Except String (Located Ty) := do
  let some path := (pathChildren stx)[0]?
    | throw "a storage index must name a resource type"
  let arguments ← if stx.isOfKind ``leanerAppliedStorageHead then
      (typeChildren stx).mapM fun argument => do
        pure (.type (← typeOf argument).value)
    else if stx.isOfKind ``leanerNamedStorageHead then pure #[]
    else throw s!"unsupported storage-index head syntax `{stx.getKind}`"
  pure { value := .named (← pathOf path) arguments, span := spanOf stx }

private partial def bindingPatternOf (stx : Syntax) : Except String BindingPattern := do
  let span := spanOf stx
  if stx.isOfKind ``leanerVariableBindingPattern then
    let some name := (identifiers stx)[0]? | throw "a variable pattern must have a name"
    pure (.variable name span (containsAtom "mut" stx))
  else if stx.isOfKind ``leanerWildcardBindingPattern then
    pure (.wildcard span)
  else if stx.isOfKind ``leanerTrueBindingPattern then
    pure (.literal (.bool true) span)
  else if stx.isOfKind ``leanerFalseBindingPattern then
    pure (.literal (.bool false) span)
  else if stx.isOfKind ``leanerCharBindingPattern then
    let some value := firstChar? stx | throw "a character pattern has an invalid literal"
    pure (.literal (.char value.toNat) span)
  else if stx.isOfKind ``leanerIntegerBindingPattern ||
      stx.isOfKind ``leanerNegativeIntegerBindingPattern then
    let some value := firstNat? stx | throw "an integer pattern has an invalid literal"
    let value := if stx.isOfKind ``leanerNegativeIntegerBindingPattern then
      -(Int.ofNat value) else Int.ofNat value
    pure (.literal (.integer value) span)
  else if stx.isOfKind ``leanerTupleBindingPattern ||
      stx.isOfKind ``leanerSingletonTupleBindingPattern then
    let elements := childrenWhereOutsideExpressions isBindingPatternSyntax stx
    pure (.tuple (← elements.mapM bindingPatternOf) span)
  else if stx.isOfKind ``leanerConstructorBindingPattern then
    let some owner := (typeChildren stx)[0]?
      | throw "a constructor pattern must name its nominal type"
    let fields ← (childrenOfKind ``leanerBindingPatternFieldSyntax stx).mapM fun field => do
      let some fieldSyntax := (fieldIdentifierChildren field)[0]?
        | throw "a constructor-pattern field must have a name"
      let name ← fieldIdentifierOf fieldSyntax
      let some child := (childrenWhereOutsideExpressions isBindingPatternSyntax field)[0]?
        | throw s!"constructor-pattern field `{name}` has no child pattern"
      pure (name, ← bindingPatternOf child)
    let variant := (childrenOfKind ``leanerBindingPatternVariantSyntax stx)[0]?.bind
      fun variantSyntax => (identifiers variantSyntax)[0]?
    pure (.constructor (← typeOf owner) variant fields span)
  else throw s!"unknown binding pattern syntax `{stx.getKind}`"

private def throwOf (stx : Syntax) : Except String ThrowKind :=
  if stx.isOfKind ``leanerAbortThrow then pure .abort
  else if stx.isOfKind ``leanerPanicThrow then pure .panic
  else if stx.isOfKind ``leanerMoveVectorErrorThrow then pure .moveVectorError
  else throw "expected `abort` or `panic`"

private def primitiveOf (name : String) (failure : Option ThrowKind) : Except String Primitive :=
  match name, failure with
  | "tuple", none => pure .tuple
  | "vector", none => pure .vector
  | "pushVector", none => pure .pushVector
  | "concatVector", none => pure .concatVector
  | "insertVector", none => pure .insertVector
  | "removeVector", none => pure .removeVector
  | "swapVector", none => pure .swapVector
  | "reverseSliceVector", none => pure .reverseSliceVector
  | "destroyEmptyVector", none => pure .destroyEmptyVector
  | "containsVector", none => pure .containsVector
  | "indexOfVector", none => pure .indexOfVector
  | "checkVectorIndex", some failure => pure (.checkVectorIndex failure)
  | "checkVectorIndexAbort", none => pure (.checkVectorIndex .abort)
  | "checkVectorIndexPanic", none => pure (.checkVectorIndex .panic)
  | "length", none => pure .length
  | "index", none => pure .index
  | "slice", none => pure .slice
  | "range", none => pure .range
  | "add", none => pure .add
  | "checkedAdd", some failure => pure (.checkedAdd failure)
  | "checkedAddAbort", none => pure (.checkedAdd .abort)
  | "checkedAddPanic", none => pure (.checkedAdd .panic)
  | "subtract", none => pure .subtract
  | "checkedSubtract", some failure => pure (.checkedSubtract failure)
  | "checkedSubtractAbort", none => pure (.checkedSubtract .abort)
  | "checkedSubtractPanic", none => pure (.checkedSubtract .panic)
  | "multiply", none => pure .multiply
  | "checkedMultiply", some failure => pure (.checkedMultiply failure)
  | "checkedMultiplyAbort", none => pure (.checkedMultiply .abort)
  | "checkedMultiplyPanic", none => pure (.checkedMultiply .panic)
  | "overflowingAdd", none => pure .overflowingAdd
  | "overflowingSubtract", none => pure .overflowingSubtract
  | "overflowingMultiply", none => pure .overflowingMultiply
  | "divide", none => pure .divide
  | "checkedDivide", some failure => pure (.checkedDivide failure)
  | "checkedDivideAbort", none => pure (.checkedDivide .abort)
  | "checkedDividePanic", none => pure (.checkedDivide .panic)
  | "modulo", none => pure .modulo
  | "checkedModulo", some failure => pure (.checkedModulo failure)
  | "checkedModuloAbort", none => pure (.checkedModulo .abort)
  | "checkedModuloPanic", none => pure (.checkedModulo .panic)
  | "bitwiseOr", none => pure .bitwiseOr
  | "bitwiseAnd", none => pure .bitwiseAnd
  | "bitwiseXor", none => pure .bitwiseXor
  | "bitwiseNot", none => pure .bitwiseNot
  | "shiftLeft", none => pure .shiftLeft
  | "checkedShiftLeft", some failure => pure (.checkedShiftLeft failure)
  | "checkedShiftLeftAbort", none => pure (.checkedShiftLeft .abort)
  | "checkedShiftLeftPanic", none => pure (.checkedShiftLeft .panic)
  | "shiftRight", none => pure .shiftRight
  | "checkedShiftRight", some failure => pure (.checkedShiftRight failure)
  | "checkedShiftRightAbort", none => pure (.checkedShiftRight .abort)
  | "checkedShiftRightPanic", none => pure (.checkedShiftRight .panic)
  | "logicalAnd", none => pure .eagerLogicalAnd
  | "logicalOr", none => pure .eagerLogicalOr
  | "logicalNot", none => pure .logicalNot
  | "equal", none => pure .equal
  | "notEqual", none => pure .notEqual
  | "less", none => pure .less
  | "greater", none => pure .greater
  | "lessEqual", none => pure .lessEqual
  | "greaterEqual", none => pure .greaterEqual
  | "negate", none => pure .negate
  | "checkedNegate", some failure => pure (.checkedNegate failure)
  | "checkedNegateAbort", none => pure (.checkedNegate .abort)
  | "checkedNegatePanic", none => pure (.checkedNegate .panic)
  | "cast", none => pure .cast
  | "checkedCast", some failure => pure (.checkedCast failure)
  | "implies", none => pure .implies
  | "equivalent", none => pure .equivalent
  | "identical", none => pure .identical
  | "copyValue", none => pure .copyValue
  | "moveValue", none => pure .moveValue
  | _, some _ => throw s!"primitive `{name}` does not accept an abort/panic annotation"
  | _, none => throw s!"unknown primitive `{name}` or missing checked-operation failure annotation"

mutual
private partial def blockExpressionOfEntries (entries : Array Syntax) (span : Span) :
    Except String Expr := do
  let statements : Array Statement ←
      (entries.filter (·.isOfKind ``leanerStatementSyntax)).mapM fun statement => do
    let some expression := (exprChildren statement)[0]?
      | throw "a `do` statement must contain an expression"
    pure (Statement.expression (← expressionOf expression))
  let bareExpressions ←
      (entries.filter (·.isOfKind ``leanerBareExpressionEntry)).mapM fun statement => do
    let some expression := (exprChildren statement)[0]?
      | throw "a bare `do` entry must contain an expression"
    pure (statement, Statement.expression (← expressionOf expression))
  let returns ← (entries.filter fun entry =>
      entry.isOfKind ``leanerReturnBlockEntry &&
        containsAtomOutsideExpressions ";" entry).mapM fun statement => do
    let some expression := (exprChildren statement)[0]?
      | throw "a `return` statement must contain an expression"
    pure (statement, Statement.expression (.return_ (← expressionOf expression) (spanOf statement)))
  let declarations ← (entries.filter fun entry =>
      entry.isOfKind ``leanerLetStatementSyntax ||
        entry.isOfKind ``leanerInferredLetStatementSyntax).mapM fun statement => do
    let patterns := childrenWhereOutsideExpressions isBindingPatternSyntax statement
    let some pattern := patterns[0]? | throw "a `let` statement must contain a pattern"
    let type ← if statement.isOfKind ``leanerLetStatementSyntax then do
        let types := signatureTypeChildren statement
        let some type := types.back? | throw "an annotated `let` statement must have a type"
        pure (some (← typeOf type))
      else pure none
    let some value := (exprChildren statement)[0]?
      | throw "a `let` statement must have an initializer"
    -- Only the declaration's own `mut` marks every binding; a `mut` inside
    -- the pattern belongs to the binding it precedes.
    pure (statement, Statement.letDecl (containsDeclarationAtom "mut" statement)
      (← bindingPatternOf pattern) type (← expressionOf value) (spanOf statement))
  let explicitResults := entries.filter fun entry =>
    entry.isOfKind ``leanerReturnBlockEntry && !containsAtomOutsideExpressions ";" entry
  if explicitResults.size > 1 then throw "a `do` block has more than one final return"
  -- A final bare expression is the block result. Lowering may still discard
  -- it when the surrounding block expects Unit; an explicit final `return`
  -- remains marked so an incompatible value is diagnosed rather than
  -- silently discarded. An earlier return is an abrupt function exit.
  let trailingBare := if explicitResults.isEmpty then
      match bareExpressions.back? with
      | some candidate =>
          if entries.all fun entry =>
              (spanOf entry).startByte <= (spanOf candidate.1).startByte then
            some candidate
          else none
      | none => none
    else none
  let trailingBareStart := trailingBare.map fun (syntaxNode, _) =>
    (spanOf syntaxNode).startByte
  let orderedWithLocations : Array (Nat × Statement) :=
    (statements.map fun statement => match statement with
      | Statement.expression value => (value.span.startByte, statement)
      | Statement.letDecl .. => (0, statement)).append
    (declarations.map fun (syntaxNode, declaration) =>
      ((spanOf syntaxNode).startByte, declaration))
    |>.append (returns.map fun (syntaxNode, statement) =>
      ((spanOf syntaxNode).startByte, statement))
    |>.append (bareExpressions.map fun (syntaxNode, statement) =>
      ((spanOf syntaxNode).startByte, statement))
    |>.qsort (·.1 < ·.1)
  -- `return expr;` is intentionally accepted both as a general expression
  -- statement and as the dedicated block-return entry. Lean's category
  -- parser can expose both wrappers; retain one statement at that source
  -- position so lowering does not emit the early return twice.
  let deduplicated := orderedWithLocations.foldl (init := #[]) fun entries entry =>
    if entries.back?.any (·.1 == entry.1) then entries else entries.push entry
  -- The same source `return` can also surface as a plain statement wrapper.
  -- Keep exactly one copy: the block result.
  let explicitResultStart := explicitResults[0]?.map fun result => (spanOf result).startByte
  let ordered := deduplicated.filter (fun entry =>
      !trailingBareStart.any (· == entry.1) &&
        !explicitResultStart.any (· == entry.1)) |>.map (·.2)
  let result ← match explicitResults[0]? with
    | some result => do
        let some expression := (exprChildren result)[0]?
          | throw "a `do` result must contain an expression"
        pure (some (.return_ (← expressionOf expression) (spanOf result)))
    | none => trailingBare.mapM fun (_, statement) => match statement with
        | .expression expression => pure expression
        | .letDecl .. => throw "an internal bare-expression entry became a declaration"
  pure (.block ordered result span)

private partial def ifBranchExpressionOf (stx : Syntax) : Except String Expr := do
  if stx.isOfKind ``leanerInlineIfBranch then
    let some expression := (exprChildren stx)[0]?
      | throw "an inline `if` branch must contain an expression"
    expressionOf expression
  else if stx.isOfKind ``leanerIndentedIfBranch then
    blockExpressionOfEntries (blockEntries stx) (spanOf stx)
  else
    throw "expected an inline expression or indented `if` branch"

private partial def expressionOf (stx : Syntax) : Except String Expr := do
  let span := spanOf stx
  if stx.isOfKind ``leanerUnitExpr then pure (.unit span)
  else if stx.isOfKind ``leanerTrueExpr then pure (.bool true span)
  else if stx.isOfKind ``leanerFalseExpr then pure (.bool false span)
  else if stx.isOfKind ``leanerCharExpr then
    let some value := firstChar? stx | throw "expected a character literal"
    pure (.char value.toNat span)
  else if stx.isOfKind ``leanerIntegerExpr then
    let some value := firstNat? stx | throw "expected an integer literal"
    pure (.integer (Int.ofNat value) span)
  else if stx.isOfKind ``leanerNegativeIntegerExpr then
    let some value := firstNat? stx | throw "expected a negative integer literal"
    pure (.integer (-(Int.ofNat value)) span)
  else if stx.isOfKind ``leanerTypedIntegerExpr ||
      stx.isOfKind ``leanerNegativeTypedIntegerExpr then
    let some value := firstNat? stx | throw "expected a typed integer literal"
    let suffixes := #["u8", "u16", "u32", "u64", "u128", "u256",
      "i8", "i16", "i32", "i64", "i128", "i256", "usize", "isize"]
    let some suffix := suffixes.find? fun suffix => containsAtom suffix stx
      | throw "expected an integer type suffix"
    let type ← match suffix with
      | "u8" => pure (.uint 8)
      | "u16" => pure (.uint 16)
      | "u32" => pure (.uint 32)
      | "u64" => pure (.uint 64)
      | "u128" => pure (.uint 128)
      | "u256" => pure (.uint 256)
      | "i8" => pure (.sint 8)
      | "i16" => pure (.sint 16)
      | "i32" => pure (.sint 32)
      | "i64" => pure (.sint 64)
      | "i128" => pure (.sint 128)
      | "i256" => pure (.sint 256)
      | "usize" => pure .uptr
      | "isize" => pure .iptr
      | suffix => throw s!"unsupported integer type suffix `{suffix}`"
    let value := if stx.isOfKind ``leanerNegativeTypedIntegerExpr then
        -(Int.ofNat value) else Int.ofNat value
    pure (.typedInteger value { value := type, span } span)
  else if stx.isOfKind ``leanerAddressExpr then
    let some value := firstString? stx | throw "expected an address literal"
    pure (.address value span)
  else if stx.isOfKind ``leanerMoveAddressExpr then
    let some _ := firstNat? stx | throw "expected a Move address literal"
    let source := stx.reprint.getD stx.prettyPrint.pretty |>.trimAscii |>.toString
    let value := (source.drop 1).trimAscii.toString
    pure (.address value span)
  else if stx.isOfKind ``leanerStringExpr then
    let some value := firstString? stx | throw "expected a string literal"
    pure (.string value span)
  else if stx.isOfKind ``leanerByteStringExpr then
    let some value := firstString? stx | throw "expected a byte-string literal"
    pure (.bytes value.toUTF8.data span)
  else if stx.isOfKind ``leanerBytesExpr then
    let values := allNats stx
    if let some invalid := values.find? (255 < ·) then
      throw s!"byte literal element {invalid} is outside the range 0..255"
    pure (.bytes (values.map UInt8.ofNat) span)
  else if stx.isOfKind ``leanerLocalExpr then
    let some name := (identifiers stx)[0]? | throw "expected a local name"
    let segments := name.splitOn "." |>.map unquoteIdentifier
    let some base := segments[0]? | throw "expected a local name"
    pure <| (segments.drop 1).foldl (fun value field => .field value field span)
      (.local base span)
  else if stx.isOfKind ``leanerParenExpr then
    let some inner := (exprChildren stx)[0]?
      | throw "parenthesized expression is empty"
    expressionOf inner
  else if stx.isOfKind ``leanerSingletonTupleExpr then
    let some inner := (exprChildren stx)[0]?
      | throw "a singleton tuple must contain one expression"
    pure (.primitive .tuple #[← expressionOf inner] span)
  else if stx.isOfKind ``leanerTupleExpr then
    pure (.primitive .tuple (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerVectorExpr then
    pure (.primitive .vector (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerRepeatVectorExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a repeated vector literal must contain one value"
    let some length := firstNatOutsideExpressions? stx
      | throw "a repeated vector literal has an invalid length"
    pure (.primitive (.repeatVector length) #[← expressionOf value] span)
  else if stx.isOfKind ``leanerTypedVectorExpr then
    let some element := (typeChildren stx)[0]?
      | throw "a typed vector literal requires an element type"
    let result : Located Ty := { value := .vector (← typeOf element), span }
    pure (.typedPrimitive .vector result (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerThrowSurfaceExpr then
    let kind := if containsAtomOutsideExpressions "abort" stx then ThrowKind.abort
      else if containsAtomOutsideExpressions "moveVectorError" stx then .moveVectorError
      else .panic
    pure (.throw_ kind (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerRuntimeAssertExpr ||
      stx.isOfKind ``leanerRuntimeAssertMacroExpr then
    let [condition, code] := (exprChildren stx).toList
      | throw "a runtime assertion requires a condition and an abort code"
    pure (.ifElse (← expressionOf condition) (.unit span)
      (some (.throw_ .abort #[← expressionOf code] span)) span)
  else if stx.isOfKind ``leanerBehaviorExpr ||
      stx.isOfKind ``leanerBehaviorAtExpr ||
      stx.isOfKind ``leanerBehaviorPreRangeExpr ||
      stx.isOfKind ``leanerBehaviorPostRangeExpr ||
      stx.isOfKind ``leanerBehaviorFullRangeExpr then
    let expressions := exprChildren stx
    let some target := expressions[0]?
      | throw "a behavior predicate requires a function-value target"
    let numbers := allNatsOutsideExpressions stx
    let kind ← if containsAtom "requires_of" stx then pure BehaviorOperation.requiresOf
      else if containsAtom "aborts_of" stx then pure .abortsOf
      else if containsAtom "ensures_of" stx then pure .ensuresOf
      else if containsAtom "result_of" stx then pure .resultOf
      else if containsAtom "unchanged_of" stx then pure .unchangedOf
      else if containsAtom "folds_of" stx then pure .foldsOf
      else if containsAtom "write_of" stx then match numbers.back? with
        | some index => pure (.writeOf index)
        | none => throw "write_of requires a mutable-reference result index"
      else throw "unknown behavior predicate"
    let range : SpecificationMemoryRange ←
      if stx.isOfKind ``leanerBehaviorAtExpr ||
          stx.isOfKind ``leanerBehaviorPreRangeExpr then
        match numbers[0]? with
        | some pre => pure { pre := some pre }
        | none => throw "a pre-state behavior predicate requires a numeric state label"
      else if stx.isOfKind ``leanerBehaviorPostRangeExpr then
        match numbers[0]? with
        | some post => pure { post := some post }
        | none => throw "a post-state behavior predicate requires a numeric state label"
      else if stx.isOfKind ``leanerBehaviorFullRangeExpr then
        match numbers[0]?, numbers[1]? with
        | some pre, some post => pure { pre := some pre, post := some post }
        | _, _ => throw "a ranged behavior predicate requires pre- and post-state labels"
      else pure {}
    let target ← expressionOf target
    let values ← (expressions.drop 1).mapM expressionOf
    pure (.specification (.behavior kind range) #[] (#[target] ++ values) span)
  else if let some operation := operatorOfSyntax? stx.getKind then
    let some info := Operators.info? operation
      | throw s!"operator `{repr operation}` is missing from the shared table"
    let arguments := exprChildren stx
    let expectedArity := if info.fixity == .prefix then 1 else 2
    unless arguments.size == expectedArity do
      throw s!"operator `{info.symbol}` requires {expectedArity} operand(s)"
    pure (.primitive (Operators.toPrimitive operation)
      (← arguments.mapM expressionOf) span)
  else if stx.isOfKind ``leanerMatchExpr then
    let expressions := exprChildren stx
    let some scrutinee := expressions[0]?
      | throw "a match expression must contain a scrutinee"
    let armSyntaxes := childrenWhereOutsideExpressions isMatchArmSyntax stx
      |>.qsort fun left right => (spanOf left).startByte < (spanOf right).startByte
    if armSyntaxes.isEmpty then throw "a match expression must contain at least one arm"
    let arms ← armSyntaxes.mapM fun arm => do
      let some pattern := (childrenWhereOutsideExpressions isBindingPatternSyntax arm)[0]?
        | throw "a match arm must contain a pattern"
      let armExpressions := exprChildren arm
      if arm.isOfKind ``leanerGuardedMatchArmSyntax then
        unless armExpressions.size == 2 do
          throw "a guarded match arm must contain a guard and a body"
        pure (← bindingPatternOf pattern, some (← expressionOf armExpressions[0]!),
          ← expressionOf armExpressions[1]!)
      else
        unless armExpressions.size == 1 do
          throw "a match arm must contain one body expression"
        pure (← bindingPatternOf pattern, none, ← expressionOf armExpressions[0]!)
    pure (.match_ (← expressionOf scrutinee) arms span)
  else if stx.isOfKind ``leanerQuantifierExpr then
    let kind := if containsAtom "forall" stx || containsAtom "∀" stx then QuantifierKind.forall
      else QuantifierKind.exists
    let binderNodes := childrenWhereOutsideExpressions (fun node =>
      node.isOfKind ``LeanerLang.leanerQuantifierBinderSyntax ||
        node.isOfKind ``LeanerLang.leanerQuantifierTypeBinderSyntax) stx
    let binders ← binderNodes.mapM fun binder => do
      let some pattern := (childrenWhereOutsideExpressions isBindingPatternSyntax binder)[0]?
        | throw "a quantifier binder must contain a pattern"
      if binder.isOfKind ``LeanerLang.leanerQuantifierTypeBinderSyntax then
        -- `x : T` quantifies over the whole type's domain.
        let some element := (typeChildren binder)[0]?
          | throw "a quantifier type binder must contain a type"
        let domain : Expr :=
          .specification .typeDomain #[← typeOf element] #[] (spanOf binder)
        pure (← bindingPatternOf pattern, domain)
      else
        let some domain := (exprChildren binder)[0]?
          | throw "a quantifier binder must contain a domain"
        pure (← bindingPatternOf pattern, ← expressionOf domain)
    let some body := (exprChildren stx).back?
      | throw "a quantifier must contain a body"
    pure (.quantifier kind binders (← expressionOf body) span)
  else if stx.isOfKind ``leanerSpecificationAnchorMarkerExpr ||
      stx.isOfKind ``leanerSpecificationAnchorMarkerBuiltinExpr then
    let some label := firstNat? stx
      | throw "a specification state anchor must carry a numeric label"
    let operation :=
      if containsAtom "spec.saveStateAnchor" stx || containsAtom "save_state_anchor" stx then
        SpecificationOperation.saveStateAnchor label
      else
        SpecificationOperation.foldsCaptureAnchor label
    pure (.specification operation #[] #[] span)
  else if stx.isOfKind ``leanerSpecificationWithStateExpr ||
      stx.isOfKind ``leanerSpecificationWithStateBuiltinExpr then
    let some label := firstNat? stx
      | throw "spec.withStateAnchor must carry a numeric label"
    let some argument := (exprChildren stx)[0]?
      | throw "spec.withStateAnchor must have an argument"
    pure (.specification (.withStateAnchor label) #[] #[← expressionOf argument] span)
  else if stx.isOfKind ``leanerSpecificationOldExpr then
    pure (.specification .old #[] (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationResultExpr then
    let some index := firstNat? stx
      | throw "spec.result must carry a numeric index"
    pure (.specification (.result index) #[] #[] span)
  else if stx.isOfKind ``leanerSpecificationInlineCallSummaryExpr then
    pure (.specification .inlineCallSummary #[]
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationVectorExpr then
    -- The element type argument is optional: imported units may carry no
    -- instantiation, in which case the lowering infers the element type.
    let elementType? ← match
        (childrenOfKind ``leanerSpecificationTypeArgumentSyntax stx)[0]? with
      | some typeArgument =>
          match (typeChildren typeArgument)[0]? with
          | some elementType => pure (some elementType)
          | none => throw "a specification vector operation has an invalid element type"
      | none => pure none
    let operation :=
      if containsAtomOutsideExpressions "spec.emptyVector" stx then
        SpecificationOperation.emptyVector
      else if containsAtomOutsideExpressions "spec.singletonVector" stx then .singletonVector
      else if containsAtomOutsideExpressions "spec.updateVector" stx then .updateVector
      else if containsAtomOutsideExpressions "spec.concatVector" stx then .concatVector
      else if containsAtomOutsideExpressions "spec.indexOfVector" stx then .indexOfVector
      else if containsAtomOutsideExpressions "spec.containsVector" stx then .containsVector
      else if containsAtomOutsideExpressions "spec.lengthVector" stx then .lengthVector
      else if containsAtomOutsideExpressions "spec.indexVector" stx then .indexVector
      else if containsAtomOutsideExpressions "spec.sliceVector" stx then .sliceVector
      else if containsAtomOutsideExpressions "spec.inVectorRange" stx then .inVectorRange
      else .vectorRange
    let types ← match elementType? with
      | some elementType => pure #[← typeOf elementType]
      | none => pure #[]
    pure (.specification operation types
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationEmptyVectorBuiltinExpr then
    let some elementType := (typeChildren stx)[0]?
      | throw "`vec` requires an element type when it has no arguments"
    pure (.specification .emptyVector #[← typeOf elementType] #[] span)
  else if stx.isOfKind ``leanerSpecificationGlobalExpr ||
      stx.isOfKind ``leanerSpecificationGlobalSurfaceExpr then
    let some resourceType := if stx.isOfKind ``leanerSpecificationGlobalExpr then do
        let typeArgument ← (childrenOfKind ``leanerSpecificationTypeArgumentSyntax stx)[0]?
        (typeChildren typeArgument)[0]?
      else (typeChildren stx)[0]?
      | throw "global requires a resource type"
    pure (.specification .global #[← typeOf resourceType]
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationInRangeExpr then
    pure (.specification .inRange #[] (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationBitVectorToIntExpr then
    pure (.specification .bitVectorToInt #[]
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationIntToBitVectorExpr then
    let some resultType := (typeChildren stx)[0]?
      | throw "spec.intToBitVector requires its fixed-width result type"
    pure (.specification .intToBitVector #[← typeOf resultType]
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecificationIntToBitVectorInferExpr then
    pure (.specification .intToBitVector #[]
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerGlobalContainsExpr ||
      stx.isOfKind ``leanerGlobalBorrowExpr || stx.isOfKind ``leanerGlobalTakeExpr ||
      stx.isOfKind ``leanerGlobalPublishExpr ||
      stx.isOfKind ``leanerGlobalExistsSurfaceExpr ||
      stx.isOfKind ``leanerBorrowGlobalSurfaceExpr ||
      stx.isOfKind ``leanerBorrowGlobalMutSurfaceExpr ||
      stx.isOfKind ``leanerMoveFromSurfaceExpr ||
      stx.isOfKind ``leanerMoveToSurfaceExpr then
    let some resource := (typeChildren stx)[0]?
      | throw "a global operation requires its resource type"
    let operation := if stx.isOfKind ``leanerGlobalContainsExpr ||
        stx.isOfKind ``leanerGlobalExistsSurfaceExpr then GlobalOperation.contains
      else if stx.isOfKind ``leanerBorrowGlobalSurfaceExpr then .borrow false
      else if stx.isOfKind ``leanerBorrowGlobalMutSurfaceExpr then .borrow true
      else if stx.isOfKind ``leanerGlobalBorrowExpr then .borrow (containsAtomOutsideExpressions "mut" stx)
      else if stx.isOfKind ``leanerGlobalTakeExpr ||
          stx.isOfKind ``leanerMoveFromSurfaceExpr then .take
      else .publish
    pure (.global operation (← typeOf resource) (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerSpecBlockExpr || stx.isOfKind ``leanerSingleSpecExpr then
    let statements := childrenWhere (fun child =>
      child.isOfKind ``leanerSpecLetStatement ||
      child.isOfKind ``leanerAssertStatement ||
      child.isOfKind ``leanerAssumeStatement ||
      child.isOfKind ``leanerLoopInvariantStatement) stx
      |>.qsort fun left right => (spanOf left).startByte < (spanOf right).startByte
    let conditions ← statements.mapM fun statement => do
      let some condition := (exprChildren statement)[0]?
        | throw "an in-body specification member must contain an expression"
      let kind ← if statement.isOfKind ``leanerSpecLetStatement then
          let some name := (identifiers statement)[0]?
            | throw "an in-body specification binding must have a name"
          pure <| SpecificationConditionKind.let_ (unquoteIdentifier name)
        else if statement.isOfKind ``leanerAssertStatement then
          pure SpecificationConditionKind.assertion
        else if statement.isOfKind ``leanerLoopInvariantStatement then
          pure SpecificationConditionKind.loopInvariant
        else pure SpecificationConditionKind.assumption
      pure (kind, ← expressionOf condition)
    pure (.specBlock conditions span)
  else if stx.isOfKind ``leanerConstructExpr then
    let some path := (pathChildren stx)[0]?
      | throw "a constructor expression must name its nominal target"
    pure (.construct (← pathOf path) (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerAppliedConstructExpr then
    let some path := (pathChildren stx)[0]?
      | throw "an applied constructor expression must name its nominal target"
    let arguments := (← (typeChildren stx).mapM typeOf).map (.type ·.value)
    let owner : Located Ty := { value := .named (← pathOf path) arguments, span }
    let variant := (childrenOfKind ``leanerConstructorVariantSyntax stx)[0]?.bind
      fun variantSyntax => (identifiers variantSyntax)[0]?
    pure (.appliedConstruct owner variant
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerStandardAppliedConstructExpr then
    let some owner := (typeChildren stx)[0]?
      | throw "an applied constructor expression must name its nominal target"
    let variant := (childrenOfKind ``leanerConstructorVariantSyntax stx)[0]?.bind
      fun variantSyntax => (identifiers variantSyntax)[0]?
    pure (.appliedConstruct (← typeOf owner) variant
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerNamedConstructExpr then
    let some owner := (typeChildren stx)[0]?
      | throw "a named constructor expression must name its nominal target"
    let variant := (childrenOfKind ``leanerConstructorVariantSyntax stx)[0]?.bind
      fun variantSyntax => (identifiers variantSyntax)[0]?
    let fields ← (childrenWhere (fun child =>
        child.isOfKind ``leanerConstructorFieldSyntax ||
          child.isOfKind ``leanerConstructorFieldShorthandSyntax) stx).mapM fun field => do
      let name ← if field.isOfKind ``leanerConstructorFieldShorthandSyntax then
          let some name := (identifiers field)[0]?
            | throw "a constructor field must have a name"
          pure name
        else
          let some fieldSyntax := (fieldIdentifierChildren field)[0]?
            | throw "a constructor field must have a name"
          fieldIdentifierOf fieldSyntax
      match (exprChildren field)[0]? with
      | some value => pure (name, ← expressionOf value)
      | none => pure (name, .local name (spanOf field))
    pure (.namedConstruct (← typeOf owner) variant fields span)
  else if stx.isOfKind ``leanerSelectExpr then
    let some owner := (typeChildren stx)[0]?
      | throw "a field-selection expression must name its nominal type"
    let some fieldSyntax := (childrenOfKind ``leanerFieldNameSyntax stx)[0]?
      | throw "a field-selection expression must name its field"
    let some fieldIdentifier := (fieldIdentifierChildren fieldSyntax)[0]?
      | throw "a field-selection expression has an invalid field name"
    let field ← fieldIdentifierOf fieldIdentifier
    let some value := (exprChildren stx)[0]?
      | throw "a field-selection expression must have one operand"
    pure (.select (← typeOf owner) field (← expressionOf value) span)
  else if stx.isOfKind ``leanerFieldExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a field expression must have a receiver"
    let some fieldSyntax := (fieldIdentifierChildren stx).back?
      | throw "a field expression must name a field"
    let field ← fieldIdentifierOf fieldSyntax
    pure <| field.splitOn "." |>.map unquoteIdentifier |>.foldl
      (fun value field => .field value field span)
      (← expressionOf value)
  else if stx.isOfKind ``leanerStorageIndexExpr then
    let some head := (childrenWhere (fun child =>
        child.isOfKind ``leanerNamedStorageHead ||
          child.isOfKind ``leanerAppliedStorageHead) stx)[0]?
      | throw "a storage index must have a type-shaped head"
    let some index := (exprChildren stx)[0]?
      | throw "a storage index must have an index"
    pure (.storageIndex (← storageHeadOf head) (← expressionOf index) span)
  else if stx.isOfKind ``leanerIndexExpr then
    let expressions := exprChildren stx
    unless expressions.size == 2 do throw "an index expression requires two operands"
    pure (.index (← expressionOf expressions[0]!) (← expressionOf expressions[1]!) span)
  else if stx.isOfKind ``leanerMembershipExpr then
    let expressions := exprChildren stx
    unless expressions.size == 2 do throw "membership requires two operands"
    pure (.membership (← expressionOf expressions[0]!)
      (← expressionOf expressions[1]!) span)
  else if stx.isOfKind ``leanerVariantTestSurfaceExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a variant test requires a value"
    let variants := identifiersOutsideExpressions stx
    if variants.isEmpty then throw "a variant test must name at least one variant"
    pure (.variantTest (← expressionOf value) variants span)
  else if stx.isOfKind ``leanerSelectVariantsExpr then
    let some owner := (typeChildren stx)[0]?
      | throw "a variant-field selection must name its nominal type"
    let fields ← (childrenOfKind ``leanerFieldNameSyntax stx).mapM fun field => do
      let some fieldIdentifier := (fieldIdentifierChildren field)[0]?
        | throw "a variant-field selection has an invalid field name"
      fieldIdentifierOf fieldIdentifier
    if fields.isEmpty then throw "a variant-field selection must name at least one field"
    let some value := (exprChildren stx)[0]?
      | throw "a variant-field selection must have one operand"
    pure (.selectVariants (← typeOf owner) fields (← expressionOf value) span)
  else if stx.isOfKind ``leanerTestVariantsExpr then
    let some owner := (typeChildren stx)[0]?
      | throw "a variant-test expression must name its nominal type"
    let variants := (childrenOfKind ``leanerVariantNameSyntax stx).flatMap identifiers
    if variants.isEmpty then throw "a variant-test expression must name at least one variant"
    let some value := (exprChildren stx)[0]?
      | throw "a variant-test expression must have one operand"
    pure (.testVariants (← typeOf owner) variants (← expressionOf value) span)
  else if stx.isOfKind ``leanerDiscriminantExpr ||
      stx.isOfKind ``leanerDiscriminantSurfaceExpr then
    let types := typeChildren stx
    unless types.size == 2 do
      throw "a discriminant expression requires its owner and result types"
    let some value := (exprChildren stx)[0]?
      | throw "a discriminant expression must have one operand"
    pure (.discriminant (← typeOf types[0]!) (← typeOf types[1]!)
      (← expressionOf value) span)
  else if stx.isOfKind ``leanerBlockExpr then
    blockExpressionOfEntries (blockEntries stx) span
  else if stx.isOfKind ``leanerGenericCallExpr ||
      stx.isOfKind ``leanerDirectGenericCallExpr then
    let some path := (pathChildren stx)[0]?
      | throw "a generic call must name its function"
    let some typeArguments := (childrenOfKind ``leanerTypeArgumentsSyntax stx)[0]?
      | throw "a generic call must carry type arguments"
    let path ← pathOf path
    let types ← (typeChildren typeArguments).mapM typeOf
    let arguments ← (exprChildren stx).mapM expressionOf
    match path.toList with
    | [name] => match dottedReceiver? name span with
        | some (receiver, method) => pure (.methodCall method none types receiver arguments span)
        | none => pure (.genericCall path types arguments span)
    | _ => pure (.genericCall path types arguments span)
  else if stx.isOfKind ``leanerTypedCallExpr ||
      stx.isOfKind ``leanerTypedCallSurfaceExpr then
    let some path := pathOutsideTypes? stx
      | throw "a typed call must name its function"
    let result? := if stx.isOfKind ``leanerTypedCallSurfaceExpr then
        (signatureTypeChildren stx).back? else (typeChildren stx)[0]?
    let some result := result?
      | throw "a typed call must carry its result type"
    let path ← pathOf path
    let result ← typeOf result
    let arguments ← (exprChildren stx).mapM expressionOf
    match path.toList with
    | [name] => match dottedReceiver? name span with
        | some (receiver, method) =>
            pure (.methodCall method (some result) #[] receiver arguments span)
        | none => pure (.typedCall path result arguments span)
    | _ => pure (.typedCall path result arguments span)
  else if stx.isOfKind ``leanerTypedGenericCallExpr ||
      stx.isOfKind ``leanerTypedGenericCallSurfaceExpr then
    let some path := pathOutsideTypes? stx
      | throw "a typed generic call must name its function"
    let some typeArguments := (childrenOfKind ``leanerTypeArgumentsSyntax stx)[0]?
      | throw "a typed generic call must carry type arguments"
    let allTypes := signatureTypeChildren stx
    let result? := if stx.isOfKind ``leanerTypedGenericCallSurfaceExpr then
        allTypes.back? else allTypes[0]?
    let some result := result?
      | throw "a typed generic call must carry its result type"
    let path ← pathOf path
    let result ← typeOf result
    let types ← (typeChildren typeArguments).mapM typeOf
    let arguments ← (exprChildren stx).mapM expressionOf
    match path.toList with
    | [name] => match dottedReceiver? name span with
        | some (receiver, method) =>
            pure (.methodCall method (some result) types receiver arguments span)
        | none => pure (.typedGenericCall path result types arguments span)
    | _ => pure (.typedGenericCall path result types arguments span)
  else if stx.isOfKind ``leanerMethodCallExpr ||
      stx.isOfKind ``leanerTypedMethodCallExpr then
    let expressions := exprChildren stx
    let some receiver := expressions[0]?
      | throw "a receiver call requires a receiver"
    -- Argument containers are parser wrappers rather than `leanerExpr`
    -- nodes, so a recursive scan can also see their local identifiers. The
    -- method name is the first identifier outside the receiver expression;
    -- taking the last one would turn `value.add(amount)` into
    -- `value.amount(amount)`.
    let some name := (identifiersOutsideExpressions stx)[0]?
      | throw "a receiver call requires a method name"
    let result ← if stx.isOfKind ``leanerTypedMethodCallExpr then
        let some result := (signatureTypeChildren stx).back?
          | throw "a typed receiver call requires a result type"
        pure (some (← typeOf result))
      else pure none
    let types ← match (childrenOfKind ``leanerTypeArgumentsSyntax stx)[0]? with
      | none => pure #[]
      | some arguments => (typeChildren arguments).mapM typeOf
    pure (.methodCall name result types (← expressionOf receiver)
      (← (expressions.drop 1).mapM expressionOf) span)
  else if stx.isOfKind ``leanerInvokeExpr || stx.isOfKind ``leanerInvokeSurfaceExpr then
    let expressions := exprChildren stx
    let some callable := expressions[0]?
      | throw "core.invoke must contain a callable operand"
    pure (.invoke (← expressionOf callable)
      (← (expressions.drop 1).mapM expressionOf) span)
  else if stx.isOfKind ``leanerClosureExpr || stx.isOfKind ``leanerFunctionValueExpr then
    let some result := (typeChildren stx)[0]?
      | throw "core.closure must carry its residual function type"
    let some path := pathOutsideTypes? stx
      | throw "core.closure must name its target function"
    pure (.closure (← pathOf path) (← typeOf result)
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerRawPlaceBorrowExpr then
    let some place := (exprChildren stx)[0]?
      | throw "core.borrowPlace must contain a place expression"
    pure (.rawBorrowValue (containsAtomOutsideExpressions "mut" stx)
      (← expressionOf place) span)
  else if stx.isOfKind ``leanerPlaceBorrowExpr then
    let some place := (placeChildren stx)[0]?
      | throw "a place borrow must contain a place"
    pure (.borrowPlace (containsAtomOutsideExpressions "mut" stx) (← placeOf place) span)
  else if stx.isOfKind ``leanerDropPlaceExpr ||
      stx.isOfKind ``leanerDropPlaceSurfaceExpr then
    let some place := (placeChildren stx)[0]?
      | throw "a drop must contain a place"
    pure (.dropPlace (← placeOf place) span)
  else if stx.isOfKind ``leanerMovePlaceSurfaceExpr ||
      stx.isOfKind ``leanerCopyPlaceSurfaceExpr ||
      stx.isOfKind ``leanerReadPlaceSurfaceExpr then
    let some place := (exprChildren stx)[0]?
      | throw "a place operation must contain a place"
    let operation := if stx.isOfKind ``leanerMovePlaceSurfaceExpr then PlaceOperation.move
      else if stx.isOfKind ``leanerCopyPlaceSurfaceExpr then .copy else .read
    pure (.placeOperation operation (← expressionOf place) span)
  else if stx.isOfKind ``leanerBorrowPlaceSurfaceExpr then
    let some place := (placeChildren stx)[0]?
      | throw "a place borrow must contain a place"
    pure (.borrowPlace (containsAtomOutsideExpressions "mut" stx) (← placeOf place) span)
  else if stx.isOfKind ``leanerBorrowValueSurfaceExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a value borrow must have one operand"
    pure (.borrowValue (containsAtomOutsideExpressions "mut" stx) (← expressionOf value) span)
  else if stx.isOfKind ``leanerValueBorrowExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a value borrow must have one operand"
    pure (.borrowValue (containsAtomOutsideExpressions "mut" stx) (← expressionOf value) span)
  else if stx.isOfKind ``leanerFreezeReferenceExpr ||
      stx.isOfKind ``leanerFreezeExplicitReferenceExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a reference freeze must have one operand"
    pure (.freezeReference (stx.isOfKind ``leanerFreezeExplicitReferenceExpr)
      (← expressionOf value) span)
  else if stx.isOfKind ``leanerDereferenceExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a reference dereference must have one operand"
    pure (.dereference (← expressionOf value) span)
  else if stx.isOfKind ``leanerDereferenceSurfaceExpr then
    let some value := (exprChildren stx)[0]?
      | throw "a reference dereference must have one operand"
    pure (.dereference (← expressionOf value) span)
  else if stx.isOfKind ``leanerMutateReferenceExpr then
    let arguments := exprChildren stx
    unless arguments.size == 2 do throw "a reference mutation must have two operands"
    pure (.mutateReference (← expressionOf arguments[0]!)
      (← expressionOf arguments[1]!) span)
  else if stx.isOfKind ``leanerRawPlaceAssignExpr then
    let expressions := exprChildren stx
    unless expressions.size == 2 do throw "core.assignPlace requires a target and value"
    pure (.rawAssignExpression (← expressionOf expressions[0]!)
      (← expressionOf expressions[1]!) span)
  else if stx.isOfKind ``leanerAssignmentExpr then
    let expressions := exprChildren stx
    unless expressions.size == 2 do throw "an assignment must have a target and value"
    let target ← expressionOf expressions[0]!
    let value ← expressionOf expressions[1]!
    match target with
    | .dereference reference _ => pure (.mutateReference reference value span)
    | target => pure (.assignExpression target value span)
  else if stx.isOfKind ``leanerPatternAssignmentExpr ||
      stx.isOfKind ``leanerPatternAssignmentSurfaceExpr then
    let some type := (typeChildren stx)[0]?
      | throw "a pattern assignment must carry its value type"
    let some pattern := (childrenWhere isBindingPatternSyntax stx)[0]?
      | throw "a pattern assignment must contain a pattern"
    let some value := (exprChildren stx)[0]?
      | throw "a pattern assignment must have a value"
    pure (.assignPattern (← bindingPatternOf pattern) (← typeOf type)
      (← expressionOf value) span)
  else if stx.isOfKind ``leanerCastSurfaceExpr then
    let some resultType := (typeChildren stx)[0]?
      | throw "a cast requires its result type"
    let some value := (exprChildren stx)[0]?
      | throw "a cast requires one operand"
    pure (.typedPrimitive .profileCast (← typeOf resultType)
      #[← expressionOf value] span)
  else if stx.isOfKind ``leanerTypedCastExpr ||
      stx.isOfKind ``leanerTypedCheckedCastExpr then
    let some resultType := (typeChildren stx)[0]?
      | throw "a typed cast requires its result type"
    let operation ← if stx.isOfKind ``leanerTypedCastExpr then pure Primitive.cast else
      let some failure := (throwChildren stx)[0]?
        | throw "a typed checked cast requires its failure mode"
      pure (.checkedCast (← throwOf failure))
    pure (.typedPrimitive operation (← typeOf resultType)
      (← (exprChildren stx).mapM expressionOf) span)
  else if stx.isOfKind ``leanerPrimitiveExpr || stx.isOfKind ``leanerCheckedPrimitiveExpr then
    let some path := (pathChildren stx)[0]?
      | throw "an application expression must name its target"
    let path ← pathOf path
    let failure ← (throwChildren stx)[0]?.mapM throwOf
    let arguments ← (exprChildren stx).mapM expressionOf
    let surfacePrimitive? := if path == #["slice"] then some Primitive.slice
      else if path == #["overflowing_add"] then some .overflowingAdd
      else if path == #["overflowing_subtract"] then some .overflowingSubtract
      else if path == #["overflowing_multiply"] then some .overflowingMultiply
      else none
    if path == #["abort"] && failure.isNone then
      pure (.throw_ .abort arguments span)
    else if path == #["panic"] && failure.isNone then
      pure (.throw_ .panic arguments span)
    else if let some primitive := surfacePrimitive? then
      if failure.isSome then throw "surface primitives do not accept a failure annotation"
      pure (.primitive primitive arguments span)
    else if path.size == 1 && path[0]!.startsWith "core.prim." then
      pure (.primitive (← primitiveOf (path[0]!.drop 10 |>.toString) failure) arguments span)
    else if failure.isSome then
      throw "only checked core primitives accept an abort/panic annotation"
    else match path.toList with
      | [name] => match dottedReceiver? name span with
          | some (receiver, method) => pure (.methodCall method none #[] receiver arguments span)
          | none => pure (.call path arguments span)
      | _ => pure (.call path arguments span)
  else if stx.isOfKind ``leanerIfExpr then
    let expressions := exprChildren stx
    let branches := childrenWhereOutsideExpressions isIfBranchSyntax stx
    unless expressions.size >= 1 && (branches.size == 1 || branches.size == 2) do
      throw "`if` requires a condition and one or two branches"
    pure (.ifElse (← expressionOf expressions[0]!)
      (← ifBranchExpressionOf branches[0]!)
      (← branches[1]?.mapM ifBranchExpressionOf) span)
  else if stx.isOfKind ``leanerLoopExpr then
    let some body := (exprChildren stx)[0]?
      | throw "`loop` requires a body"
    let label := (childrenOfKind ``leanerLoopLabelSyntax stx)[0]?.map
      (fun label => label[1].getId.getString!)
    pure (.loop (← expressionOf body) span label)
  else if stx.isOfKind ``leanerWhileExpr then
    let some condition := (exprChildren stx)[0]?
      | throw "`while` requires a condition"
    let body ← blockExpressionOfEntries (blockEntries stx) span
    -- Like a range loop, a `while` body is wholly in effect position.  Treat
    -- every indented entry after a line-ending `do` as part of that body;
    -- reserving its last bare expression as a general block result creates a
    -- different nested-block shape after round-tripping multi-entry loops.
    let body := match body with
      | .block statements (some result) bodySpan =>
          .block (statements.push (.expression result)) none bodySpan
      | body => body
    let stop := Expr.break_ none span
    let condition ← expressionOf condition
    let members := childrenOfKind ``leanerLoopSpecificationMemberSyntax stx
    let condition ← if members.isEmpty then pure condition else do
      let specification ← members.mapM fun member => do
        let some statement := (childrenWhere (fun child =>
            child.isOfKind ``leanerSpecLetStatement ||
            child.isOfKind ``leanerAssertStatement ||
            child.isOfKind ``leanerAssumeStatement ||
            child.isOfKind ``leanerLoopInvariantStatement) member)[0]?
          | throw "a loop specification member is malformed"
        let some expression := (exprChildren statement)[0]?
          | throw "a loop specification member must contain an expression"
        let kind ← if statement.isOfKind ``leanerSpecLetStatement then
            let some name := (identifiers statement)[0]?
              | throw "a loop specification binding must have a name"
            pure <| SpecificationConditionKind.let_ (unquoteIdentifier name)
          else if statement.isOfKind ``leanerAssertStatement then
            pure SpecificationConditionKind.assertion
          else if statement.isOfKind ``leanerAssumeStatement then
            pure SpecificationConditionKind.assumption
          else pure SpecificationConditionKind.loopInvariant
        pure (kind, ← expressionOf expression)
      -- A trailing `where` region belongs to this loop. Anchor it in the
      -- condition, matching LIR's verifier-oriented representation and
      -- preventing surrounding block flattening from separating the region
      -- from its loop.
      pure <| .block #[.expression (.specBlock specification span)] (some condition) span
    pure <| .loop (.ifElse condition body (some stop) span) span
  else if stx.isOfKind ``leanerForRangeExpr then
    let some iteratorSyntax := stx.getArgs.find? (·.isIdent)
      | throw "`for` requires an iterator name"
    let iterator := unquoteIdentifier <| if iteratorSyntax.getId.getPrefix.isAnonymous then
        iteratorSyntax.getId.getString!
      else iteratorSyntax.getId.toString
    let some rangeSyntax := (exprChildren stx)[0]?
      | throw "`for` requires a half-open range"
    let range ← expressionOf rangeSyntax
    let (.primitive .range #[lower, upper] _) := range
      | throw "`for` requires a half-open `lower..upper` range"
    let body ← blockExpressionOfEntries (blockEntries stx) span
    -- Every entry of a loop body is effect-position, including the last one.
    -- The general `do` parser reserves a trailing bare expression as a block
    -- result, so normalize that distinction at the `for` boundary.
    let body := match body with
      | .block statements (some result) bodySpan =>
          .block (statements.push (.expression result)) none bodySpan
      | body => body
    pure (.forRange iterator lower upper body span)
  else if stx.isOfKind ``leanerBreakExpr then
    let label := (childrenOfKind ``leanerLoopLabelSyntax stx)[0]?.map
      (fun label => label[1].getId.getString!)
    pure (.break_ (← (exprChildren stx)[0]?.mapM expressionOf) span label)
  else if stx.isOfKind ``leanerContinueExpr then
    let label := (childrenOfKind ``leanerLoopLabelSyntax stx)[0]?.map
      (fun label => label[1].getId.getString!)
    pure (.continue_ span label)
  else if stx.isOfKind ``leanerReturnExpr then
    let some value := (exprChildren stx)[0]?
      | throw "`return` requires a value"
    pure (.return_ (← expressionOf value) span)
  else throw s!"unsupported Leaner expression stx `{stx.getKind}`"
end

private def parameterOf (stx : Syntax) : Except String Parameter := do
  let some name := stx.getArgs.find? (·.isIdent)
    | throw "a parameter must have a name"
  let some type := (typeChildren stx)[0]?
    | throw s!"parameter `{name.getId}` must have a type"
  return {
    name := name.getId.getString!
    type := ← typeOf type
    mutable := containsAtomOutsideTypes "mut" stx
    span := spanOf stx }

private def modifiersOf (syntaxes : Array Syntax) : Except String FunctionModifiers := do
  let mut result : FunctionModifiers := {}
  let mut visibilitySeen := false
  for stx in syntaxes do
    if stx.isOfKind ``leanerPrivateModifier || stx.isOfKind ``leanerPublicModifier ||
        stx.isOfKind ``leanerPackageModifier || stx.isOfKind ``leanerFriendModifier then
      if visibilitySeen then throw "a function may have only one visibility modifier"
      visibilitySeen := true
      let visibility : Visibility :=
        if stx.isOfKind ``leanerPublicModifier then .public_
        else if stx.isOfKind ``leanerPackageModifier then .package
        else if stx.isOfKind ``leanerFriendModifier then .friend
        else .private_
      result := { result with visibility }
    else if stx.isOfKind ``leanerEntryModifier then
      if result.isEntry then throw "duplicate `entry` modifier"
      result := { result with isEntry := true }
    else if stx.isOfKind ``leanerNativeModifier then
      if result.isNative then throw "duplicate `native` modifier"
      result := { result with isNative := true }
    else if stx.isOfKind ``leanerOpaqueModifier then
      if result.isOpaque then throw "duplicate `opaque` modifier"
      result := { result with isOpaque := true }
    else if stx.isOfKind ``leanerDeprecatedModifier then
      if result.isDeprecated then throw "duplicate `deprecated` modifier"
      result := { result with isDeprecated := true }
    else if stx.isOfKind ``leanerViewModifier then
      if result.isView then throw "duplicate `view` modifier"
      result := { result with isView := true }
    else throw s!"unknown function modifier `{stx.getKind}`"
  return result

private def clauseOf (stx : Syntax) : Except String ContractClause := do
  let span := spanOf stx
  if stx.isOfKind ``leanerModifiesAllClause then return .modifiesAll span
  if stx.isOfKind ``leanerReadsAllClause then return .readsAll span
  if stx.isOfKind ``leanerReadsClause then
    let some type := (typeChildren stx)[0]? | throw "a reads clause requires a type"
    return .reads (← typeOf type) span
  let expressions := exprChildren stx
  let some expression := expressions[0]?
    | throw "a contract clause requires an expression"
  let expression ← expressionOf expression
  let properties := (childrenOfKind ``leanerConditionPropertiesSyntax stx).flatMap identifiers
  if stx.isOfKind ``leanerModifiesClause then return .modifies expression span
  if stx.isOfKind ``leanerLooseModifiesClause then return .modifies expression span true
  if stx.isOfKind ``leanerLetPreClause || stx.isOfKind ``leanerLetPostClause then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a contract binding requires a name"
    if stx.isOfKind ``leanerLetPreClause then
      pure (.letPre name.getId.getString! expression properties span)
    else
      pure (.letPost name.getId.getString! expression properties span)
  else if stx.isOfKind ``leanerRequiresClause then pure (.requires expression properties span)
  else if stx.isOfKind ``leanerEnsuresClause then pure (.ensures expression properties span)
  else if stx.isOfKind ``leanerAbortsIfClause then
    if expressions.size > 2 then throw "an aborts_if clause has more than one abort code"
    pure (.abortsIf expression (← expressions[1]?.mapM expressionOf) properties span)
  else if stx.isOfKind ``leanerInvariantClause then pure (.invariant expression properties span)
  else throw s!"unknown contract clause `{stx.getKind}`"

private def pragmaOf (stx : Syntax) : Except String Pragma := do
  let name ← match (identifiers stx)[0]? with
    | some name => pure name
    | none => if containsAtom "opaque" stx then pure "opaque" else throw "a pragma must have a name"
  let value ← match (exprChildren stx)[0]? with
    | some value => expressionOf value
    | none => pure (.bool true (spanOf stx))
  pure { name, value, span := spanOf stx }

private def binderOf (stx : Syntax) : Except String GenericBinder := do
  let some name := (identifiers stx)[0]? | throw "a generic binder must have a name"
  let kind ←
    if stx.isOfKind ``leanerTypeBinder || stx.isOfKind ``leanerInferredTypeBinder then
      pure BinderKind.type
    else if stx.isOfKind ``leanerConstBinder || stx.isOfKind ``leanerTypedConstBinder then
      pure .const
    else if stx.isOfKind ``leanerLifetimeBinder then pure .lifetime
    else if stx.isOfKind ``leanerEvidenceBinder then pure .evidence
    else throw s!"unknown generic binder `{stx.getKind}`"
  let abilities ← (childrenWhere isAbilitySyntax stx).mapM abilityOf
  if kind != .type && !abilities.isEmpty then
    throw s!"only type binder `{name}` may carry abilities"
  let type ← if stx.isOfKind ``leanerTypedConstBinder then
      let some type := (typeChildren stx)[0]?
        | throw s!"const binder `{name}` requires its declared type"
      some <$> typeOf type
    else pure none
  let phantom := containsAtom "phantom" stx
  if phantom && kind != .type then
    throw s!"only type binder `{name}` may be phantom"
  return { name, kind, type, abilities, phantom, span := spanOf stx }

private def fieldOf (stx : Syntax) : Except String FieldDecl := do
  let some fieldSyntax := (fieldIdentifierChildren stx)[0]?
    | throw "a field must have a name"
  let name ← fieldIdentifierOf fieldSyntax
  let some type := (typeChildren stx)[0]?
    | throw s!"field `{name}` must have a type"
  return {
    name
    type := ← typeOf type
    span := spanOf stx }

private def variantOf (stx : Syntax) : Except String VariantDecl := do
  let some name := stx.getArgs.find? (·.isIdent)
    | throw "an enum variant must have a name"
  let discriminant ← (childrenOfKind ``leanerVariantDiscriminant stx)[0]?.mapM fun node => do
    let some value := firstNat? node
      | throw s!"variant `{name.getId}` has an invalid discriminant"
    pure (Int.ofNat value)
  return {
    name := name.getId.getString!
    fields := ← (childrenOfKind ``leanerFieldSyntax stx).mapM fieldOf
    discriminant
    span := spanOf stx }

private inductive ParsedItem where
  | item (value : Item)
  | use (path : Array String)
  | friend (declaration : FriendDecl)
  | namespacePragma (value : Pragma)
  | contract (name : String) (clauses : Array ContractClause)
      (pragmas : Array Pragma) (span : Span)

private def itemOf (stx : Syntax) : Except String ParsedItem := do
  if stx.isOfKind ``leanerUseItem then
    let some path := (pathChildren stx)[0]?
      | throw "a use declaration requires a path"
    pure (.use (← pathOf path))
  else if stx.isOfKind ``leanerFriendItem then
    let some path := (pathChildren stx)[0]?
      | throw "a friend declaration requires a path"
    pure (.friend { path := ← pathOf path, span := spanOf stx })
  else if stx.isOfKind ``leanerNamespacePragmaItem then
    pure (.namespacePragma (← pragmaOf stx))
  else if stx.isOfKind ``leanerConstantItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a constant must have a name"
    let some type := (typeChildren stx)[0]?
      | throw s!"constant `{name.getId}` must have a type"
    let some value := (exprChildren stx)[0]?
      | throw s!"constant `{name.getId}` must have a value"
    pure (.item (.constant {
      name := name.getId.getString!
      type := ← typeOf type
      value := ← expressionOf value
      span := spanOf stx }))
  else if stx.isOfKind ``leanerStructItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a struct must have a name"
    pure (.item (.struct {
      name := name.getId.getString!
      generics := ← (childrenWhere isGenericBinderSyntax stx).mapM binderOf
      fields := ← (childrenOfKind ``leanerFieldSyntax stx).mapM fieldOf
      abilities := ← (declarationAbilities stx).mapM abilityOf
      attributes := ← (childrenWhere isAttributeSyntax stx).mapM attributeOf
      span := spanOf stx }))
  else if stx.isOfKind ``leanerEnumItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "an enum must have a name"
    pure (.item (.enum {
      name := name.getId.getString!
      generics := ← (childrenWhere isGenericBinderSyntax stx).mapM binderOf
      variants := ← (childrenOfKind ``leanerVariantSyntax stx).mapM variantOf
      abilities := ← (declarationAbilities stx).mapM abilityOf
      attributes := ← (childrenWhere isAttributeSyntax stx).mapM attributeOf
      span := spanOf stx }))
  else if stx.isOfKind ``leanerFunctionItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a function must have a name"
    let types := signatureTypeChildren stx
    let some result := types.back? | throw s!"function `{name.getId}` must have a result type"
    let expressions := exprChildren stx
    pure (.item (.function {
      name := name.getId.getString!
      modifiers := ← modifiersOf (childrenWhere isModifierSyntax stx)
      generics := ← (childrenWhere isGenericBinderSyntax stx).mapM binderOf
      parameters := ← (childrenOfKind ``leanerParameterSyntax stx).mapM parameterOf
      result := ← typeOf result
      body := ← expressions.back?.mapM expressionOf
      attributes := ← (childrenWhere isAttributeSyntax stx).mapM attributeOf
      span := spanOf stx }))
  else if stx.isOfKind ``leanerSpecFunctionItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a specification function must have a name"
    let types := signatureTypeChildren stx
    let some result := types.back?
      | throw s!"specification function `{name.getId}` must have a result type"
    let expressions := exprChildren stx
    pure (.item (.specFunction {
      name := name.getId.getString!
      isOpaque := containsAtom "opaque" stx
      generics := ← (childrenWhere isGenericBinderSyntax stx).mapM binderOf
      parameters := ← (childrenOfKind ``leanerParameterSyntax stx).mapM parameterOf
      result := ← typeOf result
      body := ← expressions.back?.mapM expressionOf
      attributes := ← (childrenWhere isAttributeSyntax stx).mapM attributeOf
      span := spanOf stx }))
  else if stx.isOfKind ``leanerNamespaceInvariantItem then
    let members := childrenWhere
      (·.isOfKind ``LeanerLang.leanerNamespaceInvariantMemberSyntax) stx
    if members.isEmpty then throw "a module specification must declare an invariant"
    let declarations ← members.mapM fun member => do
      let some expression := (exprChildren member)[0]?
        | throw "a module invariant requires an expression"
      pure ({
        expression := ← expressionOf expression
        properties := (childrenOfKind ``leanerConditionPropertiesSyntax member).flatMap identifiers
        span := spanOf member } : NamespaceInvariantDecl)
    pure (.item (.namespaceInvariants declarations))
  else if stx.isOfKind ``leanerContractItem || stx.isOfKind ``leanerContractWhereItem then
    let some name := stx.getArgs.find? (·.isIdent)
      | throw "a function contract must name its function"
    pure (.contract name.getId.getString!
      (← (childrenWhere isClauseSyntax stx).mapM clauseOf)
      (← (childrenWhere isPragmaSyntax stx).mapM pragmaOf) (spanOf stx))
  else throw s!"unsupported Leaner namespace item `{stx.getKind}`"

private def attachContracts (parsed : Array ParsedItem) : Except String (Array Item) := do
  let contracts := parsed.filterMap fun
    | .contract name clauses pragmas span => some (name, clauses, pragmas, span)
    | _ => none
  for (name, _, _, _) in contracts do
    if contracts.countP (·.1 == name) > 1 then
      throw s!"function `{name}` has more than one `spec` block"
    unless parsed.any fun
        | .item (.function declaration) => declaration.name == name
        | .item (.struct declaration) => declaration.name == name
        | .item (.enum declaration) => declaration.name == name
        | _ => false do
      throw s!"specification names unknown function or nominal declaration `{name}`"
  return parsed.filterMap fun
    | .contract .. => none
    | .use .. => none
    | .friend .. => none
    | .namespacePragma .. => none
    | .item (.function declaration) =>
        let contract := contracts.find? (·.1 == declaration.name)
        let clauses := contract.map (·.2.1) |>.getD #[]
        let pragmas := contract.map (·.2.2.1) |>.getD #[]
        some (.function { declaration with contract := clauses, pragmas })
    | .item (.struct declaration) =>
        let contract := contracts.find? (·.1 == declaration.name)
        let clauses := contract.map (·.2.1) |>.getD #[]
        let pragmas := contract.map (·.2.2.1) |>.getD #[]
        some (.struct { declaration with contract := clauses, pragmas })
    | .item (.enum declaration) =>
        let contract := contracts.find? (·.1 == declaration.name)
        let clauses := contract.map (·.2.1) |>.getD #[]
        let pragmas := contract.map (·.2.2.1) |>.getD #[]
        some (.enum { declaration with contract := clauses, pragmas })
    | .item item => some item

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

private def renderCompileError : CompileError → String
  | .frontend diagnostics =>
      String.intercalate "\n" (diagnostics.toList.map Diagnostic.render)
  | .lir diagnostics =>
      String.intercalate "\n" (diagnostics.toList.map fun diagnostic =>
        s!"{diagnostic.code}: {diagnostic.message}")

/-- Convert one parsed Leaner namespace/module command into the frontend AST used
by lowering. The syntax node paired with an error lets command elaboration
retain its authored diagnostic location while non-elaborator consumers (such
as the source formatter) reuse exactly the same conversion. -/
def compilationUnitOfSyntax (stx : Syntax) (sourceName : String)
    (comments : Array Comment := #[]) (namespaceDoc : String := "") :
    Except (Syntax × String) CompilationUnit := do
  let some pathSyntax := (pathChildren stx)[0]?
    | throw (stx, "a Leaner namespace requires a path")
  let path ← pathOf pathSyntax |>.mapError (pathSyntax, ·)
  let profile ← if stx.isOfKind ``leanerMoveModuleCommand then pure ProfileName.move
    else if stx.isOfKind ``leanerRustNamespaceCommand then pure ProfileName.rust
    else do
      let some profileSyntax := (childrenWhere (fun child =>
          child.isOfKind ``leanerMoveProfile || child.isOfKind ``leanerRustProfile) stx)[0]?
        | throw (stx, "a legacy Leaner namespace requires a profile")
      profileOf profileSyntax |>.mapError (profileSyntax, ·)
  if stx.isOfKind ``leanerMoveModuleCommand then
    let address := path[0]?.getD ""
    let digits := address.toList.drop 2
    let hexDigit := fun character => character.isDigit ||
      ('a' <= character && character <= 'f') || ('A' <= character && character <= 'F')
    unless path.size == 2 && address.startsWith "0x" && !digits.isEmpty &&
        digits.all hexDigit do
      throw (pathSyntax,
        "a Move module path must be exactly `0xADDRESS::module_name`")
  else if profile == .rust && path[0]?.any (fun segment => segment.startsWith "0x") then
    throw (pathSyntax, "a hexadecimal address may lead a path only in the Move profile")
  let parsedItems ← (childrenWhere isItemSyntax stx).mapM itemOf |>.mapError (stx, ·)
  let pragmas := parsedItems.filterMap fun
    | .namespacePragma pragma => some pragma
    | _ => none
  let uses := parsedItems.filterMap fun
    | .use path => some path
    | _ => none
  let friends := parsedItems.filterMap fun
    | .friend declaration => some declaration
    | _ => none
  if uses.any (·.size < 2) then
    throw (stx, "a use declaration must contain at least two path segments")
  if uses.zipIdx.any fun (path, index) => uses.take index |>.contains path then
    throw (stx, "a use declaration may occur only once")
  let parsed ← attachContracts parsedItems |>.mapError (stx, ·)
  pure {
    sourceName
    namespaces := #[{
      path, profile, doc := namespaceDoc, uses, friends, pragmas, comments,
      items := parsed, span := spanOf stx }] }

@[command_elab leanerNamespaceCommand, command_elab leanerMoveModuleCommand,
  command_elab leanerRustNamespaceCommand]
def elaborateNamespace : CommandElab := fun stx => do
  let sourceName ← getFileName
  let source := (← getFileMap).source
  let span := spanOf stx
  let comments := commentsForCommand source span
  let namespaceDoc := documentationBefore source span.startByte
  let unit ← match compilationUnitOfSyntax stx sourceName comments namespaceDoc with
    | .ok unit => pure unit
    | .error (location, message) => throwErrorAt location message
  let pathSyntax := (pathChildren stx)[0]!
  let validated ← match compile unit with
    | .ok value => pure value
    | .error error => throwErrorAt stx (renderCompileError error)
  let declarationName := pathName unit.namespaces[0]!.path
  let environment ← match registerUnit (← getEnv) declarationName validated with
    | .ok environment => pure environment
    | .error message => throwErrorAt pathSyntax message
  setEnv environment

@[command_elab checkLeanerCommand]
def elaborateCheck : CommandElab := fun stx => do
  let some pathSyntax := (pathChildren stx)[0]?
    | throwErrorAt stx "`#check_leaner` requires a namespace path"
  let path ← match pathOf pathSyntax with
    | .ok value => pure value
    | .error message => throwErrorAt pathSyntax message
  let name := pathName path
  let some unit := registeredUnit? (← getEnv) name
    | throwErrorAt pathSyntax s!"unknown Leaner namespace `{name}`"
  logInfoAt pathSyntax s!"Leaner namespace `{name}`: {unit.namespaces.size} namespace(s), \
    {unit.indexes.functionCounts.foldl (· + ·) 0} executable function(s)"

end LeanerLang

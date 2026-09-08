-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Lean parser declarations for LeanerLang

The parser owns only source shape. Resolution, typing, LIR construction, and
validation remain in `LeanerLang.Lower` and the shared checker.
-/

namespace LeanerLang

private def kwMove := Lean.Parser.nonReservedSymbol "move" true
private def kwRust := Lean.Parser.nonReservedSymbol "rust" true
private def kwUnit := Lean.Parser.nonReservedSymbol "Unit" true
private def kwNever := Lean.Parser.nonReservedSymbol "Never" true
private def kwBool := Lean.Parser.nonReservedSymbol "Bool" true
private def kwChar := Lean.Parser.nonReservedSymbol "Char" true
private def kwString := Lean.Parser.nonReservedSymbol "string" true
private def kwBytes := Lean.Parser.nonReservedSymbol "Bytes" true
private def kwAddress := Lean.Parser.nonReservedSymbol "Address" true
private def kwSigner := Lean.Parser.nonReservedSymbol "Signer" true
private def kwUInt := Lean.Parser.nonReservedSymbol "UInt" true
private def kwSInt := Lean.Parser.nonReservedSymbol "SInt" true
private def kwUPtr := Lean.Parser.nonReservedSymbol "UPtr" true
private def kwIPtr := Lean.Parser.nonReservedSymbol "IPtr" true
private def kwU8 := Lean.Parser.nonReservedSymbol "u8" true
private def kwU16 := Lean.Parser.nonReservedSymbol "u16" true
private def kwU32 := Lean.Parser.nonReservedSymbol "u32" true
private def kwU64 := Lean.Parser.nonReservedSymbol "u64" true
private def kwU128 := Lean.Parser.nonReservedSymbol "u128" true
private def kwU256 := Lean.Parser.nonReservedSymbol "u256" true
private def kwI8 := Lean.Parser.nonReservedSymbol "i8" true
private def kwI16 := Lean.Parser.nonReservedSymbol "i16" true
private def kwI32 := Lean.Parser.nonReservedSymbol "i32" true
private def kwI64 := Lean.Parser.nonReservedSymbol "i64" true
private def kwI128 := Lean.Parser.nonReservedSymbol "i128" true
private def kwI256 := Lean.Parser.nonReservedSymbol "i256" true
private def kwUSize := Lean.Parser.nonReservedSymbol "usize" true
private def kwISize := Lean.Parser.nonReservedSymbol "isize" true
private def kwNat := Lean.Parser.nonReservedSymbol "Nat" true
private def kwInt := Lean.Parser.nonReservedSymbol "Int" true
private def kwRange := Lean.Parser.nonReservedSymbol "Range" true
private def kwVector := Lean.Parser.nonReservedSymbol "Vector" true
private def kwVectorLiteral := Lean.Parser.nonReservedSymbol "vector" true
private def kwFn := Lean.Parser.nonReservedSymbol "Fn" true
private def kwConst := Lean.Parser.symbol "const"
private def kwType := Lean.Parser.nonReservedSymbol "type" true
private def kwLifetime := Lean.Parser.nonReservedSymbol "lifetime" true
private def kwEvidence := Lean.Parser.nonReservedSymbol "evidence" true
private def kwMut := Lean.Parser.nonReservedSymbol "mut" true
private def kwPrivate := Lean.Parser.nonReservedSymbol "private" true
private def kwPublic := Lean.Parser.nonReservedSymbol "public" true
private def kwPackage := Lean.Parser.nonReservedSymbol "package" true
private def kwFriend := Lean.Parser.nonReservedSymbol "friend" true
private def kwEntry := Lean.Parser.nonReservedSymbol "entry" true
private def kwNative := Lean.Parser.symbol "native"
private def kwOpaque := Lean.Parser.nonReservedSymbol "opaque" true
private def kwDeprecated := Lean.Parser.nonReservedSymbol "deprecated" true
private def kwView := Lean.Parser.nonReservedSymbol "view" true
private def kwPragma := Lean.Parser.nonReservedSymbol "pragma" true
private def kwRequires := Lean.Parser.nonReservedSymbol "requires" true
private def kwEnsures := Lean.Parser.nonReservedSymbol "ensures" true
private def kwAbortsIf := Lean.Parser.nonReservedSymbol "aborts_if" true
private def kwLetPre := Lean.Parser.nonReservedSymbol "let_pre" true
private def kwLetPost := Lean.Parser.nonReservedSymbol "let_post" true
private def kwModifies := Lean.Parser.nonReservedSymbol "modifies" true
private def kwReads := Lean.Parser.nonReservedSymbol "reads" true
private def kwAssert := Lean.Parser.nonReservedSymbol "assert" true
private def kwAssertBang := Lean.Parser.nonReservedSymbol "assert!" true
private def kwAssume := Lean.Parser.nonReservedSymbol "assume" true
private def kwInvariant := Lean.Parser.nonReservedSymbol "invariant" true
private def kwSpec := Lean.Parser.symbol "spec"
private def kwVerify := Lean.Parser.nonReservedSymbol "verify" true
private def kwFun := Lean.Parser.nonReservedSymbol "fun" true
private def kwNamespace := Lean.Parser.nonReservedSymbol "namespace" true
private def kwModule := Lean.Parser.nonReservedSymbol "module" true
private def kwUsing := Lean.Parser.nonReservedSymbol "using" true
private def kwWhere := Lean.Parser.nonReservedSymbol "where" true
private def kwUse := Lean.Parser.symbol "use"
private def kwStruct := Lean.Parser.nonReservedSymbol "struct" true
private def kwEnum := Lean.Parser.nonReservedSymbol "enum" true
private def kwHas := Lean.Parser.nonReservedSymbol "has" true
private def kwCopy := Lean.Parser.nonReservedSymbol "Copy" true
private def kwDrop := Lean.Parser.nonReservedSymbol "Drop" true
private def kwStore := Lean.Parser.nonReservedSymbol "Store" true
private def kwKey := Lean.Parser.nonReservedSymbol "Key" true
private def kwTrue := Lean.Parser.nonReservedSymbol "true" true
private def kwFalse := Lean.Parser.nonReservedSymbol "false" true
private def kwAbort := Lean.Parser.nonReservedSymbol "abort" true
private def kwPanic := Lean.Parser.nonReservedSymbol "panic" true
private def kwDo := Lean.Parser.nonReservedSymbol "do" true
private def kwLet := Lean.Parser.nonReservedSymbol "let" true
private def kwMatch := Lean.Parser.nonReservedSymbol "match" true
private def kwWith := Lean.Parser.nonReservedSymbol "with" true
private def kwIf := Lean.Parser.nonReservedSymbol "if" true
private def kwThen := Lean.Parser.symbol "then"
private def kwElse := Lean.Parser.symbol "else"
private def kwLoop := Lean.Parser.symbol "loop"
private def kwWhile := Lean.Parser.nonReservedSymbol "while" true
private def kwFor := Lean.Parser.nonReservedSymbol "for" true
private def kwBreak := Lean.Parser.nonReservedSymbol "break" true
private def kwContinue := Lean.Parser.nonReservedSymbol "continue" true
private def kwReturn := Lean.Parser.symbol "return"
private def kwNew := Lean.Parser.nonReservedSymbol "new" true
private def kwForall := Lean.Parser.nonReservedSymbol "forall" true
private def kwExists := Lean.Parser.nonReservedSymbol "exists" true
private def kwIn := Lean.Parser.nonReservedSymbol "in" true
private def kwIs := Lean.Parser.nonReservedSymbol "is" true
private def kwAs := Lean.Parser.nonReservedSymbol "as" true
private def kwSpecOld := Lean.Parser.nonReservedSymbol "spec.old" true
private def kwOld := Lean.Parser.symbol "old"
private def kwRequiresOf := Lean.Parser.nonReservedSymbol "requires_of" true
private def kwAbortsOf := Lean.Parser.nonReservedSymbol "aborts_of" true
private def kwEnsuresOf := Lean.Parser.nonReservedSymbol "ensures_of" true
private def kwResultOf := Lean.Parser.nonReservedSymbol "result_of" true
private def kwUnchangedOf := Lean.Parser.nonReservedSymbol "unchanged_of" true
private def kwFoldsOf := Lean.Parser.nonReservedSymbol "folds_of" true
private def kwWriteOf := Lean.Parser.nonReservedSymbol "write_of" true
private def kwSpecSaveStateAnchor :=
  Lean.Parser.nonReservedSymbol "spec.saveStateAnchor" true
private def kwSpecWithStateAnchor :=
  Lean.Parser.nonReservedSymbol "spec.withStateAnchor" true
private def kwSpecFoldsCaptureAnchor :=
  Lean.Parser.nonReservedSymbol "spec.foldsCaptureAnchor" true
private def kwSpecResult := Lean.Parser.nonReservedSymbol "spec.result" true
private def kwSpecInlineCallSummary :=
  Lean.Parser.nonReservedSymbol "spec.inlineCallSummary" true
private def kwSpecGlobal := Lean.Parser.nonReservedSymbol "spec.global" true
private def kwMoveSpecGlobal := Lean.Parser.nonReservedSymbol "global" true
private def kwSpecEmptyVector := Lean.Parser.nonReservedSymbol "spec.emptyVector" true
private def kwSpecSingletonVector := Lean.Parser.nonReservedSymbol "spec.singletonVector" true
private def kwSpecUpdateVector := Lean.Parser.nonReservedSymbol "spec.updateVector" true
private def kwSpecConcatVector := Lean.Parser.nonReservedSymbol "spec.concatVector" true
private def kwSpecIndexOfVector := Lean.Parser.nonReservedSymbol "spec.indexOfVector" true
private def kwSpecContainsVector := Lean.Parser.nonReservedSymbol "spec.containsVector" true
private def kwSpecLengthVector := Lean.Parser.nonReservedSymbol "spec.lengthVector" true
private def kwSpecIndexVector := Lean.Parser.nonReservedSymbol "spec.indexVector" true
private def kwSpecSliceVector := Lean.Parser.nonReservedSymbol "spec.sliceVector" true
private def kwSpecBitVectorToInt := Lean.Parser.nonReservedSymbol "spec.bitVectorToInt" true
private def kwSpecIntToBitVector := Lean.Parser.nonReservedSymbol "spec.intToBitVector" true
private def kwSpecInRange := Lean.Parser.nonReservedSymbol "spec.inRange" true
private def kwBitVectorToInt := Lean.Parser.nonReservedSymbol "bit_vector_to_int" true
private def kwIntToBitVector := Lean.Parser.nonReservedSymbol "int_to_bit_vector" true
private def kwInRange := Lean.Parser.nonReservedSymbol "in_range" true
private def kwSpecInVectorRange := Lean.Parser.nonReservedSymbol "spec.inVectorRange" true
private def kwSpecVectorRange := Lean.Parser.nonReservedSymbol "spec.vectorRange" true
private def kwSpecVec := Lean.Parser.nonReservedSymbol "vec" true
private def kwSaveStateAnchor := Lean.Parser.nonReservedSymbol "save_state_anchor" true
private def kwFoldsCaptureAnchor := Lean.Parser.nonReservedSymbol "folds_capture_anchor" true
private def kwWithStateAnchor := Lean.Parser.nonReservedSymbol "with_state_anchor" true
private def kwAddressLiteral := Lean.Parser.nonReservedSymbol "core.address" true
private def kwGlobalExists := Lean.Parser.nonReservedSymbol "exists" true
private def kwBorrowGlobal := Lean.Parser.nonReservedSymbol "borrow_global" true
private def kwBorrowGlobalMut := Lean.Parser.nonReservedSymbol "borrow_global_mut" true
private def kwMoveFrom := Lean.Parser.nonReservedSymbol "move_from" true
private def kwMoveTo := Lean.Parser.nonReservedSymbol "move_to" true
private def kwByteStringPrefix := Lean.Parser.nonReservedSymbol "b" true
private def kwGlobalContains := Lean.Parser.nonReservedSymbol "core.global.contains" true
private def kwGlobalBorrow := Lean.Parser.nonReservedSymbol "core.global.borrow" true
private def kwGlobalTake := Lean.Parser.nonReservedSymbol "core.global.take" true
private def kwGlobalPublish := Lean.Parser.nonReservedSymbol "core.global.publish" true
private def kwPrimitiveCast := Lean.Parser.nonReservedSymbol "core.prim.cast" true
private def kwPrimitiveCheckedCast := Lean.Parser.nonReservedSymbol "core.prim.checkedCast" true
private def kwConstruct := Lean.Parser.nonReservedSymbol "core.construct" true
private def kwDataSelect := Lean.Parser.nonReservedSymbol "core.data.select" true
private def kwDataSelectVariants :=
  Lean.Parser.nonReservedSymbol "core.data.selectVariants" true
private def kwDataTestVariants := Lean.Parser.nonReservedSymbol "core.data.testVariants" true
private def kwDataDiscriminant := Lean.Parser.nonReservedSymbol "core.data.discriminant" true
private def kwCoreCall := Lean.Parser.nonReservedSymbol "core.call" true
private def kwCoreInvoke := Lean.Parser.nonReservedSymbol "core.invoke" true
private def kwCoreClosure := Lean.Parser.nonReservedSymbol "core.closure" true
private def kwCoreAssignPattern := Lean.Parser.nonReservedSymbol "core.assignPattern" true
private def kwAssignPattern := Lean.Parser.nonReservedSymbol "assign_pattern" true
private def kwCoreBorrow := Lean.Parser.nonReservedSymbol "core.borrow" true
private def kwCoreBorrowPlace := Lean.Parser.nonReservedSymbol "core.borrowPlace" true
private def kwCoreAssignPlace := Lean.Parser.nonReservedSymbol "core.assignPlace" true
private def kwCoreDrop := Lean.Parser.nonReservedSymbol "core.drop" true
private def kwDropBuiltin := Lean.Parser.nonReservedSymbol "drop" true
private def kwCopyBuiltin := Lean.Parser.nonReservedSymbol "copy" true
private def kwReadBuiltin := Lean.Parser.nonReservedSymbol "core.read" true
private def kwDiscriminant := Lean.Parser.nonReservedSymbol "discriminant" true
private def kwInvoke := Lean.Parser.nonReservedSymbol "invoke" true
private def kwFunctionValue := Lean.Parser.nonReservedSymbol "function" true
private def kwCoreRefBorrow := Lean.Parser.nonReservedSymbol "core.ref.borrow" true
private def kwCoreRefFreeze := Lean.Parser.nonReservedSymbol "core.ref.freeze" true
private def kwCoreRefFreezeExplicit := Lean.Parser.nonReservedSymbol "core.ref.freezeExplicit" true
private def kwCoreRefDereference := Lean.Parser.nonReservedSymbol "core.ref.dereference" true
private def kwCoreRefMutate := Lean.Parser.nonReservedSymbol "core.ref.mutate" true
private def kwImmutable := Lean.Parser.nonReservedSymbol "immutable" true
private def kwPhantom := Lean.Parser.nonReservedSymbol "phantom" true

/-- Identifiers are contextual to LeanerLang: host-Lean keywords such as
`end` and `at`, and modifiers such as `view`, remain ordinary local names.
Clause delimiters must use `«...»` when intended as identifiers, so an
expression cannot consume the next contract clause. -/
private def leanerIdentifier : Lean.Parser.Parser :=
  ["_", "move", "rust", "Unit", "Never", "Bool", "Char", "string", "Bytes",
    "Address", "Signer", "UInt", "SInt", "UPtr", "IPtr", "Nat", "Int",
    "u8", "u16", "u32", "u64", "u128", "u256", "i8", "i16", "i32", "i64",
    "i128", "i256", "usize", "isize", "Range", "Vector", "Fn", "const",
    "type", "lifetime", "evidence", "mut", "private", "public", "package",
    "friend", "entry", "native", "opaque", "deprecated", "view", "pragma",
    "let_pre", "let_post", "modifies", "reads", "requires", "ensures",
    "aborts_if", "assert", "assume", "invariant", "spec", "fun", "module",
    "namespace", "using", "where", "struct", "enum", "has", "Copy", "Drop",
    "Store", "Key", "true", "false", "abort", "panic", "do", "let", "loop",
    "while", "for", "break", "continue", "forall", "exists", "in", "immutable", "if",
    "then", "else", "return", "old", "copy", "drop",
    "discriminant", "invoke", "function", "as", "match", "with"].foldr
      (fun keyword parser =>
        (if ["let_pre", "let_post", "modifies", "reads", "requires", "ensures",
            "aborts_if", "invariant"].contains keyword then
          Lean.Parser.notFollowedBy (Lean.Parser.nonReservedSymbol keyword true) keyword
        else Lean.Parser.notSymbol keyword) >> parser)
      Lean.Parser.rawIdent

attribute [run_builtin_parser_attribute_hooks] leanerIdentifier

builtin_initialize register_parser_alias leanerIdentifier

declare_syntax_cat leanerProfile
syntax (name := leanerMoveProfile) kwMove : leanerProfile
syntax (name := leanerRustProfile) kwRust : leanerProfile

declare_syntax_cat leanerPath
@[leanerPath_parser] def leanerPathSyntax := leading_parser
  (Lean.Parser.atomic (Lean.Parser.numLit >> "::" >> leanerIdentifier) <|>
    leanerIdentifier) >>
    Lean.Parser.many (Lean.Parser.atomic ("::" >> leanerIdentifier))

declare_syntax_cat leanerAbility
syntax (name := leanerCopyAbility) kwCopy : leanerAbility
syntax (name := leanerDropAbility) kwDrop : leanerAbility
syntax (name := leanerStoreAbility) kwStore : leanerAbility
syntax (name := leanerKeyAbility) kwKey : leanerAbility

declare_syntax_cat leanerType
declare_syntax_cat leanerTypeArgument
declare_syntax_cat leanerLifetime
syntax (name := leanerInferenceLifetime) "_" : leanerLifetime
syntax (name := leanerNamedLifetime) leanerIdentifier : leanerLifetime
syntax (name := leanerUnitType) kwUnit : leanerType
syntax (name := leanerNeverType) kwNever : leanerType
syntax (name := leanerBoolType) kwBool : leanerType
syntax (name := leanerCharType) kwChar : leanerType
syntax (name := leanerStringType) kwString : leanerType
syntax (name := leanerBytesType) kwBytes : leanerType
syntax (name := leanerAddressType) kwAddress : leanerType
syntax (name := leanerSignerType) kwSigner : leanerType
syntax (name := leanerUIntType) kwUInt "<" num ">" : leanerType
syntax (name := leanerSIntType) kwSInt "<" num ">" : leanerType
syntax (name := leanerUPtrType) kwUPtr : leanerType
syntax (name := leanerIPtrType) kwIPtr : leanerType
syntax (name := leanerU8Type) kwU8 : leanerType
syntax (name := leanerU16Type) kwU16 : leanerType
syntax (name := leanerU32Type) kwU32 : leanerType
syntax (name := leanerU64Type) kwU64 : leanerType
syntax (name := leanerU128Type) kwU128 : leanerType
syntax (name := leanerU256Type) kwU256 : leanerType
syntax (name := leanerI8Type) kwI8 : leanerType
syntax (name := leanerI16Type) kwI16 : leanerType
syntax (name := leanerI32Type) kwI32 : leanerType
syntax (name := leanerI64Type) kwI64 : leanerType
syntax (name := leanerI128Type) kwI128 : leanerType
syntax (name := leanerI256Type) kwI256 : leanerType
syntax (name := leanerUSizeType) kwUSize : leanerType
syntax (name := leanerISizeType) kwISize : leanerType
syntax (name := leanerNatType) kwNat : leanerType
syntax (name := leanerIntType) kwInt : leanerType
syntax (name := leanerRangeType) kwRange : leanerType
syntax (name := leanerVectorType) (priority := high)
  kwVector "<" leanerType ">" : leanerType
syntax (name := leanerFixedVectorType)
  (priority := high)
  kwVector "<" leanerType "," kwConst num ">" : leanerType
syntax (name := leanerFunctionType)
  kwFn "(" leanerType,* ")" "->" leanerType (kwHas leanerAbility,+)? : leanerType
syntax (name := leanerReferenceType)
  "&" ("[" leanerLifetime "]")? (kwMut)? leanerType : leanerType
syntax (name := leanerTupleType) "(" leanerType,+ ")" : leanerType
syntax (name := leanerTypeArgumentSyntax) (kwType)? leanerType : leanerTypeArgument
syntax (name := leanerIntegerConstTypeArgument)
  kwConst ("-")? num : leanerTypeArgument
syntax (name := leanerTrueConstTypeArgument) kwConst kwTrue : leanerTypeArgument
syntax (name := leanerFalseConstTypeArgument) kwConst kwFalse : leanerTypeArgument
syntax (name := leanerLifetimeTypeArgument)
  kwLifetime leanerLifetime : leanerTypeArgument
syntax (name := leanerAppliedType)
  leanerPath atomic("::" "<") leanerTypeArgument,* ">" : leanerType
syntax (name := leanerStandardAppliedType)
  leanerPath atomic("<") leanerTypeArgument,* ">" : leanerType
syntax (name := leanerNamedType) (priority := low) leanerPath : leanerType

declare_syntax_cat leanerGenericBinder
syntax (name := leanerTypeBinder)
  "{" leanerIdentifier ":" (kwPhantom)? kwType (kwHas leanerAbility,+)? "}" :
    leanerGenericBinder
syntax (name := leanerInferredTypeBinder)
  "{" leanerIdentifier (kwHas leanerAbility,+)? "}" : leanerGenericBinder
syntax (name := leanerConstBinder)
  "{" leanerIdentifier ":" kwConst "}" : leanerGenericBinder
syntax (name := leanerTypedConstBinder)
  "{" leanerIdentifier ":" kwConst leanerType "}" : leanerGenericBinder
syntax (name := leanerLifetimeBinder)
  "{" leanerIdentifier ":" kwLifetime "}" : leanerGenericBinder
syntax (name := leanerEvidenceBinder)
  "{" leanerIdentifier ":" kwEvidence "}" : leanerGenericBinder

declare_syntax_cat leanerThrow
syntax (name := leanerAbortThrow) kwAbort : leanerThrow
syntax (name := leanerPanicThrow) kwPanic : leanerThrow
syntax (name := leanerMoveVectorErrorThrow) "moveVectorError" : leanerThrow

declare_syntax_cat leanerExpr
declare_syntax_cat leanerFieldIdentifier
syntax (name := leanerNamedFieldIdentifier) leanerIdentifier : leanerFieldIdentifier
syntax (name := leanerNumericFieldIdentifier) num : leanerFieldIdentifier
syntax (name := leanerUnitExpr) "(" ")" : leanerExpr
syntax (name := leanerTrueExpr) kwTrue : leanerExpr
syntax (name := leanerFalseExpr) kwFalse : leanerExpr
syntax (name := leanerCharExpr) char : leanerExpr
syntax (name := leanerIntegerExpr) num : leanerExpr
syntax (name := leanerNegativeIntegerExpr) "-" num : leanerExpr
syntax (name := leanerTypedIntegerExpr)
  num (kwU8 <|> kwU16 <|> kwU32 <|> kwU64 <|> kwU128 <|> kwU256 <|>
    kwI8 <|> kwI16 <|> kwI32 <|> kwI64 <|> kwI128 <|> kwI256 <|>
    kwUSize <|> kwISize) : leanerExpr
syntax (name := leanerNegativeTypedIntegerExpr)
  "-" num (kwU8 <|> kwU16 <|> kwU32 <|> kwU64 <|> kwU128 <|> kwU256 <|>
    kwI8 <|> kwI16 <|> kwI32 <|> kwI64 <|> kwI128 <|> kwI256 <|>
    kwUSize <|> kwISize) : leanerExpr
syntax (name := leanerAddressExpr) kwAddressLiteral "(" str ")" : leanerExpr
syntax (name := leanerMoveAddressExpr) "@" num : leanerExpr
syntax (name := leanerStringExpr) str : leanerExpr
syntax (name := leanerByteStringExpr) atomic(kwByteStringPrefix str) : leanerExpr
syntax (name := leanerBytesExpr) "b[" num,* "]" : leanerExpr
syntax (name := leanerLocalExpr) (priority := low) leanerIdentifier : leanerExpr
syntax (name := leanerParenExpr) "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSingletonTupleExpr) "(" leanerExpr "," ")" : leanerExpr
syntax (name := leanerTupleExpr) "(" leanerExpr "," leanerExpr,+ ")" : leanerExpr
syntax (name := leanerVectorExpr) "#[" leanerExpr,* "]" : leanerExpr
syntax (name := leanerRepeatVectorExpr) "#[" leanerExpr ";" num "]" : leanerExpr
syntax (name := leanerTypedVectorExpr)
  kwVectorLiteral "<" leanerType ">" "[" leanerExpr,* "]" : leanerExpr
syntax (name := leanerRuntimeAssertExpr) (priority := high)
  kwAssert "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerRuntimeAssertMacroExpr) (priority := high)
  kwAssertBang "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerThrowSurfaceExpr) (priority := high)
  (kwAbort <|> kwPanic <|> "moveVectorError") "(" leanerExpr,* ")" : leanerExpr
declare_syntax_cat leanerBehaviorCall
syntax (name := leanerBehaviorCallSyntax)
  (kwRequiresOf <|> kwAbortsOf <|> kwEnsuresOf <|> kwResultOf <|>
    kwUnchangedOf <|> kwFoldsOf)
  "<" leanerExpr:14 ">" "(" leanerExpr,* ")" : leanerBehaviorCall
syntax (name := leanerWriteBehaviorCallSyntax)
  kwWriteOf "[" num "]" "<" leanerExpr:14 ">"
    "(" leanerExpr,* ")" : leanerBehaviorCall
syntax (name := leanerBehaviorExpr) (priority := high)
  leanerBehaviorCall : leanerExpr
syntax (name := leanerBehaviorAtExpr) (priority := high)
  "@" num "|~" leanerBehaviorCall : leanerExpr
syntax (name := leanerBehaviorPreRangeExpr) (priority := high)
  "@" num ".." "|~" leanerBehaviorCall : leanerExpr
syntax (name := leanerBehaviorPostRangeExpr) (priority := high)
  ".." "@" num "|~" leanerBehaviorCall : leanerExpr
syntax (name := leanerBehaviorFullRangeExpr) (priority := high)
  "@" num ".." "@" num "|~" leanerBehaviorCall : leanerExpr
declare_syntax_cat leanerStorageHead
syntax (name := leanerNamedStorageHead) leanerPath : leanerStorageHead
syntax (name := leanerAppliedStorageHead)
  leanerPath "<" leanerType,* ">" : leanerStorageHead
declare_syntax_cat leanerTypeArguments
syntax (name := leanerTypeArgumentsSyntax)
  atomic("::" "<") leanerType,* ">" : leanerTypeArguments
syntax:14 (name := leanerMethodCallExpr) (priority := high)
  leanerExpr:14 colGt "." leanerIdentifier (leanerTypeArguments)?
    "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerTypedMethodCallExpr) (priority := high)
  "(" leanerExpr:14 colGt "." leanerIdentifier (leanerTypeArguments)?
    "(" leanerExpr,* ")" ":" leanerType ")" : leanerExpr
syntax:14 (name := leanerFieldExpr)
  leanerExpr:14 colGt "." leanerFieldIdentifier : leanerExpr
syntax:14 (name := leanerStorageIndexExpr) (priority := low)
  atomic(leanerStorageHead colGt "[") leanerExpr "]" : leanerExpr
syntax:14 (name := leanerIndexExpr)
  leanerExpr:14 colGt "[" leanerExpr "]" ("!")? : leanerExpr
-- These static parser declarations mirror `LeanerLang.Operators.table`.
-- The table is authoritative for semantic lookup and pretty-printing.
syntax:13 (name := leanerLogicalNotExpr) "!" leanerExpr:13 : leanerExpr
syntax:13 (name := leanerBitwiseNotExpr) "~" leanerExpr:13 : leanerExpr
syntax:13 (name := leanerNegateExpr) "-" leanerExpr:13 : leanerExpr
syntax:12 (name := leanerMultiplyExpr) leanerExpr:12 colGt "*" leanerExpr:13 : leanerExpr
syntax:12 (name := leanerDivideExpr) leanerExpr:12 colGt "/" leanerExpr:13 : leanerExpr
syntax:12 (name := leanerModuloExpr) leanerExpr:12 colGt "%" leanerExpr:13 : leanerExpr
syntax:12 (name := leanerCastSurfaceExpr) leanerExpr:13 colGt kwAs leanerType : leanerExpr
syntax:11 (name := leanerAddExpr) leanerExpr:11 colGt "+" leanerExpr:12 : leanerExpr
syntax:11 (name := leanerSubtractExpr) leanerExpr:11 colGt "-" leanerExpr:12 : leanerExpr
syntax:10 (name := leanerShiftLeftExpr) leanerExpr:10 colGt "<<" leanerExpr:11 : leanerExpr
syntax:10 (name := leanerShiftRightExpr) leanerExpr:10 colGt ">>" leanerExpr:11 : leanerExpr
syntax:9 (name := leanerBitwiseAndExpr) leanerExpr:9 colGt "&" leanerExpr:10 : leanerExpr
syntax:8 (name := leanerBitwiseXorExpr) leanerExpr:8 colGt "^" leanerExpr:9 : leanerExpr
syntax:7 (name := leanerBitwiseOrExpr) leanerExpr:7 colGt "|" leanerExpr:8 : leanerExpr
syntax:6 (name := leanerRangeExpr) leanerExpr:6 colGt ".." leanerExpr:7 : leanerExpr
syntax:5 (name := leanerEqualExpr) leanerExpr:5 colGt "==" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerNotEqualExpr) leanerExpr:5 colGt "!=" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerLessExpr) leanerExpr:5 colGt "<" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerGreaterExpr) leanerExpr:5 colGt ">" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerLessEqualExpr) leanerExpr:5 colGt "<=" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerGreaterEqualExpr) leanerExpr:5 colGt ">=" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerMembershipExpr) leanerExpr:5 colGt "∈" leanerExpr:6 : leanerExpr
syntax:5 (name := leanerVariantTestSurfaceExpr)
  leanerExpr:6 colGt kwIs leanerIdentifier ("|" leanerIdentifier)* : leanerExpr
syntax:4 (name := leanerLogicalAndExpr) leanerExpr:4 colGt "&&" leanerExpr:5 : leanerExpr
syntax:3 (name := leanerLogicalOrExpr) leanerExpr:3 colGt "||" leanerExpr:4 : leanerExpr
syntax:2 (name := leanerImpliesExpr) leanerExpr:2 colGt "==>" leanerExpr:3 : leanerExpr
syntax:2 (name := leanerEquivalentExpr) leanerExpr:2 colGt "<==>" leanerExpr:3 : leanerExpr
syntax (name := leanerSpecificationIntToBitVectorExpr)
  (kwSpecIntToBitVector <|> kwIntToBitVector) "[" leanerType "]"
    "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationIntToBitVectorInferExpr) (priority := high)
  (kwSpecIntToBitVector <|> kwIntToBitVector) "(" leanerExpr ")" : leanerExpr
syntax (name := leanerGlobalContainsExpr) (priority := high)
  kwGlobalContains "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerGlobalBorrowExpr) (priority := high)
  kwGlobalBorrow "<" leanerType ">" "(" (kwImmutable <|> kwMut) "," leanerExpr ")" : leanerExpr
syntax (name := leanerGlobalTakeExpr) (priority := high)
  kwGlobalTake "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerGlobalPublishExpr) (priority := high)
  kwGlobalPublish "<" leanerType ">" "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerGlobalExistsSurfaceExpr) (priority := high)
  kwGlobalExists "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerBorrowGlobalSurfaceExpr) (priority := high)
  kwBorrowGlobal "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerBorrowGlobalMutSurfaceExpr) (priority := high)
  kwBorrowGlobalMut "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerMoveFromSurfaceExpr) (priority := high)
  kwMoveFrom "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerMoveToSurfaceExpr) (priority := high)
  kwMoveTo "<" leanerType ">" "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerConstructExpr)
  kwConstruct leanerPath "(" leanerExpr,* ")" : leanerExpr
declare_syntax_cat leanerConstructorVariant
syntax (name := leanerConstructorVariantSyntax)
  atomic("::" leanerIdentifier) : leanerConstructorVariant
syntax (name := leanerAppliedConstructExpr)
  kwConstruct leanerPath atomic("::" "<") leanerType,* ">"
    (leanerConstructorVariant)? "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerStandardAppliedConstructExpr) (priority := high)
  kwConstruct leanerType (leanerConstructorVariant)? "(" leanerExpr,* ")" : leanerExpr
declare_syntax_cat leanerConstructorField
syntax (name := leanerConstructorFieldSyntax)
  leanerFieldIdentifier ":=" leanerExpr : leanerConstructorField
syntax (name := leanerConstructorFieldShorthandSyntax)
  leanerIdentifier : leanerConstructorField
syntax (name := leanerNamedConstructExpr) (priority := low)
  kwNew leanerType (leanerConstructorVariant)?
    "{" leanerConstructorField,* "}" : leanerExpr
declare_syntax_cat leanerFieldName
syntax (name := leanerFieldNameSyntax) leanerFieldIdentifier : leanerFieldName
syntax (name := leanerSelectExpr)
  kwDataSelect "[" leanerType "," leanerFieldName "]" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSelectVariantsExpr)
  kwDataSelectVariants "[" leanerType "," leanerFieldName,+ "]"
    "(" leanerExpr ")" : leanerExpr
declare_syntax_cat leanerVariantName
syntax (name := leanerVariantNameSyntax) leanerIdentifier : leanerVariantName
syntax (name := leanerTestVariantsExpr)
  kwDataTestVariants "[" leanerType "," leanerVariantName,+ "]"
    "(" leanerExpr ")" : leanerExpr
syntax (name := leanerDiscriminantExpr)
  kwDataDiscriminant "[" leanerType "," leanerType "]"
    "(" leanerExpr ")" : leanerExpr
syntax (name := leanerDiscriminantSurfaceExpr)
  kwDiscriminant "[" leanerType "," leanerType "]"
    "(" leanerExpr ")" : leanerExpr
declare_syntax_cat leanerStatement
declare_syntax_cat leanerBlockResult
syntax (name := leanerStatementSyntax) leanerExpr ";" : leanerStatement
declare_syntax_cat leanerBindingPattern
declare_syntax_cat leanerBindingPatternField
syntax (name := leanerVariableBindingPattern) (priority := low)
  (kwMut)? leanerIdentifier : leanerBindingPattern
syntax (name := leanerWildcardBindingPattern) (priority := high) "_" : leanerBindingPattern
syntax (name := leanerTrueBindingPattern) (priority := high) kwTrue : leanerBindingPattern
syntax (name := leanerFalseBindingPattern) (priority := high) kwFalse : leanerBindingPattern
syntax (name := leanerCharBindingPattern) char : leanerBindingPattern
syntax (name := leanerIntegerBindingPattern) num : leanerBindingPattern
syntax (name := leanerNegativeIntegerBindingPattern) "-" num : leanerBindingPattern
syntax (name := leanerTupleBindingPattern)
  "(" leanerBindingPattern "," leanerBindingPattern,+ ")" : leanerBindingPattern
syntax (name := leanerSingletonTupleBindingPattern)
  "(" leanerBindingPattern "," ")" : leanerBindingPattern
syntax (name := leanerBindingPatternFieldSyntax)
  leanerFieldIdentifier ":=" leanerBindingPattern : leanerBindingPatternField
declare_syntax_cat leanerBindingPatternVariant
syntax (name := leanerBindingPatternVariantSyntax)
  atomic("::" leanerIdentifier) : leanerBindingPatternVariant
syntax (name := leanerConstructorBindingPattern)
  leanerType (leanerBindingPatternVariant)?
    "{" leanerBindingPatternField,* "}" : leanerBindingPattern
declare_syntax_cat leanerMatchArm
syntax (name := leanerMatchArmSyntax)
  -- `many1Indent` saves the arm column. The bitwise-or parser's `colGt`
  -- therefore admits `left | right` inside an arm but rejects the next
  -- aligned arm delimiter, so an arm body can use the full expression grammar.
  "|" leanerBindingPattern "=>" leanerExpr : leanerMatchArm
syntax (name := leanerGuardedMatchArmSyntax)
  "|" leanerBindingPattern kwIf leanerExpr "=>" leanerExpr : leanerMatchArm
@[leanerExpr_parser] def leanerMatchExpr := leading_parser
  kwMatch >> Lean.Parser.categoryParser `leanerExpr 0 >> kwWith >>
    Lean.Parser.withPosition (Lean.Parser.many1Indent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerMatchArm 0))
declare_syntax_cat leanerQuantifierBinder
syntax (name := leanerQuantifierBinderSyntax)
  leanerBindingPattern kwIn leanerExpr : leanerQuantifierBinder
syntax (name := leanerQuantifierTypeBinderSyntax)
  leanerBindingPattern ":" leanerType : leanerQuantifierBinder
syntax (name := leanerQuantifierExpr)
  (kwForall <|> kwExists <|> "∀" <|> "∃")
    "(" leanerQuantifierBinder (";" leanerQuantifierBinder)* ")" ","
      leanerExpr : leanerExpr
syntax (name := leanerSpecificationAnchorMarkerExpr)
  (kwSpecSaveStateAnchor <|> kwSpecFoldsCaptureAnchor)
    "[" num "]" "(" ")" : leanerExpr
syntax (name := leanerSpecificationAnchorMarkerBuiltinExpr) (priority := high)
  (kwSaveStateAnchor <|> kwFoldsCaptureAnchor) "!" "(" num ")" : leanerExpr
syntax (name := leanerSpecificationWithStateExpr)
  kwSpecWithStateAnchor "[" num "]" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationWithStateBuiltinExpr) (priority := high)
  kwWithStateAnchor "!" "(" num "," leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationOldExpr) (priority := high)
  (kwSpecOld <|> kwOld) "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationResultExpr) (priority := high)
  kwSpecResult "[" num "]" : leanerExpr
syntax (name := leanerSpecificationInlineCallSummaryExpr) (priority := high)
  kwSpecInlineCallSummary "(" leanerExpr "," leanerExpr ")" : leanerExpr
declare_syntax_cat leanerSpecificationTypeArgument
syntax (name := leanerSpecificationTypeArgumentSyntax)
  atomic("::" "<") leanerType ">" : leanerSpecificationTypeArgument
syntax (name := leanerSpecificationVectorExpr) (priority := high)
  (kwSpecEmptyVector <|> kwSpecSingletonVector <|> kwSpecUpdateVector <|>
    kwSpecConcatVector <|> kwSpecIndexOfVector <|> kwSpecContainsVector <|>
    kwSpecLengthVector <|> kwSpecIndexVector <|> kwSpecSliceVector <|>
    kwSpecInVectorRange <|> kwSpecVectorRange)
  (leanerSpecificationTypeArgument)? "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerSpecificationEmptyVectorBuiltinExpr) (priority := high)
  kwSpecVec atomic("::" "<") leanerType ">" "(" ")" : leanerExpr
syntax (name := leanerSpecificationGlobalExpr) (priority := high)
  kwSpecGlobal leanerSpecificationTypeArgument "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationGlobalSurfaceExpr) (priority := high)
  kwMoveSpecGlobal "<" leanerType ">" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationInRangeExpr) (priority := high)
  (kwSpecInRange <|> kwInRange) "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerSpecificationBitVectorToIntExpr) (priority := high)
  (kwSpecBitVectorToInt <|> kwBitVectorToInt) "(" leanerExpr ")" : leanerExpr
declare_syntax_cat leanerSpecStatement
syntax (name := leanerSpecLetStatement)
  kwLet leanerIdentifier ":=" leanerExpr (";")? : leanerSpecStatement
syntax (name := leanerAssertStatement)
  kwAssert leanerExpr (";")? : leanerSpecStatement
syntax (name := leanerAssumeStatement)
  kwAssume leanerExpr (";")? : leanerSpecStatement
syntax (name := leanerLoopInvariantStatement)
  kwInvariant leanerExpr (";")? : leanerSpecStatement
@[leanerExpr_parser] def leanerSpecBlockExpr := leading_parser
  kwSpec >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerSpecStatement 0))
syntax (name := leanerSingleSpecExpr)
  kwSpec leanerSpecStatement : leanerExpr
syntax (name := leanerLetStatementSyntax)
  kwLet (kwMut)? leanerBindingPattern ":" leanerType ":=" leanerExpr (";")? : leanerStatement
syntax (name := leanerInferredLetStatementSyntax)
  kwLet (kwMut)? leanerBindingPattern ":=" leanerExpr (";")? : leanerStatement
declare_syntax_cat leanerBlockEntry
syntax (name := leanerBlockStatementEntry) leanerStatement : leanerBlockEntry
syntax (name := leanerBareExpressionEntry) (priority := low)
  leanerExpr : leanerBlockEntry
syntax (name := leanerReturnBlockEntry)
  kwReturn leanerExpr (";")? : leanerBlockEntry
@[leanerExpr_parser] def leanerBlockExpr := leading_parser
  kwDo >> Lean.Parser.withPosition (Lean.Parser.manyIndent
    (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerBlockEntry 0))
syntax (name := leanerGenericCallExpr)
  kwCoreCall leanerPath leanerTypeArguments "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerDirectGenericCallExpr)
  leanerPath leanerTypeArguments "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerTypedCallExpr)
  kwCoreCall "[" leanerType "]" leanerPath "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerTypedGenericCallExpr)
  kwCoreCall "[" leanerType "]" leanerPath leanerTypeArguments
    "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerTypedCallSurfaceExpr) (priority := high)
  "(" leanerPath "(" leanerExpr,* ")" ":" leanerType ")" : leanerExpr
syntax (name := leanerTypedGenericCallSurfaceExpr) (priority := high)
  "(" leanerPath leanerTypeArguments "(" leanerExpr,* ")" ":" leanerType ")" : leanerExpr
syntax (name := leanerInvokeExpr) (priority := high)
  kwCoreInvoke "(" leanerExpr ("," leanerExpr,+)? ")" : leanerExpr
syntax (name := leanerClosureExpr) (priority := high)
  kwCoreClosure "[" leanerType "]" "(" leanerPath ("," leanerExpr)* ")" : leanerExpr
syntax (name := leanerInvokeSurfaceExpr) (priority := high)
  kwInvoke "(" leanerExpr ("," leanerExpr,+)? ")" : leanerExpr
syntax (name := leanerFunctionValueExpr) (priority := high)
  kwFunctionValue "[" leanerType "]" "(" leanerPath ("," leanerExpr)* ")" : leanerExpr
declare_syntax_cat leanerBorrowKind
syntax (name := leanerImmutableBorrowKind) kwImmutable : leanerBorrowKind
syntax (name := leanerMutableBorrowKind) kwMut : leanerBorrowKind
declare_syntax_cat leanerPlace
syntax:max (name := leanerLocalPlace) leanerIdentifier : leanerPlace
syntax:max (name := leanerParenPlace) "(" leanerPlace ")" : leanerPlace
syntax:max (name := leanerDerefPlace) "*" leanerPlace:max : leanerPlace
syntax:max (name := leanerFieldPlace) leanerPlace:max "." leanerFieldIdentifier : leanerPlace
syntax:13 (name := leanerBorrowPlaceSurfaceExpr)
  "&" (kwMut)? leanerPlace:max : leanerExpr
syntax:13 (name := leanerBorrowValueSurfaceExpr) (priority := low)
  "&" (kwMut)? leanerExpr:13 : leanerExpr
syntax:13 (name := leanerDereferenceSurfaceExpr) "*" leanerExpr:13 : leanerExpr
syntax:1 (name := leanerAssignmentExpr)
  leanerExpr:2 colGt ":=" leanerExpr:1 : leanerExpr
syntax (name := leanerPatternAssignmentExpr)
  kwCoreAssignPattern "[" leanerType "]"
    "(" leanerBindingPattern "," leanerExpr ")" : leanerExpr
syntax (name := leanerPatternAssignmentSurfaceExpr)
  kwAssignPattern "[" leanerType "]"
    "(" leanerBindingPattern "," leanerExpr ")" : leanerExpr
syntax (name := leanerPlaceBorrowExpr)
  kwCoreBorrow "(" leanerBorrowKind "," leanerPlace ")" : leanerExpr
syntax (name := leanerRawPlaceBorrowExpr)
  kwCoreBorrowPlace "(" leanerBorrowKind "," leanerExpr ")" : leanerExpr
syntax (name := leanerRawPlaceAssignExpr)
  kwCoreAssignPlace "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerDropPlaceExpr)
  kwCoreDrop "(" leanerPlace ")" : leanerExpr
syntax (name := leanerDropPlaceSurfaceExpr)
  kwDropBuiltin "(" leanerPlace ")" : leanerExpr
syntax (name := leanerMovePlaceSurfaceExpr)
  kwMove "(" leanerExpr ")" : leanerExpr
syntax (name := leanerCopyPlaceSurfaceExpr)
  kwCopyBuiltin "(" leanerExpr ")" : leanerExpr
syntax (name := leanerReadPlaceSurfaceExpr)
  kwReadBuiltin "(" leanerExpr ")" : leanerExpr
syntax (name := leanerValueBorrowExpr)
  kwCoreRefBorrow "(" leanerBorrowKind "," leanerExpr ")" : leanerExpr
syntax (name := leanerFreezeReferenceExpr)
  kwCoreRefFreeze "(" leanerExpr ")" : leanerExpr
syntax (name := leanerFreezeExplicitReferenceExpr)
  kwCoreRefFreezeExplicit "(" leanerExpr ")" : leanerExpr
syntax (name := leanerDereferenceExpr)
  kwCoreRefDereference "(" leanerExpr ")" : leanerExpr
syntax (name := leanerMutateReferenceExpr)
  kwCoreRefMutate "(" leanerExpr "," leanerExpr ")" : leanerExpr
syntax (name := leanerTypedCastExpr) (priority := high)
  kwPrimitiveCast "[" leanerType "]" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerTypedCheckedCastExpr) (priority := high)
  kwPrimitiveCheckedCast "[" leanerThrow "," leanerType "]" "(" leanerExpr ")" : leanerExpr
syntax (name := leanerPrimitiveExpr)
  leanerPath colGt "(" leanerExpr,* ")" : leanerExpr
syntax (name := leanerCheckedPrimitiveExpr) (priority := high)
  leanerPath "[" leanerThrow "]" colGt "(" leanerExpr,* ")" : leanerExpr
declare_syntax_cat leanerIfBranch
syntax (name := leanerInlineIfBranch) leanerExpr : leanerIfBranch
syntax (name := leanerIndentedIfBranch) leanerBlockEntry+ : leanerIfBranch
private def leanerIfBranchParser :=
  (Lean.Parser.node ``leanerIndentedIfBranch <|
    Lean.Parser.checkLinebreakBefore >>
      Lean.Parser.withPosition (Lean.Parser.many1Indent
        (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerBlockEntry 0))) <|>
  (Lean.Parser.node ``leanerInlineIfBranch <|
    Lean.Parser.categoryParser `leanerExpr 0)
@[leanerExpr_parser] def leanerIfExpr := leading_parser
  kwIf >> Lean.Parser.categoryParser `leanerExpr 0 >> kwThen >>
    -- Save the position immediately after each delimiter so `many1Indent`
    -- can establish the offside column for a branch beginning on the next
    -- line, while the inline branch remains available on the same line.
    Lean.Parser.withPosition leanerIfBranchParser >>
    Lean.Parser.optional (kwElse >>
      Lean.Parser.withPosition leanerIfBranchParser)
declare_syntax_cat leanerLoopLabel
syntax (name := leanerLoopLabelSyntax) "@" leanerIdentifier : leanerLoopLabel
syntax (name := leanerLoopExpr)
  kwLoop (leanerLoopLabel)? leanerExpr : leanerExpr
declare_syntax_cat leanerLoopSpecificationMember
syntax (name := leanerLoopSpecificationMemberSyntax)
  leanerSpecStatement : leanerLoopSpecificationMember
@[leanerExpr_parser] def leanerWhileExpr := leading_parser
  kwWhile >> Lean.Parser.categoryParser `leanerExpr 0 >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerBlockEntry 0)) >>
    Lean.Parser.optional (Lean.Parser.ppLine >> kwWhere >>
      Lean.Parser.withPosition (Lean.Parser.manyIndent
        (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerLoopSpecificationMember 0)))
@[leanerExpr_parser] def leanerForRangeExpr := leading_parser
  kwFor >> leanerIdentifier >> kwIn >>
    Lean.Parser.categoryParser `leanerExpr 0 >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `leanerBlockEntry 0))
@[leanerExpr_parser] def leanerBreakExpr := leading_parser
  Lean.Parser.withPosition <| kwBreak >>
    Lean.Parser.optional (Lean.Parser.categoryParser `leanerLoopLabel 0) >> Lean.Parser.optional
    (Lean.Parser.checkLineEq "a `break` value must start on the same line as `break`" >>
      Lean.Parser.categoryParser `leanerExpr 0)
@[leanerExpr_parser] def leanerContinueExpr := leading_parser
  Lean.Parser.withPosition <| kwContinue >>
    Lean.Parser.optional (Lean.Parser.categoryParser `leanerLoopLabel 0) >>
    Lean.Parser.notFollowedBy
      (Lean.Parser.checkLineEq "continue has no value" >>
        Lean.Parser.categoryParser `leanerExpr 0)
      "`continue` takes no value or function call; use a plain or labeled `continue`"
syntax:1 (name := leanerReturnExpr) kwReturn leanerExpr:1 : leanerExpr

declare_syntax_cat leanerParameter
syntax (name := leanerParameterSyntax)
  (kwMut)? leanerIdentifier ":" leanerType : leanerParameter

declare_syntax_cat leanerModifier
syntax (name := leanerPrivateModifier) kwPrivate : leanerModifier
syntax (name := leanerPublicModifier) kwPublic : leanerModifier
syntax (name := leanerPackageModifier) kwPackage : leanerModifier
syntax (name := leanerFriendModifier) kwFriend : leanerModifier
syntax (name := leanerEntryModifier) kwEntry : leanerModifier
syntax (name := leanerNativeModifier) kwNative : leanerModifier
syntax (name := leanerOpaqueModifier) kwOpaque : leanerModifier
syntax (name := leanerDeprecatedModifier) kwDeprecated : leanerModifier
syntax (name := leanerViewModifier) kwView : leanerModifier

declare_syntax_cat leanerClause
declare_syntax_cat leanerConditionProperties
syntax (name := leanerConditionPropertiesSyntax) "[" leanerIdentifier,* "]" : leanerConditionProperties
syntax (name := leanerLetPreClause)
  kwLetPre (leanerConditionProperties)? leanerIdentifier ":=" leanerExpr (";")? : leanerClause
syntax (name := leanerLetPostClause)
  kwLetPost (leanerConditionProperties)? leanerIdentifier ":=" leanerExpr (";")? : leanerClause
syntax (name := leanerRequiresClause)
  kwRequires (leanerConditionProperties)? leanerExpr (";")? : leanerClause
syntax (name := leanerEnsuresClause)
  kwEnsures (leanerConditionProperties)? leanerExpr (";")? : leanerClause
syntax (name := leanerAbortsIfClause)
  kwAbortsIf (leanerConditionProperties)? leanerExpr (kwWith leanerExpr)? (";")? : leanerClause
syntax (name := leanerInvariantClause)
  kwInvariant (leanerConditionProperties)? leanerExpr (";")? : leanerClause
syntax (name := leanerModifiesClause) kwModifies leanerExpr (";")? : leanerClause
syntax (name := leanerLooseModifiesClause) kwModifies leanerExpr "," "*" (";")? : leanerClause
syntax (name := leanerModifiesAllClause) kwModifies "*" (";")? : leanerClause
syntax (name := leanerReadsClause) kwReads leanerType (";")? : leanerClause
syntax (name := leanerReadsAllClause) kwReads "*" (";")? : leanerClause
syntax (name := leanerPragmaClause)
  kwPragma (leanerIdentifier <|> kwOpaque) ("=" leanerExpr)? (";")? : leanerClause

declare_syntax_cat leanerField
syntax (name := leanerFieldSyntax) atomic(leanerFieldIdentifier ":") leanerType : leanerField

declare_syntax_cat leanerVariant
declare_syntax_cat leanerDiscriminant
syntax (name := leanerVariantDiscriminant) "=" num : leanerDiscriminant
syntax (name := leanerVariantSyntax)
  "|" leanerIdentifier ("(" leanerField,* ")")? (leanerDiscriminant)? : leanerVariant

declare_syntax_cat leanerItem
syntax (name := leanerUseItem) kwUse leanerPath : leanerItem
syntax (name := leanerFriendItem) kwFriend leanerPath (";")? : leanerItem
syntax (name := leanerNamespacePragmaItem)
  kwPragma (leanerIdentifier <|> kwOpaque) ("=" leanerExpr)? (";")? : leanerItem
syntax (name := leanerConstantItem)
  (docComment)? kwConst leanerIdentifier ":" leanerType ":=" leanerExpr : leanerItem
declare_syntax_cat leanerAttribute
declare_syntax_cat leanerAttributeListSyntax
syntax (name := leanerAttributeSyntax)
  leanerIdentifier ("(" leanerAttribute,+ ")")? : leanerAttribute
syntax (name := leanerAttributeAssignmentSyntax)
  leanerIdentifier "=" (num <|> str <|> leanerIdentifier) : leanerAttribute
syntax (name := leanerAttributeList) "@[" leanerAttribute,+ "]" : leanerAttributeListSyntax

syntax (name := leanerStructItem)
  (docComment)? (leanerAttributeListSyntax)? kwStruct leanerIdentifier leanerGenericBinder* (kwHas leanerAbility,+)? kwWhere
    ppLine ppIndent(leanerField*) : leanerItem
syntax (name := leanerEnumItem)
  (docComment)? (leanerAttributeListSyntax)? kwEnum leanerIdentifier leanerGenericBinder*
    (kwHas leanerAbility,+)? kwWhere
    ppLine ppIndent(leanerVariant*) : leanerItem
syntax (name := leanerFunctionItem)
  (docComment)? (leanerAttributeListSyntax)? leanerModifier* kwFun leanerIdentifier leanerGenericBinder* "(" leanerParameter,* ")"
    "->" leanerType (":=" leanerExpr)? : leanerItem
syntax (name := leanerSpecFunctionItem) (priority := high)
  (docComment)? (leanerAttributeListSyntax)? (kwOpaque)? kwSpec kwFun leanerIdentifier leanerGenericBinder* "(" leanerParameter,* ")"
    ":" leanerType (":=" leanerExpr)? : leanerItem
syntax (name := leanerContractItem)
  kwSpec leanerIdentifier "{" leanerClause* "}" : leanerItem
syntax (name := leanerContractWhereItem)
  kwSpec leanerIdentifier kwWhere ppLine ppIndent(leanerClause*) : leanerItem
declare_syntax_cat leanerNamespaceInvariantMember
syntax (name := leanerNamespaceInvariantMemberSyntax)
  kwInvariant (leanerConditionProperties)? leanerExpr (";")? :
    leanerNamespaceInvariantMember
syntax (name := leanerNamespaceInvariantItem) (priority := high)
  kwSpec kwModule kwWhere ppLine ppIndent(leanerNamespaceInvariantMember*) : leanerItem
syntax (name := leanerVerifyItem)
  kwVerify leanerIdentifier ("by" Lean.Parser.Tactic.tacticSeq)? : leanerItem

syntax (name := leanerNamespaceCommand)
  "leaner" kwNamespace leanerPath kwUsing leanerProfile kwWhere
    ppLine ppIndent(leanerItem*) : command
syntax (name := leanerMoveModuleCommand)
  "leaner" kwModule leanerPath kwWhere
    ppLine ppIndent(leanerItem*) : command
syntax (name := leanerRustNamespaceCommand)
  "leaner" kwNamespace leanerPath kwWhere
    ppLine ppIndent(leanerItem*) : command

syntax (name := checkLeanerCommand) "#check_leaner" leanerPath : command

end LeanerLang

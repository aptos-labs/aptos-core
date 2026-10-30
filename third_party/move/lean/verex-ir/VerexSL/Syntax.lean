-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Lean parser declarations for VerexSL

The parser owns only source shape. Resolution, typing, VIR construction, and
validation remain in `VerexSL.Lower` and the shared checker.
-/

namespace VerexSL

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
private def kwStateDomain := Lean.Parser.nonReservedSymbol "StateDomain" true
private def kwPublish := Lean.Parser.nonReservedSymbol "publish" true
private def kwRemove := Lean.Parser.nonReservedSymbol "remove" true
private def kwUpdate := Lean.Parser.nonReservedSymbol "update" true
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
private def kwAbortsWith := Lean.Parser.nonReservedSymbol "aborts_with" true
private def kwAbortsIf := Lean.Parser.nonReservedSymbol "aborts_if" true
private def kwLetPre := Lean.Parser.nonReservedSymbol "let_pre" true
private def kwLetPost := Lean.Parser.nonReservedSymbol "let_post" true
private def kwModifies := Lean.Parser.nonReservedSymbol "modifies" true
private def kwModifiesOf := Lean.Parser.nonReservedSymbol "modifies_of" true
private def kwReads := Lean.Parser.nonReservedSymbol "reads" true
private def kwAssert := Lean.Parser.nonReservedSymbol "assert" true
private def kwAssertBang := Lean.Parser.nonReservedSymbol "assert!" true
private def kwAssume := Lean.Parser.nonReservedSymbol "assume" true
private def kwInvariant := Lean.Parser.nonReservedSymbol "invariant" true
private def kwGhost := Lean.Parser.nonReservedSymbol "ghost" true
private def kwApply := Lean.Parser.nonReservedSymbol "apply" true
private def kwSplit := Lean.Parser.nonReservedSymbol "split" true
private def kwLemma := Lean.Parser.nonReservedSymbol "lemma" true
private def kwProof := Lean.Parser.nonReservedSymbol "proof" true
private def kwLean := Lean.Parser.nonReservedSymbol "lean" true
private def kwSpec := Lean.Parser.symbol "spec"
private def kwFun := Lean.Parser.nonReservedSymbol "fun" true
private def kwNamespace := Lean.Parser.nonReservedSymbol "namespace" true
private def kwModule := Lean.Parser.nonReservedSymbol "module" true
private def kwUsing := Lean.Parser.nonReservedSymbol "using" true
private def kwWhere := Lean.Parser.nonReservedSymbol "where" true
private def kwDecreases := Lean.Parser.nonReservedSymbol "decreases" true
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
private def kwDataUpdateField := Lean.Parser.nonReservedSymbol "core.data.updateField" true
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
private def kwDowncast := Lean.Parser.nonReservedSymbol "downcast" true
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

/-- Identifiers are contextual to VerexSL: host-Lean keywords such as
`end` and `at`, and modifiers such as `view`, remain ordinary local names.
Clause delimiters must use `«...»` when intended as identifiers, so an
expression cannot consume the next contract clause. -/
private def verexIdentifier : Lean.Parser.Parser :=
  ["_", "move", "rust", "Unit", "Never", "Bool", "Char", "string", "Bytes",
    "Address", "Signer", "UInt", "SInt", "UPtr", "IPtr", "Nat", "Int",
    "u8", "u16", "u32", "u64", "u128", "u256", "i8", "i16", "i32", "i64",
    "i128", "i256", "usize", "isize", "Range", "Vector", "Fn", "const",
    "type", "lifetime", "evidence", "mut", "private", "public", "package",
    "friend", "entry", "native", "opaque", "deprecated", "view", "pragma",
    "let_pre", "let_post", "modifies", "modifies_of", "reads", "requires", "ensures",
    "aborts_if", "aborts_with", "assert", "assume", "invariant", "spec", "fun", "module",
    "namespace", "using", "where", "struct", "enum", "has", "Copy", "Drop",
    "Store", "Key", "true", "false", "abort", "panic", "do", "let", "loop",
    "while", "for", "break", "continue", "forall", "exists", "in", "immutable", "if",
    "then", "else", "return", "old", "copy", "drop",
    "discriminant", "invoke", "function", "as", "match", "with"].foldr
      (fun keyword parser =>
        (if ["let_pre", "let_post", "modifies", "modifies_of", "reads", "requires", "ensures",
            "aborts_if", "aborts_with", "invariant"].contains keyword then
          Lean.Parser.notFollowedBy (Lean.Parser.nonReservedSymbol keyword true) keyword
        else Lean.Parser.notSymbol keyword) >> parser)
      Lean.Parser.rawIdent

attribute [run_builtin_parser_attribute_hooks] verexIdentifier

builtin_initialize register_parser_alias verexIdentifier

declare_syntax_cat verexProfile
syntax (name := verexMoveProfile) kwMove : verexProfile
syntax (name := verexRustProfile) kwRust : verexProfile

declare_syntax_cat verexPath
@[verexPath_parser] def verexPathSyntax := leading_parser
  (Lean.Parser.atomic (Lean.Parser.numLit >> "::" >> verexIdentifier) <|>
    verexIdentifier) >>
    Lean.Parser.many (Lean.Parser.atomic ("::" >> verexIdentifier))

/-- A path with at least one `::`: a qualified name. -/
declare_syntax_cat verexQualifiedName
@[verexQualifiedName_parser] def verexQualifiedNameSyntax := leading_parser
  (Lean.Parser.atomic (Lean.Parser.numLit >> "::" >> verexIdentifier) <|>
    Lean.Parser.atomic (verexIdentifier >> "::" >> verexIdentifier)) >>
    Lean.Parser.many (Lean.Parser.atomic ("::" >> verexIdentifier))

declare_syntax_cat verexAbility
syntax (name := verexCopyAbility) kwCopy : verexAbility
syntax (name := verexDropAbility) kwDrop : verexAbility
syntax (name := verexStoreAbility) kwStore : verexAbility
syntax (name := verexKeyAbility) kwKey : verexAbility

declare_syntax_cat verexType
declare_syntax_cat verexTypeArgument
declare_syntax_cat verexLifetime
syntax (name := verexInferenceLifetime) "_" : verexLifetime
syntax (name := verexNamedLifetime) verexIdentifier : verexLifetime
syntax (name := verexUnitType) kwUnit : verexType
syntax (name := verexNeverType) kwNever : verexType
syntax (name := verexBoolType) kwBool : verexType
syntax (name := verexCharType) kwChar : verexType
syntax (name := verexStringType) kwString : verexType
syntax (name := verexBytesType) kwBytes : verexType
syntax (name := verexAddressType) kwAddress : verexType
syntax (name := verexSignerType) kwSigner : verexType
syntax (name := verexUIntType) kwUInt "<" num ">" : verexType
syntax (name := verexSIntType) kwSInt "<" num ">" : verexType
syntax (name := verexUPtrType) kwUPtr : verexType
syntax (name := verexIPtrType) kwIPtr : verexType
syntax (name := verexU8Type) kwU8 : verexType
syntax (name := verexU16Type) kwU16 : verexType
syntax (name := verexU32Type) kwU32 : verexType
syntax (name := verexU64Type) kwU64 : verexType
syntax (name := verexU128Type) kwU128 : verexType
syntax (name := verexU256Type) kwU256 : verexType
syntax (name := verexI8Type) kwI8 : verexType
syntax (name := verexI16Type) kwI16 : verexType
syntax (name := verexI32Type) kwI32 : verexType
syntax (name := verexI64Type) kwI64 : verexType
syntax (name := verexI128Type) kwI128 : verexType
syntax (name := verexI256Type) kwI256 : verexType
syntax (name := verexUSizeType) kwUSize : verexType
syntax (name := verexISizeType) kwISize : verexType
syntax (name := verexNatType) kwNat : verexType
syntax (name := verexIntType) kwInt : verexType
syntax (name := verexRangeType) kwRange : verexType
syntax (name := verexStateDomainType) kwStateDomain : verexType
syntax (name := verexVectorType) (priority := high)
  kwVector "<" verexType ">" : verexType
syntax (name := verexFixedVectorType)
  (priority := high)
  kwVector "<" verexType "," kwConst num ">" : verexType
-- An ability list ends before a comma not followed by an ability, so a function
-- type can precede the next parameter or type argument.
syntax (name := verexFunctionType)
  kwFn "(" verexType,* ")" "->" verexType
    (kwHas verexAbility (atomic("," verexAbility))*)? : verexType
syntax (name := verexReferenceType)
  "&" ("[" verexLifetime "]")? (kwMut)? verexType : verexType
syntax (name := verexTupleType) "(" verexType,+ ")" : verexType
syntax (name := verexTypeArgumentSyntax) (kwType)? verexType : verexTypeArgument
syntax (name := verexIntegerConstTypeArgument)
  kwConst ("-")? num : verexTypeArgument
syntax (name := verexTrueConstTypeArgument) kwConst kwTrue : verexTypeArgument
syntax (name := verexFalseConstTypeArgument) kwConst kwFalse : verexTypeArgument
syntax (name := verexLifetimeTypeArgument)
  kwLifetime verexLifetime : verexTypeArgument
syntax (name := verexAppliedType)
  verexPath atomic("::" "<") verexTypeArgument,* ">" : verexType
syntax (name := verexStandardAppliedType)
  verexPath atomic("<") verexTypeArgument,* ">" : verexType
syntax (name := verexNamedType) (priority := low) verexPath : verexType

declare_syntax_cat verexGenericBinder
/-- A phantom type parameter; any other type parameter is written bare, as
`{T}` or `{T has Copy}`. -/
syntax (name := verexTypeBinder)
  "{" verexIdentifier ":" kwPhantom kwType (kwHas verexAbility,+)? "}" :
    verexGenericBinder
syntax (name := verexInferredTypeBinder)
  "{" verexIdentifier (kwHas verexAbility,+)? "}" : verexGenericBinder
syntax (name := verexConstBinder)
  "{" verexIdentifier ":" kwConst "}" : verexGenericBinder
syntax (name := verexTypedConstBinder)
  "{" verexIdentifier ":" kwConst verexType "}" : verexGenericBinder
syntax (name := verexLifetimeBinder)
  "{" verexIdentifier ":" kwLifetime "}" : verexGenericBinder
syntax (name := verexEvidenceBinder)
  "{" verexIdentifier ":" kwEvidence "}" : verexGenericBinder

declare_syntax_cat verexThrow
syntax (name := verexAbortThrow) kwAbort : verexThrow
syntax (name := verexPanicThrow) kwPanic : verexThrow
syntax (name := verexMoveVectorErrorThrow) "moveVectorError" : verexThrow
syntax (name := verexMoveArithmeticErrorThrow) "moveArithmeticError" : verexThrow

declare_syntax_cat verexExpr
declare_syntax_cat verexFieldIdentifier
syntax (name := verexNamedFieldIdentifier) verexIdentifier : verexFieldIdentifier
syntax (name := verexNumericFieldIdentifier) num : verexFieldIdentifier
syntax (name := verexUnitExpr) "(" ")" : verexExpr
syntax (name := verexTrueExpr) kwTrue : verexExpr
syntax (name := verexFalseExpr) kwFalse : verexExpr
syntax (name := verexCharExpr) char : verexExpr
syntax (name := verexIntegerExpr) num : verexExpr
syntax (name := verexNegativeIntegerExpr) "-" num : verexExpr
syntax (name := verexTypedIntegerExpr)
  num (kwU8 <|> kwU16 <|> kwU32 <|> kwU64 <|> kwU128 <|> kwU256 <|>
    kwI8 <|> kwI16 <|> kwI32 <|> kwI64 <|> kwI128 <|> kwI256 <|>
    kwUSize <|> kwISize) : verexExpr
syntax (name := verexNegativeTypedIntegerExpr)
  "-" num (kwU8 <|> kwU16 <|> kwU32 <|> kwU64 <|> kwU128 <|> kwU256 <|>
    kwI8 <|> kwI16 <|> kwI32 <|> kwI64 <|> kwI128 <|> kwI256 <|>
    kwUSize <|> kwISize) : verexExpr
syntax (name := verexAddressExpr) kwAddressLiteral "(" str ")" : verexExpr
syntax (name := verexMoveAddressExpr) "@" num : verexExpr
/-- A Move named address as a value: `@std`, the address `std` stands for. -/
syntax (name := verexMoveAliasAddressExpr) "@" ident : verexExpr
syntax (name := verexStringExpr) str : verexExpr
syntax (name := verexByteStringExpr) atomic(kwByteStringPrefix str) : verexExpr
syntax (name := verexBytesExpr) "b[" num,* "]" : verexExpr
syntax (name := verexLocalExpr) (priority := low) verexIdentifier : verexExpr
syntax (name := verexParenExpr) "(" verexExpr ")" : verexExpr
syntax (name := verexSingletonTupleExpr) "(" verexExpr "," ")" : verexExpr
syntax (name := verexTupleExpr) "(" verexExpr "," verexExpr,+ ")" : verexExpr
syntax (name := verexVectorExpr) "#[" verexExpr,* "]" : verexExpr
syntax (name := verexRepeatVectorExpr) "#[" verexExpr ";" num "]" : verexExpr
syntax (name := verexTypedVectorExpr)
  kwVectorLiteral "<" verexType ">" "[" verexExpr,* "]" : verexExpr
syntax (name := verexRuntimeAssertExpr) (priority := high)
  kwAssert "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexRuntimeAssertMacroExpr) (priority := high)
  kwAssertBang "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexThrowSurfaceExpr) (priority := high)
  (kwAbort <|> kwPanic <|> "moveVectorError" <|> "moveArithmeticError") "(" verexExpr,* ")" : verexExpr
declare_syntax_cat verexBehaviorCall
syntax (name := verexBehaviorCallSyntax)
  (kwRequiresOf <|> kwAbortsOf <|> kwEnsuresOf <|> kwResultOf <|>
    kwUnchangedOf <|> kwFoldsOf)
  "<" verexExpr:14 ">" "(" verexExpr,* ")" : verexBehaviorCall
syntax (name := verexWriteBehaviorCallSyntax)
  kwWriteOf "[" num "]" "<" verexExpr:14 ">"
    "(" verexExpr,* ")" : verexBehaviorCall
syntax (name := verexBehaviorExpr) (priority := high)
  verexBehaviorCall : verexExpr
-- A two-state reading at state labels: `S |~ e`, `..S |~ e`, `S.. |~ e`,
-- `S..T |~ e`; binds below implication, so `S.. |~ a == b` reads the whole
-- equation at `S`.
declare_syntax_cat verexStateRange
syntax (name := verexStateRangeBothSyntax)
  verexIdentifier ".." verexIdentifier : verexStateRange
syntax (name := verexStateRangePreSyntax) verexIdentifier ".." : verexStateRange
syntax (name := verexStateRangePostSyntax) ".." verexIdentifier : verexStateRange
syntax (name := verexStateRangeAtSyntax) verexIdentifier : verexStateRange
syntax:1 (name := verexLabeledExpr) (priority := high)
  atomic(verexStateRange "|~") verexExpr:1 : verexExpr
-- State-change predicates between the states of the enclosing labels.
syntax (name := verexSpecificationPublishExpr) (priority := high)
  kwPublish "<" verexType ">" "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexSpecificationRemoveExpr) (priority := high)
  kwRemove "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationUpdateExpr) (priority := high)
  kwUpdate "<" verexType ">" "(" verexExpr "," verexExpr ")" : verexExpr
declare_syntax_cat verexStorageHead
syntax (name := verexNamedStorageHead) verexPath : verexStorageHead
syntax (name := verexAppliedStorageHead)
  verexPath "<" verexType,* ">" : verexStorageHead
declare_syntax_cat verexTypeArguments
syntax (name := verexTypeArgumentsSyntax)
  atomic("::" "<") verexType,* ">" : verexTypeArguments
syntax:14 (name := verexMethodCallExpr) (priority := high)
  verexExpr:14 colGt "." verexIdentifier (verexTypeArguments)?
    colGt "(" verexExpr,* ")" : verexExpr
syntax (name := verexTypedMethodCallExpr) (priority := high)
  "(" verexExpr:14 colGt "." verexIdentifier (verexTypeArguments)?
    "(" verexExpr,* ")" ":" verexType ")" : verexExpr
syntax:14 (name := verexFieldExpr)
  verexExpr:14 colGt "." verexFieldIdentifier : verexExpr
syntax:14 (name := verexStorageIndexExpr) (priority := low)
  atomic(verexStorageHead colGt "[") verexExpr "]" : verexExpr
syntax:14 (name := verexIndexExpr)
  verexExpr:14 colGt "[" verexExpr "]" ("!")? : verexExpr
-- These static parser declarations mirror `VerexSL.Operators.table`.
-- The table is authoritative for semantic lookup and pretty-printing.
syntax:13 (name := verexLogicalNotExpr) "!" verexExpr:13 : verexExpr
syntax:13 (name := verexBitwiseNotExpr) "~" verexExpr:13 : verexExpr
syntax:13 (name := verexNegateExpr) "-" verexExpr:13 : verexExpr
syntax:12 (name := verexMultiplyExpr) verexExpr:12 colGt "*" verexExpr:13 : verexExpr
syntax:12 (name := verexDivideExpr) verexExpr:12 colGt "/" verexExpr:13 : verexExpr
syntax:12 (name := verexModuloExpr) verexExpr:12 colGt "%" verexExpr:13 : verexExpr
syntax:12 (name := verexCastSurfaceExpr) verexExpr:13 colGt kwAs verexType : verexExpr
syntax:11 (name := verexAddExpr) verexExpr:11 colGt "+" verexExpr:12 : verexExpr
syntax:11 (name := verexSubtractExpr) verexExpr:11 colGt "-" verexExpr:12 : verexExpr
syntax:10 (name := verexShiftLeftExpr) verexExpr:10 colGt "<<" verexExpr:11 : verexExpr
syntax:10 (name := verexShiftRightExpr) verexExpr:10 colGt ">>" verexExpr:11 : verexExpr
syntax:9 (name := verexBitwiseAndExpr) verexExpr:9 colGt "&" verexExpr:10 : verexExpr
syntax:8 (name := verexBitwiseXorExpr) verexExpr:8 colGt "^" verexExpr:9 : verexExpr
syntax:7 (name := verexBitwiseOrExpr) verexExpr:7 colGt "|" verexExpr:8 : verexExpr
syntax:6 (name := verexRangeExpr) verexExpr:6 colGt ".." verexExpr:7 : verexExpr
syntax:5 (name := verexEqualExpr) verexExpr:5 colGt "==" verexExpr:6 : verexExpr
syntax:5 (name := verexNotEqualExpr) verexExpr:5 colGt "!=" verexExpr:6 : verexExpr
syntax:5 (name := verexLessExpr) verexExpr:5 colGt "<" verexExpr:6 : verexExpr
syntax:5 (name := verexGreaterExpr) verexExpr:5 colGt ">" verexExpr:6 : verexExpr
syntax:5 (name := verexLessEqualExpr) verexExpr:5 colGt "<=" verexExpr:6 : verexExpr
syntax:5 (name := verexGreaterEqualExpr) verexExpr:5 colGt ">=" verexExpr:6 : verexExpr
syntax:5 (name := verexMembershipExpr) verexExpr:5 colGt "∈" verexExpr:6 : verexExpr
syntax:5 (name := verexVariantTestSurfaceExpr)
  verexExpr:6 colGt kwIs verexIdentifier ("|" verexIdentifier)* : verexExpr
syntax:4 (name := verexLogicalAndExpr) verexExpr:4 colGt "&&" verexExpr:5 : verexExpr
syntax:3 (name := verexLogicalOrExpr) verexExpr:3 colGt "||" verexExpr:4 : verexExpr
syntax:2 (name := verexImpliesExpr) verexExpr:2 colGt "==>" verexExpr:3 : verexExpr
syntax:2 (name := verexEquivalentExpr) verexExpr:2 colGt "<==>" verexExpr:3 : verexExpr
syntax (name := verexSpecificationIntToBitVectorExpr)
  (kwSpecIntToBitVector <|> kwIntToBitVector) "[" verexType "]"
    "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationIntToBitVectorInferExpr) (priority := high)
  (kwSpecIntToBitVector <|> kwIntToBitVector) "(" verexExpr ")" : verexExpr
syntax (name := verexGlobalContainsExpr) (priority := high)
  kwGlobalContains "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexGlobalBorrowExpr) (priority := high)
  kwGlobalBorrow "<" verexType ">" "(" (kwImmutable <|> kwMut) "," verexExpr ")" : verexExpr
syntax (name := verexGlobalTakeExpr) (priority := high)
  kwGlobalTake "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexGlobalPublishExpr) (priority := high)
  kwGlobalPublish "<" verexType ">" "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexGlobalExistsSurfaceExpr) (priority := high)
  kwGlobalExists "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexBorrowGlobalSurfaceExpr) (priority := high)
  kwBorrowGlobal "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexBorrowGlobalMutSurfaceExpr) (priority := high)
  kwBorrowGlobalMut "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexMoveFromSurfaceExpr) (priority := high)
  kwMoveFrom "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexMoveToSurfaceExpr) (priority := high)
  kwMoveTo "<" verexType ">" "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexConstructExpr)
  kwConstruct verexPath "(" verexExpr,* ")" : verexExpr
declare_syntax_cat verexConstructorVariant
syntax (name := verexConstructorVariantSyntax)
  atomic("::" verexIdentifier) : verexConstructorVariant
syntax (name := verexAppliedConstructExpr)
  kwConstruct verexPath atomic("::" "<") verexType,* ">"
    (verexConstructorVariant)? "(" verexExpr,* ")" : verexExpr
syntax (name := verexStandardAppliedConstructExpr) (priority := high)
  kwConstruct verexType (verexConstructorVariant)? "(" verexExpr,* ")" : verexExpr
declare_syntax_cat verexConstructorField
syntax (name := verexConstructorFieldSyntax)
  verexFieldIdentifier ":=" verexExpr : verexConstructorField
syntax (name := verexConstructorFieldShorthandSyntax)
  verexIdentifier : verexConstructorField
-- Atomic up to its brace, so that a local named `new` is an identifier
-- wherever no construction follows it.
syntax (name := verexNamedConstructExpr) (priority := low)
  atomic(kwNew verexType (verexConstructorVariant)? "{") verexConstructorField,* "}" :
    verexExpr
declare_syntax_cat verexFieldName
syntax (name := verexFieldNameSyntax) verexFieldIdentifier : verexFieldName
syntax (name := verexSelectExpr)
  kwDataSelect "[" verexType "," verexFieldName "]" "(" verexExpr ")" : verexExpr
/-- `Variant.field`: a named field arrives in the variant's dotted identifier,
a numeric one after it. -/
declare_syntax_cat verexVariantFieldName
syntax (name := verexVariantFieldNameSyntax)
  verexIdentifier (noWs "." noWs num)? : verexVariantFieldName
syntax (name := verexSelectVariantsExpr)
  kwDataSelectVariants "[" verexType "," verexVariantFieldName,+ "]"
    "(" verexExpr ")" : verexExpr
declare_syntax_cat verexVariantName
syntax (name := verexVariantNameSyntax) verexIdentifier : verexVariantName
syntax (name := verexTestVariantsExpr)
  kwDataTestVariants "[" verexType "," verexVariantName,+ "]"
    "(" verexExpr ")" : verexExpr
syntax (name := verexUpdateFieldExpr)
  kwDataUpdateField "(" verexExpr "," verexFieldIdentifier "," verexExpr ")" : verexExpr
syntax (name := verexDiscriminantExpr)
  kwDataDiscriminant "[" verexType "," verexType "]"
    "(" verexExpr ")" : verexExpr
syntax (name := verexDiscriminantSurfaceExpr)
  kwDiscriminant "[" verexType "," verexType "]"
    "(" verexExpr ")" : verexExpr
declare_syntax_cat verexStatement
declare_syntax_cat verexBlockResult
syntax (name := verexStatementSyntax) verexExpr ";" : verexStatement
declare_syntax_cat verexBindingPattern
declare_syntax_cat verexBindingPatternField
syntax (name := verexVariableBindingPattern) (priority := low)
  (kwMut)? verexIdentifier : verexBindingPattern
syntax (name := verexWildcardBindingPattern) (priority := high) "_" : verexBindingPattern
syntax (name := verexTrueBindingPattern) (priority := high) kwTrue : verexBindingPattern
syntax (name := verexFalseBindingPattern) (priority := high) kwFalse : verexBindingPattern
syntax (name := verexCharBindingPattern) char : verexBindingPattern
syntax (name := verexIntegerBindingPattern) num : verexBindingPattern
syntax (name := verexNegativeIntegerBindingPattern) "-" num : verexBindingPattern
syntax (name := verexTupleBindingPattern)
  "(" verexBindingPattern "," verexBindingPattern,+ ")" : verexBindingPattern
syntax (name := verexSingletonTupleBindingPattern)
  "(" verexBindingPattern "," ")" : verexBindingPattern
syntax (name := verexBindingPatternFieldSyntax)
  verexFieldIdentifier ":=" verexBindingPattern : verexBindingPatternField
declare_syntax_cat verexBindingPatternVariant
syntax (name := verexBindingPatternVariantSyntax)
  atomic("::" verexIdentifier) : verexBindingPatternVariant
syntax (name := verexConstructorBindingPattern)
  verexType (verexBindingPatternVariant)?
    "{" verexBindingPatternField,* "}" : verexBindingPattern
declare_syntax_cat verexMatchArm
syntax (name := verexMatchArmSyntax)
  -- `many1Indent` saves the arm column. The bitwise-or parser's `colGt`
  -- therefore admits `left | right` inside an arm but rejects the next
  -- aligned arm delimiter, so an arm body can use the full expression grammar.
  "|" verexBindingPattern "=>" verexExpr : verexMatchArm
syntax (name := verexGuardedMatchArmSyntax)
  "|" verexBindingPattern kwIf verexExpr "=>" verexExpr : verexMatchArm
@[verexExpr_parser] def verexMatchExpr := leading_parser
  kwMatch >> Lean.Parser.categoryParser `verexExpr 0 >> kwWith >>
    Lean.Parser.withPosition (Lean.Parser.many1Indent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexMatchArm 0))
declare_syntax_cat verexQuantifierBinder
syntax (name := verexQuantifierBinderSyntax)
  verexBindingPattern kwIn verexExpr : verexQuantifierBinder
syntax (name := verexQuantifierTypeBinderSyntax)
  verexBindingPattern ":" verexType : verexQuantifierBinder
syntax (name := verexQuantifierExpr)
  (kwForall <|> kwExists <|> "∀" <|> "∃")
    "(" verexQuantifierBinder (";" verexQuantifierBinder)* ")" ","
      verexExpr : verexExpr
syntax (name := verexSpecificationAnchorMarkerExpr)
  (kwSpecSaveStateAnchor <|> kwSpecFoldsCaptureAnchor)
    "[" num "]" "(" ")" : verexExpr
syntax (name := verexSpecificationAnchorMarkerBuiltinExpr) (priority := high)
  (kwSaveStateAnchor <|> kwFoldsCaptureAnchor) "!" "(" num ")" : verexExpr
syntax (name := verexSpecificationWithStateExpr)
  kwSpecWithStateAnchor "[" num "]" "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationWithStateBuiltinExpr) (priority := high)
  kwWithStateAnchor "!" "(" num "," verexExpr ")" : verexExpr
syntax (name := verexSpecificationOldExpr) (priority := high)
  (kwSpecOld <|> kwOld) "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationResultExpr) (priority := high)
  kwSpecResult "[" num "]" : verexExpr
syntax (name := verexSpecificationInlineCallSummaryExpr) (priority := high)
  kwSpecInlineCallSummary "(" verexExpr "," verexExpr ")" : verexExpr
declare_syntax_cat verexSpecificationTypeArgument
syntax (name := verexSpecificationTypeArgumentSyntax)
  atomic("::" "<") verexType ">" : verexSpecificationTypeArgument
syntax (name := verexSpecificationVectorExpr) (priority := high)
  (kwSpecEmptyVector <|> kwSpecSingletonVector <|> kwSpecUpdateVector <|>
    kwSpecConcatVector <|> kwSpecIndexOfVector <|> kwSpecContainsVector <|>
    kwSpecLengthVector <|> kwSpecIndexVector <|> kwSpecSliceVector <|>
    kwSpecInVectorRange <|> kwSpecVectorRange)
  (verexSpecificationTypeArgument)? "(" verexExpr,* ")" : verexExpr
syntax (name := verexSpecificationEmptyVectorBuiltinExpr) (priority := high)
  kwSpecVec atomic("::" "<") verexType ">" "(" ")" : verexExpr
syntax (name := verexSpecificationGlobalExpr) (priority := high)
  kwSpecGlobal verexSpecificationTypeArgument "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationGlobalSurfaceExpr) (priority := high)
  kwMoveSpecGlobal "<" verexType ">" "(" verexExpr ")" : verexExpr
syntax (name := verexSpecificationInRangeExpr) (priority := high)
  (kwSpecInRange <|> kwInRange) "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexSpecificationBitVectorToIntExpr) (priority := high)
  (kwSpecBitVectorToInt <|> kwBitVectorToInt) "(" verexExpr ")" : verexExpr
declare_syntax_cat verexSpecStatement
syntax (name := verexSpecLetStatement)
  kwLet verexIdentifier ":=" verexExpr (";")? : verexSpecStatement
syntax (name := verexAssertStatement)
  kwAssert verexExpr (";")? : verexSpecStatement
syntax (name := verexAssumeStatement)
  kwAssume verexExpr (";")? : verexSpecStatement
syntax (name := verexLoopInvariantStatement)
  kwInvariant verexExpr (";")? : verexSpecStatement
/-- Applies lemmas: a lemma instance under implications and universal
quantifiers. -/
syntax (name := verexApplyStatement)
  kwApply verexExpr (";")? : verexSpecStatement
/-- Splits cases on a Boolean or an enum's variant. -/
syntax (name := verexSplitStatement)
  kwSplit verexExpr (";")? : verexSpecStatement
/-- Updates a ghost field of a local or of a mutable reference's referent: an
assignment to the field. -/
syntax (name := verexUpdateStatement)
  kwUpdate verexExpr (";")? : verexSpecStatement
@[verexExpr_parser] def verexSpecBlockExpr := leading_parser
  kwSpec >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexSpecStatement 0))
syntax (name := verexSingleSpecExpr)
  kwSpec verexSpecStatement : verexExpr
/-- A step of the function's proof: verified with the function, and not run
where a caller inlines it. -/
@[verexExpr_parser] def verexProofBlockExpr := leading_parser
  kwProof >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexSpecStatement 0))
syntax (name := verexSingleProofExpr)
  kwProof verexSpecStatement : verexExpr
syntax (name := verexLetStatementSyntax)
  kwLet (kwMut)? verexBindingPattern ":" verexType ":=" verexExpr (";")? : verexStatement
syntax (name := verexInferredLetStatementSyntax)
  kwLet (kwMut)? verexBindingPattern ":=" verexExpr (";")? : verexStatement
/-- A declaration without initializer: the local is assigned before it is read,
on every path, as Move's `let x: T;` is. -/
syntax (name := verexDeclareStatementSyntax)
  kwLet (kwMut)? verexBindingPattern ":" verexType (";")? : verexStatement
declare_syntax_cat verexBlockEntry
syntax (name := verexBlockStatementEntry) verexStatement : verexBlockEntry
syntax (name := verexBareExpressionEntry) (priority := low)
  verexExpr : verexBlockEntry
syntax (name := verexReturnBlockEntry)
  kwReturn verexExpr (";")? : verexBlockEntry
@[verexExpr_parser] def verexBlockExpr := leading_parser
  kwDo >> Lean.Parser.withPosition (Lean.Parser.manyIndent
    (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexBlockEntry 0))
syntax (name := verexGenericCallExpr)
  kwCoreCall verexPath verexTypeArguments "(" verexExpr,* ")" : verexExpr
syntax (name := verexDirectGenericCallExpr)
  verexPath verexTypeArguments "(" verexExpr,* ")" : verexExpr
syntax (name := verexTypedCallExpr)
  kwCoreCall "[" verexType "]" verexPath "(" verexExpr,* ")" : verexExpr
syntax (name := verexTypedGenericCallExpr)
  kwCoreCall "[" verexType "]" verexPath verexTypeArguments
    "(" verexExpr,* ")" : verexExpr
syntax (name := verexTypedCallSurfaceExpr) (priority := high)
  "(" verexPath "(" verexExpr,* ")" ":" verexType ")" : verexExpr
syntax (name := verexTypedGenericCallSurfaceExpr) (priority := high)
  "(" verexPath verexTypeArguments "(" verexExpr,* ")" ":" verexType ")" : verexExpr
syntax (name := verexInvokeExpr) (priority := high)
  kwCoreInvoke "(" verexExpr ("," verexExpr,+)? ")" : verexExpr
/-- A parameter slot of a closure's target: a captured value, or `_` for a
parameter supplied at invocation. -/
declare_syntax_cat verexClosureSlot
syntax (name := verexCapturedSlot) verexExpr : verexClosureSlot
syntax (name := verexSuppliedSlot) "_" : verexClosureSlot
syntax (name := verexClosureExpr) (priority := high)
  kwCoreClosure "[" verexType "]" "(" verexPath (verexTypeArguments)?
    ("," verexClosureSlot)* ")" : verexExpr
syntax (name := verexInvokeSurfaceExpr) (priority := high)
  kwInvoke "(" verexExpr ("," verexExpr,+)? ")" : verexExpr
syntax (name := verexFunctionValueExpr) (priority := high)
  kwFunctionValue "[" verexType "]" "(" verexPath (verexTypeArguments)?
    ("," verexClosureSlot)* ")" : verexExpr
declare_syntax_cat verexBorrowKind
syntax (name := verexImmutableBorrowKind) kwImmutable : verexBorrowKind
syntax (name := verexMutableBorrowKind) kwMut : verexBorrowKind
declare_syntax_cat verexPlace
syntax:max (name := verexLocalPlace) verexIdentifier : verexPlace
syntax:max (name := verexParenPlace) "(" verexPlace ")" : verexPlace
syntax:max (name := verexDerefPlace) "*" verexPlace:max : verexPlace
syntax:max (name := verexFieldPlace) verexPlace:max "." verexFieldIdentifier : verexPlace
/-- The value of a place as the variant `Enum::Variant` names: its fields
are that variant's. -/
syntax (name := verexDowncastPlace) kwDowncast verexPlace kwAs verexPath : verexPlace
syntax:13 (name := verexBorrowPlaceSurfaceExpr)
  "&" (kwMut)? verexPlace:max : verexExpr
syntax:13 (name := verexBorrowValueSurfaceExpr) (priority := low)
  "&" (kwMut)? verexExpr:13 : verexExpr
syntax:13 (name := verexDereferenceSurfaceExpr) "*" colGt verexExpr:13 : verexExpr
syntax:1 (name := verexAssignmentExpr)
  verexExpr:2 colGt ":=" verexExpr:1 : verexExpr
syntax (name := verexPatternAssignmentExpr)
  kwCoreAssignPattern "[" verexType "]"
    "(" verexBindingPattern "," verexExpr ")" : verexExpr
syntax (name := verexPatternAssignmentSurfaceExpr)
  kwAssignPattern "[" verexType "]"
    "(" verexBindingPattern "," verexExpr ")" : verexExpr
syntax (name := verexPlaceBorrowExpr)
  kwCoreBorrow "(" verexBorrowKind "," verexPlace ")" : verexExpr
syntax (name := verexRawPlaceBorrowExpr)
  kwCoreBorrowPlace "(" verexBorrowKind "," verexExpr ")" : verexExpr
syntax (name := verexRawPlaceAssignExpr)
  kwCoreAssignPlace "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexDropPlaceExpr)
  kwCoreDrop "(" verexPlace ")" : verexExpr
syntax (name := verexDropPlaceSurfaceExpr)
  kwDropBuiltin "(" verexPlace ")" : verexExpr
syntax (name := verexMovePlaceSurfaceExpr)
  kwMove "(" verexExpr ")" : verexExpr
syntax (name := verexCopyPlaceSurfaceExpr)
  kwCopyBuiltin "(" verexExpr ")" : verexExpr
syntax (name := verexReadPlaceSurfaceExpr)
  kwReadBuiltin "(" verexExpr ")" : verexExpr
syntax (name := verexValueBorrowExpr)
  kwCoreRefBorrow "(" verexBorrowKind "," verexExpr ")" : verexExpr
syntax (name := verexFreezeReferenceExpr)
  kwCoreRefFreeze "(" verexExpr ")" : verexExpr
syntax (name := verexFreezeExplicitReferenceExpr)
  kwCoreRefFreezeExplicit "(" verexExpr ")" : verexExpr
syntax (name := verexDereferenceExpr)
  kwCoreRefDereference "(" verexExpr ")" : verexExpr
syntax (name := verexMutateReferenceExpr)
  kwCoreRefMutate "(" verexExpr "," verexExpr ")" : verexExpr
syntax (name := verexTypedCastExpr) (priority := high)
  kwPrimitiveCast "[" verexType "]" "(" verexExpr ")" : verexExpr
syntax (name := verexTypedCheckedCastExpr) (priority := high)
  kwPrimitiveCheckedCast "[" verexThrow "," verexType "]" "(" verexExpr ")" : verexExpr
syntax (name := verexPrimitiveExpr)
  verexPath colGt "(" verexExpr,* ")" : verexExpr
syntax (name := verexCheckedPrimitiveExpr) (priority := high)
  verexPath "[" verexThrow "]" colGt "(" verexExpr,* ")" : verexExpr
declare_syntax_cat verexIfBranch
syntax (name := verexInlineIfBranch) verexExpr : verexIfBranch
syntax (name := verexIndentedIfBranch) verexBlockEntry+ : verexIfBranch
private def verexIfBranchParser :=
  (Lean.Parser.node ``verexIndentedIfBranch <|
    Lean.Parser.checkLinebreakBefore >>
      Lean.Parser.withPosition (Lean.Parser.many1Indent
        (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexBlockEntry 0))) <|>
  (Lean.Parser.node ``verexInlineIfBranch <|
    Lean.Parser.categoryParser `verexExpr 0)
@[verexExpr_parser] def verexIfExpr := leading_parser
  kwIf >> Lean.Parser.categoryParser `verexExpr 0 >> kwThen >>
    -- Save the position immediately after each delimiter so `many1Indent`
    -- can establish the offside column for a branch beginning on the next
    -- line, while the inline branch remains available on the same line.
    Lean.Parser.withPosition verexIfBranchParser >>
    Lean.Parser.optional (kwElse >>
      Lean.Parser.withPosition verexIfBranchParser)
declare_syntax_cat verexLoopLabel
syntax (name := verexLoopLabelSyntax) "@" verexIdentifier : verexLoopLabel
syntax (name := verexLoopExpr)
  kwLoop (verexLoopLabel)? verexExpr : verexExpr
declare_syntax_cat verexLoopSpecificationMember
syntax (name := verexLoopSpecificationMemberSyntax)
  verexSpecStatement : verexLoopSpecificationMember
@[verexExpr_parser] def verexWhileExpr := leading_parser
  kwWhile >> Lean.Parser.categoryParser `verexExpr 0 >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexBlockEntry 0)) >>
    Lean.Parser.optional (Lean.Parser.ppLine >> kwWhere >>
      Lean.Parser.withPosition (Lean.Parser.manyIndent
        (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexLoopSpecificationMember 0)))
@[verexExpr_parser] def verexForRangeExpr := leading_parser
  kwFor >> verexIdentifier >> kwIn >>
    Lean.Parser.categoryParser `verexExpr 0 >> kwDo >>
    Lean.Parser.withPosition (Lean.Parser.manyIndent
      (Lean.Parser.ppLine >> Lean.Parser.categoryParser `verexBlockEntry 0))
@[verexExpr_parser] def verexBreakExpr := leading_parser
  Lean.Parser.withPosition <| kwBreak >>
    Lean.Parser.optional (Lean.Parser.categoryParser `verexLoopLabel 0) >> Lean.Parser.optional
    (Lean.Parser.checkLineEq "a `break` value must start on the same line as `break`" >>
      Lean.Parser.categoryParser `verexExpr 0)
@[verexExpr_parser] def verexContinueExpr := leading_parser
  Lean.Parser.withPosition <| kwContinue >>
    Lean.Parser.optional (Lean.Parser.categoryParser `verexLoopLabel 0) >>
    Lean.Parser.notFollowedBy
      (Lean.Parser.checkLineEq "continue has no value" >>
        Lean.Parser.categoryParser `verexExpr 0)
      "`continue` takes no value or function call; use a plain or labeled `continue`"
syntax:1 (name := verexReturnExpr) kwReturn verexExpr:1 : verexExpr

declare_syntax_cat verexParameter
syntax (name := verexParameterSyntax)
  (kwMut)? verexIdentifier ":" verexType : verexParameter

declare_syntax_cat verexModifier
syntax (name := verexPrivateModifier) kwPrivate : verexModifier
syntax (name := verexPublicModifier) kwPublic : verexModifier
syntax (name := verexPackageModifier) kwPackage : verexModifier
syntax (name := verexFriendModifier) kwFriend : verexModifier
syntax (name := verexEntryModifier) kwEntry : verexModifier
syntax (name := verexNativeModifier) kwNative : verexModifier
syntax (name := verexOpaqueModifier) kwOpaque : verexModifier
syntax (name := verexDeprecatedModifier) kwDeprecated : verexModifier
syntax (name := verexViewModifier) kwView : verexModifier

declare_syntax_cat verexClause
declare_syntax_cat verexConditionProperty
syntax (name := verexConditionPropertySyntax)
  verexIdentifier ("=" verexExpr)? : verexConditionProperty
declare_syntax_cat verexConditionProperties
syntax (name := verexConditionPropertiesSyntax) "[" verexConditionProperty,* "]" : verexConditionProperties
syntax (name := verexLetPreClause)
  kwLetPre (verexConditionProperties)? verexIdentifier ":=" verexExpr (";")? : verexClause
syntax (name := verexLetPostClause)
  kwLetPost (verexConditionProperties)? verexIdentifier ":=" verexExpr (";")? : verexClause
syntax (name := verexRequiresClause)
  kwRequires (verexConditionProperties)? verexExpr (";")? : verexClause
syntax (name := verexEnsuresClause)
  kwEnsures (verexConditionProperties)? verexExpr (";")? : verexClause
syntax (name := verexAbortsIfClause)
  kwAbortsIf (verexConditionProperties)? verexExpr (kwWith verexExpr)? (";")? : verexClause
syntax (name := verexAbortsWithClause)
  kwAbortsWith (verexConditionProperties)? verexExpr ("," verexExpr)* (";")? : verexClause
syntax (name := verexInvariantClause)
  kwInvariant (verexConditionProperties)? verexExpr (";")? : verexClause
/-- A ghost field of every value of the struct or enum, with the value a pack
gives it (`doc/designs/ghost-fields.md`). -/
syntax (name := verexGhostClause)
  kwGhost verexIdentifier ":" verexType (":=" verexExpr)? (";")? : verexClause
syntax (name := verexModifiesClause) kwModifies verexExpr (";")? : verexClause
syntax (name := verexLooseModifiesClause) kwModifies verexExpr "," "*" (";")? : verexClause
syntax (name := verexModifiesAllClause) kwModifies "*" (";")? : verexClause
syntax (name := verexModifiesOfClause)
  kwModifiesOf "<" verexIdentifier ">" "(" verexParameter,* ")" verexExpr,+ (";")? : verexClause
syntax (name := verexModifiesOfAllClause)
  kwModifiesOf "<" verexIdentifier ">" "*" (";")? : verexClause
syntax (name := verexReadsClause) kwReads verexType (";")? : verexClause
syntax (name := verexReadsAllClause) kwReads "*" (";")? : verexClause
syntax (name := verexPragmaClause)
  kwPragma (verexIdentifier <|> kwOpaque) ("=" verexExpr)? (";")? : verexClause
/-- A pragma naming a declaration of another module. -/
syntax (name := verexQualifiedPragmaClause)
  kwPragma verexIdentifier "=" verexQualifiedName (";")? : verexClause

declare_syntax_cat verexField
syntax (name := verexFieldSyntax) atomic(verexFieldIdentifier ":") verexType : verexField

declare_syntax_cat verexVariant
declare_syntax_cat verexDiscriminant
syntax (name := verexVariantDiscriminant) "=" num : verexDiscriminant
syntax (name := verexVariantSyntax)
  "|" verexIdentifier ("(" verexField,* ")")? (verexDiscriminant)? : verexVariant

declare_syntax_cat verexItem
syntax (name := verexUseItem) kwUse verexPath : verexItem
syntax (name := verexFriendItem) kwFriend verexPath (";")? : verexItem
syntax (name := verexNamespacePragmaItem)
  kwPragma (verexIdentifier <|> kwOpaque) ("=" verexExpr)? (";")? : verexItem
syntax (name := verexConstantItem)
  (docComment)? kwConst verexIdentifier ":" verexType ":=" verexExpr : verexItem
declare_syntax_cat verexAttribute
declare_syntax_cat verexAttributeListSyntax
syntax (name := verexAttributeSyntax)
  verexIdentifier ("(" verexAttribute,+ ")")? : verexAttribute
syntax (name := verexAttributeAssignmentSyntax)
  verexIdentifier "=" (num <|> str <|> verexIdentifier) : verexAttribute
syntax (name := verexAttributeList) "@[" verexAttribute,+ "]" : verexAttributeListSyntax

syntax (name := verexStructItem)
  (docComment)? (verexAttributeListSyntax)? kwStruct verexIdentifier verexGenericBinder* (kwHas verexAbility,+)? kwWhere
    ppLine ppIndent(verexField*) : verexItem
syntax (name := verexEnumItem)
  (docComment)? (verexAttributeListSyntax)? kwEnum verexIdentifier verexGenericBinder*
    (kwHas verexAbility,+)? kwWhere
    ppLine ppIndent(verexVariant*) : verexItem
syntax (name := verexFunctionItem)
  (docComment)? (verexAttributeListSyntax)? verexModifier* kwFun verexIdentifier verexGenericBinder* "(" verexParameter,* ")"
    "->" verexType (":=" verexExpr)? : verexItem
declare_syntax_cat verexSpecDecreases
/-- The measure a recursive specification function decreases. -/
syntax (name := verexSpecDecreasesSyntax) kwDecreases verexExpr:2 : verexSpecDecreases
syntax (name := verexSpecFunctionItem) (priority := high)
  (docComment)? (verexAttributeListSyntax)? (kwOpaque)? kwSpec kwFun verexIdentifier verexGenericBinder* "(" verexParameter,* ")"
    (":" verexType)? (verexSpecDecreases)? (":=" verexExpr)? : verexItem
/-- A lemma: its `requires` clauses imply its `ensures` clauses, established
by the steps after `proof`. -/
syntax (name := verexLemmaItem) (priority := high)
  (docComment)? kwSpec kwLemma verexIdentifier verexGenericBinder* "(" verexParameter,* ")"
    (verexSpecDecreases)? kwWhere ppLine ppIndent(verexClause*
      (kwProof ppLine ppIndent(verexSpecStatement*))?) : verexItem
syntax (name := verexContractItem)
  kwSpec verexIdentifier "{" verexClause* "}" : verexItem
syntax (name := verexContractWhereItem)
  kwSpec verexIdentifier kwWhere ppLine ppIndent(verexClause*) : verexItem
declare_syntax_cat verexNamespaceInvariantMember
/-- An invariant of the module; a generic one at the instantiations of the
memory a verification uses. -/
syntax (name := verexNamespaceInvariantMemberSyntax)
  kwInvariant verexGenericBinder* (verexConditionProperties)? verexExpr (";")? :
    verexNamespaceInvariantMember
/-- An axiom of the module: a proposition assumed by every verification of
its functions and never an obligation; a generic one at the instantiations a
verification uses. -/
syntax (name := verexNamespaceAxiomMemberSyntax)
  "axiom" verexGenericBinder* verexExpr (";")? : verexNamespaceInvariantMember
syntax (name := verexNamespaceInvariantItem) (priority := high)
  kwSpec kwModule kwWhere ppLine ppIndent(verexNamespaceInvariantMember*) : verexItem
/-- A theorem among a module's items: a lemma about the module's
specification functions for its authored proofs. -/
def verexTheorem : Lean.Parser.Parser :=
  Lean.Parser.node `VerexSL.verexTheorem
    (Lean.Parser.Command.declModifiers false >> Lean.Parser.Command.theorem)

attribute [run_builtin_parser_attribute_hooks] verexTheorem

builtin_initialize register_parser_alias verexTheorem

/-- Theorems are elaborated in the module's namespace once its items are
registered, in source order, and `verify` items after them. -/
syntax (name := verexTheoremItem)
  (atomic(Lean.Parser.Command.open " in "))? verexTheorem : verexItem
/-- `open N …` opens namespaces for the module's theorems and authored proofs,
wherever it stands among the items; `VerexIR` is always open for them. -/
syntax (name := verexOpenItem) Lean.Parser.Command.open : verexItem
/-- A step of an authored proof. -/
declare_syntax_cat verexProofStep (behavior := symbol)
/-- `lean => tac` runs Lean tactics on the goals of the step's scope. -/
syntax (name := verexLeanStep) kwLean " => " Lean.Parser.Tactic.tacticSeq : verexProofStep
/-- The steps of an authored proof, one per line. -/
syntax verexProofBlock := many1Indent(verexProofStep)
/-- `verify! f` runs in the editor, where `verex.skip_verify` skips the others. -/
syntax (name := verexVerifyItem)
  ("verify!" <|> "verify") verexIdentifier ("by" verexProofBlock)? : verexItem

syntax (name := verexNamespaceCommand)
  "verex" kwNamespace verexPath kwUsing verexProfile kwWhere
    ppLine ppIndent(manyIndent(verexItem)) : command
syntax (name := verexMoveModuleCommand)
  "verex" kwModule verexPath kwWhere
    ppLine ppIndent(manyIndent(verexItem)) : command
syntax (name := verexRustNamespaceCommand)
  "verex" kwNamespace verexPath kwWhere
    ppLine ppIndent(manyIndent(verexItem)) : command

syntax (name := checkVerexCommand) "#check_verex" verexPath : command

end VerexSL

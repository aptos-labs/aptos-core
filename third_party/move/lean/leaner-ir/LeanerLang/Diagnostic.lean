-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Ast

namespace LeanerLang

inductive Severity where
  | error
  | warning
  deriving Repr, BEq, Inhabited

/-- Frontend diagnostic produced before the shared LIR validation boundary. -/
structure Diagnostic where
  severity : Severity := .error
  code : String
  message : String
  span : Option Span := none
  deriving Repr, BEq, Inhabited

def Diagnostic.error (code message : String) (span : Option Span := none) : Diagnostic :=
  { code, message, span }

def Diagnostic.render (diagnostic : Diagnostic) : String :=
  let range := match diagnostic.span with
    | none => ""
    | some span => s!" [{span.startByte}, {span.endByte})"
  s!"{diagnostic.code}{range}: {diagnostic.message}"

abbrev Result (α : Type) := Except (Array Diagnostic) α

def Result.error (code message : String) (span : Option Span := none) : Result α :=
  Except.error #[Diagnostic.error code message span]

end LeanerLang

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.MonoVM.Payload
import LeanerIR

/-!
# Normalized differential outcomes

The pure core of the MonoVM differential harness: it maps what each engine
produced onto one neutral shape that the fixture baselines record. Neither
side is the baseline, so nothing here decides agreement — the recorded
outcomes do, by being equal or not.

Normalization is where representation differences are removed so that a diff
means a semantic difference. Integers compare numerically (the fixture's
signature fixes the widths on both sides, so width tags carry no meaning
here) and addresses compare by identity, not spelling. Exhaustion is carried
as its own `inconclusive` shape rather than as a result: the two sides meter
different resources, so one running out is never evidence about the other.
-/

namespace LeanerE2ETests.MonoVM

open LeanerIR

/-- One comparable value shape. `unmodeled` marks shapes the comparison has
not been given a meaning for yet; comparing one is a driver error, never a
silent pass. -/
inductive NormValue where
  | unit
  | bool (value : Bool)
  | integer (value : Int)
  | address (value : String)
  | vector (elements : Array NormValue)
  | unmodeled (description : String)
  deriving BEq, Repr

/-- The normalized outcome of one call on one side of the comparison. -/
inductive NormOutcome where
  | returned (values : Array NormValue)
  | aborted (code : Int)
  /-- The program failed at runtime rather than aborting. `failure` is the
  engine's classification, which is what comparison uses; `message` is
  diagnostic text carried for triage. -/
  | failed (failure : String) (message : String)
  | inconclusive (reason : String)
  | error (message : String)
  deriving BEq, Repr

/-- Canonicalizes an address literal to its semantic identity. The two engines
render the same address differently — one emits the shortest hex literal, the
other a padded or differently-cased one — and that is representation, not
meaning, so the comparison must not see it. -/
def canonicalAddress (literal : String) : String :=
  let digits := if literal.startsWith "0x" || literal.startsWith "0X"
    then literal.drop 2 else literal.toSlice
  let digits := digits.toString.toLower
  let trimmed := (digits.dropWhile (· == '0')).toString
  "0x" ++ (if trimmed.isEmpty then "0" else trimmed)

/-- Names the adapter stage a failure came from. `Repr` would spell the Lean
constructor path, which is noise in a recorded result. -/
def stageText : Stage → String
  | .compile => "compile"
  | .load => "load"
  | .run => "run"
  | .abi => "abi"
  | .internal => "internal"

/-- Normalizes an adapter response outcome. -/
def normalizeOutcome : Outcome → NormOutcome
  | .returned values _ _ => .returned (values.map normalizeValue)
  | .aborted code _ _ => .aborted (Int.ofNat code)
  | .exhausted .gas => .inconclusive "the adapter ran out of gas"
  | .exhausted .heap => .inconclusive "the adapter ran out of heap"
  | .failed failure message => .failed failure message
  | .error stage message =>
      .error s!"adapter {stageText stage} failure: {message}"

where
  normalizeValue : Value → NormValue
    | .unit => .unit
    | .bool value => .bool value
    | .integer _ _ value =>
      match value.toInt? with
      | some integer => .integer integer
      | none => .unmodeled s!"integer literal {value}"
    | .address value => .address (canonicalAddress value)
    | .vector elements => .vector (elements.map normalizeValue)

/-- Normalizes a LeanerIR interpreter result for one call. Abort locations
and messages are diagnostics, not compared values; the abort code is the
integer argument of the abort throw. -/
def normalizeLeanOutcome
    (result : Except LocatedInterpreterError (RuntimeState × LocatedOutcome)) :
    NormOutcome :=
  match result with
  | .error located =>
      match located.value with
      | .outOfFuel => .inconclusive "the LeanerIR interpreter ran out of fuel"
      | failure =>
          .error
            s!"interpreter failure at namespace {located.primary.namespaceId.index}:               {repr failure}"
  | .ok (_, outcome) =>
      match outcome.value with
      | .returned values => .returned (values.map normalizeRuntimeValue)
      | .threw kind arguments =>
          match kind, arguments.toList with
          | .abort, [.integer code] => .aborted code
          | .abort, _ => .error "abort throw carried no single integer code"
          | other, _ => .error s!"unexpected throw kind {repr other}"

where
  normalizeRuntimeValue : RuntimeValue → NormValue
    | .unit => .unit
    | .bool value => .bool value
    | .integer value => .integer value
    | .address value => .address (canonicalAddress value)
    | .string value => .unmodeled s!"string {value}"
    | .bytes value => .unmodeled "bytes"
    | .character value => .unmodeled "character"
    | .signer value => .unmodeled s!"signer {value}"
    | .vector elements => .vector (elements.map normalizeRuntimeValue)
    | .tuple elements => .unmodeled "tuple"
    | .nominal name variant _ => .unmodeled s!"nominal {repr name} {repr variant}"
    | .closure _ _ => .unmodeled "closure"
    | .borrow _ _ => .unmodeled "reference"
    | .loanHole _ => .unmodeled "reference"

/-- Whether two normalized outcomes diverge, which is what a fixture baseline
marks as an error.

Exhaustion on either side is never a divergence: the engines meter different
resources — the interpreter is fuelled, the adapter is gas- and heap-bounded —
so one running out is no evidence about the other. Everything else that is not
an equal pair is: differing values or abort codes, an outcome kind that does
not match, and an `error` on either side, which means no comparable outcome
was obtained at all. -/
def divergent : NormOutcome → NormOutcome → Bool
  | .inconclusive _, _ => false
  | _, .inconclusive _ => false
  | .returned left, .returned right => left != right
  | .aborted left, .aborted right => left != right
  -- Only the classification is compared. The diagnostic text beside it is
  -- each engine's own wording, and two implementations legitimately describe
  -- the same failure differently.
  | .failed left _, .failed right _ => left != right
  | _, _ => true

/-! ## Baseline rendering

The differential result of a fixture is a reviewable test asset: the recorded
outcome of every engine on every step. Only normalized content is rendered, so
the baseline is deterministic — gas usage, collection counts, abort locations,
and adapter build identity are triage output, not recorded expectations.
-/

/-- Renders one normalized value. -/
def NormValue.render : NormValue → String
  | .unit => "()"
  | .bool value => toString value
  | .integer value => toString value
  | .address value => value
  | .vector elements =>
      "[" ++ String.intercalate ", " (elements.map NormValue.render).toList ++ "]"
  | .unmodeled description => s!"<{description}>"

/-- Renders one engine's normalized outcome. -/
def NormOutcome.render : NormOutcome → String
  | .returned values =>
      if values.isEmpty then "returned ()"
      else "returned " ++ String.intercalate ", " (values.map NormValue.render).toList
  | .aborted code => s!"aborted {code}"
  | .failed failure message => s!"failed {failure} ({message})"
  | .inconclusive reason => s!"inconclusive ({reason})"
  | .error message => s!"error ({message})"

/-! ## Unit tests

These evaluate at elaboration time and fail the build on wrong normalization.
They stay runtime-oriented (BEq, not decidable equality) because normalized
values contain arrays.
-/

private def returned (values : Array NormValue) : NormOutcome := .returned values

private def expect (what : String) (condition : Bool) : IO Unit := do
  unless condition do
    throw <| IO.userError s!"normalization unit test failed: {what}"

#eval expect "adapter gas exhaustion is not a result"
  (normalizeOutcome (.exhausted .gas) ==
    NormOutcome.inconclusive "the adapter ran out of gas")

#eval expect "adapter heap exhaustion is not a result"
  (normalizeOutcome (.exhausted .heap) ==
    NormOutcome.inconclusive "the adapter ran out of heap")

#eval expect "equal results do not diverge"
  (!divergent (returned #[.integer 3]) (returned #[.integer 3]))

#eval expect "different results diverge"
  (divergent (returned #[.integer 3]) (returned #[.integer 4]))

#eval expect "equal abort codes do not diverge"
  (!divergent (.aborted 42) (.aborted 42))

#eval expect "a result and an abort diverge"
  (divergent (returned #[.integer 42]) (.aborted 42))

#eval expect "adapter exhaustion is never a divergence"
  (!divergent (.inconclusive "the adapter ran out of gas") (.aborted 42))

#eval expect "interpreter exhaustion is never a divergence"
  (!divergent (.aborted 42) (.inconclusive "the interpreter ran out of fuel"))

#eval expect "a runtime failure is propagated, not treated as a harness error"
  (normalizeOutcome (.failed "InvalidOperation" "Add: under/overflow") ==
    NormOutcome.failed "InvalidOperation" "Add: under/overflow")

#eval expect "the same failure classification does not diverge"
  (!divergent (.failed "InvalidOperation" "Add: under/overflow")
    (.failed "InvalidOperation" "arithmetic error"))

#eval expect "different failure classifications diverge"
  (divergent (.failed "InvalidOperation" "x") (.failed "RuntimeLimitExceeded" "x"))

#eval expect "a runtime failure and an abort diverge"
  (divergent (.failed "InvalidOperation" "Add: under/overflow") (.aborted 300))

#eval expect "an error on either side diverges"
  (divergent (.error "adapter load failure") (returned #[.integer 3]) &&
    divergent (returned #[.integer 3]) (.error "interpreter failure"))

#eval expect "abort normalization keeps the code"
  (normalizeOutcome (.aborted 7 none none) == .aborted 7)

#eval expect "integer value normalization parses the decimal string"
  (normalizeOutcome (.returned #[.integer 64 false "10"] 0 0) ==
    NormOutcome.returned #[NormValue.integer 10])

#eval expect "address rendering differences are not semantic differences"
  (returned #[.address (canonicalAddress "0x0000000000000047")] ==
    returned #[.address (canonicalAddress "0x47")])

#eval expect "distinct addresses still mismatch"
  (canonicalAddress "0x47" != canonicalAddress "0x48")

#eval expect "the zero address canonicalizes to a digit"
  (canonicalAddress "0x0000" == "0x0")

#eval expect "an empty result list renders as unit"
  (NormOutcome.render (.returned #[]) == "returned ()")

#eval expect "nested vector results render structurally"
  (NormOutcome.render (returned #[.vector #[.vector #[.integer 1, .integer 2], .vector #[]]]) ==
    "returned [[1, 2], []]")

#eval expect "an exhausted outcome records its reason"
  (NormOutcome.render (.inconclusive "the adapter ran out of gas") ==
    "inconclusive (the adapter ran out of gas)")

end LeanerE2ETests.MonoVM

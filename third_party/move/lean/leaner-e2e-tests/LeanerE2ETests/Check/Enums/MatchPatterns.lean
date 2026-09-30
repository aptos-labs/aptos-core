-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Matches and refutable patterns

A match over a move-only enum, a guarded or nested match, a match over
integer literals, and a nested destructuring `let` are matches of the
denotation. A value no pattern fits makes the profile's mismatch throw:
in Move, an abort with the incomplete-match code `0xCA26CBD9BE0B0001`,
both for a match without an arm for it and for a destructuring `let` or
assignment.
-/

namespace LeanerLang.Tests.Check.Enums.MatchPatterns

leaner module 0x42::match_patterns where
  enum Token has Drop where
    | Word (length : u64)
    | Pair (left : u64, right : u64)
    | Done

  enum Atom has Copy, Drop where
    | None
    | Number (value : u64)

  enum Envelope has Copy, Drop where
    | Empty
    | One (value : Atom)

  fun weight(token : Token) -> u64 :=
    match token with
      | Token::Word { length := n } => n
      | Token::Pair { left := l, right := r } => l + r
      | Token::Done {} => 0
  spec weight where
    pragma aborts_if_is_partial
    ensures token is Done ==> result == 0

  fun pair_sum(l : u64, r : u64) -> u64 := weight(new Token::Pair { left := l, right := r })
  spec pair_sum where
    pragma aborts_if_is_partial
    ensures result == l + r

  fun weight_incorrect(token : Token) -> u64 :=
    match token with
      | Token::Word { length := n } => n
      | Token::Pair { left := l, right := r } => l + r
      | Token::Done {} => 1
  spec weight_incorrect where
    pragma aborts_if_is_partial
    ensures token is Done ==> result == 0 -- error: the last arm gives 1 for `Done`

  fun capped(token : Token) -> u64 :=
    match token with
      | Token::Word { length := n } if n > 10 => 10
      | Token::Word { length := n } => n
      | _ => 0
  spec capped where
    ensures result <= 10

  fun capped_incorrect(token : Token) -> u64 :=
    match token with
      | Token::Word { length := n } if n > 10 => 11
      | Token::Word { length := n } => n
      | _ => 0
  spec capped_incorrect where
    ensures result <= 10 -- error: the guarded arm gives 11

  fun capped_atom(a : Atom) -> u64 :=
    match a with
      | Atom::Number { value := v } if v > 10 => 10
      | Atom::Number { value := v } => v
      | Atom::None {} => 0
  spec capped_atom where
    ensures result <= 10

  fun inner(e : Envelope) -> u64 :=
    match e with
      | Envelope::One { value := Atom::Number { value := v } } => v
      | _ => 0
  spec inner where
    ensures e is Empty ==> result == 0

  fun classify(x : u64) -> u64 :=
    match x with
      | 0 => 100
      | 1 => 200
      | _ => x
  spec classify where
    ensures x == 1 ==> result == 200
    ensures x > 1 ==> result == x

  fun sum3(a : u64, b : u64, c : u64) -> u64 := do
    let (x, (y, z)) := (a, (b, c))
    x + y + z
  spec sum3 where
    pragma aborts_if_is_partial
    ensures result == a + b + c

  fun unpack(token : Token) -> u64 := do
    let Token::Word { length := n } := token
    n
  spec unpack where
    aborts_if !(token is Word) with 14566554180833181697

  fun unpack_incorrect(token : Token) -> u64 := do
    let Token::Word { length := n } := token
    n
  spec unpack_incorrect where
    aborts_if false -- error: a value of another variant aborts

  fun unpack_word(n : u64) -> u64 := unpack(new Token::Word { length := n })
  spec unpack_word where
    aborts_if false
    ensures result == n

  fun assign(token : Token) -> u64 := do
    let mut l : u64 := 0
    let mut r : u64 := 0
    assign_pattern[Token](Token::Pair { left := l, right := r }, token)
    l + r
  spec assign where
    pragma aborts_if_is_partial
    aborts_if !(token is Pair) with 14566554180833181697

-- The runtime makes the same throw.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let token (variant : String) (fields : Array RuntimeValue := #[]) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ (some variant) fields
  let mismatch : Outcome := .threw .abort #[.integer moveIncompleteMatchAbortCode]
  assertRuns `«0x42».match_patterns #[
    ⟨"unpack", #[token "Word" #[.integer 7]], .returned #[.integer 7], {}⟩,
    ⟨"unpack", #[token "Done"], mismatch, {}⟩,
    ⟨"assign", #[token "Pair" #[.integer 3, .integer 4]], .returned #[.integer 7], {}⟩,
    ⟨"assign", #[token "Word" #[.integer 3]], mismatch, {}⟩,
    ⟨"capped", #[token "Word" #[.integer 30]], .returned #[.integer 10], {}⟩,
    ⟨"classify", #[.integer 1], .returned #[.integer 200], {}⟩]

end LeanerLang.Tests.Check.Enums.MatchPatterns

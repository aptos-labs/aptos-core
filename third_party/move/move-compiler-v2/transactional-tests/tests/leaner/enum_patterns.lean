--# publish

import Move

module LeanerEnumPatterns where

  @[move_enum]
  inductive Atom where
    | none
    | number (value : U64)
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Envelope where
    | empty
    | one (value : Atom)
    | two (left right : Atom)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun nested_total (envelope : Envelope) : U64 :=
    match envelope with
    | .one (.number value) => value
    | .two (.number left) (.number right) => left + right
    | _ => 0

  fun one_number (value : U64) : U64 :=
    nested_total (.one (.number value))

  fun one_none : U64 := nested_total (.one .none)

  fun two_numbers (left right : U64) : U64 :=
    nested_total (.two (.number left) (.number right))

  fun left_missing (right : U64) : U64 :=
    nested_total (.two .none (.number right))

  fun right_missing (left : U64) : U64 :=
    nested_total (.two (.number left) .none)

/-! ## Tests -/

--# run 0x0::LeanerEnumPatterns::one_number --args 7u64

--# run 0x0::LeanerEnumPatterns::one_none

--# run 0x0::LeanerEnumPatterns::two_numbers --args 4u64 5u64

--# run 0x0::LeanerEnumPatterns::left_missing --args 5u64

--# run 0x0::LeanerEnumPatterns::right_missing --args 4u64

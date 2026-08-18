--# publish

import Move

move_module LeanerEnumPatterns where

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

  fun nestedTotal (envelope : Envelope) : U64 :=
    match envelope with
    | .one (.number value) => value
    | .two (.number left) (.number right) => left + right
    | _ => 0

  fun oneNumber (value : U64) : U64 :=
    nestedTotal (.one (.number value))

  fun oneNone : U64 := nestedTotal (.one .none)

  fun twoNumbers (left right : U64) : U64 :=
    nestedTotal (.two (.number left) (.number right))

  fun leftMissing (right : U64) : U64 :=
    nestedTotal (.two .none (.number right))

  fun rightMissing (left : U64) : U64 :=
    nestedTotal (.two (.number left) .none)

/-! ## Tests -/

--# run 0x0::LeanerEnumPatterns::oneNumber --args 7u64

--# run 0x0::LeanerEnumPatterns::oneNone

--# run 0x0::LeanerEnumPatterns::twoNumbers --args 4u64 5u64

--# run 0x0::LeanerEnumPatterns::leftMissing --args 5u64

--# run 0x0::LeanerEnumPatterns::rightMissing --args 4u64

--# publish

import Move

open scoped Move

module LeanerGenerics where

  @[move_struct]
  structure Box (T : Type) where
    value : T
    deriving Copy, Drop, Store

  @[move_struct]
  structure Pair (T U : Type) where
    first : T
    second : U
    deriving Copy, Drop, Store

  @[move_struct]
  structure Vault (T : Type) where
    value : T
    deriving Key

  @[move_enum]
  inductive Choice (T : Type) where
    | none
    | some (value : T)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun identity {T : Type} (value : T) : T := value

  fun box {T : Type} (value : T) : Box T := { value }

  fun unbox {T : Type} (value : Box T) : T := value.value

  fun swap {T U : Type} (value : Pair T U) : Pair U T :=
    { first := value.second, second := value.first }

  fun choose {T : Type} (fallback : T) (choice : Choice T) : T :=
    match choice with
    | .none => fallback
    | .some inner => inner

  fun singleton {T : Type} (value : T) : Move.Vector T := vector![value]

  fun publish {T : Type} (signer : &Signer) (value : T) : Action Unit :=
    moveTo signer ({ value } : Vault T)

  fun contains {T : Type} (address : Address) : Action Bool :=
    existsAt (Vault T) address

  @[move_public]
  fun round_trip (value : U64) : U64 :=
    unbox (box (identity value))

  @[move_public]
  fun enum_round_trip (value : U64) : U64 :=
    let choice : Choice U64 := .some value
    choose 0 choice

  @[move_public]
  fun swap_first (first second : U64) : U64 :=
    (swap ({ first, second } : Pair U64 U64)).first

  @[move_public]
  fun generic_vector_length (value : U64) : U64 :=
    Move.Vector.length (singleton value)

  @[move_public]
  fun equal_u64 (left right : U64) : Bool := left == right

  @[move_public]
  fun publish_u64 (signer : &Signer) (value : U64) : Action Unit :=
    publish signer value

  @[move_public]
  fun publish_bool (signer : &Signer) (value : Bool) : Action Unit :=
    publish signer value

  @[move_public]
  fun take_u64 (address : Address) : Action U64 := do
    let vault ← moveFrom (Vault U64) address
    let Vault { value } := vault
    pure value

  @[move_public]
  fun take_bool (address : Address) : Action Bool := do
    let vault ← moveFrom (Vault Bool) address
    let Vault { value } := vault
    pure value

  @[move_public]
  fun has_u64 (address : Address) : Action Bool := contains (T := U64) address

  @[move_public]
  fun has_bool (address : Address) : Action Bool := contains (T := Bool) address

/-! ## Tests -/

--# run 0x0::LeanerGenerics::round_trip --args 37u64

--# run 0x0::LeanerGenerics::enum_round_trip --args 19u64

--# run 0x0::LeanerGenerics::swap_first --args 3u64 41u64

--# run 0x0::LeanerGenerics::generic_vector_length --args 23u64

--# run 0x0::LeanerGenerics::equal_u64 --args 17u64 17u64

--# run 0x0::LeanerGenerics::equal_u64 --args 17u64 18u64

-- The same generic resource at two instantiations must occupy distinct
-- storage keys. Both publications at 0x42 therefore succeed.
--# run --args 29u64 --signers 0x42 -- 0x0::LeanerGenerics::publish_u64

--# run --args true --signers 0x42 -- 0x0::LeanerGenerics::publish_bool

--# run 0x0::LeanerGenerics::has_u64 --args @0x42

--# run 0x0::LeanerGenerics::has_bool --args @0x42

--# run 0x0::LeanerGenerics::take_u64 --args @0x42

--# run 0x0::LeanerGenerics::take_bool --args @0x42

-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerGenerics where
  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where
    value : T

  struct Pair {T has Copy, Drop, Store} {U has Copy, Drop, Store} has Copy, Drop, Store where
    first : T
    second : U

  struct Vault {T has Store} has Key where
    value : T

  enum Choice {T has Copy, Drop, Store} has Copy, Drop, Store where
    | None
    | Some (value : T)

  fun identity {T}(value : T) -> T := value

  fun box {T has Copy, Drop, Store}(value : T) -> Box<T> := new Box<T> { value }

  fun unbox {T has Copy, Drop, Store}(value : Box<T>) -> T := value.value

  fun swap {T has Copy, Drop, Store} {U has Copy, Drop, Store}(value : Pair<T, U>) -> Pair<U, T> :=
    new Pair<U, T> { first := value.second, second := value.first }

  fun choose {T has Copy, Drop, Store}(fallback : T, choice : Choice<T>) -> T :=
    match choice with
      | Choice<T>::None {} => fallback
      | Choice<T>::Some { value := value } => value

  fun singleton {T}(value : T) -> Vector<T> := vector<T>[value]

  fun publish {T has Store}(account : &Signer, value : T) -> Unit :=
    move_to<Vault<T> >(account, new Vault<T> { value })

  fun contains {T has Store}(address : Address) -> Bool := exists<Vault<T> >(address)

  public fun round_trip(value : u64) -> u64 :=
    unbox::<u64>(box::<u64>(identity::<u64>(value)))

  public fun enum_round_trip(value : u64) -> u64 := do
    let choice := new Choice<u64>::Some { value }
    choose::<u64>(0, choice)

  public fun swap_first(first : u64, second : u64) -> u64 :=
    swap::<u64, u64>(new Pair<u64, u64> { first, second }).first

  public fun generic_vector_length(value : u64) -> u64 := singleton::<u64>(value).length

  public fun equal_u64(left : u64, right : u64) -> Bool := left == right

  public fun publish_u64(account : &Signer, value : u64) -> Unit := publish::<u64>(account, value)

  public fun publish_bool(account : &Signer, value : Bool) -> Unit :=
    publish::<Bool>(account, value)

  public fun take_u64(address : Address) -> u64 := do
    let Vault<u64> { value := value } := move_from<Vault<u64> >(address)
    value

  public fun take_bool(address : Address) -> Bool := do
    let Vault<Bool> { value := value } := move_from<Vault<Bool> >(address)
    value

  public fun has_u64(address : Address) -> Bool := contains::<u64>(address)

  public fun has_bool(address : Address) -> Bool := contains::<Bool>(address)

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

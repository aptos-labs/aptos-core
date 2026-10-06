-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerClosures where
  struct Op has Copy, Drop where
    f : Fn(u64) -> u64 has Copy, Drop

  struct Holder has Key where
    f : Fn(u64) -> u64 has Copy, Drop, Store

  fun add(x : u64, y : u64) -> u64 := x + y

  fun sub(x : u64, y : u64) -> u64 := x - y

  fun pick {T has Copy, Drop}(first : Bool, a : T, b : T) -> T := if first then a else b

  fun check(limit : u64, x : u64) -> u64 := if x <= limit then x else abort(7)

  public fun scale(factor : u64, x : u64) -> u64 := factor * x

  fun adder(x : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](add, x, _)

  fun apply(x : u64, f : Fn(u64) -> u64 has Copy, Drop) -> u64 := invoke(f, x)

  public fun leading(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x, _)
    invoke(f, y)

  public fun trailing(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)
    invoke(f, x)

  public fun generic(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)

  public fun returned(x : u64, y : u64) -> u64 := invoke(adder(x), y)

  public fun field(x : u64, y : u64) -> u64 := do
    let op := new Op { f := adder(x) }
    invoke(op.f, y)

  public fun higher_order(x : u64, y : u64) -> u64 :=
    apply(y, function[Fn(u64) -> u64 has Copy, Drop](sub, _, x))

  public fun checked(limit : u64, x : u64) -> u64 :=
    apply(x, function[Fn(u64) -> u64 has Copy, Drop](check, limit, _))

  public fun same(x : u64, y : u64) -> Bool := adder(x) == adder(y)

  public fun publish(account : &Signer, factor : u64) -> Unit :=
    move_to<Holder>(account, new Holder { f := function[Fn(u64) -> u64 has Copy, Drop, Store](scale, factor, _) })

  public fun take(address : Address, x : u64) -> u64 := do
    let Holder { f := f } := move_from<Holder>(address)
    invoke(f, x)

--# run 0x0::LeanerClosures::leading --args 3u64 4u64

--# run 0x0::LeanerClosures::trailing --args 10u64 4u64

--# run 0x0::LeanerClosures::generic --args true 1u64 2u64

--# run 0x0::LeanerClosures::generic --args false 1u64 2u64

--# run 0x0::LeanerClosures::returned --args 5u64 6u64

--# run 0x0::LeanerClosures::field --args 7u64 8u64

--# run 0x0::LeanerClosures::higher_order --args 2u64 9u64

--# run 0x0::LeanerClosures::higher_order --args 9u64 2u64

--# run 0x0::LeanerClosures::checked --args 10u64 3u64

--# run 0x0::LeanerClosures::checked --args 10u64 30u64

--# run 0x0::LeanerClosures::same --args 4u64 4u64

--# run 0x0::LeanerClosures::same --args 4u64 5u64

--# run --args 6u64 --signers 0x42 -- 0x0::LeanerClosures::publish

--# run 0x0::LeanerClosures::take --args @0x42 7u64

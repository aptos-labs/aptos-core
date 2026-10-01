-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerEnumPayloads where
  enum Choice has Copy, Drop, Store where
    | Left (value : u64)
    | Right (value : u64)

  enum Batch has Copy, Drop, Store where
    | Empty
    | Items (values : Vector<u64>)

  enum Positional has Copy, Drop, Store where
    | Pair (_0 : u64, _1 : u64)

  enum Wrapper has Copy, Drop, Store where
    | Wrap (value : u64)

  fun choose(right : Bool, value : u64) -> Choice :=
    if right then new Choice::Right { value } else new Choice::Left { value }

  fun score(choice : Choice) -> u64 :=
    match choice with
      | Choice::Left { value := value } => value + 1
      | Choice::Right { value := value } => value + 2

  fun choose_and_score(right : Bool, value : u64) -> u64 := score(choose(right, value))

  fun is_right(choice : Choice) -> u64 :=
    match choice with
      | Choice::Right { value := _ } => 1
      | _ => 0

  fun batch_length(batch : Batch) -> u64 :=
    match batch with
      | Batch::Empty {} => 0
      | Batch::Items { values := values } => values.length

  fun populated_batch() -> u64 :=
    batch_length(new Batch::Items { values := vector<u64>[4, 5, 6, 7] })

  fun empty_batch() -> u64 := batch_length(new Batch::Empty {})

  fun positional_total(value : Positional) -> u64 :=
    match value with
      | Positional::Pair { _0 := left, _1 := right } => left + right

  fun wrapped_value(value : Wrapper) -> u64 :=
    match value with
      | Wrapper::Wrap { value := inner } => inner

  fun left_score(value : u64) -> u64 := score(new Choice::Left { value })

  fun right_score(value : u64) -> u64 := score(new Choice::Right { value })

  fun left_is_right(value : u64) -> u64 := is_right(new Choice::Left { value })

  fun right_is_right(value : u64) -> u64 := is_right(new Choice::Right { value })

  fun make_positional(left : u64, right : u64) -> u64 :=
    positional_total(new Positional::Pair { _0 := left, _1 := right })

  fun make_wrapper(value : u64) -> u64 := wrapped_value(new Wrapper::Wrap { value })

  fun vector_of_enums() -> u64 := do
    let values := vector<Choice>[new Choice::Left { value := 4 }, new Choice::Right { value := 5 }]
    score(values[1])

  fun replace_enum_element() -> u64 := do
    let mut values := vector<Choice>[new Choice::Left { value := 1 }]
    let selected := &mut values[0]
    *selected := new Choice::Right { value := 7 }
    score(values[0])

--# run 0x0::LeanerEnumPayloads::choose_and_score --args false 10u64

--# run 0x0::LeanerEnumPayloads::choose_and_score --args true 10u64

--# run 0x0::LeanerEnumPayloads::left_score --args 10u64

--# run 0x0::LeanerEnumPayloads::right_score --args 10u64

--# run 0x0::LeanerEnumPayloads::left_is_right --args 10u64

--# run 0x0::LeanerEnumPayloads::right_is_right --args 10u64

--# run 0x0::LeanerEnumPayloads::populated_batch

--# run 0x0::LeanerEnumPayloads::empty_batch

--# run 0x0::LeanerEnumPayloads::make_positional --args 8u64 9u64

--# run 0x0::LeanerEnumPayloads::make_wrapper --args 23u64

--# run 0x0::LeanerEnumPayloads::vector_of_enums

--# run 0x0::LeanerEnumPayloads::replace_enum_element

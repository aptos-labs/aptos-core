-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerEnumPatterns where
  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : u64)

  enum Envelope has Copy, Drop, Store where
    | Empty
    | One (value : Atom)
    | Two (left : Atom, right : Atom)

  -- By-value payload patterns bind variables or wildcards, so the nested
  -- patterns are spelled as nested matches with wildcard fallbacks.
  fun nested_total(envelope : Envelope) -> u64 :=
    match envelope with
      | Envelope::One { value := atom } =>
          match atom with
            | Atom::Number { value := value } => value
            | _ => 0
      | Envelope::Two { left := left_atom, right := right_atom } =>
          match left_atom with
            | Atom::Number { value := left } =>
                match right_atom with
                  | Atom::Number { value := right } => left + right
                  | _ => 0
            | _ => 0
      | _ => 0

  fun one_number(value : u64) -> u64 :=
    nested_total(new Envelope::One { value := new Atom::Number { value } })

  fun one_none() -> u64 := nested_total(new Envelope::One { value := new Atom::None {} })

  fun two_numbers(left : u64, right : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::Number { value := left },
      right := new Atom::Number { value := right }
    })

  fun left_missing(right : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::None {},
      right := new Atom::Number { value := right }
    })

  fun right_missing(left : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::Number { value := left },
      right := new Atom::None {}
    })

--# run 0x0::LeanerEnumPatterns::one_number --args 7u64

--# run 0x0::LeanerEnumPatterns::one_none

--# run 0x0::LeanerEnumPatterns::two_numbers --args 4u64 5u64

--# run 0x0::LeanerEnumPatterns::left_missing --args 5u64

--# run 0x0::LeanerEnumPatterns::right_missing --args 4u64

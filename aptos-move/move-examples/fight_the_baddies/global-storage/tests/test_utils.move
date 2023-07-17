#[test_only]
module fight_the_baddies::test_utils {
use std::string;
use fight_the_baddies::character::{Self, Character};

public fun get_baddy(): Character {
  character::generate_baddy(
    string::utf8(b"Bob"),
    string::utf8(b"male"),
  )
}

public fun get_hero(): Character {
  character::generate_hero(
    string::utf8(b"Alice"),
    string::utf8(b"female"),
  )
}
}

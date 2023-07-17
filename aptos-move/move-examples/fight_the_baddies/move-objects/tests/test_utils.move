#[test_only]
module fight_the_baddies::test_utils {
use std::string;
use aptos_framework::object::Object;
use fight_the_baddies::character::{Self, Character};

public fun get_baddy(owner: &signer): Object<Character> {
  character::create_baddy(
    owner,
    string::utf8(b"Bob"),
    string::utf8(b"male"),
  )
}

public fun get_hero(owner: &signer): Object<Character> {
  character::create_hero(
    owner,
    string::utf8(b"Alice"),
    string::utf8(b"female"),
  )
}
}

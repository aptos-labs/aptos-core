#[test_only]
module fight_the_baddies::end_to_end {
use std::string;
use fight_the_baddies::character::{Self, Character};
use fight_the_baddies::fight;
use fight_the_baddies::weapon;

#[test]
fun generate_and_destroy_hero() {
  let hero = get_hero();
  assert!(character::is_hero(&hero), 0);
  assert!(character::name(&hero) == string::utf8(b"Alice"), 1);
  assert!(character::gender(&hero) == string::utf8(b"female"), 2);
  assert!(character::strength(&hero) == character::effective_strength(&hero), 3);
  character::equip_weapon(&mut hero, weapon::generate_sword(string::utf8(b"katana")));
  assert!(character::strength(&hero) < character::effective_strength(&hero), 3);
  fight::sacrifice(hero);
}

#[test]
fun generate_and_destroy_baddy() {
  let baddy = get_baddy();
  assert!(character::is_baddy(&baddy), 0);
  assert!(character::name(&baddy) == string::utf8(b"Bob"), 1);
  assert!(character::gender(&baddy) == string::utf8(b"male"), 2);
  fight::sacrifice(baddy);
}

#[test]
/// The goal with this test is to get level ups, in this scenario the sword is over-powering,
/// which enables single blows to fell the baddy.
fun end_to_end() {
  let hero = get_hero();
  character::equip_weapon(&mut hero, weapon::generate_sword(string::utf8(b"katana")));
  let current_level = character::level(&hero);
  while (current_level == character::level(&hero)) {
    let baddy = get_baddy();
    fight::fight(&mut hero, &mut baddy);
    fight::eliminate(baddy);
  };
  // We should only get one level up given the current mechanics
  assert!(current_level + 1 == character::level(&hero), 0);
  fight::sacrifice(hero);
  // The test will timeout if it cannot get here.
}

fun get_baddy(): Character {
  character::generate_baddy(
    string::utf8(b"Bob"),
    string::utf8(b"male"),
  )
}

fun get_hero(): Character {
  character::generate_hero(
    string::utf8(b"Alice"),
    string::utf8(b"female"),
  )
}
}

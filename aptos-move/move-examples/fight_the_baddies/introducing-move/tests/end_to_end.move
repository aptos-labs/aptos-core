#[test_only]
module fight_the_baddies::end_to_end {
use std::string;
use fight_the_baddies::character::{Self, Character};
use fight_the_baddies::fight;

#[test]
fun generate_and_destroy_hero() {
  let hero = get_hero();
  assert!(character::is_hero(&hero), 0);
  assert!(character::name(&hero) == string::utf8(b"Alice"), 1);
  assert!(character::gender(&hero) == string::utf8(b"female"), 2);
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
/// The goal with this test is to get level ups, so we'll create a hero who always kill steals
/// without taking damage. Then we'll create new heroes along the way to beat the baddy to near
/// death.
fun end_to_end() {
  let main_hero = get_hero();
  let current_level = character::level(&main_hero);
  while (current_level == character::level(&main_hero)) {
    let sad_hero = get_hero();
    let baddy = get_baddy();
    let main_str = character::strength(&main_hero);
    let baddy_def = character::defense(&baddy);
    while (character::health(&baddy) > 0) {
      let baddy_health = character::health(&baddy);
      if (baddy_health + (baddy_def as u32) <= (main_str as u32)) {
          fight::fight(&mut main_hero, &mut baddy)
      } else {
          fight::fight(&mut sad_hero, &mut baddy)
      };
    };
    fight::sacrifice(sad_hero);
    fight::eliminate(baddy);
  };
  // We should only get one level up given the current mechanics
  assert!(current_level + 1 == character::level(&main_hero), 0);
  fight::sacrifice(main_hero);
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

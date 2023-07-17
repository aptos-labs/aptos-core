module fight_the_baddies::fight {
use std::error;
use aptos_framework::object::Object;
use fight_the_baddies::character::{Self, Character};
use fight_the_baddies::level_up;

/// Character was not a hero.
const ENOT_HERO: u64 = 1;
/// Character was not a baddy.
const ENOT_BADDY: u64 = 2;
/// Hero character has 0 health, i.e., is dead
const EHERO_DEAD: u64 = 3;
/// Baddy character has 0 health, i.e., is dead
const EBADDY_DEAD: u64 = 4;
/// Character is not daead.
const ENOT_DEAD: u64 = 5;

public fun fight(hero: &Object<Character>, baddy: &Object<Character>) {
  assert!(character::is_hero(hero), error::invalid_argument(ENOT_HERO));
  assert!(character::is_baddy(baddy), error::invalid_argument(ENOT_BADDY));
  assert!(character::health(hero) > 0, error::invalid_argument(EHERO_DEAD));
  assert!(character::health(baddy) > 0, error::invalid_argument(EBADDY_DEAD));

  attack(hero, baddy);
  if (character::health(baddy) > 0) {
    attack(baddy, hero);
    if (character::health(hero) == 0) {
      level_up::level_up(baddy, hero);
    }
  } else {
    level_up::level_up(hero, baddy);
  }
}

fun attack(left: &Object<Character>, right: &Object<Character>) {
  let left_str = character::effective_strength(left);
  let right_def = character::defense(right);

  // Avoid the potential overflow and set the minimum damage to 1.
  let damage = if (left_str > right_def) {
    left_str - right_def
  } else {
    1
  };
  let damage = (damage as u32);

  let right_health = character::health(right);
  let new_health = if (right_health > damage) {
    right_health - damage
  } else {
    0
  };
  character::set_health(right, new_health);
}

public fun sacrifice(character: Object<Character>) {
  character::destroy(character);
}

public fun eliminate(character: Object<Character>) {
  assert!(character::health(&character) == 0, error::invalid_argument(ENOT_DEAD));
  character::destroy(character);
}
}

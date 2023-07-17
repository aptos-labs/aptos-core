module fight_the_baddies::level_up {
use std::error;
use aptos_framework::object::Object;
use fight_the_baddies::character::{Self, Character};

friend fight_the_baddies::fight;

/// Losing character is not dead.
const ENOT_DEAD: u64 = 1;
/// Winning character is dead.
const EDEAD: u64 = 2;

const BASE_EXP: u64 = 25;
const MAX_LEVEL: u64 = 255;
const MAX_EXP: u64 = 25500;

/// Provide experience and level increase. The algorith is that each level requires
/// 100 experience points. Each victory for the same level results in 25 experience
/// points. That amount is shifted left for each level greater and right for each
/// level less than.
public(friend) fun level_up(winner: &Object<Character>, loser: &Object<Character>) {
  assert!(character::health(winner) > 0, error::invalid_argument(EDEAD));
  assert!(character::health(loser) == 0, error::invalid_argument(ENOT_DEAD));

  let winner_level = character::level(winner);
  let loser_level = character::level(loser);
  let win_exp = if (winner_level > loser_level) {
    BASE_EXP >> (winner_level - loser_level)
  } else {
    BASE_EXP << (loser_level - winner_level)
  };

  let current_exp = character::experience(winner);
  let new_exp = if (MAX_EXP < current_exp + win_exp) {
    MAX_EXP
  } else {
    current_exp + win_exp
  };
  character::set_experience(winner, new_exp);

  let current_level = (character::level(winner) as u64);
  let next_level = new_exp / 100;
  if (current_level < MAX_LEVEL && current_level < next_level) {
    let next_level = if (MAX_LEVEL < next_level) {
      MAX_LEVEL
    } else {
      next_level
    };
    character::set_level(winner, (next_level as u8));
  };
}
}

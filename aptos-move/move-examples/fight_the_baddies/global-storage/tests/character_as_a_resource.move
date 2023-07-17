#[test_only]
module fight_the_baddies::character_as_a_resource {
use std::string;

use fight_the_baddies::character;
use fight_the_baddies::fight;
use fight_the_baddies::test_utils;
use fight_the_baddies::weapon;

#[test(owner = @0xa11ce)]
fun store_and_load(owner: &signer) {
  let hero = test_utils::get_hero();
  character::equip_weapon(&mut hero, weapon::generate_sword(string::utf8(b"katana")));
  let experience = character::experience(&hero);
  assert!(experience == 0, 1);

  let baddy = test_utils::get_baddy();
  fight::fight(&mut hero, &mut baddy);
  fight::eliminate(baddy);
    
  let experience = character::experience(&hero);
  assert!(experience > 0, 1);

  character::store(owner, hero);
  let resumed_hero = character::load(owner);
  let resumed_experience = character::experience(&resumed_hero);
  assert!(resumed_experience == experience, 2);
  assert!(character::strength(&resumed_hero) < character::effective_strength(&resumed_hero), 3);

  character::store(owner, resumed_hero);
}

#[test(owner = @0xa11ce)]
#[expected_failure(abort_code = 0x80001, location = fight_the_baddies::character)]
fun store_two(owner: &signer) {
  let hero = test_utils::get_hero();
  character::store(owner, hero);
  let hero = test_utils::get_hero();
  character::store(owner, hero);
}

#[test(owner = @0xa11ce)]
#[expected_failure(abort_code = 0x60002, location = fight_the_baddies::character)]
fun load_none(owner: &signer) {
  let char = character::load(owner);
  fight::sacrifice(char);
}
}

#[test_only]
module fight_the_baddies::character_as_a_resource {
use std::string;
use fight_the_baddies::character;
use fight_the_baddies::fight;
use fight_the_baddies::test_utils;

#[test(owner = @0xa11ce)]
fun store_and_load(owner: &signer) {
  let hero = test_utils::get_hero();
  let experience = character::experience(&hero);
  assert!(experience == 0, 1);

  let baddy = test_utils::get_baddy();
  while (character::health(&baddy) > 0) {
    fight::fight(&mut hero, &mut baddy);
  };

  let experience = character::experience(&hero);
  assert!(experience > 0, 1);

  let hero_name = character::name(&hero);
  character::store(owner, hero);
  let baddy_name = character::name(&baddy);
  character::store(owner, baddy);

  let resumed_hero = character::load(owner, hero_name);
  let resumed_experience = character::experience(&resumed_hero);
  assert!(resumed_experience == experience, 2);
  character::store(owner, resumed_hero);

  let resumed_baddy = character::load(owner, baddy_name);
  let resumed_health = character::health(&resumed_baddy);
  assert!(resumed_health == 0, 3);
  character::store(owner, resumed_baddy);
}

#[test(owner = @0xa11ce)]
#[expected_failure(abort_code = 0x10001, location = fight_the_baddies::character)]
fun store_same_name(owner: &signer) {
  let hero = test_utils::get_hero();
  character::store(owner, hero);
  let hero = test_utils::get_hero();
  character::store(owner, hero);
}

#[test(owner = @0xa11ce)]
#[expected_failure(abort_code = 0x60002, location = fight_the_baddies::character)]
fun load_none(owner: &signer) {
  let char = character::load(owner, string::utf8(b"alice"));
  fight::sacrifice(char);
}

#[test(owner = @0xa11ce)]
#[expected_failure(abort_code = 0x60003, location = fight_the_baddies::character)]
fun load_missing(owner: &signer) {
  let hero = test_utils::get_hero();
  character::store(owner, hero);
  let char = character::load(owner, string::utf8(b"fake_alice"));
  fight::sacrifice(char);
}
}

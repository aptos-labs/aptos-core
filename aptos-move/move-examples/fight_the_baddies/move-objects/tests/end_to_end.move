#[test_only]
module fight_the_baddies::end_to_end {
use std::signer;
use std::string;

use aptos_framework::account;

use fight_the_baddies::character;
use fight_the_baddies::fight;
use fight_the_baddies::test_utils;
use fight_the_baddies::weapon;

#[test(owner = @0xa11ce)]
fun generate_and_destroy_hero(owner: &signer) {
  account::create_account_for_test(signer::address_of(owner));
  let hero = test_utils::get_hero(owner);
  assert!(character::is_hero(&hero), 0);
  assert!(character::name(&hero) == string::utf8(b"Alice"), 1);
  assert!(character::gender(&hero) == string::utf8(b"female"), 2);
  let weapon = weapon::create_axe(owner, string::utf8(b"hatchet"));
  character::equip_weapon(owner, hero, weapon);
  assert!(character::strength(&hero) < character::effective_strength(&hero), 3);
  fight::sacrifice(hero);
}

#[test(owner = @0xa11ce)]
fun generate_and_destroy_baddy(owner: &signer) {
  account::create_account_for_test(signer::address_of(owner));
  let baddy = test_utils::get_baddy(owner);
  assert!(character::is_baddy(&baddy), 0);
  assert!(character::name(&baddy) == string::utf8(b"Bob"), 1);
  assert!(character::gender(&baddy) == string::utf8(b"male"), 2);
  fight::sacrifice(baddy);
}

#[test(owner = @0xa11ce)]
/// The goal with this test is to get level ups, so we'll create a hero who always kill steals
/// without taking damage. Then we'll create new heroes along the way to beat the baddy to near
/// death.
fun end_to_end(owner: &signer) {
  account::create_account_for_test(signer::address_of(owner));
  let hero = test_utils::get_hero(owner);
  let weapon = weapon::create_sword(owner, string::utf8(b"katana"));
  character::equip_weapon(owner, hero, weapon);

  let current_level = character::level(&hero);
  while (current_level == character::level(&hero)) {
    let baddy = test_utils::get_baddy(owner);
    fight::fight(&hero, &baddy);
    fight::eliminate(baddy);
  };
  // We should only get one level up given the current mechanics
  assert!(current_level + 1 == character::level(&hero), 0);
  fight::sacrifice(hero);
  // The test will timeout if it cannot get here.
}
}

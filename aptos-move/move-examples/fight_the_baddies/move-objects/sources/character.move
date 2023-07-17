module fight_the_baddies::character {
use std::error;
use std::option::{Self, Option};
use std::string::{Self, String};

use aptos_framework::object::{Self, DeleteRef, Object};

use fight_the_baddies::weapon::{Self, Weapon};

friend fight_the_baddies::level_up;
friend fight_the_baddies::fight;

/// There was no character found at the specified address.
const ECHARACTER_NOT_FOUND: u64 = 1;

#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct Character has key {
  name: String,
  gender: String,
  affinity: String,
  level: u8,
  health: u32,
  defense: u16,
  strength: u16,
  agility: u16,
  experience: u64,
  weapon: Option<Object<Weapon>>,
  delete_ref: DeleteRef,
}

public fun create_hero(creator: &signer, name: String, gender: String): Object<Character> {
  let constructor_ref = object::create_object_from_account(creator);
  let obj_signer = object::generate_signer(&constructor_ref);
  let delete_ref = object::generate_delete_ref(&constructor_ref);

  let character = Character {
    name,
    gender,
    affinity: string::utf8(b"good"),
    level: 1,
    health: 32,
    defense: 5,
    strength: 3,
    agility: 8,
    experience: 0,
    weapon: option::none(),
    delete_ref,
  };
	move_to(&obj_signer, character);
	object::object_from_constructor_ref(&constructor_ref)
}

public fun create_baddy(creator: &signer, name: String, gender: String): Object<Character> {
  let constructor_ref = object::create_object_from_account(creator);
  let obj_signer = object::generate_signer(&constructor_ref);
  let delete_ref = object::generate_delete_ref(&constructor_ref);

  let character = Character {
    name,
    gender,
    affinity: string::utf8(b"bad"),
    level: 1,
    health: 8,
    defense: 2,
    strength: 1,
    agility: 3,
    experience: 0,
    weapon: option::none(),
    delete_ref,
  };
	move_to(&obj_signer, character);
	object::object_from_constructor_ref(&constructor_ref)
}

inline fun assert_exists<T: key>(character: &Object<T>): address {
	let character_address = object::object_address(character);
	assert!(
		exists<Character>(character_address),
		error::not_found(ECHARACTER_NOT_FOUND),
	);
  character_address
}

inline fun borrow<T: key>(character: &Object<T>): &Character acquires Character {
	let character_address = assert_exists(character);
	borrow_global<Character>(character_address)
}

inline fun borrow_mut<T: key>(character: &Object<T>): &mut Character acquires Character {
	let character_address = assert_exists(character);
	borrow_global_mut<Character>(character_address)
}

public fun is_hero<T: key>(character: &Object<T>): bool acquires Character {
  borrow(character).affinity == string::utf8(b"good")
}

public fun is_baddy<T: key>(character: &Object<T>): bool acquires Character {
  borrow(character).affinity == string::utf8(b"bad")
}

public fun name<T: key>(character: &Object<T>): String acquires Character {
  borrow(character).name
}

public fun gender<T: key>(character: &Object<T>): String acquires Character {
  borrow(character).gender
}

public fun level<T: key>(character: &Object<T>): u8 acquires Character {
  borrow(character).level
}

public fun health<T: key>(character: &Object<T>): u32 acquires Character {
  borrow(character).health
}

public fun defense<T: key>(character: &Object<T>): u16 acquires Character {
  borrow(character).defense
}

public fun strength<T: key>(character: &Object<T>): u16 acquires Character {
  borrow(character).strength
}

public fun agility<T: key>(character: &Object<T>): u16 acquires Character {
  borrow(character).agility
}

public fun effective_strength<T: key>(character: &Object<T>): u16 acquires Character {
  let character = borrow(character);
  let weapon = if (option::is_some(&character.weapon)) {
    weapon::strength(option::borrow(&character.weapon))
  } else {
    0
  };
  weapon + character.strength
}

public fun effective_agility<T: key>(character: &Object<T>): u16 acquires Character {
  let character = borrow(character);
  let weapon = if (option::is_some(&character.weapon)) {
    weapon::weight(option::borrow(&character.weapon))
  } else {
    0
  };
  if (character.agility < weapon) {
    0
  } else {
    character.agility - weapon
  }
}

public fun experience<T: key>(character: &Object<T>): u64 acquires Character {
  borrow(character).experience
}

public(friend) fun set_level<T: key>(character: &Object<T>, level: u8) acquires Character {
  borrow_mut(character).level = level
}

public(friend) fun set_health<T: key>(character: &Object<T>, health: u32) acquires Character {
  borrow_mut(character).health = health
}

public(friend) fun set_defense<T: key>(character: &Object<T>, defense: u16) acquires Character {
  borrow_mut(character).defense = defense
}

public(friend) fun set_strength<T: key>(character: &Object<T>, strength: u16) acquires Character {
  borrow_mut(character).strength = strength
}

public(friend) fun set_agility<T: key>(character: &Object<T>, agility: u16) acquires Character {
  borrow_mut(character).agility = agility
}

public(friend) fun set_experience<T: key>(character: &Object<T>, experience: u64) acquires Character {
  borrow_mut(character).experience = experience
}

public fun equip_weapon(
  owner: &signer,
  character: Object<Character>,
  weapon: Object<Weapon>,
) acquires Character {
  object::transfer_to_object(owner, character, weapon);
  let character = borrow_mut(&character);
  if (option::is_some(&character.weapon)) {
    let _old_weapon = option::extract(&mut character.weapon);
  };
  option::fill(&mut character.weapon, weapon);
}

public(friend) fun destroy<T: key>(character: Object<T>) acquires Character {
  let character_address = assert_exists(&character);
  let Character {
    name: _,
    gender: _,
    affinity: _,
    level: _,
    health: _,
    defense: _,
    strength: _,
    agility: _,
    experience: _,
    weapon,
    delete_ref,
  } = move_from<Character>(character_address);

  if (option::is_some(&weapon)) {
    let weapon_obj = option::destroy_some(weapon);
    weapon::destroy(weapon_obj);
  };
  object::delete(delete_ref);
}
}

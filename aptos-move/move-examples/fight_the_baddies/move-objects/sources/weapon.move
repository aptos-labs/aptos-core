module fight_the_baddies::weapon {
use std::error;
use std::string::{Self, String};

use aptos_framework::object::{Self, DeleteRef, Object};

friend fight_the_baddies::character;

/// The was no weapon found at the specified address.
const EWEAPON_NOT_FOUND: u64 = 1;

#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct Weapon has key {
  name: String,
  type: String,
  strength: u16,
  weight: u16,
  delete_ref: DeleteRef,
}

inline fun create_weapon(
  creator: &signer,
  name: String,
  type: String,
  strength: u16,
  weight: u16,
): Object<Weapon> {
  let constructor_ref = object::create_object_from_account(creator);
  let delete_ref = object::generate_delete_ref(&constructor_ref);
  let weapon = Weapon { name, type, strength, weight, delete_ref };
  let obj_signer = object::generate_signer(&constructor_ref);
  move_to(&obj_signer, weapon);
  object::object_from_constructor_ref(&constructor_ref)
}

public fun create_knife(creator: &signer, name: String): Object<Weapon> {
  create_weapon(creator, name, string::utf8(b"knife"), 2, 1)
}

public fun create_sword(creator: &signer, name: String): Object<Weapon> {
  create_weapon(creator, name, string::utf8(b"sword"), 10, 4)
}

public fun create_axe(creator: &signer, name: String): Object<Weapon> {
  create_weapon(creator, name, string::utf8(b"axe"), 17, 6)
}

public(friend) fun destroy<T: key>(weapon: Object<T>) acquires Weapon {
  let weapon_address = assert_exists(&weapon);
  let Weapon {
    name: _,
    type: _,
    strength: _,
    weight: _,
    delete_ref,
  } = move_from<Weapon>(weapon_address);
  object::delete(delete_ref);
}

inline fun assert_exists<T:key>(weapon: &Object<T>): address {
  let weapon_address = object::object_address(weapon);
  assert!(
    exists<Weapon>(weapon_address),
    error::not_found(EWEAPON_NOT_FOUND),
  );
  weapon_address
}

inline fun borrow<T: key>(weapon: &Object<T>): &Weapon acquires Weapon {
  let weapon_address = assert_exists(weapon);
  borrow_global<Weapon>(weapon_address)
}

public fun name<T: key>(weapon: &Object<T>): String acquires Weapon {
  borrow(weapon).name
}

public fun type<T: key>(weapon: &Object<T>): String acquires Weapon {
  borrow(weapon).type
}

public fun strength<T: key>(weapon: &Object<T>): u16 acquires Weapon {
  borrow(weapon).strength
}

public fun weight<T: key>(weapon: &Object<T>): u16 acquires Weapon {
  borrow(weapon).weight
}
}

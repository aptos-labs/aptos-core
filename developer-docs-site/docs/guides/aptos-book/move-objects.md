---
title: "Move Objects"
---

# Move Objects

The previous section introduced global storage and two mechanisms for placing structs into storage: `key` and `store`. While both approaches work and are intuitive, they leave a lot of components up to the developer to implement. Aptos introduces Move Objects to extend upon the basics of the Move model and offer a framework for creating and managing a globally accessible set of heterogeneous resources that expedites and improves the developer experience.

Why objects? Well minimally an object provide many features including:

- A common framework that defines ownership of an asset. No need to have one-off asset management as was demonstrated for hero in the previous chapter, nor come up with mechanisms to differentiate between similar yet different assets, e.g.., those that share the same common name.
- Global addressability of all objects that only requires virtual or referenced nesting of assets. Weapons were only accessible if you knew which hero had them and further nesting of assets make it much more complex to manage both in terms of code and data model.
- Events or metadata that make it easier to reason about changes to the object. An area not yet touched upon, Move Objects not only represent an asset but can express events as actions are taken involving the objects, making it easier for off-chain understanding of complex on-chain interactions.
- A permission framework that makes data convenient and safe to use. Limiting the possibility that gaining access to a `signer`s could be abused to manipulate or affect data.

We’ll go into each one of these in greater depth and further emphasize the utility of Objects over resources alone.

## Characters as Objects

For the most part, swapping from resources to objects is largely a set of API changes, but doing so unlocks a lot of opportunities.

```rust
/// Path: fight_the_baddies/move-objects/sources/character.move
module fight_the_baddies::character {
use std::error;
use std::option::{Self, Option};
use std::string::{Self, String};

use aptos_framework::object::{Self, Object};

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
}

inline fun create_character(creator: &signer, character: Character): Object<Character> {
  let constructor_ref = object::create_object_from_account(creator);
  let obj_signer = object::generate_signer(&constructor_ref);
  move_to(&obj_signer, character);
  object::object_from_constructor_ref(&constructor_ref)
}

public fun create_hero(creator: &signer, name: String, gender: String): Object<Character> {
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
  };
  create_character(creator, character)
}

public fun create_baddy(creator: &signer, name: String, gender: String): Object<Character> {
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
  };
  create_character(creator, character)
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

public fun experience<T: key>(character: &Object<T>): u64 acquires Character {
  borrow(character).experience
}

public(friend) fun set_experience<T: key>(character: &Object<T>, experience: u64) acquires Character {
  borrow_mut(character).experience = experience
}
}
```

Objects are a collection of resources stored within global storage at a single address, hence an object can effectively be represented by an address. The syntax for representing Objects in Move is `Object<T>`, where the `T` denotes a resource within the Object. The only contents of `Object<T>` is the address where the Object is stored. The terms object id and address are typically used interchangeably, but as an address can point to data other than objects, the preferred terminology is object id or object identifier.

As objects are not ephemeral, their identifiers need not be stored but can be generated dynamically so long as an object actually exists at the underlying address by calling [`object::address_to_object<T>`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_address_to_object), where `T` represents a specific resource within the object set. The [`ObjectCore`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_ObjectCore) resource is the guaranteed to be stored within an object.

### Creating Objects

Objects can be created in a handful of ways:
* The context of a transaction, namely, the sender's state via a call to [`object::create_object_from_account`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_create_object_from_account)
* From an existing object, [`object::create_object_from_object`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_create_object_from_object)
* Via a deterministic seed or string via [`object::create_named_object`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_create_named_object).

Note, the first two of these generate a random object id. Outside the execution within Move, that is from the transactoin layer, the only way to infer the actual object id is by parsing the output of a transaction.

### Object Ownership

At the time of creation, objects are owned by the entity that created it. This can be verified with the function [`aptos_framework::object::is_owner`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_is_owner) and the owner can be retrieved by [`aptos_framework::object::owner`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_owner):
```
let constructor_ref = object::create_object_from_account(creator);
let object = object::object_from_constructor_ref(&constructor_ref)
assert!(object::is_owner(object, signer::address_of(creator)), 0);
assert!(signer::address_of(creator) == object::owner(object), 1);
```

Behind the scenes, ownership is managed by the `owner` field on [`ObjectCore`](http://aptos.dev/reference/move?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_ObjectCore). The `owner` field is an address of another entity on-chain that owns this resource. As we will discuss in the next section, ownership only inherently gifts the ability to transfer the object, but it can also be used by higher-level applications to dictate other behaviors. It is also important to recognize that the `owner` field does not dictate any expectations or requirements beyond it being a valid address, this inherently makes feasible for objects to own objects, inherent composability. It also implies that undefined global can "own" objects too.

### Resource Groups

The attribute syntax `#[resource_group_member(group = aptos_framework::object::ObjectGroup)]` designates a resource as a member of a resource group. The `resource_group_member` attribute supports a single parameter, `group`, that indicates the resource group membership of this resource. From a pure Move perspective, resource groups have no meaning. Resource groups indicate to the Aptos VM storage layer that all resources within a group share a common storage slot. In Aptos, this can provide valuable performance implications. Within Aptos, each storage slot is represented by a node in a Merkle tree, therefore reads and writes are `logN` in terms of computation and `32 logN` bytes. Hence, Aptos allows for advanced developer the ability to define how certain resources are stored to optimize for allocated space and read/write performance. Each additional slot has a modest cost on storage prices; however, storing more than 1 KB within a slot can have an impact on gas prices. In general, the gas prices on Aptos are low, therefore the current recommendation is to fault to fewer slots when the expected size of storage is unknown.

This code only extends an existing resource group. That resource group is the [`aptos_framework::object::ObjectGroup`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_ObjectGroup), which is defined in the module `aptos_framework::object`:
```
#[resource_group(scope = global)]
/// A shared resource group for storing object resources together in storage.
struct ObjectGroup {}
```

The attribute `resource_group` supports a single parameter `scope`. `scope` indicates which modules and the resources therein can be members of this group. There are three options: 
* `global` -- can be any module at any address within the system
* `address` -- any module within the same address as the resource group
* `module_` -- only other resources within the same module

### Inline Functions

Unlike the resource or stored value approaches in Move, which pass around structs to various functions, objects involve passing around object ids or `Object<T>`. As a result, the entire interface for accessing objects tends to be load resource within an object from storage and then perform an operation. Inline functions allow a developer to write the resource loading and verification code once and then reuse it by function alone. This is very powerful, because Move cannot directly return references of global storage from a function, but it can return it from an inline function. That is because at compile time, the Move compiler actually copies the inline functions definition directly into the code of the caller.

So in effect, when we see this:
```
borrow(character).experience
```

The compiler converts the code into:
```
let character_address = object::object_address(character);
assert!(
  exists<Character>(character_address),
  error::not_found(ECHARACTER_NOT_FOUND),
);

borrow_global<Character>(character_address).experience
```

## Weapon Objects

With weapons, we can begin to see the inherent composability offered by Move Objects that let one object own and indirectly contain another, as the `Character` effectively stores a `Weapon`:

```rust
/// Path: fight_the_baddies/move-objects/sources/weapon.move
module fight_the_baddies::weapon {
use std::error;
use std::string::{Self, String};

use aptos_framework::object::{Self, Object};

friend fight_the_baddies::character;

/// The was no weapon found at the specified address.
const EWEAPON_NOT_FOUND: u64 = 1;

#[resource_group_member(group = aptos_framework::object::ObjectGroup)]
struct Weapon has key {
  name: String,
  type: String,
  strength: u16,
  weight: u16,
}

inline fun create_weapon(creator: &signer, weapon: Weapon): Object<Weapon> {
  let constructor_ref = object::create_object_from_account(creator);
  let obj_signer = object::generate_signer(&constructor_ref);
  move_to(&obj_signer, weapon);
  object::object_from_constructor_ref(&constructor_ref)
}

public fun create_knife(creator: &signer, name: String): Object<Weapon> {
  let weapon = Weapon {
    name,
    type: string::utf8(b"knife"),
    strength: 2,
    weight: 1,
  };
  create_weapon(creator, weapon)
}

...

inline fun borrow<T: key>(weapon: &Object<T>): &Weapon acquires Weapon {
  let weapon_address = object::object_address(weapon);
  assert!(
    exists<Weapon>(weapon_address),
    error::not_found(EWEAPON_NOT_FOUND),
  );
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
```

To support weapons on the character, we extend it as thus:

```rust
/// Path: fight_the_baddies/move-objects/sources/character.move
module fight_the_baddies::character {
...

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
}

inline fun create_character(creator: &signer, character: Character): Object<Character> {
  let constructor_ref = object::create_object_from_account(creator);
  let obj_signer = object::generate_signer(&constructor_ref);
  move_to(&obj_signer, character);
  object::object_from_constructor_ref(&constructor_ref)
}

public fun create_hero(creator: &signer, name: String, gender: String): Object<Character> {
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
  };
  create_character(creator, character)
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
}
```

### Transferring Objects

The tail end of the character code snippet contains some code for equipping weapons, which equates to transferring owner of an object from the user to the character. The only critical realization here is that these weapons are actually objects themselves. If this leads to confusion, read the next section; however the conversion of weapons into objects is largely the same as characters into objects.

Each object allows the owner to seamlessly transfer ownership, uniterally, to another address even with many different operations:
* [`object::transfer_to_object`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_transfer_to_object) -- that allows an owner to transfer the object to another object creating a composable object.
* [`object::transfer`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_transfer) -- that allows the owner to transfer to another address, even if that address has no state.
* [`object::transfer_call`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/object.md#0x1_object_transfer_call) -- that allows a transaction to directly transfer the object from its owner to an address.

There are a lot more nuances to the transferring of objects, but we will explore those concepts deeper into the development of our game.

### Distinguishing Ownership and Object IDs

In the above example, the `Character` resource stores `weapon: Option<Object<Weapon>>`. We use this to simplify the lookup of potential weapons held by the character. It is imperative to note that by default storing this content does not implicitly mean that the `Character` actually owns the weapon. It is only by virtue of looking at the `owner` field can we verify this. While an application need not actually apply this level of ownership for equipping items, composability of this fashion makes it easier to reason about logical relationships. It is also important to note that there is no explicit requirement that when ownership is transferred that all remaining `Option<Object<Weapon>>` are updated to `None`. Therefore the on-chain logic must adhere to ensuring these constraints are met.

## Deleting Objects

Recall in the earlier examples, we leveraged our fight code to call `character::destroy` via a friend function when the character was either killed or the game was over. While resources can be removed from global storage using `move_from`, however, the `move_from` semantics if not guarded can be abused by any entity to delete objects arbitrarily. Specifically, if a call to `move_from<ObjectCore>(addr)` is not appropriately guarded, then anything can delete an object. To ensure data sanity, Move Objects introduce a capability called `delete_ref` that asserts that an entity is privileged to delete an object. It is only with this capability that an object can be deleted.

```
/// Path: fight_the_baddies/move-objects/sources/character.move
...

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

...

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
```

```
/// Path: fight_the_baddies/move-objects/sources/weapon.move
...

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
```

Thus in order to perform the delete, the `DeleteRef` must be stored in the object and guarded by logic that makes sure that this operation is safe. The `destroy` function in `character` and `weapon` limit the callers to other trusted modules by virtue of friend. Calling these function calls the `move_from` for the local resource but also cleans up the core Move Object resource, `ObjectCore`, but `character::destroy` enters `weapon::destroy` if the character holds a weapon, ensuring that upon conclusion all game data is deleted.

## Application Implications

The end-to-end test gives us a glimpse into the development experience for using objects within an application. So long as the developer builds an appropriate interface on their module, the actual application logic largely resembles that when using resources directly.

```
/// Path: fight_the_baddies/move-objects/tests/end_to_end.move
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
```

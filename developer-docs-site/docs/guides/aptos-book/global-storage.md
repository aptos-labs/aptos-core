---
title: "Global Storage"
---

# Global Storage

Until this point, the longevity of the game and characters is the execution of a single function, we have yet to introduce persistent or global storage. We continue our exploration of core Move concepts and refrain from introducing Aptos-specific concepts until the next section. By the end of this section, you will have written a complete and useful on-chain game!

Persistent data is stored in global storage. Global storage is represented as the following:

```rust
Resources: Map<address, Map<name, vec<u8>>;
Modules: Map<address, Map<module_name, vec<u8>>;

Where `module_name` is the module name and `name` is the struct name.
```

Applications cannot directly read or write to module storage and resource storage requires the use of certain functions and other annotations on the struct, as we’ll cover.

## Making Our Characters Persistent

This is our first option to storing characters by making them resources within storage. A resource is a top level struct in the global storage. Returning back to the earlier code, we make the following modifications:

```rust
/// Path: fight_the_baddies/sources/character.move
module fight_the_baddies::character {
use std::error;
use std::option::{Self, Option};
use std::signer;
use std::string::{Self, String};

use fight_the_baddies::weapon::{Self, Weapon};

friend fight_the_baddies::level_up;
friend fight_the_baddies::fight;

/// Only a single character resource can be stored at a given address.
const ECANNOT_STORE_MULTIPLE_CHARACTERS: u64 = 1;
/// There is no character resource stored at this address.
const ENO_CHARACTER_STORED: u64 = 2;

/// Key enables this to be stored to persistent storage
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
  weapon: Option<Weapon>,
}

public function store(owner: &signer, character: Character) {
  assert!(
    !exists<Character>(signer::address_of(owner)),
    error::already_exists(ECANNOT_STORE_MULTIPLE_CHARACTERS),
  );
  move_to(owner, character);
}

/// Protected load function, only the owner can load the character
public function load(owner: &signer): Character acquires Character {
  let owner_addr = signer::address_of(owner);
  assert!(
    exists<Character>(owner_addr),
    error::not_found(ENO_CHARACTER_STORED),
  );
  move_from(owner_addr)
}

...
}
```

### Signers and Basic Account Concepts

The above code introduces the concept of a `signer`. The `signer` represents ownership of an address from the perspective of global storage. As we begin to explore how Move binds to real applications, we’ll see that the `signer` is the entity that submitted a transaction. Recall in [Blockchain Basics](blockchain-basics.md#a-simple-transaction), Alice submitted a transaction to the blockchain to create Bob's account and give him 1,000 `AptosCoin`.

### Storing Global Data

The above snippet introduces the `key` ability for structs. `key` allows a structure to be stored into persistent storage via the `move_to` function and extracted from storage via the `move_from` function. A struct stored in global storage is known as a **resource**. Note that while Move does support the ability to borrow data from storage, a function cannot return a reference to data stored in global storage. As a result, it is not applicable yet. There is also an `exists` function that lets us check if a resource exists at an address.

It is important to note that all these storage functions can only access structs defined within the module. One module cannot directly access another modules global storage.

The biggest limitation to this model is that it can only support a single character per account. This is because storage, by default, has no concept of lists.

### Storing Nested Data

By default, Move would not allow `Weapon` to be stored with the `Character`. To enable that, we added the `store` ability to weapon:

```rust
struct Weapon has store {
  name: String,
  type: String,
  strength: u16,
  weight: u16,
}
```

This minor change allows additional structs to be stored into global storage as part of a resource but not as a resource itself. We’ll go over this again in more detail in the next section.

### Testing

```rust
/// Path: fight_the_baddies/tests/character_as_a_resource.move
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
```

**Signers in Test Cases**

As signers are so critical to the global storage mode, we need a means to generate them in test cases. Move offers this functionality via parameters in the `test` attribute. Specifically, `name = @address` will ensure that the parameter in the function with `name` will get a signer for the address defined at `@address`. Note, Move uses the `@` character to represent address values.

**Verifying Global Storage**

`store_and_load` demonstrates an important concept in Move, that is resource conservation. Because the `Character` struct does not have the `drop` ability, we must either explicitly destroy the character or alternatively we can place it into storage, as done on the last line `character::store`.

**Testing Negative Cases**

In testing, it is often critical to evaluate that errors are properly handled. We added `store_two` and `load_none` to demonstrate thendis. Specifically these functions have the `expected_failure` attribute, the `abort_code` , and the module or `location` where the `abort_code` was triggered. This means the test will only pass if the function aborts with the `abort_code` emitted from the `location` specified in the test attributes.

## Storing Multiple Characters

In order to store multiple characters, we need to make a couple of changes around the way in which the character is stored. That is, it is no longer a resource but instead must be stored within a resource that enables storing multiples. For that, we will employ the Move standard library’s vector.

```rust
/// Path fight_the_baddies/sources/character.move
module fight_the_baddies::character {
use std::error;
use std::signer;
use std::string::{Self, String};
use std::vector;

friend fight_the_baddies::level_up;
friend fight_the_baddies::fight;

/// The character store already has a charcter with that name.
const ENAME_AREADY_IN_USE: u64 = 1;
/// There is no character store at the specified address.
const ENO_CHARACTERS_STORED: u64 = 2;
/// There was no character by the given name found at the specified address.
const ECHARACTER_NOT_FOUND: u64 = 3;

struct Character has store {
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

struct CharacterStore has key {
  characters: vector<Character>,
}

public fun store(owner: &signer, character: Character) acquires CharacterStore {
  let owner_addr = signer::address_of(owner);
  if (!exists<CharacterStore>(owner_addr)) {
    move_to(owner, CharacterStore { characters: vector::empty() });
  };

  let new_name = name(&character);
  let character_store = borrow_global_mut<CharacterStore>(owner_addr);

  let idx = 0;
  while (idx < vector::length(&character_store.characters)) {
    let existing_character = vector::borrow(&character_store.characters, idx);
    assert!(
      new_name != name(existing_character),
      error::invalid_argument(ENAME_AREADY_IN_USE),
    );
    idx = idx + 1;
  };

  vector::push_back(&mut character_store.characters, character);
}

/// Protected load function, only the owner can load the character
public fun load(owner: &signer, lookup_name: String): Character acquires CharacterStore {
  let owner_addr = signer::address_of(owner);
  assert!(
    exists<CharacterStore>(owner_addr),
    error::not_found(ENO_CHARACTERS_STORED),
  );

  let character_store = borrow_global_mut<CharacterStore>(owner_addr);

  let idx = 0;
  while (idx < vector::length(&character_store.characters)) {
    let character = vector::borrow(&character_store.characters, idx);
    if (name(character) == lookup_name) {
      return vector::swap_remove(&mut character_store.characters, idx)
    };
    idx = idx + 1;
  };

  abort(error::not_found(ECHARACTER_NOT_FOUND))
}
```

### Storing Multiple Structs of the Same Type

The attribute `store` let’s you store the struct within a nested struct that eventually is part of global storage. This layer of indirection allows developers a lot of flexibility in where data is stored. In the above example, the `CharacterStore` becomes the value in global storage. It contains a `vector<Character>` . The `vector` let’s us place many characters into global storage without creating the conflicts of shared space that limited the `Character` as a resource.

In the above example, each character is expected to have a unique name within the `CharacterStore` of a user. While there are other methods for creating uniqueness, such as giving each character a unique numeric id, this was chosen for brevity as our destination method resolves this constraint.

### Testing

The testing code this time introduces no new concepts and is presented below for completeness:

```rust
/// Path: fight_the_baddies/tests/character_as_a_resource.move
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
```

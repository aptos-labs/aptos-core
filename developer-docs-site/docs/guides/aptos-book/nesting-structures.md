---
title: "Nesting Structures"
---

# Nesting Structures

The previous section covered an introduction to the various facilities in basic move development. The majority of developers need to store data within data, as mentioned earlier, `String` is an example of this. In order to add personalization to our game, we would like to customize characters with new weapons and equipment. In Move this involves exploring the concepts of nested structs and optional values. It also begins our path into more subtle complexities of developing with basic Move that amplifies our direction in Aptos.

## Defining Weapons

For the most part, adding weapons involves adding a new source `weapon.move` and a few modifications to the other files to address the impact of this new functionality. The core change to our existing game play is that weapons allow a character to inflict additional damage based upon the strength of the weapon. If there is no weapon, there is no increase in strength. We also add in the weight field for potential future use:

```rust
/// Path: fight_the_baddies/nesting-structures/sources/weapon.move

module fight_the_baddies::weapon {
use std::string::{Self, String};

friend fight_the_baddies::character;
struct Weapon {
  name: String,
  type: String,
  strength: u16,
  weight: u16,
}

public fun generate_knife(name: String): Weapon {
  Weapon {
    name,
    type: string::utf8(b"knife"),
    strength: 2,
    weight: 1,
  }
}

public fun generate_sword(name: String): Weapon {
  Weapon {
    name,
    type: string::utf8(b"sword"),
    strength: 10,
    weight: 4,
  }
}

public fun generate_axe(name: String): Weapon {
  Weapon {
    name,
    type: string::utf8(b"axe"),
    strength: 17,
    weight: 6,
  }
}

public fun name(weapon: &Weapon): String {
  weapon.name
}

public fun type(weapon: &Weapon): String {
  weapon.type
}

public fun strength(weapon: &Weapon): u16 {
  weapon.strength
}

public fun weight(weapon: &Weapon): u16 {
  weapon.weight
}

public(friend) fun destroy(weapon: Weapon) {
  let Weapon {
    name: _,
    type: _,
    strength: _,
    weight: _,
  } = weapon;
}
}
```

There’s nothing that new in the above module from previous.

## Equipping Weapons

In this section, we make a few modifications to the character module to take into consideration weapons:

```rust
/// Path: fight_the_baddies/nesting-structures/sources/character.move

module fight_the_baddies::character {
use std::option::{Self, Option};
use std::string::{Self, String};
use fight_the_baddies::weapon::{Self, Weapon};

friend fight_the_baddies::level_up;
friend fight_the_baddies::fight;

struct Character {
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

public fun generate_hero(name: String, gender: String): Character {
  Character {
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
  }
}

public fun generate_baddy(name: String, gender: String): Character {
  Character {
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
  }
}

...

public fun effective_strength(character: &Character): u16 {
  let weapon = if (option::is_some(&character.weapon)) {
    weapon::strength(option::borrow(&character.weapon))
  } else {
    0
  };
  weapon + character.strength
}

public fun effective_agility(character: &Character): u16 {
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

...

public fun equip_weapon(character: &mut Character, weapon: Weapon) {
  if (option::is_some(&character.weapon)) {
    let old_weapon = option::extract(&mut character.weapon);
    weapon::destroy(old_weapon);
  };
  option::fill(&mut character.weapon, weapon);
}

public(friend) fun destroy(character: Character) {
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
  } = character;

  if (option::is_some(&weapon)) {
    weapon::destroy(option::extract(&mut weapon));
  };
  option::destroy_none(weapon);
}
}
```

### Nesting Structs

Move allows structs to be seamlessly nested within each other. This was demonstrated by allowing a `Character` to contain a `Weapon` and has been demonstrated prior by having the various structs contain strings. When a struct is contained within another struct the entirety of that data is stored collectively.

For example, when we create the weapon via `generate_sword`, the weapon is returned to the caller as a value. That means we are passing around the entirety of struct. When the struct is placed into the `Option` within the `weapon` field on `Character`, the operation moves the value or the instance of the struct into that storage location.

This is an example of nesting struct values within each other.

### Introducing Options

There are often times where there is no default value for a struct, especially at time of creation. In the above example, characters may be empty handed, there may be no default weapon. To represent these occurrences, Move provides an `Option` container. This allows the `Character` instance to be created without adding in boiler plate code for an non-existent weapon.

There are some caveats with using an `Option`:

- There are no implicit interactions with an `Option` so values must be explicitly borrowed, checked, extracted.
- `Option` requires negligible more storage (a byte) and slightly more execution costs due to the additional function calls.
- On the storage side, `Option` leverages the same layout as a `Vector`, which may be confusing at first glance, but does make for a rather simplified view point for an empty or non-empty field.

## Implications to Gameplay

The testing harness makes a few adjustments to make use of weapons. Notably, that a character can equip a weapon and that weapon can give the character a higher effective strength. We then demonstrate this by simplifying the `end_to_end` test below:

```rust
/// Path: fight_the_baddies/nesting-structures/sources/end_to_end.move
#[test_only]
module fight_the_baddies::end_to_end {
use std::string;
use fight_the_baddies::character::{Self, Character};
use fight_the_baddies::fight;
use fight_the_baddies::weapon;

#[test]
fun generate_and_destroy_hero() {
  let hero = get_hero();
  assert!(character::is_hero(&hero), 0);
  assert!(character::name(&hero) == string::utf8(b"Alice"), 1);
  assert!(character::gender(&hero) == string::utf8(b"female"), 2);
  assert!(character::strength(&hero) == character::effective_strength(&hero), 3);
  character::equip_weapon(&mut hero, weapon::generate_sword(string::utf8(b"katana")));
  assert!(character::strength(&hero) < character::effective_strength(&hero), 3);
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
/// The goal with this test is to get level ups, in this scenario the sword is over-powering,
/// which enables single blows to fell the baddy.
fun end_to_end() {
  let hero = get_hero();
  character::equip_weapon(&mut hero, weapon::generate_sword(string::utf8(b"katana")));
  let current_level = character::level(&hero);
  while (current_level == character::level(&hero)) {
    let baddy = get_baddy();
    fight::fight(&mut hero, &mut baddy);
    fight::eliminate(baddy);
  };
  // We should only get one level up given the current mechanics
  assert!(current_level + 1 == character::level(&hero), 0);
  fight::sacrifice(hero);
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
```

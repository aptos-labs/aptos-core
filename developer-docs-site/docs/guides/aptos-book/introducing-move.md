---
title: "Introducing Move"
---

# Introducing Move

With environment setup behind us, we begin our efforts towards building *Fight the Baddies*. We begin by understanding how to build applications in Move, the Web3 programming language at the core of Aptos. In contrast to existing Web3 languages, Move optimizes around safety and security that makes it easy to reason about software applications from a security perspective. Aptos builds upon the foundations of Move to support constructs like objects and accounts that can seamlessly be brought into other Move environments to enhance the development of rich, Web3 applications.

To begin, we start by specifying the build configuration followed by the core of the application, the data model. Much like other programming languages, Move’s data model is based upon structures or structs. A struct can contain various primitive types as well as other structs, however, it cannot contain itself, Move does not support recursive data structures. By the end of this section, we will have built a limited demo that leverages Move data structures from core Move.

All files for this chapter can be found in the [Aptos-Core repository](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/fight_the_baddies/introducing-move).

## Packages

A Move application consists of a series of packages where a package consists of one or more Move modules. Each Move module typically corresponds to a single file of Move code. To get our project started, we will build the following layout:

```
fight_the_baddies
├── Move.toml      (toml file describing the package)
├── sources        (directory containing all application code)
├── doc_templates  (documentation template directory)
└── tests          (test code that sits outside the application code)
```

Your `Move.toml` code will look like the following:

```toml
# Path: fight_the_baddies/introducing-move/Move.toml
[package]
name = 'FightTheBaddies'
version = '0.0.1'

[addresses]
fight_the_baddies = "_"

[dependencies.MoveStdlib]
git = 'https://github.com/aptos-labs/aptos-core.git'
rev = 'framework-mainnet'
subdir = 'aptos-move/framework/move-stdlib'
```

The above configuration file or manifest comprises three key components:
1. The package definition itself, which has the name of the package and the version, we chose `0.0.1`.
2. The addresses section allows the developers to set literal or named account addresses, the value `"_"` means undefined and we will need to specify it during operations.
3. Each dependecy must be imported via the configuration.

More details can be found within the [Move.toml section of the Move Book](../../../move/book/packages#movetoml).

## Core Data

The core of our game are the characters. A character has an affinity, evil or good, various attributes, a name, a gender, and a few attributes:

```rust
/// Path: fight_the_baddies/introducing-move/sources/character.move
module fight_the_baddies::character {
use std::string::String;

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
}
}
```

This module defines the named address `fight_the_baddies` with the module `character`. The file name for this should be `character.move`. The struct itself shows the field labels and types including an example of nesting other structs like `String` and several primitives.

The following is a subset of the core Move primitives:

```rust
address - a 256-bit value representing a location a location in global storage
bool - True/False
u8 - 8-bit unsigned integer
u16 - 16-bit unsigned integer
u32 - 32-bit unsigned integer
u64 - 64-bit unsigned integer
u128 - 128-bit unsigned integer
u256 - 256-bit unsigned integer
```

## Accessing Data

Now that we know the basics of data structure layout, we need a means to create the data structures and interact with them. As part of Move’s security primitives, a struct defined in one module cannot be directly created, manipulated, or read in another. Specifically, the following example would fail to compile:

```rust
/// Path: fight_the_baddies/introducing-move/sources/game.move
/// Note: this is not a real file, it is intended as a demonstration
module fight_the_baddies::game {
  use std::string::{Self, String};
  use baddies::character::Character;

  public fun generate_character(name: String): Character {
    Character {
      name,
      gender: string::utf8(b"female"),
      affinity: string::utf8(b"good"),
      level: 1,
      health: 32,
      defense: 5,
      strength: 3,
      agility: 8,
      experience: 0,
    }
  }
}
```

Instead, all functions that directly create and manipulate must be defined in the original module, this allows the module to define what functionality is exported for other modules developers, and users of the application. For example, it prevents anyone outside the game logic itself from creating a super hero in *Fight the Baddies*.

```rust
/// Path: fight_the_baddies/introducing-move/sources/character.move
module baddies::character {
use std::string::{Self, String};
...

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
  }
}

public fun is_hero(character: &Character): bool {
  character.affinity == string::utf8(b"good")
}

public fun is_baddy(character: &Character): bool {
  character.affinity == string::utf8(b"bad")
}

public fun name(character: &Character): String {
  character.name
}

public fun gender(character: &Character): String {
  character.gender
}

public fun level(character: &Character): u8 {
  character.level
}

public fun health(character: &Character): u32 {
  character.health
}

public fun defense(character: &Character): u16 {
  character.defense
}

public fun strength(character: &Character): u16 {
  character.strength
}

public fun agility(character: &Character): u16 {
  character.agility
}

public fun experience(character: &Character): u64 {
  character.experience
}
}
```

Note, this guide represents earlier snippets of code with `...` to keep repeated code to a minimum.

There are several aspects of Move that stand out in the above code:

### Importing Modules

We can import a module or structs and functions within it. `use address::module` imports only the module, whereas `use address::module::{Self, Struct, func}` would import the module, the struct `Struct` and the function `func`.

An import can be directly referenced by the top-level name imported.

If we did `use std::string`, we would have to define string types as `string::String`, whereas `use std::string::String` allows us to define types as `String`.

### Building and Using Structs

When creating a struct, if a variable shares the name of a field, for example, `name: name` can be shortened to `name`.

Structure fields are accessed via dot notation, e.g., `character.agility` returns the agility field of the given character.

### Calling Functions

Functions must be defined `public` to be accessed outside the module. A non-public module is inaccessible from outside that module.

When we call the public functions, we pass in a reference `&Character`, this means we’re literally passing around the character’s data back into the module to read fields.

### Struct Fields and the Copy Ability

The fields are returned as values. Certain values can be seamlessly copied, for example:

```rust
struct Copyable has copy {
  value: u8,
}

struct NotCopyable {
  value: u8,
}

public fun copy_copyable(copyable: &Copyable): Copyable {
  *copyable
}

public fun copy_non_copyable(non_copyable: &NonCopyable): NonCopyable {
  NotCopyable {
    value: non_copyable.value,
  }
}
```

`Copyable` can be copied in any module and in any code without reservations. `NonCopyable` is only copyable because of the existence of the `copy_non_copyable` function, otherwise it is not.

### General Overview of Struct Abilities

Struct definitions allow for a handful of abilities or behavior modifiers to be attributed to the struct. The general format is `struct Foo has ability0, ability1, ability3 { ... }`

There are 4 struct abilities in Move:

- Copy — enables implicit copy of a struct without calling into the module. A struct that has copy can be duplicated.
- Drop — a struct can be destroyed implicitly, there is no need to explicitly call a deconstructor or delete the struct.
- Key — the struct can be stored directly in global storage.
- Store — the struct can be stored indirectly in global storage.

While these terms may seem foreign at this point, they are covered in greater depth in their respective sections.

### To Return References or Values

An acute observer may notice that we return a copy of the `String` when reading the `name` or `gender` attributes. `String` supports the `copy` ability and can be implicitly copied. In the above code, we could have also returned a `&String` as well, saving the VM from copying the `String` bytes. However, as we begin our exploration of storage, such semantics are currently not supported, hence we default to return by copying the value here.

## Mutating Data

Now that we have the ability to build characters, let’s get them fighting each other, taking damage, dying, and getting level ups.

```rust
/// Path: fight_the_baddies/introducing-move/sources/character.move
module baddies::character {
...
friend fight_the_baddies::level_up;
friend fight_the_baddies::fight;

public(friend) fun set_level(character: &mut Character, level: u8) {
  character.level = level
}

public(friend) fun set_health(character: &mut Character, health: u32) {
  character.health = health
}

public(friend) fun set_defense(character: &mut Character, defense: u16) {
  character.defense = defense
}

public(friend) fun set_strength(character: &mut Character, strength: u16) {
  character.strength = strength
}

public(friend) fun set_agility(character: &mut Character, agility: u16) {
  character.agility = agility
}

public(friend) fun set_experience(character: &mut Character, experience: u64) {
  character.experience = experience
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
  } = character;
}
```

In this extension of `character.move` , we introduce several new concepts:

### Module Accessibility via friend

The notation `friend baddies::level_ups`, means that the module `baddies::level_ups` can call into any `public(friend)` function within this module. Only modules that have `friend` can access `public(friend)` and those modules must be defined at the same address and within the same package.

`friend` enables us to both reduce the code size per file and locate common logic, among many other benefits.

### Mutable Struct References

Much like Rust, in Move, a data structure can only be mutated or changed if there’s a mutable handle to it or own it, such as `& mut Character` or `Character`. 

### Destruction and Dropping

In Move, structs can not be implicitly deleted unless properly permissioned, `Character` has no such permission, so an explicit `destroy` function exists for the application to clean up characters that are no longer needed. Sometimes, structs benefit from implicit deletion, this can be done by using the `drop` ability on the struct:

```rust
struct Dropable has drop { }
public fun drop_me(droppable: Droppable) {}
```

This compiles without issue, whereas if `Droppable` did not have `drop` , compilation would fail.

## Data Access Across Modules

```rust
/// Path: fight_the_baddies/introducing-move/sources/fight.move
module fight_the_baddies::fight {
use std::error;
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

public fun fight(hero: &mut Character, baddy: &mut Character) {
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

fun attack(left: &mut Character, right: &mut Character) {
  let left_str = character::strength(left);
  let right_def = character::defense(right);

  // Avoid the potential underflow and set the minimum damage to 1.
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

public fun sacrifice(character: Character) {
  character::destroy(character);
}

public fun eliminate(character: Character) {
  assert!(character::health(&character) == 0, error::invalid_argument(ENOT_DEAD));
  character::destroy(character);
}
}
```

### Constants

Move supports constants for creating easily identifiable values and avoiding magic numbers within code. Currently constants can only be used within the module that define it, so if you need to expose a constant across the module boundaries, export it as a function.

### Conditional Evaluation

Like Rust, Move allows a value to be set as the output of conditional evaluation. That is, one can call `let value = if (something) value_a else value_b` . This paradigm allows for developers to avoid creating unnecessary mutable values.

### Assertions and Errors

As we will see in later sections, Move’s default error messages can leave the developer confused about where the issue was. To that end, it is quite natural to add many invariants to the code. In the above code, the invariants provide more protection against users leveraging the functions incorrectly.

All the Move error types are defined in [error.move](https://aptos.dev/reference/move/?branch=mainnet&page=move-stdlib/doc/error.md#0x1_error). The comments above errors provide value to applications. This is returned during run-time if a assertion is violated.

Later on, we will demonstrate other mechanisms that provide better type-safety to limit the need for assertions in code.

### Integer Underflow and Overflow

In Move, if you ever have an integer, even temporarily have a value outside its range, then the application will immediately abort. This includes even complex operations that might ultimately result in a viable number. For example, the following will both underflow:

`5 - 8`

`5 - 8 + 5`

So it is imperative to check conditions around these numbers. Move currently lacks signed integers, so there is no way to temporarily cast the numbers to evade bounds checking.

### Casting

Move allows for casting between different types; however, it must be done explicitly. In the above snippet, `let damage = (damage as u32);` causes there to be a new variable named damage that is a `u32` whereas the original damage was a `u16`. If this was not done, the code would fail to compile due to an implicit type coercion of `u16` to a `u32`. It is important to note that the Move compiler is rather rigid in the syntax for casting `let damage = damage as u32;` fails to compile.

### Explicit Destruction of Structs

As mentioned earlier, `Character` could have had the ability `drop`, that would have let users implicitly delete the structs by letting them leave scope once they are no longer useful. Of course, in this game, we want to keep all characters accounted for and thus have an explicit destructor: `characer::destroy` . That function still requires a layer of indirection as it can only be called locally or via other `friend` modules. For this purpose, the `fight` module offers two functions: `eliminate` that destroys a `Character` with zero health and `sacrifice` that destroys any `Character`. At this point in the game play, they have limited special logic, but over time we can add more complexity.

This pattern is known as a **hot potato**, one can pass around a struct but must explicitly destroy it via another well-defined endpoint.

We’ll see this code leveraged in the test code presented in a bit.

## Rounding out the Basic Game Experience

In a traditional RPG game, as the character gains levels, the amount of experience gained from a victorious fight decreases based upon the level of the character receiving the experience. Alternatively, the amount of experience required per-level increases non-linearly. For this model, we chose a simple bit shifting model.

```rust
/// Path: fight_the_baddies/introducing-move/sources/level_up.move
module fight_the_baddies::level_up {
use std::error;
use fight_the_baddies::character::{Self, Character};

friend fight_the_baddies::fight;

/// Losing character is not dead.
const ENOT_DEAD: u64 = 1;
/// Winning character is dead.
const EDEAD: u64 = 2;

const BASE_EXP: u64 = 25;
const MAX_LEVEL: u64 = 255;
const MAX_EXP: u64 = 25500;

/// Provide experience and level increase. The algorithm is that each level requires
/// 100 experience points. Each victory for the same level results in 25 experience
/// points. That amount is shifted left for each level greater and right for each
/// level less than.
public(friend) fun level_up(winner: &mut Character, loser: &mut Character) {
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
```

At this point, the above code should introduce no new concepts, but simply provides completeness to the application thus far. Note, the current level up scheme leaves out changes to attributes, we will return to that in a later section, when we introduce randomness.

## Building the Code

Earlier, we set up the Aptos CLI, as a reminder, you can follow this guide to [install the Aptos CLI](../../tools/install-cli/).

Now let’s examine some test code that proves the code written thus far actually works!

First we can verify that the code compiles by executing:

```bash
aptos move compile \
  --package-dir fight_the_baddies \
  --named-addresses fight_the_baddies=0xf00ba5
```

This assumes that the package `fight_the_baddies` with the `Move.toml` and `sources` is located in the current directory. Note, `fight_the_baddies` was set to `"_"` in the `Move.toml` , so to build, we must supply a named address. We have chosen the address `0xf00ba5` at random.

## End to End Tests

Now that we have a working CLI and code that can be built, it is time to build a tests. First the test code:

```rust
/// Path: fight_the_baddies/introducing-move/tests/end_to_end.move
#[test_only]
module fight_the_baddies::end_to_end {
use std::string;
use fight_the_baddies::character::{Self, Character};
use fight_the_baddies::fight;
use fight_the_baddies::test_utils;

#[test]
fun generate_and_destroy_hero() {
  let hero = test_utils::get_hero();
  assert!(character::is_hero(&hero), 0);
  assert!(character::name(&hero) == string::utf8(b"Alice"), 1);
  assert!(character::gender(&hero) == string::utf8(b"female"), 2);
  fight::sacrifice(hero);
}

#[test]
fun generate_and_destroy_baddy() {
  let baddy = test_utils::get_baddy();
  assert!(character::is_baddy(&baddy), 0);
  assert!(character::name(&baddy) == string::utf8(b"Bob"), 1);
  assert!(character::gender(&baddy) == string::utf8(b"male"), 2);
  fight::sacrifice(baddy);
}

#[test]
/// The goal with this test is to get level ups, so we'll create a hero who always kill steals
/// without taking damage. Then we'll create new heroes along the way to beat the baddy to near
/// death.
fun end_to_end() {
  let main_hero = test_utils::get_hero();
  let current_level = character::level(&main_hero);

  while (current_level == character::level(&main_hero)) {
    let sad_hero = test_utils::get_hero();
    let baddy = test_utils::get_baddy();
    let main_str = character::strength(&main_hero);
    let baddy_def = character::defense(&baddy);

    while (character::health(&baddy) > 0) {
      let baddy_health = character::health(&baddy);
      if (baddy_health + (baddy_def as u32) <= (main_str as u32)) {
          fight::fight(&mut main_hero, &mut baddy)
      } else {
          fight::fight(&mut sad_hero, &mut baddy)
      };
    };
    fight::sacrifice(sad_hero);
    fight::eliminate(baddy);
  };

  // We should only get one level up given the current mechanics
  assert!(current_level + 1 == character::level(&main_hero), 0);
  fight::sacrifice(main_hero);
  // The test will timeout if it cannot get here.
}
}
```

```rust
/// Path: fight_the_baddies/introducing-move/tests/test_utils.move
#[test_only]
module fight_the_baddies::test_utils {
use std::string;
use fight_the_baddies::character::{Self, Character};
public fun get_baddy(): Character {
  character::generate_baddy(
    string::utf8(b"Bob"),
    string::utf8(b"male"),
  )
}
public fun get_hero(): Character {
  character::generate_hero(
    string::utf8(b"Alice"),
    string::utf8(b"female"),
  )
}
}
```

### Running Tests
In order to run the tests, execute the following command:

```bash
aptos move test \
  --package-dir fight_the_baddies \
  --named-addresses fight_the_baddies=0xf00ba5
```

This is very similar to the compile command, but it searches the `sources` and `tests` diretory for any functions labeled `#[test]` it also enables compiling all code marked as `#[test_only]`. As demonstrated above, those components allow us to mix production code with test code. This also means we can have our test code next to our production code without worrying whether it will be accessible to production deployments. Anything with `#[test]` or `#[test_only]` will not be compiled in production mode.

### About the Tests

The tests above are pretty simple. It uses a bit of knowledge of the game mechanics to ensure that a hero will defeat a series of baddies to get a level up. Of course, the game currently doesn’t have the mechanism to add life back to the hero, so we are left using `sad_hero` to do all the dirty work, so that `main_hero` can claim the kill.

### Leveraging the Explicit Destroy

In this test, we create several heroes, `sad_hero` that sacrifice their health to basically kill the `baddy` before the `main_hero` swoops in for the kill without taking any damage. Because of the lack of `drop` on `Character` . We explicitly call into `character::eliminate` for `baddy` , as it is dead. We also explicitly call into `character::sacrifice` for `sad_hero` , since it is most likely still alive.

## Conclusion

By the end of this section, we have built a trivial RPG. We can create new characters, level them up, and destroy them. We have a fully functioning yet not persistent Move program. But who wants to play an RPG that can only persist the duration of a single call? In the next section, we’ll understand how persistent storage works in Move and Aptos Move.

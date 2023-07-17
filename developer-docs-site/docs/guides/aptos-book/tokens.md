## Move Objects

The previous section introduced global storage and two mechanisms for placing structs into storage: `key` and `store`. While both approaches work and are intuitive, they leave a lot of components up to the developer to implement. Aptos introduces Move Objects to extend upon the basics of the Move model and offer a framework for creating and managing a globally accessible set of heterogeneous resources that can expedite and improve the developer experience.

Why objects? Well minimally an object gets you access to many cool features including:

- A common framework that defines ownership of an asset. No need to have one-off asset management as was demonstrated for hero in the previous chapter, nor come up with mechanisms to differentiate between similar yet different assets, e.g.., those that share the same common name.
- Global addressability of all objects that only requires virtual or referenced nesting of assets. Weapons were only accessible if you knew which hero had them and further nesting of assets make it much more complex to manage both in terms of code and data model.
- Events or metadata that make it easier to reason about changes to the object. An area not yet touched upon, Move Objects not only represent an asset but can express events as actions are taken involving the objects, making it easier for off-chain understanding of complex on-chain interactions.
- A permission framework that makes data convenient and safe to use. Limiting the possibility that `signer`s could be abused to manipulate or affect data.

We’ll go into each one of these in greater depth and further emphasize the utility of Objects over resources alone.

### Characters as Objects

For the most part, swapping from resources to objects is largely a set of API changes, but doing so unlocks a lot of opportunities.

```rust
/// Path: fight_the_baddies/sources/character.move
module fight_the_baddies::character {
use std::error;
use std::option::{Self, Option};
use std::string::{Self, String};

use aptos_framework::object::{Self, Object};

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
    weapon: option::none(),
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
}
```

************************************What is a Move Object?************************************

An object an Aptos is represented by a set of resources including the `ObjectCore` resource stored at an address within global storage. Objects offer interoperability by defining a core set of primitives via the `ObjectCore` resource. `ObjectCore` defines several basic properties of the object including who the owner is, whether it can be transferred and events notifying transferring. It also allows for users to add events that are sourced from the object.

********************************Creating Objects********************************

### Creating Objects

### Resource Groups

### Weapon Objects

```rust
/// Path: fight_the_baddies/sources/weapon.move
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

public fun create_sword(creator: &signer, name: String): Object<Weapon> {
  let weapon = Weapon {
    name,
    type: string::utf8(b"sword"),
    strength: 10,
    weight: 4,
  };
  create_weapon(creator, weapon)
}

public fun create_axe(creator: &signer, name: String): Object<Weapon> {
  let weapon = Weapon {
    name,
    type: string::utf8(b"axe"),
    strength: 17,
    weight: 6,
  };
  create_weapon(creator, weapon)
}

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

## Enter Aptos

Before pressing forward in our fight against the baddies, let’s explore other aspects of the Aptos to better understand how it can be used extend our gaming mechanics both on and off-chain. By the end of this chapter, you will understand:

- How to run and interact with a local Aptos testnet
- The on-chain Aptos account
- How to emit events on-chain and read them off-chain
- Setup a shell script for end-to-end tests

### Running a Local Testnet

To begin, please [install the Aptos CLI](https://aptos.dev/tools/install-cli/), first. Note, a lot of this content can be found in [Aptos.dev site](https://aptos.dev/nodes/local-testnet/using-cli-to-run-a-local-testnet).

****************************Prepare a User****************************

As most of our interaction will be with the local testnet, we’ll create two identities

```bash
# Create alice's profile
aptos init \
  --profile alice \
  --rest-url http://localhost:8080 \
  --faucet-url http://localhost:8081

# Create bob's profile
aptos init \
  --profile alice \
  --rest-url http://localhost:8080 \
  --faucet-url http://localhost:8081
```

The addresses and private keys will persist for all future testnets, so we will not need to call init again. Instead we will always start with a fresh testnet.

****************************************Starting the Testnet****************************************

To start a local testnet, execute the following command:

```bash
aptos node run-local-testnet --with-faucet --force-restart --assume-yes
```

This guide always assumes a pristine state, hence the inclusion of `--force-restart` 

The output from the command should be similar to the following:

```bash
Completed generating configuration:
        Log file: "/home/davidiw/aptos/aptos-core/.aptos/testnet/validator.log"
        Test dir: "/home/davidiw/aptos/aptos-core/.aptos/testnet"
        Aptos root key path: "/home/davidiw/aptos/aptos-core/.aptos/testnet/mint.key"
        Waypoint: 0:e881e7134588985689c47a8c5c6a15dd4d95f72e5e68ec6246a2f3a6d65ddc45
        ChainId: testing
        REST API endpoint: http://0.0.0.0:8080
        Metrics endpoint: http://0.0.0.0:9101/metrics
        Aptosnet fullnode network endpoint: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit

Faucet is running. Faucet endpoint: http://0.0.0.0:8081
```

**************************************Creating and Funding the Account**************************************

As we already have profiles for Alice and Bob, we can execute the following commands to get them some useful test coins:

```bash
aptos account fund-with-faucet --profile alice --account alice
aptos account fund-with-faucet --profile bob --account bob
```

### The Aptos Account

Within this book, we will leverage some of the fields and functions associated with the Aptos Account and so we provide some details to help build context on nuances of Aptos.

When we create and fund an account, it causes the Move Aptos run-time to actually create [accounts on-chain](https://aptos.dev/concepts/accounts) and store some testnet Apt coin with those accounts.

An account on Aptos is a collection of metadata that describes ownership of an account and a set of assets managed by that account. In terms of the technical aspects, there’s a 1:1 mapping between address and account. An account is a collection of resources stored at a specific address. Furthermore, the Aptos [`Account`](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/account.md#0x1_account) structure embeds relevant metadata like the account sequence number, authentication key, and a globally unique identifier generator.

When funding an account, that creates a second resource, the `[CoinStore](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/coin.md#0x1_coin)` for `[AptosCoin](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/aptos_coin.md#0x1_aptos_coin)` . This is the utility token for the Aptos blockchain and is consumed as gas for each transaction submitted.

### Understanding Transaction Output

The most basic transaction on the Aptos blockchain is `0x1::coin::transfer` , this represents transfer of a specified amount of `AptosCoin` from the account sending the transaction to the account addressed in the parameters.

In the following output, the account `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b` submitted and executed a transaction that called the following function

```bash
{
  "version": "13629679",
  "gas_used": "4",
  "success": true,
  "vm_status": "Executed successfully",
  "changes": [
    {
      "address": "0xb258b91eee04111039320a85b0c24a2dd433909e14a6b5c32ee722e0fdecfddc",
      "data": {
        "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
        "data": {
          "coin": {
            "value": "1000"
          },
          "deposit_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
                "creaton_num": "2",
              }
            }
          },
          ...
        }
      },
      "type": "write_resource"
    },
    ...
  ],
  "sender": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
  "sequence_number": "0",
  "max_gas_amount": "2000",
  "gas_unit_price": "1",
  "expiration_timestamp_secs": "1660616127",
  "payload": {
    "function": "0x1::coin::transfer",
    "type_arguments": [
      "0x1::aptos_coin::AptosCoin"
    ],
    "arguments": [
      "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      "1000"
    ],
    "type": "entry_function_payload"
  },
  "events": [
    {
      "key": "0x0300000000000000810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
      "guid": {
        "id": {
          "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
          "creation_num": "3"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::WithdrawEvent",
      "data": {
        "amount": "1000"
      }
    },
    {
      "key": "0x02000000000000005098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      guid": {
        "id": {
          "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
          "creation_num": "2"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::DepositEvent",
      "data": {
        "amount": "1000"
      }
    }
  ],
  "timestamp": "1660615531147935",
  "type": "user_transaction"
}
```

### Emitting Events On-Chain and Reading Them Off-Chain

Storage efficiency is key to a blockchain as it scales. The way in which transactions store their output to the blockchain can be impacted as a result thereof, impeding readability as a result. In fact, even updating a small chunk of data can result in a relatively large update if that data lives within a much larger struct. As a result, determining the specific effects of a transaction can be very difficult.

Aptos supports events in Move that allow developers to indicate state changes. Events are represented as a Move struct and stored as part of the transaction output. Events must be emitted from event handles, wherein each handle allocates a unique, sequential sequence number for each event. Thus events are fully countable. The only caveat with events is that their contents are driven by the developer, ideally they accurately represent the changes in storage, but they might also not. It is imperative to verify the authenticity of events both when processing them and prior to using them in production.

### A Simple Approach to End-to-End Tests

Aptos accounts, events, transactions, and running a local blockchain for end-to-end testing.

## Aptos Objects

Objects, extensions, and value of global storage versus store

## Aptos Tokens

Unification on a global type for interoperability, marketplaces, swaps

## On the Aptos Blockchain

Understanding game state, users, indexers, and transactions

## Advanced Concepts

Randomness

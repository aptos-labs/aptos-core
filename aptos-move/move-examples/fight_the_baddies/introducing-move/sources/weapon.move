module fight_the_baddies::weapon {
use std::string::{Self, String};

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
    strength: 5,
    weight: 4,
  }
}

public fun generate_axe(name: String): Weapon {
  Weapon {
    name,
    type: string::utf8(b"axe"),
    strength: 7,
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

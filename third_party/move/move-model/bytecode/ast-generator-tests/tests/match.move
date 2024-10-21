module 0x815::m {
    enum Entity has drop {
        Person { id: u64 },
        Institution { id: u64, admin: u64 }
    }

    fun id(self: &Entity): u64 {
        match (self) {
            Person{id} => *id,
            Institution{id, ..} => *id
        }
    }

    fun id2(self: Entity): u64 {
        match (self) {
            Person{id} if id > 0 => id,
            Institution{id, ..} => id,
            _ => 0
        }
    }
}

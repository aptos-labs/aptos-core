module 0x815::m {
    enum Entity {
        Person { id: u64 },
        Institution { id: u64, admin: u64 }
    }

    fun id2(self: &Entity): u64 {
        match (self) {
            Person{id} if *id > 0 => *id,
            Institution{id, ..} => *id,
            _ => 0
        }
    }
}

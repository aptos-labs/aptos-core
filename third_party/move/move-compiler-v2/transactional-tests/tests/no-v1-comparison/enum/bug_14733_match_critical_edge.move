// This source tests various scenarios of matches which introduce critical edges, which
// are eliminated by the split-critical-edge-processor. This kind of matches
// are also the only kind of code which can introduce those edges, so we are also
// testing functionality of the processor here.

//# publish
module 0x815::m {
    enum Entity has drop {
        Person { id: u64 },
        Institution { id: u64, admin: Admin }
    }

    enum Admin has drop {
        Superuser,
        User(u64)
    }

    // Ensure function is inlined so we create nested matches in the next one.
    inline fun id(self: &Entity): u64 {
        match (self) {
            Person{id} if *id > 10 => *id,
            Institution{id, ..} => *id,
            _ => 0
        }
    }

    fun admin_id(self: &Entity): u64 {
        match (self) {
            Institution{admin: Admin::Superuser, ..} => 1 + self.id(),
            Institution{admin: Admin::User(id), ..} if *id > 10 => *id + self.id(),
            Institution{admin: Admin::User(id), ..} if  *id <= 10 => self.id() + 5,
            _ => self.id()

        }
    }

    fun test1(): u64 {
        let e = Entity::Person{id: 22};
        e.id()
    }

    fun test2(): u64 {
        let e = Entity::Person{id: 10};
        e.id()
    }

    fun test3(): u64 {
        let e = Entity::Institution { id: 10, admin: Admin::Superuser };
        e.id()
    }

    fun test4(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::Superuser};
        e.admin_id()
    }

    fun test5(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::User(23)};
        e.admin_id()
    }

    fun test6(): u64 {
        let e = Entity::Institution{id: 20, admin: Admin::User(8)};
        e.admin_id()
    }

    fun test7(): u64 {
        let e = Entity::Person{id: 15};
        e.admin_id()
    }
}

//# run 0x815::m::test1

//# run 0x815::m::test2

//# run 0x815::m::test3

//# run 0x815::m::test4

//# run 0x815::m::test5

//# run 0x815::m::test6

//# run 0x815::m::test7
